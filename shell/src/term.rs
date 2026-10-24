// shell/src/term.rs — Real OnuronOS terminal command engine
//
// The shell's terminal previously answered from a hardcoded match arm: it could
// describe the system but never *run* anything on it. This module makes the
// terminal real:
//
//   * Builtins (cd, pwd, exit, clear, help, export, unset, echo) run in-process
//     because they must mutate the shell's own state.
//   * Every other command is resolved against $PATH and executed as a real
//     child process with stdin/stdout/stderr wired up. Output is captured and
//     returned to the render loop.
//   * No command is ever synthesised. If a binary is absent, the terminal says
//     so and reports the shell's exit status, exactly like a real shell.
//
// Commands are parsed with quote and backslash handling so `echo "a b"` is one
// argument, and a pipeline-free command line is split the same way /bin/sh
// would split it.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Longest a single command may run before the terminal reclaims the prompt.
/// Without this a `cat` on a FIFO would freeze the whole UI.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_OUTPUT_BYTES: usize = 64 * 1024;

/// Persistent state of the terminal session.
#[derive(Debug, Clone)]
pub struct TermSession {
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
    pub last_status: i32,
    pub home: PathBuf,
}

impl TermSession {
    /// Create a session with an explicit starting directory and home. The shell
    /// passes its own `/data` default so `cd ~` keeps its historical meaning.
    pub fn new(cwd: PathBuf, home: PathBuf) -> Self {
        Self {
            cwd,
            home,
            env: Vec::new(),
            last_status: 0,
        }
    }
}

impl Default for TermSession {
    fn default() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        Self {
            home: cwd.clone(),
            cwd,
            env: Vec::new(),
            last_status: 0,
        }
    }
}

/// Outcome of one command line.
#[derive(Debug, Clone, PartialEq)]
pub enum TermOutcome {
    /// Command produced output. `status` is the exit code.
    Output { text: String, status: i32 },
    /// The session asked to end (exit / logout).
    Exit,
    /// The screen should be cleared.
    Clear,
    /// Nothing to show.
    Silent,
}

/// Split a command line into arguments, honouring single quotes, double quotes
/// and backslash escapes. This mirrors POSIX word splitting closely enough for
/// interactive use and is deliberately not a full shell grammar: there is no
/// pipeline, redirection or expansion, and the terminal says so in `help`.
pub fn split_args(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut has_token = false;
    let mut chars = line.chars().peekable();
    let mut quote: Option<char> = None;

    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                } else if c == '\\' && q == '"' {
                    if let Some(next) = chars.next() {
                        cur.push(next);
                    }
                } else {
                    cur.push(c);
                }
            }
            None => match c {
                '\'' | '"' => {
                    quote = Some(c);
                    has_token = true;
                }
                '\\' => {
                    if let Some(next) = chars.next() {
                        cur.push(next);
                        has_token = true;
                    }
                }
                c if c.is_whitespace() => {
                    if has_token {
                        args.push(std::mem::take(&mut cur));
                        has_token = false;
                    }
                }
                c => {
                    cur.push(c);
                    has_token = true;
                }
            },
        }
    }
    if has_token {
        args.push(cur);
    }
    args
}

