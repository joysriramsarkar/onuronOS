// android/agent/src/main.rs — NilAgent: Android container IPC bridge
//
// Runs inside the LXC/Waydroid Android container and provides a Unix socket
// at /dev/socket/nilagent that the host NilOS can connect to.
//
// Protocol (newline-delimited JSON):
//   Host → Container:
//     { "action": "START_ACTIVITY", "package": "org.mozilla.fenix", "activity": ".App", "extras": { ... } }
//     { "action": "SEND_BROADCAST",  "intent": "android.intent.action.SEND", "extras": { ... } }
//     { "action": "GET_INSTALLED",   }
//     { "action": "STOP_APP",        "package": "..." }
//     { "action": "PING" }
//
//   Container → Host:
//     { "status": "OK",   "result": { ... } }
//     { "status": "ERR",  "message": "..." }

#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use serde::{Deserialize, Serialize};

#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(unix)]
use tokio::net::UnixListener;

#[cfg(unix)]
const SOCKET_PATH: &str = "/dev/socket/nilagent";
#[cfg(unix)]
const INSTALLED_APPS_DB: &str = "/data/data";

// ── Wire types ────────────────────────────────────────────────────────────────

#[cfg(unix)]
#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "SCREAMING_SNAKE_CASE")]
enum HostCommand {
    Ping,
    StartActivity {
        package: String,
        #[serde(default)]
        activity: String,
        #[serde(default)]
        extras: serde_json::Value,
    },
    SendBroadcast {
        intent: String,
        #[serde(default)]
        extras: serde_json::Value,
    },
    GetInstalled,
    StopApp {
        package: String,
    },
}

#[cfg(unix)]
#[derive(Debug, Serialize)]
struct AgentResponse {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[cfg(unix)]
impl AgentResponse {
    fn ok(result: serde_json::Value) -> Self {
        Self { status: "OK", result: Some(result), message: None }
    }
    fn err(msg: impl Into<String>) -> Self {
        Self { status: "ERR", result: None, message: Some(msg.into()) }
    }
}

// ── Intent dispatching ────────────────────────────────────────────────────────

/// Launch an Activity via `am start`.
#[cfg(unix)]
async fn start_activity(package: &str, activity: &str, extras: &serde_json::Value) -> AgentResponse {
    // Build `am start -n pkg/activity` invocation
    let component = if activity.is_empty() {
        package.to_string()
    } else {
        format!("{package}/{activity}")
    };

    let mut cmd = tokio::process::Command::new("am");
    cmd.args(["start", "-n", &component]);

    // Flatten extras as --es / --ez / --ei flags
    if let Some(obj) = extras.as_object() {
        for (k, v) in obj {
            match v {
                serde_json::Value::String(s) => { cmd.args(["--es", k, s]); }
                serde_json::Value::Bool(b)   => { cmd.args(["--ez", k, if *b { "true" } else { "false" }]); }
                serde_json::Value::Number(n) => { cmd.args(["--ei", k, &n.to_string()]); }
                _ => {}
            }
        }
    }

    match cmd.output().await {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if out.status.success() {
                println!("[nilagent] START_ACTIVITY {component}: OK");
                AgentResponse::ok(serde_json::json!({ "component": component, "am_output": stdout.trim() }))
            } else {
                let msg = format!("am start failed: {stderr}");
                println!("[nilagent] START_ACTIVITY {component}: FAIL — {msg}");
                AgentResponse::err(msg)
            }
        }
        Err(e) => AgentResponse::err(format!("exec am: {e}")),
    }
}

/// Send a broadcast intent via `am broadcast`.
#[cfg(unix)]
async fn send_broadcast(intent: &str, extras: &serde_json::Value) -> AgentResponse {
    let mut cmd = tokio::process::Command::new("am");
    cmd.args(["broadcast", "-a", intent]);

    if let Some(obj) = extras.as_object() {
        for (k, v) in obj {
            if let serde_json::Value::String(s) = v {
                cmd.args(["--es", k, s]);
            }
        }
    }

    match cmd.output().await {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            println!("[nilagent] BROADCAST {intent}: OK");
            AgentResponse::ok(serde_json::json!({ "intent": intent, "result": stdout.trim() }))
        }
        Ok(out) => AgentResponse::err(String::from_utf8_lossy(&out.stderr).into_owned()),
        Err(e)  => AgentResponse::err(format!("exec am: {e}")),
    }
}

