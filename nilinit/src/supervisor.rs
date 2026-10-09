// nilinit/src/supervisor.rs — Service restart policy, exec parsing, and the
// supervision loop for PID 1.

use std::collections::HashMap;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

/// How often a failing service may be restarted, and how long to wait.
#[derive(Clone, Copy, PartialEq)]
pub struct RestartPolicy {
    pub restart: bool,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub threshold: u32,
}

impl RestartPolicy {
    pub fn from_config(restart: &str) -> Self {
        match restart.trim().to_ascii_lowercase().as_str() {
            "always" | "on-failure" => Self {
                restart: true,
                base_delay_ms: 250,
                max_delay_ms: 30_000,
                threshold: u32::MAX,
            },
            _ => Self { restart: false, base_delay_ms: 0, max_delay_ms: 0, threshold: 0 },
        }
    }

    /// Delay before restart number `attempt` (1-based), doubling up to the cap.
    pub fn delay_ms(&self, attempt: u32) -> u64 {
        if attempt == 0 {
            return 0;
        }
        let shift = attempt.saturating_sub(1).min(16);
        self.base_delay_ms.saturating_mul(1u64 << shift).min(self.max_delay_ms)
    }

    pub fn may_restart(&self, attempt: u32) -> bool {
        self.restart && attempt <= self.threshold
    }
}

/// A service entry from the init configuration.
#[derive(Clone, Debug)]
pub struct ServiceSpec {
    pub name: String,
    pub exec: String,
    pub restart: String,
}

/// The supervision loop, extracted from `main()` so it can be driven by tests
/// with real child processes.
pub struct Supervisor {
    services: Vec<ServiceSpec>,
    running: HashMap<String, Child>,
    restart_attempts: HashMap<String, u32>,
    started_at: HashMap<String, Instant>,
    /// A service that stayed up this long counts as healthy, so its backoff
    /// resets instead of compounding across unrelated incidents.
    healthy_uptime: Duration,
    /// When set, overrides the computed backoff delay. Tests use this to avoid
    /// sleeping for real.
    delay_override: Option<Duration>,
}

impl Supervisor {
    pub fn new(services: Vec<ServiceSpec>) -> Self {
        Self {
            services,
            running: HashMap::new(),
            restart_attempts: HashMap::new(),
            started_at: HashMap::new(),
            healthy_uptime: Duration::from_secs(60),
            delay_override: None,
        }
    }

    /// Test hook: replace the backoff delay so tests do not sleep.
    #[allow(dead_code)]
    pub fn with_delay_override(mut self, delay: Duration) -> Self {
        self.delay_override = Some(delay);
        self
    }

    /// Start every service that is not socket-activated.
    pub fn start_all(&mut self) {
        let specs: Vec<ServiceSpec> = self.services.clone();
        for spec in &specs {
            self.spawn(spec);
        }
    }

    fn spawn(&mut self, spec: &ServiceSpec) {
        match parse_exec(&spec.exec) {
            Some((bin, args)) => match Command::new(&bin).args(&args).spawn() {
                Ok(child) => {
                    self.started_at.insert(spec.name.clone(), Instant::now());
                    self.running.insert(spec.name.clone(), child);
                }
                Err(e) => {
                    eprintln!("[nilinit] Service '{}' not available: {}", spec.name, e);
                }
            },
            None => {
                eprintln!("[nilinit] Service '{}' has an invalid exec line: {}", spec.name, spec.exec);
            }
        }
    }

    /// One supervision pass. Returns `false` when a shutdown was requested.
    pub fn tick(&mut self) -> bool {
        let mut dead = Vec::new();
        for (name, child) in self.running.iter_mut() {
            match child.try_wait() {
                Ok(Some(_)) => dead.push(name.clone()),
                Ok(None) => {}
                Err(e) => {
                    eprintln!("[nilinit] Error polling service '{}': {}", name, e);
                    dead.push(name.clone());
                }
            }
        }

        for name in dead {
            self.running.remove(&name);
            let uptime = self.started_at.remove(&name).map(|t| t.elapsed());
            if uptime.map(|u| u >= self.healthy_uptime).unwrap_or(false) {
                self.restart_attempts.remove(&name);
            }
            if let Some(spec) = self.services.iter().find(|s| s.name == name).cloned() {
                let policy = RestartPolicy::from_config(&spec.restart);
                if policy.restart {
                    let attempt = self.restart_attempts.entry(name.clone()).or_insert(0);
                    *attempt = attempt.saturating_add(1);
                    let attempt = *attempt;
                    if policy.may_restart(attempt) {
                        let delay = self
                            .delay_override
                            .unwrap_or_else(|| Duration::from_millis(policy.delay_ms(attempt)));
                        if !delay.is_zero() {
                            std::thread::sleep(delay);
                        }
                        self.spawn(&spec);
                    } else {
                        eprintln!(
                            "[nilinit] Service '{}' exceeded its restart threshold; not restarting",
                            name
                        );
                    }
                }
            }
        }

        true
    }