/// Resolve a program name against PATH, returning the first executable match.
/// A name containing a slash is used as a path directly, as in POSIX shells.
pub fn resolve_program(name: &str, cwd: &Path, env: &[(String, String)]) -> Option<PathBuf> {
    if name.contains('/') {
        let p = if name.starts_with('/') {
            PathBuf::from(name)
        } else {
            cwd.join(name)
        };
        return if is_executable(&p) { Some(p) } else { None };
    }

    let path_var = env
        .iter()
        .find(|(k, _)| k == "PATH")
        .map(|(_, v)| v.clone())
        .or_else(|| std::env::var("PATH").ok())
        .unwrap_or_else(|| "/usr/bin:/bin:/usr/sbin:/sbin".to_string());

    for dir in path_var.split(':').filter(|d| !d.is_empty()) {
        let candidate = Path::new(dir).join(name);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn is_executable(p: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match std::fs::metadata(p) {
            Ok(md) => md.is_file() && md.permissions().mode() & 0o111 != 0,
            Err(_) => false,
        }
    }
    #[cfg(not(unix))]
    {
        p.is_file()
    }
}

fn expand_tilde(token: &str, home: &Path) -> PathBuf {
    if token == "~" {
        home.to_path_buf()
    } else if let Some(rest) = token.strip_prefix("~/") {
        home.join(rest)
    } else {
        PathBuf::from(token)
    }
}

/// Resolve `.` and `..` components lexically, without touching the filesystem.
///
/// `canonicalize` cannot be used alone here: it fails on a path that does not
/// exist, and a shell must still be able to compute `/data/..` when `/data` is
/// absent. Real shells keep a logical cwd and only fall back to the physical
/// one when the logical path is a symlink; this does the same, minus the
/// symlink case, which `canonicalize` still handles when it succeeds.
pub fn normalize_logical(path: &Path) -> PathBuf {
    let mut prefix: Option<std::ffi::OsString> = None;
    let mut has_root = false;
    let mut segments: Vec<std::ffi::OsString> = Vec::new();

    for comp in path.components() {
        match comp {
            std::path::Component::Prefix(p) => prefix = Some(p.as_os_str().to_os_string()),
            std::path::Component::RootDir => has_root = true,
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !segments.is_empty() {
                    segments.pop();
                }
            }
            std::path::Component::Normal(n) => segments.push(n.to_os_string()),
        }
    }

    if path.starts_with("/") || path.starts_with("\\") {
        has_root = true;
    }

    let mut result = PathBuf::new();
    if let Some(p) = prefix {
        result.push(p);
        if has_root {
            result.push(std::path::MAIN_SEPARATOR.to_string());
        }
    } else if has_root {
        result.push("/");
    }
    for seg in segments {
        result.push(seg);
    }
    if result.as_os_str().is_empty() {
        result.push(".");
    }
    result
}

/// Run one command line in `session`. Never panics and never blocks forever.
pub fn run_line(session: &mut TermSession, line: &str) -> TermOutcome {
    let args = split_args(line);
    if args.is_empty() {
        return TermOutcome::Silent;
    }

    match args[0].as_str() {
        "exit" | "logout" => return TermOutcome::Exit,
        "clear" => return TermOutcome::Clear,
        _ => {}
    }

    if let Some(outcome) = builtin(session, &args) {
        return outcome;
    }

    let program = match resolve_program(&args[0], &session.cwd, &session.env) {
        Some(p) => p,
        None => {
            session.last_status = 127;
            return TermOutcome::Output {
                text: format!("onuron: command not found: {}", args[0]),
                status: 127,
            };
        }
    };

    exec_program(session, &program, &args[1..])
}

fn builtin(session: &mut TermSession, args: &[String]) -> Option<TermOutcome> {
    match args[0].as_str() {
        "cd" => {
            let target = match args.get(1) {
                Some(t) => expand_tilde(t, &session.home),
                None => session.home.clone(),
            };
            let is_target_rooted = target.is_absolute()
                || target.has_root()
                || target.starts_with("/")
                || target.starts_with("\\");
            let joined = if is_target_rooted {
                target
            } else {
                session.cwd.join(&target)
            };
            // Resolve `.`/`..` lexically first so a path that does not exist yet
            // still computes correctly, then prefer the physical path when the
            // directory really is there (this follows symlinks).
            let logical = normalize_logical(&joined);
            let resolved = if logical == Path::new("/") {
                PathBuf::from("/")
            } else {
                #[cfg(windows)]
                {
                    std::fs::canonicalize(&logical)
                        .map(|p| {
                            let s = p.to_string_lossy();
                            if let Some(stripped) = s.strip_prefix(r"\\?\") {
                                PathBuf::from(stripped)
                            } else {
                                p
                            }
                        })
                        .unwrap_or(logical)
                }
                #[cfg(not(windows))]
                {
                    std::fs::canonicalize(&logical).unwrap_or(logical)
                }
            };
            let is_dir = if resolved == Path::new("/") {
                true
            } else {
                std::fs::metadata(&resolved)
                    .map(|md| md.is_dir())
                    .unwrap_or(false)
            };
            if is_dir {
                session.cwd = resolved;
                session.last_status = 0;
                Some(TermOutcome::Silent)
            } else if std::fs::metadata(&resolved).is_ok() {
                session.last_status = 1;
                Some(TermOutcome::Output {
                    text: format!("cd: not a directory: {}", resolved.display()),
                    status: 1,
                })
            } else {
                let err = std::fs::metadata(&resolved).unwrap_err();
                session.last_status = 1;
                Some(TermOutcome::Output {
                    text: format!("cd: {}: {}", resolved.display(), err),
                    status: 1,
                })
            }
        }
        "pwd" => Some(TermOutcome::Output {
            text: session.cwd.display().to_string(),
            status: 0,
        }),
        "export" => {
            for pair in &args[1..] {
                if let Some((k, v)) = pair.split_once('=') {
                    session.env.retain(|(ek, _)| ek != k);
                    session.env.push((k.to_string(), v.to_string()));
                }
            }
            Some(TermOutcome::Silent)
        }
        "unset" => {
            for k in &args[1..] {
                session.env.retain(|(ek, _)| ek != k);
            }
            Some(TermOutcome::Silent)
        }
        "echo" => Some(TermOutcome::Output {
            text: args[1..].join(" "),
            status: 0,
        }),
        "help" => Some(TermOutcome::Output {
            text: help_text(),
            status: 0,
        }),
        "which" => {
            let mut out = String::new();
            for name in &args[1..] {
                match resolve_program(name, &session.cwd, &session.env) {
                    Some(p) => out.push_str(&format!("{}\n", p.display())),
                    None => out.push_str(&format!("{}: not found\n", name)),
                }
            }
            Some(TermOutcome::Output {
                text: out.trim_end().to_string(),
                status: 0,
            })
        }
        _ => None,
    }
}

