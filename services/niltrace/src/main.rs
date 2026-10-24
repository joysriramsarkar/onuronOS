// services/niltrace/src/main.rs — Kernel ftrace to Chrome JSON Profiler (niltrace)
// Interacts with Linux ftrace (/sys/kernel/tracing) and userspace trace_marker,
// converts kernel scheduling, irq, and UI markers into Chrome/Perfetto Trace Event JSON format.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const TRACE_SOCK_PATH: &str = "/run/nilos/trace.sock";
pub const TRACE_OUTPUT_DIR: &str = "/data/perf";
pub const SYSFS_TRACING_PRIMARY: &str = "/sys/kernel/tracing";
pub const SYSFS_TRACING_DEBUG: &str = "/sys/kernel/debug/tracing";

/// Chrome Trace Event format (Perfetto / about:tracing compatible)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ChromeTraceEvent {
    pub name: String,
    pub cat: String,
    pub ph: String, // "B" (begin), "E" (end), "X" (complete), "i" (instant), "C" (counter)
    pub ts: f64,    // timestamp in microseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dur: Option<f64>, // duration in microseconds (for "X")
    pub pid: u32,
    pub tid: u32,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub args: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChromeTraceDocument {
    #[serde(rename = "traceEvents")]
    pub trace_events: Vec<ChromeTraceEvent>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TraceCommand {
    pub cmd: String, // "start", "stop", "status", "mark"
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub marker: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TraceStatusResponse {
    pub active: bool,
    pub buffer_size_kb: u64,
    pub recorded_events: usize,
    pub tracing_path: String,
    pub last_trace_file: Option<String>,
}

/// Discovers the active ftrace mount point
pub fn find_ftrace_path() -> Option<PathBuf> {
    let primary = Path::new(SYSFS_TRACING_PRIMARY);
    if primary.exists() && primary.join("tracing_on").exists() {
        return Some(primary.to_path_buf());
    }
    let debug = Path::new(SYSFS_TRACING_DEBUG);
    if debug.exists() && debug.join("tracing_on").exists() {
        return Some(debug.to_path_buf());
    }
    None
}

/// Enables or disables kernel ftrace recording
pub fn set_tracing_enabled(trace_dir: &Path, enabled: bool) -> std::io::Result<()> {
    let flag_path = trace_dir.join("tracing_on");
    let val = if enabled { "1\n" } else { "0\n" };
    fs::write(flag_path, val)
}

/// Clears the kernel trace ring buffer
pub fn clear_trace_buffer(trace_dir: &Path) -> std::io::Result<()> {
    let buf_path = trace_dir.join("trace");
    fs::write(buf_path, "")
}

/// Enables a specific event subsystem (e.g. "sched", "irq", "power")
pub fn enable_trace_event(trace_dir: &Path, category: &str, event: &str, enable: bool) -> std::io::Result<()> {
    let event_path = trace_dir.join("events").join(category).join(event).join("enable");
    if event_path.exists() {
        let val = if enable { "1\n" } else { "0\n" };
        fs::write(event_path, val)?;
    }
    Ok(())
}

/// Writes a userspace trace marker to /sys/kernel/tracing/trace_marker
pub fn write_trace_marker(trace_dir: &Path, tag: &str) -> std::io::Result<()> {
    let marker_path = trace_dir.join("trace_marker");
    if marker_path.exists() {
        let mut file = OpenOptions::new().write(true).open(marker_path)?;
        file.write_all(tag.as_bytes())?;
        file.write_all(b"\n")?;
    }
    Ok(())
}

/// Parses a single ftrace raw log line into a ChromeTraceEvent.
/// Handles both standard kernel sched/irq logs and Android/Onuron Atrace/Ftrace markers.
/// Example line 1:
///   nilshell-423   [001] d... 1234.567890: sched_switch: prev_comm=swapper/0 prev_pid=0 ==> next_comm=nilshell next_pid=423
/// Example line 2:
///   nilui-1042     [002] .... 1234.600000: tracing_mark_write: B|1042|OnuronFrameDraw
/// Example line 3:
///   nilui-1042     [002] .... 1234.605000: tracing_mark_write: E|1042
pub fn parse_ftrace_line(line: &str) -> Option<ChromeTraceEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    // Split at colon separating header from message
    // e.g.: "nilshell-423 [001] .... 1234.567890: sched_switch: ..."
    let parts: Vec<&str> = trimmed.splitn(2, ": ").collect();
    if parts.len() < 2 {
        return None;
    }

    let header = parts[0];
    let payload = parts[1];

    // Parse header tokens: [comm-pid, [cpu], flags?, timestamp]
    let header_tokens: Vec<&str> = header.split_whitespace().collect();
    if header_tokens.len() < 3 {
        return None;
    }

    // Extract comm and pid from first token, e.g. "nilshell-423" or "<idle>-0"
    let comm_pid = header_tokens[0];
    let (comm, pid) = if let Some(last_dash) = comm_pid.rfind('-') {
        let name = &comm_pid[..last_dash];
        let p: u32 = comm_pid[last_dash + 1..].parse().unwrap_or(0);
        (name.to_string(), p)
    } else {
        (comm_pid.to_string(), 0)
    };

    // The timestamp is usually the last token of the header
    let ts_str = header_tokens.last()?;
    let ts_sec: f64 = ts_str.parse().ok()?;
    let ts_us = ts_sec * 1_000_000.0;

    // Check if this is a tracing_mark_write (atrace/userspace marker)
    if payload.starts_with("tracing_mark_write:") || payload.starts_with("B|") || payload.starts_with("E|") || payload.starts_with("C|") {
        let marker_content = if let Some(stripped) = payload.strip_prefix("tracing_mark_write:") {
            stripped.trim()
        } else {
            payload
        };

        return parse_atrace_marker(marker_content, ts_us, pid);
    }

    // Kernel event parsing (sched_switch, irq, etc.)
    let mut args = HashMap::new();
    args.insert("comm".to_string(), comm);

    // Extract event name (up to next colon or space)
    let event_name = if let Some(idx) = payload.find(':') {
        let en = &payload[..idx].trim();
        let kv_part = &payload[idx + 1..].trim();
        for kv in kv_part.split_whitespace() {
            if let Some(eq_idx) = kv.find('=') {
                let k = &kv[..eq_idx];
                let v = &kv[eq_idx + 1..];
                args.insert(k.to_string(), v.to_string());
            }
        }
        en.to_string()
    } else {
        payload.split_whitespace().next().unwrap_or("kernel_event").to_string()
    };

    let cat = if event_name.starts_with("sched") {
        "sched"
    } else if event_name.starts_with("irq") {
        "irq"
    } else if event_name.starts_with("power") || event_name.starts_with("cpu") {
        "power"
    } else {
        "kernel"
    };

    Some(ChromeTraceEvent {
        name: event_name,
        cat: cat.to_string(),
        ph: "i".to_string(), // instant event
        ts: ts_us,
        dur: None,
        pid,
        tid: pid,
        args,
    })
}

/// Parses Atrace-style userspace markers:
/// B|<pid>|<name> -> Phase "B" (Begin)
/// E|<pid>        -> Phase "E" (End)
/// C|<pid>|<counter_name>|<val> -> Phase "C" (Counter)
pub fn parse_atrace_marker(marker: &str, ts_us: f64, default_pid: u32) -> Option<ChromeTraceEvent> {
    let tokens: Vec<&str> = marker.split('|').collect();
    if tokens.is_empty() {
        return None;
    }

    match tokens[0] {
        "B" => {
            let pid = tokens.get(1).and_then(|p| p.parse().ok()).unwrap_or(default_pid);
            let name = tokens.get(2).unwrap_or(&"slice").to_string();
            Some(ChromeTraceEvent {
                name,
                cat: "userspace".to_string(),
                ph: "B".to_string(),
                ts: ts_us,
                dur: None,
                pid,
                tid: pid,
                args: HashMap::new(),
            })
        }
        "E" => {
            let pid = tokens.get(1).and_then(|p| p.parse().ok()).unwrap_or(default_pid);
            Some(ChromeTraceEvent {
                name: "".to_string(),
                cat: "userspace".to_string(),
                ph: "E".to_string(),
                ts: ts_us,
                dur: None,
                pid,
                tid: pid,
                args: HashMap::new(),
            })
        }
        "C" => {
            let pid = tokens.get(1).and_then(|p| p.parse().ok()).unwrap_or(default_pid);
            let counter_name = tokens.get(2).unwrap_or(&"counter").to_string();
            let val = tokens.get(3).unwrap_or(&"0").to_string();
            let mut args = HashMap::new();
            args.insert("value".to_string(), val);
            Some(ChromeTraceEvent {
                name: counter_name,
                cat: "counter".to_string(),
                ph: "C".to_string(),
                ts: ts_us,
                dur: None,
                pid,
                tid: pid,
                args,
            })
        }
        _ => None,
    }
}

/// Converts raw trace content to Chrome Trace Event JSON document
pub fn convert_ftrace_buffer_to_chrome_json(raw_trace: &str) -> String {
    let mut events = Vec::new();
    for line in raw_trace.lines() {
        if let Some(event) = parse_ftrace_line(line) {
            events.push(event);
        }
    }

    let doc = ChromeTraceDocument { trace_events: events };
    serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{\"traceEvents\":[]}".to_string())
}

/// State tracking for active trace session
pub struct TraceSessionState {
    pub active: bool,
    pub ftrace_path: Option<PathBuf>,
    pub recorded_events_count: usize,
    pub last_exported_file: Option<String>,
}

pub struct NilTraceService {
    pub state: Arc<Mutex<TraceSessionState>>,
}

impl NilTraceService {
    pub fn new() -> Self {
        let ftrace_path = find_ftrace_path();
        Self {
            state: Arc::new(Mutex::new(TraceSessionState {
                active: false,
                ftrace_path,
                recorded_events_count: 0,
                last_exported_file: None,
            })),
        }
    }

    pub fn start_tracing(&self, categories: &[String]) -> Result<(), String> {
        let mut st = self.state.lock().unwrap();
        let path_opt = st.ftrace_path.clone();
        if let Some(ref path) = path_opt {
            clear_trace_buffer(path).map_err(|e| e.to_string())?;
            for cat in categories {
                let _ = enable_trace_event(path, cat, "enable", true);
            }
            set_tracing_enabled(path, true).map_err(|e| e.to_string())?;
            st.active = true;
            println!("[niltrace] Kernel tracing enabled on {:?}", path);
            Ok(())
        } else {
            // Simulated / non-root environment fallback
            st.active = true;
            println!("[niltrace] Tracing session started in userspace fallback mode.");
            Ok(())
        }
    }

    pub fn stop_and_export(&self, out_dir: &Path) -> Result<PathBuf, String> {
        let mut st = self.state.lock().unwrap();
        let _ = fs::create_dir_all(out_dir);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let out_path = out_dir.join(format!("trace_{}.json", timestamp));

        let mut trace_content = String::new();
        if let Some(ref path) = st.ftrace_path {
            let _ = set_tracing_enabled(path, false);
            let trace_file_path = path.join("trace");
            if let Ok(mut f) = File::open(trace_file_path) {
                let _ = f.read_to_string(&mut trace_content);
            }
        }

        let json = convert_ftrace_buffer_to_chrome_json(&trace_content);
        fs::write(&out_path, json).map_err(|e| e.to_string())?;

        st.active = false;
        st.last_exported_file = Some(out_path.to_string_lossy().to_string());
        println!("[niltrace] Trace exported to {:?}", out_path);
        Ok(out_path)
    }

    pub fn get_status(&self) -> TraceStatusResponse {
        let st = self.state.lock().unwrap();
        TraceStatusResponse {
            active: st.active,
            buffer_size_kb: 4096,
            recorded_events: st.recorded_events_count,
            tracing_path: st
                .ftrace_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "none".to_string()),
            last_trace_file: st.last_exported_file.clone(),
        }
    }
}