/// List installed packages by reading /data/data directory entries.
#[cfg(unix)]
async fn get_installed() -> AgentResponse {
    let mut packages: Vec<String> = Vec::new();

    // Primary: pm list packages
    if let Ok(out) = tokio::process::Command::new("pm")
        .args(["list", "packages", "-3"])   // -3 = third-party only
        .output()
        .await
    {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            if let Some(pkg) = line.strip_prefix("package:") {
                packages.push(pkg.trim().to_string());
            }
        }
    }

    // Fallback: enumerate /data/data
    if packages.is_empty() {
        if let Ok(mut rd) = tokio::fs::read_dir(INSTALLED_APPS_DB).await {
            while let Ok(Some(entry)) = rd.next_entry().await {
                packages.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }

    println!("[nilagent] GET_INSTALLED → {} packages", packages.len());
    AgentResponse::ok(serde_json::json!({ "packages": packages }))
}

/// Force-stop a package via `am force-stop`.
#[cfg(unix)]
async fn stop_app(package: &str) -> AgentResponse {
    match tokio::process::Command::new("am")
        .args(["force-stop", package])
        .output()
        .await
    {
        Ok(out) if out.status.success() => {
            println!("[nilagent] STOP_APP {package}: OK");
            AgentResponse::ok(serde_json::json!({ "package": package }))
        }
        Ok(out) => AgentResponse::err(String::from_utf8_lossy(&out.stderr).into_owned()),
        Err(e)  => AgentResponse::err(format!("exec am: {e}")),
    }
}

// ── Connection handler ────────────────────────────────────────────────────────

#[cfg(unix)]
async fn handle_connection(stream: tokio::net::UnixStream) {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim().to_string();
        if line.is_empty() { continue; }

        println!("[nilagent] ← {line}");

        let response: AgentResponse = match serde_json::from_str::<HostCommand>(&line) {
            Err(e) => AgentResponse::err(format!("JSON parse error: {e}")),
            Ok(cmd) => match cmd {
                HostCommand::Ping => AgentResponse::ok(serde_json::json!({ "pong": true })),
                HostCommand::StartActivity { package, activity, extras } =>
                    start_activity(&package, &activity, &extras).await,
                HostCommand::SendBroadcast { intent, extras } =>
                    send_broadcast(&intent, &extras).await,
                HostCommand::GetInstalled =>
                    get_installed().await,
                HostCommand::StopApp { package } =>
                    stop_app(&package).await,
            },
        };

        let mut resp_line = serde_json::to_string(&response).unwrap_or_default();
        resp_line.push('\n');
        println!("[nilagent] → {resp_line}");
        if writer.write_all(resp_line.as_bytes()).await.is_err() {
            break;
        }
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("[nilagent] NilOS Android Container Agent starting...");

    #[cfg(unix)]
    {
        if let Some(dir) = Path::new(SOCKET_PATH).parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::remove_file(SOCKET_PATH);

        let listener = match UnixListener::bind(SOCKET_PATH) {
            Ok(l) => {
                println!("[nilagent] Listening on {SOCKET_PATH}");
                l
            }
            Err(e) => {
                eprintln!("[nilagent] FATAL: bind {SOCKET_PATH}: {e}");
                std::process::exit(1);
            }
        };

        loop {
            match listener.accept().await {
                Ok((stream, _)) => { tokio::spawn(handle_connection(stream)); }
                Err(e) => { eprintln!("[nilagent] accept error: {e}"); }
            }
        }
    }

    #[cfg(not(unix))]
    println!("[nilagent] Not running on Unix — Android agent is a Linux-only binary.");
}