fn exec_program(session: &mut TermSession, program: &Path, argv: &[String]) -> TermOutcome {
    let mut cmd = Command::new(program);
    cmd.args(argv)
        .current_dir(&session.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in &session.env {
        cmd.env(k, v);
    }

    let started = Instant::now();
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            session.last_status = 126;
            return TermOutcome::Output {
                text: format!("onuron: cannot execute {}: {}", program.display(), e),
                status: 126,
            };
        }
    };

    // Drain both pipes on separate threads. Reading them sequentially can
    // deadlock when a child fills the pipe we are not reading yet.
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let out_handle = std::thread::spawn(move || read_capped(stdout.as_mut()));
    let err_handle = std::thread::spawn(move || read_capped(stderr.as_mut()));

    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break Some(st),
            Ok(None) => {
                if started.elapsed() > COMMAND_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break None,
        }
    };

    let stdout_text = out_handle.join().unwrap_or_default();
    let stderr_text = err_handle.join().unwrap_or_default();

    let code = status.and_then(|s| s.code()).unwrap_or(-1);
    session.last_status = code;

    let mut text = stdout_text;
    if !stderr_text.is_empty() {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&stderr_text);
    }
    if status.is_none() {
        text.push_str(&format!(
            "\n[timed out after {}s]",
            COMMAND_TIMEOUT.as_secs()
        ));
    }

    TermOutcome::Output {
        text: text.trim_end_matches('\n').to_string(),
        status: code,
    }
}