fn main() {
    println!("[niltrace] ftrace to Chrome JSON Profiler daemon starting...");
    let service = NilTraceService::new();

    let status = service.get_status();
    println!(
        "[niltrace] Status: active={}, ftrace_path={}",
        status.active, status.tracing_path
    );

    // Keep service active in daemon loop
    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sched_switch_line() {
        let raw = "   nilshell-423   [001] d... 1234.567890: sched_switch: prev_comm=swapper/0 prev_pid=0 ==> next_comm=nilshell next_pid=423";
        let ev = parse_ftrace_line(raw).expect("Failed to parse sched_switch");
        assert_eq!(ev.name, "sched_switch");
        assert_eq!(ev.cat, "sched");
        assert_eq!(ev.ph, "i");
        assert_eq!(ev.pid, 423);
        assert_eq!(ev.args.get("prev_comm").unwrap(), "swapper/0");
        assert_eq!(ev.args.get("next_comm").unwrap(), "nilshell");
        assert_eq!(ev.args.get("next_pid").unwrap(), "423");
        assert!((ev.ts - 1234567890.0).abs() < 1.0);
    }

    #[test]
    fn test_parse_atrace_markers() {
        // Begin slice
        let raw_b = "nilui-1042 [002] .... 1234.600000: tracing_mark_write: B|1042|OnuronFrameDraw";
        let ev_b = parse_ftrace_line(raw_b).expect("Failed to parse Begin marker");
        assert_eq!(ev_b.name, "OnuronFrameDraw");
        assert_eq!(ev_b.ph, "B");
        assert_eq!(ev_b.pid, 1042);
        assert_eq!(ev_b.cat, "userspace");

        // End slice
        let raw_e = "nilui-1042 [002] .... 1234.605000: tracing_mark_write: E|1042";
        let ev_e = parse_ftrace_line(raw_e).expect("Failed to parse End marker");
        assert_eq!(ev_e.ph, "E");
        assert_eq!(ev_e.pid, 1042);

        // Counter
        let raw_c = "nilui-1042 [002] .... 1234.606000: tracing_mark_write: C|1042|fps|60";
        let ev_c = parse_ftrace_line(raw_c).expect("Failed to parse Counter marker");
        assert_eq!(ev_c.ph, "C");
        assert_eq!(ev_c.name, "fps");
        assert_eq!(ev_c.args.get("value").unwrap(), "60");
    }

    #[test]
    fn test_convert_ftrace_buffer_to_chrome_json() {
        let raw = "\
# tracer: nop
#
   nilshell-423   [001] d... 100.000000: sched_switch: prev_comm=idle prev_pid=0 ==> next_comm=nilshell next_pid=423
   nilshell-423   [001] .... 100.000500: tracing_mark_write: B|423|RenderScene
   nilshell-423   [001] .... 100.008000: tracing_mark_write: E|423
";
        let json_str = convert_ftrace_buffer_to_chrome_json(raw);
        let doc: ChromeTraceDocument = serde_json::from_str(&json_str).expect("Valid JSON");
        assert_eq!(doc.trace_events.len(), 3);
        assert_eq!(doc.trace_events[0].name, "sched_switch");
        assert_eq!(doc.trace_events[1].name, "RenderScene");
        assert_eq!(doc.trace_events[1].ph, "B");
        assert_eq!(doc.trace_events[2].ph, "E");
    }
}