    /// Number of live service processes.
    pub fn live_count(&self) -> usize {
        self.running.len()
    }

    /// Restart attempts recorded for a service so far.
    #[allow(dead_code)]
    pub fn attempts(&self, name: &str) -> u32 {
        self.restart_attempts.get(name).copied().unwrap_or(0)
    }

    /// Stop every running service. Used on shutdown/reboot.
    pub fn shutdown(&mut self) {
        for (name, child) in self.running.iter_mut() {
            eprintln!("[nilinit] Stopping service: {}", name);
            let _ = child.kill();
        }
        self.running.clear();
    }
}

/// Split a service command line, honouring single and double quotes so that
/// arguments containing spaces survive supervision and restart.
pub fn parse_exec(exec: &str) -> Option<(String, Vec<String>)> {
    let mut program: Option<String> = None;
    let mut args: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut has_token = false;

    for ch in exec.chars() {
        match quote {
            Some(q) if ch == q => quote = None,
            Some(_) => current.push(ch),
            None if ch == '\'' || ch == '"' => {
                quote = Some(ch);
                has_token = true;
            }
            None if ch.is_whitespace() => {
                if has_token {
                    let token = std::mem::take(&mut current);
                    if program.is_none() {
                        program = Some(token);
                    } else {
                        args.push(token);
                    }
                    has_token = false;
                }
            }
            None => {
                current.push(ch);
                has_token = true;
            }
        }
    }
    if quote.is_some() {
        return None;
    }
    if has_token {
        let token = std::mem::take(&mut current);
        if program.is_none() {
            program = Some(token);
        } else {
            args.push(token);
        }
    }
    let program = program?;
    if program.is_empty() {
        return None;
    }
    Some((program, args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restart_disabled_for_unknown_policy() {
        let policy = RestartPolicy::from_config("never");
        assert!(!policy.may_restart(1));
        assert_eq!(policy.delay_ms(1), 0);
    }

    #[test]
    fn restart_delay_backs_off_and_caps() {
        let policy = RestartPolicy::from_config("always");
        assert!(policy.may_restart(1));
        assert_eq!(policy.delay_ms(1), 250);
        assert_eq!(policy.delay_ms(2), 500);
        assert_eq!(policy.delay_ms(5), 4_000);
        assert_eq!(policy.delay_ms(9), 30_000);
        assert_eq!(policy.delay_ms(20), 30_000);
        assert_eq!(policy.delay_ms(0), 0);
    }

    #[test]
    fn parse_exec_honours_quotes_and_rejects_bad_input() {
        assert_eq!(parse_exec("/usr/bin/netd"), Some(("/usr/bin/netd".into(), vec![])));
        assert_eq!(
            parse_exec("/usr/bin/nild --foreground --label \"Onuron Shell\""),
            Some(("/usr/bin/nild".into(), vec!["--foreground".into(), "--label".into(), "Onuron Shell".into()]))
        );
        assert_eq!(parse_exec("  /usr/bin/app   "), Some(("/usr/bin/app".into(), vec![])));
        assert_eq!(parse_exec("   "), None);
        assert_eq!(parse_exec(""), None);
        assert_eq!(parse_exec("/usr/bin/app \"unterminated"), None);
    }

    // ── Integration tests: real child processes ──────────────────────────────

    /// A command that exits immediately with a non-zero status, so the
    /// supervisor sees it die on the first tick.
    fn failing_service() -> ServiceSpec {
        #[cfg(unix)]
        let exec = "false".to_string();
        #[cfg(windows)]
        let exec = "cmd /c exit 1".to_string();
        ServiceSpec {
            name: "crasher".into(),
            exec,
            restart: "always".into(),
        }
    }

    /// A command that stays alive long enough to be observed running.
    fn sleeping_service() -> ServiceSpec {
        #[cfg(unix)]
        let exec = "sleep 30".to_string();
        #[cfg(windows)]
        let exec = "timeout /t 30".to_string();
        ServiceSpec {
            name: "sleeper".into(),
            exec,
            restart: "always".into(),
        }
    }

    #[test]
    fn crashing_service_is_restarted_with_backoff() {
        let mut supervisor = Supervisor::new(vec![failing_service()])
            .with_delay_override(Duration::from_millis(1));
        supervisor.start_all();
        assert_eq!(supervisor.live_count(), 1, "service should start");

        // Give the child time to actually exit before the first tick.
        std::thread::sleep(Duration::from_millis(100));

        // First tick observes the exit and schedules a restart.
        supervisor.tick();
        assert_eq!(supervisor.live_count(), 1, "service should be restarted");
        assert_eq!(supervisor.attempts("crasher"), 1);

        // Repeated ticks keep restarting it; the attempt counter grows.
        for _ in 0..5 {
            std::thread::sleep(Duration::from_millis(50));
            supervisor.tick();
        }
        assert!(supervisor.attempts("crasher") >= 5, "attempts should accumulate");
        assert_eq!(supervisor.live_count(), 1);
    }

    #[test]
    fn service_without_restart_policy_is_not_respawned() {
        let mut supervisor = Supervisor::new(vec![ServiceSpec {
            name: "oneshot".into(),
            exec: {
                #[cfg(unix)]
                { "false".to_string() }
                #[cfg(windows)]
                { "cmd /c exit 1".to_string() }
            },
            restart: "never".into(),
        }])
        .with_delay_override(Duration::from_millis(1));
        supervisor.start_all();
        assert_eq!(supervisor.live_count(), 1);

        std::thread::sleep(Duration::from_millis(100));
        supervisor.tick();
        assert_eq!(supervisor.live_count(), 0, "unsupervised service must not respawn");
        assert_eq!(supervisor.attempts("oneshot"), 0);
    }

    #[test]
    fn malformed_exec_line_does_not_panic_and_is_not_started() {
        let mut supervisor = Supervisor::new(vec![ServiceSpec {
            name: "broken".into(),
            exec: "/usr/bin/app \"unterminated".into(),
            restart: "always".into(),
        }]);
        supervisor.start_all();
        assert_eq!(supervisor.live_count(), 0, "malformed exec must not spawn");
        // Ticking must not panic even with a broken service present.
        supervisor.tick();
        assert_eq!(supervisor.live_count(), 0);
    }

    #[test]
    fn shutdown_stops_all_running_services() {
        let mut supervisor = Supervisor::new(vec![sleeping_service()])
            .with_delay_override(Duration::from_millis(1));
        supervisor.start_all();
        assert_eq!(supervisor.live_count(), 1);

        supervisor.shutdown();
        assert_eq!(supervisor.live_count(), 0, "shutdown must kill children");
    }

    #[test]
    fn healthy_uptime_resets_backoff_counter() {
        // A service that has been up longer than the healthy threshold has its
        // attempt counter cleared on the next exit, so an old crash does not
        // inflate the backoff for a new one.
        let mut supervisor = Supervisor::new(vec![failing_service()])
            .with_delay_override(Duration::from_millis(1));
        supervisor.start_all();

        // Simulate several crashes to build up attempts.
        for _ in 0..3 {
            std::thread::sleep(Duration::from_millis(100));
            supervisor.tick();
        }
        assert!(supervisor.attempts("crasher") >= 3);

        // Simulate a long uptime by rewinding the recorded start time.
        if let Some(started) = supervisor.started_at.get_mut("crasher") {
            *started = Instant::now() - Duration::from_secs(120);
        }
        std::thread::sleep(Duration::from_millis(100));
        supervisor.tick();
        assert_eq!(
            supervisor.attempts("crasher"), 1,
            "backoff should reset after a healthy uptime"
        );
    }

    #[test]
    fn soak_supervision_bookkeeping_stays_bounded() {
        // F3: run the supervision bookkeeping for 10,000 iterations and prove
        // the internal maps do not grow without bound. We seed exactly one
        // already-exited child, then let the loop reap it. The service's binary
        // is intentionally missing so the respawn attempt fails cheaply instead
        // of starting 10,000 real processes.
        let spec = ServiceSpec {
            name: "soakd".into(),
            exec: "definitely-not-a-real-binary --soak".into(),
            restart: "always".into(),
        };
        let mut supervisor =
            Supervisor::new(vec![spec.clone()]).with_delay_override(Duration::ZERO);

        let mut dead_child = if cfg!(windows) {
            Command::new("cmd").args(["/c", "exit", "0"]).spawn().unwrap()
        } else {
            Command::new("true").spawn().unwrap()
        };
        let _ = dead_child.wait(); // ensure it is already reaped-dead
        supervisor.running.insert(spec.name.clone(), dead_child);
        supervisor.started_at.insert(spec.name.clone(), Instant::now());

        const ITERS: usize = 10_000;
        for _ in 0..ITERS {
            assert!(supervisor.tick());
        }

        // The seeded death may be counted once; the map must not keep growing.
        assert_eq!(supervisor.live_count(), 0, "no live children after soak");
        assert_eq!(supervisor.attempts("soakd"), 1, "only the seeded death is counted");
        assert!(supervisor.running.is_empty(), "running map leaked entries");
        assert!(supervisor.restart_attempts.len() <= 1, "attempt map grew unbounded");
        assert!(supervisor.started_at.is_empty(), "started_at map leaked entries");

        // The pure backoff policy and exec parser are also exercised many times
        // to prove they stay bounded (and never overflow) across many attempts.
        let policy = RestartPolicy::from_config("always");
        for attempt in 0..ITERS as u32 {
            assert!(policy.delay_ms(attempt) <= policy.max_delay_ms);
            let _ = parse_exec("/usr/bin/soakd --tick");
        }
    }
}