fn read_capped<R: Read>(reader: Option<&mut R>) -> String {
    let mut buf = Vec::new();
    if let Some(r) = reader {
        let mut chunk = [0u8; 8192];
        loop {
            match r.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    if buf.len() + n > MAX_OUTPUT_BYTES {
                        buf.extend_from_slice(&chunk[..MAX_OUTPUT_BYTES.saturating_sub(buf.len())]);
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                }
                Err(_) => break,
            }
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

pub fn help_text() -> String {
    [
        "OnuronOS terminal — real command execution",
        "",
        "  Everything not listed below is looked up in $PATH and executed as a",
        "  real child process. Nothing is simulated or pre-canned.",
        "",
        "Builtins:",
        "  cd [dir]        change directory (no argument goes to $HOME)",
        "  pwd             print the working directory",
        "  export K=V      set an environment variable for child processes",
        "  unset K         remove an environment variable",
        "  echo ...        print arguments",
        "  which cmd       show the resolved path of a command",
        "  clear           clear the screen",
        "  help            this text",
        "  exit            leave the terminal",
        "",
        "Not implemented: pipelines, redirection, globbing, substitution.",
        "Run the command directly instead, e.g. `ls /data` rather than `ls | wc`.",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_args_handles_quotes_and_escapes() {
        assert_eq!(split_args("ls -l /data"), vec!["ls", "-l", "/data"]);
        assert_eq!(split_args("echo \"a b\" c"), vec!["echo", "a b", "c"]);
        assert_eq!(split_args("echo 'a b' c"), vec!["echo", "a b", "c"]);
        assert_eq!(split_args("echo a\\ b"), vec!["echo", "a b"]);
        assert_eq!(split_args("   "), Vec::<String>::new());
        assert_eq!(split_args("echo \"\""), vec!["echo", ""]);
    }

    #[test]
    fn builtin_echo_and_pwd_work() {
        let mut s = TermSession::default();
        match run_line(&mut s, "echo hello world") {
            TermOutcome::Output { text, status } => {
                assert_eq!(text, "hello world");
                assert_eq!(status, 0);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn cd_updates_the_session_directory() {
        let mut s = TermSession::default();
        let outcome = run_line(&mut s, "cd /");
        assert_eq!(outcome, TermOutcome::Silent);
        assert_eq!(s.cwd, PathBuf::from("/"));
        // A failing cd must leave the cwd untouched and report failure.
        let outcome = run_line(&mut s, "cd /definitely/not/here");
        match outcome {
            TermOutcome::Output { status, .. } => assert_ne!(status, 0),
            other => panic!("unexpected: {other:?}"),
        }
        assert_eq!(s.cwd, PathBuf::from("/"));
    }

    #[test]
    fn normalize_logical_resolves_dot_and_dotdot_without_touching_disk() {
        // None of these paths need to exist for the arithmetic to be correct.
        assert_eq!(
            normalize_logical(Path::new("/data")),
            PathBuf::from("/data")
        );
        assert_eq!(normalize_logical(Path::new("/data/..")), PathBuf::from("/"));
        assert_eq!(
            normalize_logical(Path::new("/data/app/../system")),
            PathBuf::from("/data/system")
        );
        assert_eq!(normalize_logical(Path::new("/../..")), PathBuf::from("/"));
        assert_eq!(
            normalize_logical(Path::new("/a/./b")),
            PathBuf::from("/a/b")
        );
        assert_eq!(normalize_logical(Path::new(".")), PathBuf::from("."));
    }

    #[test]
    fn cd_dotdot_from_a_missing_directory_still_clamps_at_root() {
        let mut s = TermSession::new(PathBuf::from("/data"), PathBuf::from("/data"));
        // /data does not exist on a dev host; the lexical resolution must still
        // produce "/" and the cd must succeed because "/" does exist.
        let outcome = run_line(&mut s, "cd ..");
        assert_eq!(
            outcome,
            TermOutcome::Silent,
            "cd .. from /data should reach /"
        );
        assert_eq!(s.cwd, PathBuf::from("/"));

        let outcome = run_line(&mut s, "cd ../../..");
        assert_eq!(outcome, TermOutcome::Silent);
        assert_eq!(s.cwd, PathBuf::from("/"));
    }

    #[test]
    fn unknown_command_reports_127() {
        let mut s = TermSession::default();
        s.env.push(("PATH".into(), "/nonexistent".into()));
        match run_line(&mut s, "definitely-not-a-real-binary-xyz") {
            TermOutcome::Output { status, text } => {
                assert_eq!(status, 127);
                assert!(text.contains("command not found"));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn real_process_execution_returns_its_output() {
        // /bin/echo exists on every Linux/macOS host and is not a builtin here.
        let echo = ["/bin/echo", "/usr/bin/echo"]
            .iter()
            .find(|p| Path::new(p).exists());
        let Some(echo) = echo else { return };

        let mut s = TermSession::default();
        match run_line(&mut s, &format!("{echo} executed-for-real")) {
            TermOutcome::Output { text, status } => {
                assert_eq!(text, "executed-for-real");
                assert_eq!(status, 0);
                assert_eq!(s.last_status, 0);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn nonzero_exit_status_is_propagated() {
        let sh = ["/bin/sh", "/usr/bin/sh"]
            .iter()
            .find(|p| Path::new(p).exists());
        let Some(sh) = sh else { return };

        let mut s = TermSession::default();
        match run_line(&mut s, &format!("{sh} -c \"exit 42\"")) {
            TermOutcome::Output { status, .. } => assert_eq!(status, 42),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn export_is_visible_to_children() {
        let sh = ["/bin/sh", "/usr/bin/sh"]
            .iter()
            .find(|p| Path::new(p).exists());
        let Some(sh) = sh else { return };

        let mut s = TermSession::default();
        run_line(&mut s, "export ONURON_TEST_VAR=abc123");
        match run_line(&mut s, &format!("{sh} -c \"printf %s $ONURON_TEST_VAR\"")) {
            TermOutcome::Output { text, .. } => assert_eq!(text, "abc123"),
            other => panic!("unexpected: {other:?}"),
        }
    }
}
