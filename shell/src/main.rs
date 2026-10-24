// shell/src/main.rs — NilOS Complete Mobile UI Engine
// Screen stack: OOBE → Lockscreen → Home Launcher → Apps
// Renders via ANSI escape codes on /dev/console + /dev/ttyS0

use std::fs::{self};
use std::io::{self, BufRead, Write};
use std::path::Path;

mod browser;
mod i18n;
mod notes;
mod pin;
mod simulated;
mod term;

// ─── ANSI Colors ──────────────────────────────────────────────────────────────
const R: &str = "\x1b[0m"; // Reset
const B: &str = "\x1b[1m"; // Bold
const CL: &str = "\x1b[2J\x1b[H"; // Clear screen

const FG_CYAN: &str = "\x1b[1;36m";
const FG_GREEN: &str = "\x1b[1;32m";
const FG_YELLOW: &str = "\x1b[1;33m";
const FG_BLUE: &str = "\x1b[1;34m";
const FG_MAGENTA: &str = "\x1b[1;35m";
const FG_WHITE: &str = "\x1b[1;37m";
const FG_RED: &str = "\x1b[1;31m";
const FG_GRAY: &str = "\x1b[0;90m";
const FG_BLACK_ON_WHITE: &str = "\x1b[30;47m";

// ─── Data paths ───────────────────────────────────────────────────────────────
const OOBE_DONE: &str = "/data/nilos/oobe_done";
const PIN_FILE: &str = "/data/nilos/pin_hash";
const USER_CONF: &str = "/data/nilos/user.conf";
const CONTACTS_DIR: &str = "/data/contacts";
const CONTACTS_FILE: &str = "/data/contacts/contacts.json";
const CALLS_FILE: &str = "/data/calls/call_log.json";
const SMS_DIR: &str = "/data/sms";
const SMS_THREADS_FILE: &str = "/data/sms/threads.json";
const NOTES_FILE: &str = "/data/notes/notes.json";
const CALC_HISTORY_FILE: &str = "/data/nilos/calc_history.json";
const PLAYLIST_FILE: &str = "/data/music/playlist.json";
const DCIM_DIR: &str = "/data/dcim";

pub fn live_clock_and_date() -> (String, String, String) {
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let tz_offset_mins: i64 = fs::read_to_string("/data/nilos/tz_offset_mins")
        .ok()
        .and_then(|s| s.trim().parse::<i64>().ok())
        .unwrap_or(360); // Default UTC+6 (Dhaka / Bangladesh Standard Time)
    let local_secs = (now_secs as i64 + tz_offset_mins * 60).max(0) as u64;

    let hours24 = ((local_secs / 3600) % 24) as u8;
    let mins = ((local_secs / 60) % 60) as u8;
    let am_pm = if hours24 >= 12 { "PM" } else { "AM" };
    let hours12 = match hours24 % 12 {
        0 => 12,
        h => h,
    };
    let time_12h = format!("{:02}:{:02} {}", hours12, mins, am_pm);
    let time_24h = format!("{:02}:{:02}", hours24, mins);

    let total_days = (local_secs / 86400) as i64;
    let dow = ((total_days + 4) % 7 + 7) % 7;
    let weekdays = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ];
    let z = total_days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    let months = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let date_str = format!(
        "{}, {} {}, {}",
        weekdays[dow as usize],
        months[(m - 1) as usize],
        d,
        year
    );
    (time_12h, time_24h, date_str)
}

pub fn directory_usage_summary(path: &Path) -> (usize, u64) {
    let mut files = 0usize;
    let mut bytes = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    let (sub_f, sub_b) = directory_usage_summary(&entry.path());
                    files += sub_f;
                    bytes += sub_b;
                } else if ft.is_file() {
                    files += 1;
                    bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
                }
            }
        }
    }
    (files, bytes)
}

// ─── Screen Enum ──────────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq)]
enum Screen {
    // OOBE flow
    OobeWelcome,
    OobeName,
    OobePin,
    OobeConfirmPin,
    OobeDone,
    // Normal boot flow
    Lockscreen,
    Home,
    // Apps
    AppPhone,
    AppMessages,
    AppFiles,
    AppSettings,
    AppNilPkg,
    AppSoftBus,
    AppAndroid,
    AppTerminal,
    AppCalculator,
    AppNotes,
    AppBrowser,
    AppMusic,
    AppCamera,
    // Dialogs
    NotificationShade,
}

// ─── App State ────────────────────────────────────────────────────────────────
struct AppState {
    user_name: String,
    pin: String,
    pending_pin: String,
    #[allow(dead_code)]
    pin_input: String,
    sms_threads: Vec<(String, String, String)>, // (contact, last_msg, time)
    contacts: Vec<(String, String)>,            // (name, number)
    call_log: Vec<(String, String, String)>,    // (peer, number, timestamp)
    compose_to: String,
    compose_body: String,
    terminal_history: Vec<String>,
    /// Real command-execution session (cwd, exported env, exit status).
    term: term::TermSession,
    /// Real network browser state.
    browser: browser::BrowserState,
    #[allow(dead_code)]
    settings_cursor: usize,
    files_path: String,
    #[allow(dead_code)]
    pkg_cursor: usize,
    call_number: String,
    wifi_enabled: bool,
    cellular_enabled: bool,
    signal_bars: u8,
    term_cwd: String,
    calc_val: String,
    calc_history: Vec<(String, String)>,
    music_playing: bool,
    music_track: usize,
    music_tracks: Vec<(String, String, String)>, // (title, artist, duration/path)
    notes: Vec<notes::NoteItem>,
    notes_path: std::path::PathBuf,
    camera_id: u32,
    camera_torch: bool,
    camera_captures: u32,
    camera_last_status: String,
    locale: i18n::Locale,
    screen_reader: bool,
    power_profile: String,
    high_refresh_rate: bool,
    brightness: u8,
}

impl AppState {
    fn load() -> Self {
        let user_name = fs::read_to_string(USER_CONF)
            .unwrap_or_else(|_| String::from("NilOS User"))
            .trim()
            .to_string();

        let pin = fs::read_to_string(PIN_FILE)
            .unwrap_or_default()
            .trim()
            .to_string();

        let notes_path = std::path::PathBuf::from(NOTES_FILE);
        let notes = notes::load_notes(&notes_path);

        let mut sms_threads: Vec<(String, String, String)> = fs::read_to_string(SMS_THREADS_FILE)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        // Also import legacy .txt SMS files from disk if present
        if let Ok(entries) = fs::read_dir(SMS_DIR) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|e| e.to_str()) == Some("txt") {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        let lines: Vec<&str> = content.lines().collect();
                        if lines.len() >= 3 {
                            let item = (
                                lines[0].to_string(),
                                lines[1].to_string(),
                                lines[2].to_string(),
                            );
                            if !sms_threads.contains(&item) {
                                sms_threads.push(item);
                            }
                        }
                    }
                }
            }
        }

        let mut contacts: Vec<(String, String)> = fs::read_to_string(CONTACTS_FILE)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        if let Ok(entries) = fs::read_dir(CONTACTS_DIR) {
            for entry in entries.flatten() {
                if entry.path().file_name().and_then(|f| f.to_str()) == Some("contacts.json") {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let parts: Vec<&str> = content.splitn(2, '\n').collect();
                    if parts.len() == 2 {
                        let c = (parts[0].trim().into(), parts[1].trim().into());
                        if !contacts.contains(&c) {
                            contacts.push(c);
                        }
                    }
                }
            }
        }

        let call_log: Vec<(String, String, String)> = fs::read_to_string(CALLS_FILE)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let calc_history: Vec<(String, String)> = fs::read_to_string(CALC_HISTORY_FILE)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let music_tracks = Self::discover_music_tracks();
        let power_profile = fs::read_to_string("/data/nilos/power_profile")
            .unwrap_or_else(|_| "balanced".into())
            .trim()
            .to_string();

        AppState {
            user_name,
            pin,
            pending_pin: String::new(),
            pin_input: String::new(),
            sms_threads,
            contacts,
            call_log,
            compose_to: String::new(),
            compose_body: String::new(),
            terminal_history: vec![
                "OnuronOS terminal — real command execution (see 'help')".into(),
                "Commands resolve against $PATH and run as child processes.".into(),
            ],
            term: term::TermSession::new("/data".into(), "/data".into()),
            browser: browser::BrowserState::default(),
            settings_cursor: 0,
            files_path: "/data".into(),
            pkg_cursor: 0,
            call_number: String::new(),
            wifi_enabled: true,
            cellular_enabled: true,
            signal_bars: 4,
            term_cwd: "/data".into(),
            calc_val: "0".into(),
            calc_history,
            music_playing: false,
            music_track: 0,
            music_tracks,
            notes,
            notes_path,
            camera_id: 0,
            camera_torch: false,
            camera_captures: 0,
            camera_last_status: "Ready".into(),
            locale: i18n::Locale::Bn,
            screen_reader: false,
            power_profile,
            high_refresh_rate: true,
            brightness: 80,
        }
    }

    fn discover_music_tracks() -> Vec<(String, String, String)> {
        let mut tracks: Vec<(String, String, String)> = fs::read_to_string(PLAYLIST_FILE)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        for dir in [
            "/data/music",
            "/data/media",
            "/storage/music",
            "storage/home/joy/Music",
        ] {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        let ext_l = ext.to_lowercase();
                        if ["mp3", "wav", "ogg", "flac", "m4a"].contains(&ext_l.as_str()) {
                            let stem = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Track")
                                .to_string();
                            let p_str = path.to_string_lossy().to_string();
                            if !tracks.iter().any(|(t, _, _)| t == &stem) {
                                tracks.push((stem, "Local Media".into(), p_str));
                            }
                        }
                    }
                }
            }
        }
        tracks
    }

    fn save_user(&self) {
        let _ = fs::create_dir_all("/data/nilos");
        let _ = fs::write(USER_CONF, &self.user_name);
    }

    fn save_pin(&self) {
        let _ = fs::create_dir_all("/data/nilos");
        if let Err(e) = pin::write_pin_file(PIN_FILE, &self.pin) {
            eprintln!("[nilshell] WARNING: could not persist PIN record: {e}");
        }
    }

    fn mark_oobe_done(&self) {
        let _ = fs::create_dir_all("/data/nilos");
        let _ = fs::write(OOBE_DONE, "done");
    }

    fn save_sms_threads(&self) {
        let _ = fs::create_dir_all(SMS_DIR);
        if let Ok(json) = serde_json::to_vec_pretty(&self.sms_threads) {
            let _ = fs::write(SMS_THREADS_FILE, json);
        }
    }

    fn save_contacts(&self) {
        let _ = fs::create_dir_all(CONTACTS_DIR);
        if let Ok(json) = serde_json::to_vec_pretty(&self.contacts) {
            let _ = fs::write(CONTACTS_FILE, json);
        }
    }

    fn save_call_log(&self) {
        if let Some(parent) = Path::new(CALLS_FILE).parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_vec_pretty(&self.call_log) {
            let _ = fs::write(CALLS_FILE, json);
        }
    }

    fn save_calc_history(&self) {
        let _ = fs::create_dir_all("/data/nilos");
        if let Ok(json) = serde_json::to_vec_pretty(&self.calc_history) {
            let _ = fs::write(CALC_HISTORY_FILE, json);
        }
    }

    fn save_playlist(&self) {
        if let Some(parent) = Path::new(PLAYLIST_FILE).parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_vec_pretty(&self.music_tracks) {
            let _ = fs::write(PLAYLIST_FILE, json);
        }
    }
}

// ─── Sink (stdout only — nilinit already dup2'd the correct console fd) ────────
struct Sink;

impl Sink {
    fn new() -> Self {
        Sink
    }

    fn print(&mut self, text: &str) {
        print!("{}", text);
        let _ = io::stdout().flush();
    }

    fn println(&mut self, text: &str) {
        println!("{}", text);
        let _ = io::stdout().flush();
    }
}

fn read_line() -> String {
    let stdin = io::stdin();
    let mut line = String::new();
    let _ = stdin.lock().read_line(&mut line);
    line.trim().to_string()
}

/// Battery indicator for the status bar and lockscreen.
///
/// The reading comes from `powerd` through the `nilpower` client, not from
/// `/sys/class/power_supply`. The shell is a native application and must not
/// read hardware directly (AGENTS.md §6); going through the service also means
/// the HAL (sysfs, Android Host Bridge, or QEMU) is resolved in one place.
///
/// When `powerd` is unreachable the payload is flagged `is_simulated` and the
/// visible badge is appended, so a placeholder is never mistaken for a reading.
fn get_battery_display() -> String {
    let info = nilpower::PowerClient::battery_info().unwrap_or_default();
    if info.is_simulated {
        format!("🔋 {}%  {}", info.capacity, simulated::badge())
    } else if info.is_charging {
        format!("🔋 {}% ⚡", info.capacity)
    } else {
        format!("🔋 {}%", info.capacity)
    }
}

/// Battery telemetry from `powerd`, for screens that need more than the
/// one-line indicator.
fn get_battery_info() -> nilprotocol::BatteryInfoPayload {
    nilpower::PowerClient::battery_info().unwrap_or_default()
}

/// Wi-Fi and cellular indicators for the status bar.
///
/// Network state comes from `netd` through the `nilnet` client rather than from
/// `/sys/class/net`, for the same reason as the battery: the shell is a native
/// application and the interface naming rules (`wl*`, `rmnet*`, ...) belong to
/// the network service, not to the UI.
///
/// `state.wifi_enabled` / `state.cellular_enabled` remain meaningful as *user
/// intent* (the airplane-mode toggles and the `wifi`/`cell` shell commands), so
/// they are honoured when netd reports no corresponding hardware. When netd does
/// report a real interface, the daemon's view wins and the "(sim)" suffix is
/// dropped.
fn get_network_display(state: &AppState) -> (String, String) {
    let net = nilnet::NetClient::state().unwrap_or_default();

    // A real link of a given type, as reported by netd.
    let real_wifi = net
        .interfaces
        .iter()
        .find(|i| i.conn_type == nilprotocol::ConnectionType::Wifi);
    let real_cell = net
        .interfaces
        .iter()
        .find(|i| i.conn_type == nilprotocol::ConnectionType::Cellular);
    let real_eth = net
        .interfaces
        .iter()
        .find(|i| i.conn_type == nilprotocol::ConnectionType::Ethernet);

    let wifi_str = match real_wifi {
        Some(iface) if iface.carrier_connected => "📶 Wi-Fi".to_string(),
        Some(_) => "✕ Wi-Fi".to_string(),
        None => {
            if real_eth.map(|i| i.carrier_connected).unwrap_or(false) {
                "🌐 Eth".to_string()
            } else if state.wifi_enabled {
                "📶 Wi-Fi".to_string()
            } else {
                "✕ Wi-Fi".to_string()
            }
        }
    };

    let cell_str = match real_cell {
        Some(iface) if iface.carrier_connected => {
            // Report the type netd actually detected, not a hard-coded "5G".
            format!("▂▄▆█ {}", iface.conn_type.label())
        }
        Some(_) => "____ No Svc".to_string(),
        None => {
            if state.cellular_enabled {
                // No modem hardware: `signal_bars` is a user-set demo value and
                // is labelled as such.
                match state.signal_bars {
                    4 => "▂▄▆█".to_string(),
                    3 => "▂▄▆_".to_string(),
                    2 => "▂▄__".to_string(),
                    1 => "▂___".to_string(),
                    _ => "____ No Svc".to_string(),
                }
            } else {
                "✈️ Offline".to_string()
            }
        }
    };

    (wifi_str, cell_str)
}

fn status_bar(state: &AppState) -> String {
    let (wifi_ico, cell_str) = get_network_display(state);
    let batt = get_battery_display();
    format!(
        "{}  {} │ {} │ {} │ {}  {}",
        FG_BLACK_ON_WHITE, wifi_ico, cell_str, state.user_name, batt, R
    )
}

// ─── OOBE Screens ─────────────────────────────────────────────────────────────
fn draw_oobe_welcome(sink: &mut Sink) {
    sink.print(CL);
    sink.println(&format!("{}", FG_CYAN));
    sink.println("  ╭───────────────────────────────────────────────────────╮");
    sink.println("  │                                                       │");
    sink.println("  │         🌟  Welcome to NilOS  🌟                      │");
    sink.println("  │                                                       │");
    sink.println("  │   A lightweight, secure, memory-safe mobile OS       │");
    sink.println("  │   built with Linux LTS + 100% Rust userspace.        │");
    sink.println("  │                                                       │");
    sink.println("  │   This wizard will set up your device in 3 steps:    │");
    sink.println("  │                                                       │");
    sink.println(&format!(
        "  │   {}●{} Step 1: Enter your name                          {}│",
        FG_GREEN, FG_CYAN, FG_CYAN
    ));
    sink.println(&format!(
        "  │   {}○{} Step 2: Set a PIN (for lockscreen)               {}│",
        FG_GRAY, FG_CYAN, FG_CYAN
    ));
    sink.println(&format!(
        "  │   {}○{} Step 3: Done!                                    {}│",
        FG_GRAY, FG_CYAN, FG_CYAN
    ));
    sink.println("  │                                                       │");
    sink.println("  │   Language: English (default) — more coming in v1.1  │");
    sink.println("  │                                                       │");
    sink.println("  ╰───────────────────────────────────────────────────────╯");
    sink.println(&format!("{}", R));
    sink.print(&format!(
        "  {}Press Enter to begin setup...{} ",
        FG_YELLOW, R
    ));
}

fn draw_oobe_name(sink: &mut Sink) {
    sink.print(CL);
    sink.println(&format!("{}", FG_CYAN));
    sink.println("  ╭───────────────────────────────────────────────────────╮");
    sink.println("  │  Step 1 / 3 — What's your name?                      │");
    sink.println("  ├───────────────────────────────────────────────────────┤");
    sink.println("  │                                                       │");
    sink.println("  │   Your name will appear on the lock screen and       │");
    sink.println("  │   in system notifications.                            │");
    sink.println("  │                                                       │");
    sink.println("  │   Examples: Sarkar, Joy, Rina, মোঃ সরকার             │");
    sink.println("  │                                                       │");
    sink.println("  ╰───────────────────────────────────────────────────────╯");
    sink.println(R);
    sink.print(&format!("  {}Enter your name: {}", FG_YELLOW, R));
}

fn draw_oobe_pin(sink: &mut Sink) {
    sink.print(CL);
    sink.println(&format!("{}", FG_CYAN));
    sink.println("  ╭───────────────────────────────────────────────────────╮");
    sink.println("  │  Step 2 / 3 — Set a PIN                               │");
    sink.println("  ├───────────────────────────────────────────────────────┤");
    sink.println("  │                                                       │");
    sink.println("  │   Your PIN protects the lock screen.                  │");
    sink.println("  │   Use 4–8 digits. Store it safely.                    │");
    sink.println("  │                                                       │");
    sink.println("  │   Stored salted + stretched (SHA-256, 100k rounds).   │");
    sink.println("  │                                                       │");
    sink.println("  ╰───────────────────────────────────────────────────────╯");
    sink.println(R);
    sink.print(&format!("  {}Enter PIN (4-8 digits): {}", FG_YELLOW, R));
}

fn draw_oobe_confirm_pin(sink: &mut Sink) {
    sink.print(CL);
    sink.println(&format!("{}", FG_CYAN));
    sink.println("  ╭───────────────────────────────────────────────────────╮");
    sink.println("  │  Confirm your PIN                                     │");
    sink.println("  ╰───────────────────────────────────────────────────────╯");
    sink.println(R);
    sink.print(&format!("  {}Re-enter PIN to confirm: {}", FG_YELLOW, R));
}

fn draw_oobe_done(sink: &mut Sink, name: &str) {
    sink.print(CL);
    sink.println(&format!("{}{}", FG_GREEN, B));
    sink.println("  ╭───────────────────────────────────────────────────────╮");
    sink.println("  │                                                       │");
    sink.println("  │   ✅  Setup Complete!                                  │");
    sink.println("  │                                                       │");
    sink.println(&format!(
        "  │   Welcome, {}!{}",
        name,
        " ".repeat(43usize.saturating_sub(name.len()))
    ));
    sink.println("  │   Your NilOS device is ready.                        │");
    sink.println("  │                                                       │");
    sink.println("  │   Your data is stored in:                            │");
    sink.println("  │   • /data/nilos/    — system config                  │");
    sink.println("  │   • /data/app/      — installed packages             │");
    sink.println("  │   • /data/contacts/ — contact list                   │");
    sink.println("  │   • /data/sms/      — message threads                │");
    sink.println("  │                                                       │");
    sink.println("  ╰───────────────────────────────────────────────────────╯");
    sink.println(R);
    sink.print(&format!(
        "  {}Press Enter to continue to lock screen...{} ",
        FG_CYAN, R
    ));
}

// ─── Lock Screen ──────────────────────────────────────────────────────────────
fn draw_lockscreen(sink: &mut Sink, state: &AppState, error: bool) {
    let (time_12h, _time_24h, date_str) = live_clock_and_date();
    sink.print(CL);
    sink.println(&format!("{}", FG_BLUE));
    sink.println("  ╭────────────────────────────────────────────────────────╮");
    sink.println("  │                                                        │");
    sink.println(&format!(
        "  │               {}{}{}{}                            │",
        FG_WHITE, B, time_12h, FG_BLUE
    ));
    sink.println(&format!("  │         {:<46} │", date_str));
    sink.println(&format!(
        "  │           OnuronOS Secured  •  {}               │",
        get_battery_display()
    ));
    sink.println("  │                                                        │");
    sink.println("  ├────────────────────────────────────────────────────────┤");
    sink.println("  │                                                        │");
    sink.println(&format!(
        "  │   🔒 {}{}  {}{}                                    │",
        FG_WHITE, B, state.user_name, FG_BLUE
    ));
    sink.println("  │                                                        │");
    if error {
        sink.println(&format!(
            "  │   {}⚠ Wrong PIN — try again{}                           │",
            FG_RED, FG_BLUE
        ));
    } else {
        sink.println("  │                                                        │");
    }
    sink.println("  ╰────────────────────────────────────────────────────────╯");
    sink.println(R);
    sink.print(&format!("  {}Enter PIN to unlock: {}", FG_YELLOW, R));
}

// ─── Home Launcher ────────────────────────────────────────────────────────────
fn draw_home(sink: &mut Sink, state: &AppState) {
    let (time_12h, time_24h, date_str) = live_clock_and_date();
    let batt = get_battery_display();
    sink.print(CL);
    sink.println(&status_bar(state));
    let wifi_badge = if state.wifi_enabled {
        "📶 Wi-Fi"
    } else {
        "✕ Wi-Fi"
    };
    let cell_badge = if state.cellular_enabled {
        match state.signal_bars {
            4 => "▂▄▆█ 5G",
            3 => "▂▄▆_ 5G",
            2 => "▂▄__ 4G",
            1 => "▂___ 3G",
            _ => "____ No Svc",
        }
    } else {
        "✈️ Offline"
    };
    sink.println("  ╭────────────────────────────────────────────────────────╮");
    sink.println(&format!(
        "  │  {}  {}    {}{}{}{}    🔔  {}       │",
        wifi_badge, cell_badge, FG_WHITE, B, time_12h, FG_CYAN, batt
    ));
    sink.println("  ├────────────────────────────────────────────────────────┤");
    sink.println("  │                                                        │");
    sink.println(&format!(
        "  │                   {}{}{}{}                            │",
        FG_WHITE, B, time_24h, FG_CYAN
    ));
    sink.println(&format!("  │            {:<43} │", date_str));
    sink.println("  │                                                        │");
    sink.println(&format!(
        "  │   {}┌────────────────────────────────────────────────┐{}   │",
        FG_BLUE, FG_CYAN
    ));
    sink.println(&format!(
        "  │   {}│  🔍  Search ('search <query>')                 │{}   │",
        FG_WHITE, FG_CYAN
    ));
    sink.println(&format!(
        "  │   {}└────────────────────────────────────────────────┘{}   │",
        FG_BLUE, FG_CYAN
    ));
    sink.println("  │                                                        │");
    sink.println(&format!(
        "  │  {}[1] 📞 Phone{}      {}[2] 💬 Messages{}     {}[3] 📁 Files{}      │",
        FG_GREEN, FG_CYAN, FG_YELLOW, FG_CYAN, FG_BLUE, FG_CYAN
    ));
    sink.println("  │   VoLTE Dialer     Encrypted SMS     File Manager     │");
    sink.println("  │                                                        │");
    sink.println(&format!(
        "  │  {}[4] ⚙️  Settings{}  {}[5] 📦 NilPkg{}    {}[6] 🔄 SoftBus{}    │",
        FG_MAGENTA, FG_CYAN, FG_GREEN, FG_CYAN, FG_CYAN, FG_CYAN
    ));
    sink.println("  │   System Config    Package Manager   Mesh Network     │");
    sink.println(&format!(
        "  │  {}[7] 🤖 Android{}   {}[8] 💻 Terminal{}   {}[9] 🌟 Hello (.nilax){}│",
        FG_YELLOW, FG_CYAN, FG_WHITE, FG_CYAN, FG_GREEN, FG_CYAN
    ));
    sink.println("  │   AOSP Container   Diagnostic CLI    Native Sandbox   │");
    sink.println("  │                                                        │");
    sink.println(&format!(
        "  │  {}[10] 🧮 Calculator{}  {}[11] 📝 Notes{}   {}[12] 🎵 Music{}      │",
        FG_CYAN, FG_CYAN, FG_YELLOW, FG_CYAN, FG_GREEN, FG_CYAN
    ));
    sink.println("  │   Alap Math        NilLang Notebook  Sonic Lab Player │");
    sink.println("  │                                                        │");
    sink.println(&format!(
        "  │  {}[13] 📷 Camera{}    {}[14] 🌐 Browser{}                     │",
        FG_MAGENTA, FG_CYAN, FG_BLUE, FG_CYAN
    ));
    sink.println("  │   HDR Cam Subsystem  Text HTTP Client                 │");
    sink.println("  │                                                        │");
    sink.println(&format!(
        "  │  {}[n] 🔔 Notifications  [l] 🔒 Lock screen  [run <id>]{}   │",
        FG_GRAY, FG_CYAN
    ));
    sink.println("  ├────────────────────────────────────────────────────────┤");
    sink.println(&format!(
        "  │  {}[📞]  [💬]  [📁]  [⚙️]  [📦]  [🧮]  [📝]  [🎵]{}           │",
        FG_WHITE, FG_CYAN
    ));
    sink.println(&format!(
        "  │                    {}━━━━━━━━{}                             │",
        FG_WHITE, FG_CYAN
    ));
    sink.println("  ╰────────────────────────────────────────────────────────╯");
    sink.println(R);
    sink.print(&format!(
        "  {}Choice (1-14 / search <q> / run <id> / n / l): {}",
        FG_YELLOW, R
    ));
}

// ─── Phone App ────────────────────────────────────────────────────────────────
fn draw_phone(sink: &mut Sink, state: &AppState) {
    sink.print(CL);
    let tel_sock_live = Path::new("/run/nilos/telephony.sock").exists();
    let header_badge = if tel_sock_live {
        "[LIVE TELEPHONYD]"
    } else {
        simulated::badge()
    };
    sink.println(&format!(
        "{}  📞 OnuronOS Phone  —  telephonyd VoLTE + Contacts  {}{}",
        FG_GREEN, header_badge, R
    ));
    sink.println(&format!(
        "{}  ──────────────────────────────────────────{}",
        FG_GRAY, R
    ));
    sink.println("");

    if !state.call_number.is_empty() {
        sink.println(&format!(
            "  {}Dialing: {}{}{}",
            FG_GREEN, FG_WHITE, state.call_number, R
        ));
    } else {
        sink.println(&format!(
            "  {}[ Enter phone number or contact name below ]{}",
            FG_YELLOW, R
        ));
    }

    sink.println("");
    sink.println("  ┌─────────┬─────────┬─────────┐");
    sink.println("  │  1      │  2 ABC  │  3 DEF  │");
    sink.println("  ├─────────┼─────────┼─────────┤");
    sink.println("  │  4 GHI  │  5 JKL  │  6 MNO  │");
    sink.println("  ├─────────┼─────────┼─────────┤");
    sink.println("  │  7 PQRS │  8 TUV  │  9 WXYZ │");
    sink.println("  ├─────────┼─────────┼─────────┤");
    sink.println("  │    *    │  0  +   │    #    │");
    sink.println("  └─────────┴─────────┴─────────┘");
    sink.println("");
    sink.println("  Recent Calls (/data/calls/call_log.json):");
    if state.call_log.is_empty() {
        sink.println(&format!("    {}(No recent calls recorded){}", FG_GRAY, R));
    } else {
        for (peer, num, ts) in state.call_log.iter().take(6) {
            sink.println(&format!(
                "    {}• {:<14} {:<16} {}{}",
                FG_GRAY, peer, num, ts, R
            ));
        }
    }
    sink.println("");
    sink.println("  Contacts (/data/contacts):");
    if state.contacts.is_empty() {
        sink.println(&format!(
            "    {}(No contacts saved yet — use 'add <name> | <number>'){}",
            FG_GRAY, R
        ));
    } else {
        for (name, number) in &state.contacts {
            sink.println(&format!("    {}• {} — {}{}", FG_GREEN, name, number, R));
        }
    }
    sink.println("");
    sink.println(&format!(
        "  {}Commands: 'call <num>', 'add <name> | <num>', 'del <name>', 'clear', 'home', 'back'{}",
        FG_YELLOW, R
    ));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── Messages App ─────────────────────────────────────────────────────────────
fn draw_messages(sink: &mut Sink, state: &AppState) {
    sink.print(CL);
    sink.println(&format!(
        "{}  💬 OnuronOS Messages  —  Persistent SMS Store (/data/sms){}",
        FG_YELLOW, R
    ));
    sink.println(&format!(
        "{}  ──────────────────────────────────────────{}",
        FG_GRAY, R
    ));
    sink.println("");
    sink.println("  Message Threads:");
    if state.sms_threads.is_empty() {
        sink.println(&format!(
            "  {}(No message threads yet — type 'new' to compose){}",
            FG_GRAY, R
        ));
        sink.println("");
    } else {
        for (i, (contact, msg, time)) in state.sms_threads.iter().enumerate() {
            sink.println(&format!(
                "  {}[{}]{} {} {}({}){}\n       {}…{}",
                FG_CYAN,
                i + 1,
                R,
                contact,
                FG_GRAY,
                time,
                R,
                FG_GRAY,
                R
            ));
            sink.println(&format!("       {}", msg));
            sink.println("");
        }
    }
    sink.println(&format!(
        "  {}Commands: 'new', 'read <n>', 'del <n>', 'home', 'back'{}",
        FG_YELLOW, R
    ));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── Files App ────────────────────────────────────────────────────────────────
fn draw_files(sink: &mut Sink, state: &AppState) {
    sink.print(CL);
    sink.println(&format!("{}  📁 NilOS File Manager{}", FG_BLUE, R));
    sink.println(&format!("{}  Path: {}{}", FG_GRAY, state.files_path, R));
    sink.println(&format!(
        "{}  ──────────────────────────────────────────{}",
        FG_GRAY, R
    ));
    sink.println("");

    if let Ok(entries) = fs::read_dir(&state.files_path) {
        let mut items: Vec<String> = entries
            .flatten()
            .map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                if is_dir {
                    format!("📁 {}/", name)
                } else {
                    let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                    format!("📄 {}  ({} bytes)", name, size)
                }
            })
            .collect();
        items.sort();
        for item in &items {
            sink.println(&format!("    {}", item));
        }
        if items.is_empty() {
            sink.println(&format!("    {}(empty directory){}", FG_GRAY, R));
        }
    } else {
        sink.println(&format!("    {}Cannot read directory{}", FG_RED, R));
    }

    sink.println("");
    sink.println("  Bookmarks: [1] /data  [2] /data/app  [3] /etc  [4] /tmp");
    sink.println(&format!(
        "  {}Commands: 'cd <path>', '1'-'4' bookmarks, 'home', 'back'{}",
        FG_YELLOW, R
    ));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── Settings App ────────────────────────────────────────────────────────────
fn draw_settings(sink: &mut Sink) {
    sink.print(CL);
    sink.println(&format!("{}  ⚙️  NilOS System Settings{}", FG_MAGENTA, R));
    sink.println(&format!(
        "{}  ──────────────────────────────────────────{}",
        FG_GRAY, R
    ));
    sink.println("");

    let sections = [
        ("🌐", "Network", "Wi-Fi, Bluetooth, SoftBus, Data"),
        ("🔊", "Sound", "Volume, Route, Mute via audiod"),
        ("📱", "Display", "Brightness, Refresh Rate, DRM/KMS"),
        ("🔒", "Security", "PIN Change, SELinux Policy, fscrypt"),
        ("🔋", "Battery", "Live Sysfs Telemetry & Power Governor"),
        ("📦", "Storage", "Live /data Directory Usage & Packages"),
        (
            "♿",
            "Accessibility",
            "Bengali/English Locale & Screen Reader",
        ),
        ("ℹ️ ", "About", "Live Kernel, Uptime & Architecture"),
    ];

    for (i, (icon, name, desc)) in sections.iter().enumerate() {
        sink.println(&format!(
            "  {}[{}]{} {} {}{}  {}{}",
            FG_CYAN,
            i + 1,
            R,
            icon,
            FG_WHITE,
            name,
            FG_GRAY,
            R
        ));
        sink.println(&format!("       {}{}{}", FG_GRAY, desc, R));
        sink.println("");
    }

    sink.println(&format!(
        "  {}  OS: OnuronOS 1.0 (Phase 3 Active){}",
        FG_GRAY, R
    ));
    sink.println(&format!(
        "  {}  SELinux: {} · Telemetry: ZERO{}",
        FG_GRAY,
        get_selinux_status(),
        R
    ));
    sink.println("");
    sink.println(&format!(
        "  {}Commands: '1'-'8' to enter section, 'home', 'back'{}",
        FG_YELLOW, R
    ));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

fn draw_settings_section(sink: &mut Sink, section: usize, state: &AppState) {
    let names = [
        "Network",
        "Sound",
        "Display",
        "Security",
        "Battery",
        "Storage",
        "Accessibility",
        "About",
    ];
    let name = names.get(section).copied().unwrap_or("Unknown");
    sink.print(CL);
    sink.println(&format!("{}  ⚙️  Settings › {}{}", FG_MAGENTA, name, R));
    sink.println(&format!(
        "{}  ──────────────────────────────────────────{}",
        FG_GRAY, R
    ));
    sink.println("");
    match section {
        0 => {
            sink.println(&format!(
                "  Wi-Fi:       [{}]  (Type 'toggle wifi' or 'wifi' to toggle)",
                if state.wifi_enabled { "ON" } else { "OFF" }
            ));
            sink.println(&format!(
                "  Mobile Data: [{}]  (Type 'toggle data' or 'data' to toggle)",
                if state.cellular_enabled { "ON" } else { "OFF" }
            ));
            sink.println(&format!(
                "  Signal:      [{}/4 Bars] (Type 'signal 1'-'signal 4' to adjust)",
                state.signal_bars
            ));
            sink.println(&format!(
                "  Bluetooth:   [btd socket: {}]",
                get_socket_status("/run/nilos/bt.sock")
            ));
            sink.println(&format!(
                "  Telephony:   [telephonyd socket: {}]",
                get_socket_status("/run/nilos/telephony.sock")
            ));
            sink.println(&format!(
                "  SoftBus:     [bus socket: {}]",
                get_socket_status("/run/nilos/bus.sock")
            ));
        }
        1 => {
            let status =
                nilaudio::AudioClient::get_status().unwrap_or(nilprotocol::AudioStatusPayload {
                    volume: 75,
                    is_muted: false,
                    sink_name: "Speaker (ALSA Default)".into(),
                });
            sink.println(&format!("  Sound Output:    [{}]", status.sink_name));
            sink.println(&format!(
                "  Master Volume:   [{}%] {}",
                status.volume,
                if status.is_muted { "[MUTED]" } else { "" }
            ));
            sink.println(&format!(
                "  Audio Daemon:    [socket: {}]",
                get_socket_status("/run/onuron/audio.sock")
            ));
            sink.println(&format!("  {}Commands: 'vol <0-100>', 'mute', 'route <speaker|earpiece|headset|bluetooth>'{}", FG_CYAN, R));
        }
        2 => {
            let drm_status = if Path::new("/dev/dri/card0").exists() {
                "DRM/KMS Direct (/dev/dri/card0) [HARDWARE]".to_string()
            } else {
                format!("DRM/KMS Direct (Software Fallback) {}", simulated::badge())
            };
            sink.println(&format!("  Compositor:      {}", drm_status));
            sink.println(&format!(
                "  Refresh Rate:    {} (Dynamic tear-free triple buffering)",
                if state.high_refresh_rate {
                    "120Hz"
                } else {
                    "60Hz"
                }
            ));
            sink.println(&format!("  Brightness:      [{}%]", state.brightness));
            sink.println(&format!(
                "  Touch Input:     [socket: {}]",
                get_socket_status("/run/nilos/input.sock")
            ));
            sink.println(&format!(
                "  {}Commands: '120hz', '60hz', 'bright <10-100>'{}",
                FG_CYAN, R
            ));
        }
        3 => {
            sink.println(&format!(
                "  PIN Lock:        [{}]",
                if state.pin.is_empty() {
                    "UNSET"
                } else {
                    "ENABLED (Salted SHA-256 100k)"
                }
            ));
            sink.println(&format!("  SELinux Policy:  {}", get_selinux_status()));
            sink.println(&format!("  fscrypt v2:      {}", get_fscrypt_status()));
            sink.println(
                "  Namespace Sand:  Enabled for all apps via nilrt (CLONE_NEWPID|CLONE_NEWNS)",
            );
            sink.println("  Telemetry:       ZERO — no data leaves device");
            sink.println(&format!(
                "  {}Commands: 'pin <new 4-8 digit PIN>' to update device PIN{}",
                FG_CYAN, R
            ));
        }
        4 => {
            // Telemetry comes from powerd via nilpower; the shell does not read
            // /sys/class/power_supply itself.
            let info = get_battery_info();
            let sim = if info.is_simulated {
                format!("  {}", simulated::badge())
            } else {
                String::new()
            };
            sink.println(&format!("  Battery Level:   🔋 {}%{}", info.capacity, sim));
            sink.println(&format!("  Charge Status:   {}", info.status));
            sink.println(&format!("  Voltage:         {} mV", info.voltage_mv));
            sink.println(&format!("  Temperature:     {:.1} °C", info.temp_c));
            sink.println(&format!("  Battery Health:  {}", info.health));
            sink.println(&format!(
                "  Power Profile:   [{}]",
                state.power_profile.to_uppercase()
            ));
            sink.println(&format!(
                "  Power Daemon:    [{}]",
                match nilpower::PowerClient::socket_path() {
                    Some(p) => format!("socket: {p}"),
                    None => "OFFLINE — telemetry unavailable".to_string(),
                }
            ));
            sink.println(&format!(
                "  {}Commands: 'profile <balanced|powersave|performance>'{}",
                FG_CYAN, R
            ));
        }
        5 => {
            let (data_files, data_bytes) = directory_usage_summary(Path::new("/data"));
            let (tmp_files, tmp_bytes) = directory_usage_summary(Path::new("/tmp"));
            let pkgs = get_installed_packages();
            sink.println(&format!(
                "  /data Usage:     {} files ({} bytes)",
                data_files, data_bytes
            ));
            sink.println(&format!(
                "  /tmp Usage:      {} files ({} bytes)",
                tmp_files, tmp_bytes
            ));
            sink.println(&format!(
                "  Installed Apps:  {} (.nilax packages in {})",
                pkgs.len(),
                nilpkg::get_app_dir().display()
            ));
            sink.println(&format!("  Notes Stored:    {} notes", state.notes.len()));
            sink.println(&format!(
                "  SMS Threads:     {} threads",
                state.sms_threads.len()
            ));
        }
        6 => {
            sink.println(&format!(
                "  Active Locale:   [{}] ({})",
                state.locale.as_str(),
                i18n::t("app_launcher", state.locale)
            ));
            sink.println(&format!(
                "  Screen Reader:   [{}]",
                if state.screen_reader {
                    "ENABLED"
                } else {
                    "DISABLED"
                }
            ));
            let sample_node = i18n::AccessibilityNode::new(
                i18n::AccessibilityRole::Button,
                i18n::t("cmd_save", state.locale),
            );
            sink.println(&format!(
                "  TTS Preview:     {}",
                sample_node.screen_reader_text(state.locale)
            ));
            sink.println(&format!(
                "  {}Commands: 'lang bn', 'lang en', 'sr on', 'sr off'{}",
                FG_CYAN, R
            ));
        }
        7 => {
            let kernel_rel = fs::read_to_string("/proc/sys/kernel/osrelease")
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| "Linux 6.6.110-lts (host/virtual)".into());
            let uptime_s = fs::read_to_string("/proc/uptime")
                .ok()
                .and_then(|s| s.split_whitespace().next().map(|v| format!("{}s", v)))
                .unwrap_or_else(|| "active".into());
            sink.println("  OnuronOS Ver:    1.0.0 (Phase 3 Active)");
            sink.println(&format!("  Kernel:          {}", kernel_rel));
            sink.println("  Userspace:       100% Memory-Safe Rust (nilinit PID 1)");
            sink.println(&format!("  Architecture:    {}", std::env::consts::ARCH));
            sink.println(&format!("  System Uptime:   {}", uptime_s));
            sink.println("  License:         GNU AGPLv3");
        }
        _ => {}
    }
    sink.println("");
    sink.println(&format!(
        "  {}Press 'back' to return to Settings, 'home' for launcher{}",
        FG_YELLOW, R
    ));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

fn get_socket_status(path: &str) -> String {
    if Path::new(path).exists() {
        "Active (Live Daemon Socket)".into()
    } else {
        format!("Offline / Simulated {}", simulated::badge())
    }
}

fn get_selinux_status() -> String {
    if let Ok(val) = fs::read_to_string("/sys/fs/selinux/enforce") {
        match val.trim() {
            "1" => "Enforcing (Live Kernel Query)".into(),
            "0" => "Permissive (Live Kernel Query)".into(),
            _ => "Unknown (Kernel Query Error)".into(),
        }
    } else {
        format!("Simulated (Unverified Kernel State) {}", simulated::badge())
    }
}

fn get_fscrypt_status() -> String {
    let p = Path::new("/data/.fscrypt");
    if p.exists() {
        "Active (fscrypt v2 keyring policy mounted)".into()
    } else {
        format!(
            "Simulated (Unverified Storage Policy) {}",
            simulated::badge()
        )
    }
}

fn find_binary(bin_name: &str) -> std::path::PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(bin_name);
            if candidate.is_file() {
                return candidate;
            }
            #[cfg(target_os = "windows")]
            {
                let candidate_exe = dir.join(format!("{}.exe", bin_name));
                if candidate_exe.is_file() {
                    return candidate_exe;
                }
            }
        }
    }
    let target_debug = std::path::Path::new("target").join("debug").join(bin_name);
    if target_debug.is_file() {
        return target_debug;
    }
    #[cfg(target_os = "windows")]
    {
        let target_debug_exe = std::path::Path::new("target")
            .join("debug")
            .join(format!("{}.exe", bin_name));
        if target_debug_exe.is_file() {
            return target_debug_exe;
        }
    }
    std::path::PathBuf::from(bin_name)
}

fn launch_app(sink: &mut Sink, app_id: &str) {
    sink.println(&format!(
        "\n  {}🚀 Launching {} via nilrt sandbox...{}",
        FG_GREEN, app_id, R
    ));
    let launcher = find_binary("nilrt-launch");
    match std::process::Command::new(&launcher).arg(app_id).status() {
        Ok(status) => {
            sink.println(&format!(
                "  {}Application finished with exit status: {}{}",
                FG_GREEN, status, R
            ));
        }
        Err(e) => {
            sink.println(&format!(
                "  {}Failed to run nilrt-launch ({}): {}{}",
                FG_RED,
                launcher.display(),
                e,
                R
            ));
        }
    }
    sink.print("  Press Enter to continue...");
    read_line();
}

fn get_installed_packages() -> Vec<(String, String)> {
    let mut pkgs = Vec::new();
    let app_root = nilpkg::get_app_dir();
    if let Ok(entries) = std::fs::read_dir(&app_root) {
        for entry in entries.flatten() {
            let manifest_path = entry.path().join("manifest.json");
            if manifest_path.is_file() {
                if let Ok(text) = std::fs::read_to_string(&manifest_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                        let name = val
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        let version = val
                            .get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("1.0.0")
                            .to_string();
                        let id = val
                            .get("app_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        pkgs.push((id, format!("{} v{}", name, version)));
                    }
                }
            }
        }
    }
    pkgs
}

// ─── NilPkg App ──────────────────────────────────────────────────────────────
fn draw_nilpkg(sink: &mut Sink) {
    sink.print(CL);
    sink.println(&format!(
        "{}  📦 NilPkg — Atomic Signed Package Manager (.nilax){}",
        FG_GREEN, R
    ));
    sink.println(&format!(
        "{}  ──────────────────────────────────────────{}",
        FG_GRAY, R
    ));
    sink.println("");
    sink.println(&format!(
        "  {}Installed Native Packages (.nilax):{}",
        FG_WHITE, R
    ));
    let app_dir = nilpkg::get_app_dir();
    let pkgs = get_installed_packages();
    if pkgs.is_empty() {
        sink.println(&format!(
            "    {}No third-party packages installed in {}{}",
            FG_GRAY,
            app_dir.display(),
            R
        ));
        sink.println(&format!(
            "    {}Run: 'install <path.nilax>' to install a signed package{}",
            FG_YELLOW, R
        ));
    } else {
        for (id, desc) in &pkgs {
            sink.println(&format!("    {}• {} ({}){}", FG_GREEN, id, desc, R));
        }
    }
    sink.println("");
    sink.println(&format!("  {}Core System Modules:{}", FG_WHITE, R));
    sink.println(&format!(
        "    {}• com.nil.shell     v1.0.0  — OnuronOS Compositor & Shell{}",
        FG_GREEN, R
    ));
    sink.println(&format!(
        "    {}• com.nil.nilinit   v1.0.0  — PID 1 Supervisor{}",
        FG_GREEN, R
    ));
    sink.println(&format!(
        "    {}• com.nil.nilhal    v1.0.0  — Hardware Abstraction Layer{}",
        FG_GREEN, R
    ));
    sink.println(&format!(
        "    {}• com.nil.nilrt     v1.0.0  — Sandboxed Native Runtime{}",
        FG_GREEN, R
    ));
    sink.println("");
    sink.println(&format!(
        "  {}Storage: {}  •  Keys: {}{}",
        FG_GRAY,
        app_dir.display(),
        nilpkg::get_key_dir().display(),
        R
    ));
    sink.println("");
    sink.println(&format!("  {}Commands: 'install <path>', 'upgrade <dir>', 'rollback <id>', 'uninstall <id>', 'verify <id>', 'trust <pub>', 'revoke <pub>', 'run <id>', 'list', 'home'{}", FG_YELLOW, R));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── SoftBus App ─────────────────────────────────────────────────────────────
fn draw_softbus(sink: &mut Sink) {
    sink.print(CL);
    let sock_live = nilbus_client::SoftBusClient::is_daemon_active();
    let badge_str = if sock_live {
        "[LIVE DAEMON]"
    } else {
        simulated::badge()
    };
    sink.println(&format!(
        "{}  🔄 SoftBus — Distributed Peer Mesh Engine  {}{}",
        FG_CYAN, badge_str, R
    ));
    sink.println(&format!(
        "{}  ──────────────────────────────────────────{}",
        FG_GRAY, R
    ));
    sink.println("");

    if sock_live {
        sink.println(&format!(
            "  {}Control Socket: /run/nilos/bus.sock [ACTIVE]{}",
            FG_GREEN, R
        ));
        sink.println("  mDNS-SD Discovery: Running on local network (QUIC TLS 1.3)");
    } else {
        sink.println(&format!(
            "  {}Control Socket: /run/nilos/bus.sock [OFFLINE — using persistent peer registry]{}",
            FG_YELLOW, R
        ));
    }
    sink.println("");
    sink.println(&format!("  {}Known / Discovered Peers:{}", FG_WHITE, R));
    match nilbus_client::SoftBusClient::get_nearby_peers() {
        Ok(peers) if !peers.is_empty() => {
            for p in peers {
                sink.println(&format!(
                    "    {}● {}  ({}:{}){}",
                    FG_GREEN, p.device_id, p.addr, p.quic_port, R
                ));
            }
        }
        _ => {
            sink.println(&format!(
                "    {}(No active or paired peers — use 'pair <device_id> <ip:port>' or 'scan'){}",
                FG_GRAY, R
            ));
        }
    }
    sink.println("");
    sink.println(&format!("  {}Commands: 'scan', 'pair <id> <ip:port>', 'unpair <id>', 'ping <id>', 'handoff <id> <app> <data>', 'home', 'back'{}", FG_YELLOW, R));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── Android App ─────────────────────────────────────────────────────────────
fn draw_android(sink: &mut Sink) {
    sink.print(CL);
    sink.println(&format!(
        "{}  🤖 Android Compatibility Layer (Track C){}",
        FG_YELLOW, R
    ));
    sink.println(&format!(
        "{}  ──────────────────────────────────────────{}",
        FG_GRAY, R
    ));
    sink.println("");

    // Query the real daemon. If it is not running we say so; we never invent a
    // container state.
    match nilandroid_client::AndroidBridge::status() {
        Ok(st) => {
            let (badge, color) = if st.is_running() {
                ("RUNNING", FG_GREEN)
            } else {
                ("STOPPED", FG_YELLOW)
            };
            sink.println(&format!("  {}Container: {}{}{}", FG_WHITE, color, badge, R));
            sink.println(&format!("  {}", st.status_line));
            if let Some(pid) = st.pid {
                sink.println(&format!("  • init PID:          {}", pid));
            }
            sink.println(&format!("  • rootfs:            {}", st.rootfs));
        }
        Err(e) => {
            sink.println(&format!("  {}Container: UNAVAILABLE{}", FG_RED, R));
            sink.println(&format!("  {}bridge error: {}{}", FG_GRAY, e, R));
            sink.println(&format!(
                "  {}Install a container rootfs, then start nilandroidd.{}",
                FG_GRAY, R
            ));
        }
    }

    sink.println("");
    sink.println("  • Isolation:         PID/mount/IPC/UTS/NET namespaces + cgroup v2");
    sink.println("  • Architecture Note: Android runs as an isolated guest, never as");
    sink.println("                       part of the native Onuron platform spine.");
    sink.println("");
    sink.println(&format!(
        "  {}IPC Socket: /run/nilos/android.sock{}",
        FG_GRAY, R
    ));
    sink.println("");
    sink.println(&format!(
        "  {}Commands: 'status', 'start', 'stop', 'open <uri>', 'home', 'back'{}",
        FG_YELLOW, R
    ));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── Browser App ─────────────────────────────────────────────────────────────
fn draw_browser(sink: &mut Sink, state: &AppState) {
    sink.print(CL);
    sink.println(&format!(
        "{}  🌐 Browser — real HTTP/HTTPS fetch (text mode){}",
        FG_WHITE, R
    ));
    sink.println(&format!(
        "{}  ──────────────────────────────────────────{}",
        FG_GRAY, R
    ));

    if let Some(err) = &state.browser.error {
        sink.println(&format!("  {}✕ {}{}", FG_RED, err, R));
        sink.println(&format!(
            "  {}Check the network: is netd up and is the interface configured?{}",
            FG_GRAY, R
        ));
    } else if let Some(page) = &state.browser.page {
        sink.println(&format!(
            "  {}[{}]{} {}",
            FG_GREEN, page.status, R, page.title
        ));
        sink.println(&format!(
            "  {}{}{}{}",
            FG_GRAY,
            page.final_url,
            if page.truncated { " [truncated]" } else { "" },
            R
        ));
        sink.println(&format!(
            "{}  ──────────────────────────────────────────{}",
            FG_GRAY, R
        ));
        // Show a window of the page, starting at the scroll offset.
        let view_height = 16;
        let start = state.browser.scroll.min(page.lines.len());
        let end = (start + view_height).min(page.lines.len());
        for line in &page.lines[start..end] {
            sink.println(&format!("  {}", line));
        }
        if end < page.lines.len() {
            sink.println(&format!(
                "  {}… {} more lines (press 'd' to scroll){}",
                FG_GRAY,
                page.lines.len() - end,
                R
            ));
        }
    } else {
        sink.println("  Type an address to begin.");
        sink.println("  A bare word is treated as a search query.");
    }

    sink.println(&format!(
        "{}  ──────────────────────────────────────────{}",
        FG_GRAY, R
    ));
    sink.println(&format!("  Status: {}", state.browser.status));
    sink.println(&format!(
        "  {}Enter a URL, or: 'd' down  'u' up  'top'  'home'  'back'{}",
        FG_GRAY, R
    ));
    sink.print(&format!("  {}url> {}", FG_CYAN, R));
}

// ─── Terminal App ─────────────────────────────────────────────────────────────
fn draw_terminal(sink: &mut Sink, state: &AppState) {
    sink.print(CL);
    sink.println(&format!(
        "{}  💻 OnuronOS terminal — real command execution{}",
        FG_WHITE, R
    ));
    sink.println(&format!(
        "{}  Commands run as child processes. 'help' lists builtins, 'exit' closes.{}",
        FG_GRAY, R
    ));
    sink.println("");
    for line in &state.terminal_history {
        sink.println(&format!("  {}", line));
    }
    sink.println("");
    sink.print(&format!("  {}nilos:{}$ {}", FG_GREEN, state.term_cwd, R));
}

fn eval_simple_math(expr: &str) -> Result<String, ()> {
    let normalized = i18n::to_ascii_digits(expr).replace("π", "pi");
    let chars: Vec<char> = normalized.chars().filter(|c| !c.is_whitespace()).collect();
    if chars.is_empty() {
        return Err(());
    }

    #[derive(Debug, PartialEq, Clone)]
    enum Token {
        Num(f64),
        Ident(String),
        Plus,
        Minus,
        Mul,
        Div,
        Mod,
        Pow,
        LParen,
        RParen,
    }

    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Mul);
                i += 1;
            }
            '/' => {
                tokens.push(Token::Div);
                i += 1;
            }
            '%' => {
                tokens.push(Token::Mod);
                i += 1;
            }
            '^' => {
                tokens.push(Token::Pow);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            c if c.is_ascii_digit() || c == '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                let num = s.parse::<f64>().map_err(|_| ())?;
                tokens.push(Token::Num(num));
            }
            c if c.is_ascii_alphabetic() => {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let ident: String = chars[start..i].iter().collect::<String>().to_lowercase();
                match ident.as_str() {
                    "pi" => tokens.push(Token::Num(std::f64::consts::PI)),
                    "e" => tokens.push(Token::Num(std::f64::consts::E)),
                    "sqrt" | "sin" | "cos" | "tan" | "log" | "ln" | "abs" | "floor" | "ceil"
                    | "round" => {
                        tokens.push(Token::Ident(ident));
                    }
                    _ => return Err(()),
                }
            }
            _ => return Err(()),
        }
    }

    struct Parser {
        tokens: Vec<Token>,
        pos: usize,
    }

    impl Parser {
        fn parse_expr(&mut self) -> Result<f64, ()> {
            let mut left = self.parse_term()?;
            while self.pos < self.tokens.len() {
                match self.tokens[self.pos] {
                    Token::Plus => {
                        self.pos += 1;
                        let right = self.parse_term()?;
                        left += right;
                    }
                    Token::Minus => {
                        self.pos += 1;
                        let right = self.parse_term()?;
                        left -= right;
                    }
                    _ => break,
                }
            }
            Ok(left)
        }

        fn parse_term(&mut self) -> Result<f64, ()> {
            let mut left = self.parse_power()?;
            while self.pos < self.tokens.len() {
                match self.tokens[self.pos] {
                    Token::Mul => {
                        self.pos += 1;
                        let right = self.parse_power()?;
                        left *= right;
                    }
                    Token::Div => {
                        self.pos += 1;
                        let right = self.parse_power()?;
                        if right == 0.0 {
                            return Err(());
                        }
                        left /= right;
                    }
                    Token::Mod => {
                        self.pos += 1;
                        let right = self.parse_power()?;
                        if right == 0.0 {
                            return Err(());
                        }
                        left %= right;
                    }
                    _ => break,
                }
            }
            Ok(left)
        }

        fn parse_power(&mut self) -> Result<f64, ()> {
            let base = self.parse_factor()?;
            if self.pos < self.tokens.len() && self.tokens[self.pos] == Token::Pow {
                self.pos += 1;
                let exp = self.parse_power()?;
                Ok(base.powf(exp))
            } else {
                Ok(base)
            }
        }

        fn parse_factor(&mut self) -> Result<f64, ()> {
            if self.pos >= self.tokens.len() {
                return Err(());
            }
            match self.tokens[self.pos].clone() {
                Token::Plus => {
                    self.pos += 1;
                    self.parse_factor()
                }
                Token::Minus => {
                    self.pos += 1;
                    let val = self.parse_factor()?;
                    Ok(-val)
                }
                Token::Num(n) => {
                    self.pos += 1;
                    Ok(n)
                }
                Token::Ident(func) => {
                    self.pos += 1;
                    if self.pos >= self.tokens.len() || self.tokens[self.pos] != Token::LParen {
                        return Err(());
                    }
                    self.pos += 1;
                    let arg = self.parse_expr()?;
                    if self.pos >= self.tokens.len() || self.tokens[self.pos] != Token::RParen {
                        return Err(());
                    }
                    self.pos += 1;
                    let val = match func.as_str() {
                        "sqrt" => {
                            if arg < 0.0 {
                                return Err(());
                            }
                            arg.sqrt()
                        }
                        "sin" => arg.sin(),
                        "cos" => arg.cos(),
                        "tan" => arg.tan(),
                        "log" => {
                            if arg <= 0.0 {
                                return Err(());
                            }
                            arg.log10()
                        }
                        "ln" => {
                            if arg <= 0.0 {
                                return Err(());
                            }
                            arg.ln()
                        }
                        "abs" => arg.abs(),
                        "floor" => arg.floor(),
                        "ceil" => arg.ceil(),
                        "round" => arg.round(),
                        _ => return Err(()),
                    };
                    Ok(val)
                }
                Token::LParen => {
                    self.pos += 1;
                    let val = self.parse_expr()?;
                    if self.pos < self.tokens.len() && self.tokens[self.pos] == Token::RParen {
                        self.pos += 1;
                        Ok(val)
                    } else {
                        Err(())
                    }
                }
                _ => Err(()),
            }
        }
    }

    let mut parser = Parser { tokens, pos: 0 };
    let res = parser.parse_expr()?;
    if parser.pos != parser.tokens.len() || res.is_nan() || res.is_infinite() {
        return Err(());
    }

    if res.fract().abs() < 1e-10 && res >= (i64::MIN as f64) && res <= (i64::MAX as f64) {
        Ok(format!("{}", res.round() as i64))
    } else {
        let s = format!("{:.4}", res);
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        Ok(trimmed.to_string())
    }
}

/// Execute one terminal command line and append its real output to the
/// scrollback. The actual work happens in `term::run_line`: builtins are
/// handled in-process, everything else is resolved against $PATH and spawned as
/// a child process. Nothing here fabricates output.
fn handle_terminal_cmd(_sink: &mut Sink, state: &mut AppState, cmd: &str) {
    use term::TermOutcome;

    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return;
    }

    let prompt = format!(
        "{} {} $ {}",
        state.user_name,
        state.term.cwd.display(),
        trimmed
    );
    state.terminal_history.push(prompt);

    let outcome = term::run_line(&mut state.term, trimmed);
    state.term_cwd = state.term.cwd.display().to_string();

    match outcome {
        TermOutcome::Silent => {}
        TermOutcome::Exit => {
            state
                .terminal_history
                .push("(terminal session ended; press Enter to reopen)".into());
        }
        TermOutcome::Clear => {
            state.terminal_history.clear();
            return;
        }
        TermOutcome::Output { text, status } => {
            if !text.is_empty() {
                for line in text.lines() {
                    state.terminal_history.push(line.to_string());
                }
            }
            if status != 0 {
                state
                    .terminal_history
                    .push(format!("[exit status {}]", status));
            }
        }
    }

    while state.terminal_history.len() > 200 {
        state.terminal_history.remove(0);
    }
}

// ─── Calculator App ───────────────────────────────────────────────────────────
fn draw_calculator(sink: &mut Sink, state: &AppState) {
    sink.print(CL);
    sink.println(&format!(
        "{}  🧮 OnuronOS Scientific Calculator — Alap Math Engine{}",
        FG_CYAN, R
    ));
    sink.println(&format!(
        "{}  ──────────────────────────────────────────{}",
        FG_GRAY, R
    ));
    sink.println("");
    sink.println("  ┌────────────────────────────────────────────┐");
    sink.println(&format!("  │  Display: {:>32} │", state.calc_val));
    sink.println("  ├────────────────────────────────────────────┤");
    sink.println("  │  [sin]  [cos]  [tan]  [sqrt]  [log]  [ln]  │");
    sink.println("  │   [ 7 ]   [ 8 ]   [ 9 ]   [ / ]   [ ^ ]    │");
    sink.println("  │   [ 4 ]   [ 5 ]   [ 6 ]   [ * ]   [ % ]    │");
    sink.println("  │   [ 1 ]   [ 2 ]   [ 3 ]   [ - ]   [ ( ]    │");
    sink.println("  │   [ 0 ]   [ . ]   [ C ]   [ + ]   [ ) ]    │");
    sink.println("  └────────────────────────────────────────────┘");
    if !state.calc_history.is_empty() {
        sink.println("");
        sink.println("  Recent Calculations:");
        for (expr, res) in state.calc_history.iter().rev().take(4) {
            sink.println(&format!("    {}• {} = {}{}", FG_GRAY, expr, res, R));
        }
    }
    sink.println("");
    sink.println(&format!(
        "  {}Type an expression (e.g. 'sqrt(144) + 2^3', '25 * 4', 'C'), 'home', 'back'{}",
        FG_YELLOW, R
    ));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── Notes App ────────────────────────────────────────────────────────────────
fn draw_notes(sink: &mut Sink, state: &AppState) {
    sink.print(CL);
    sink.println(&format!(
        "{}  📝 NilLang Notes — Persistent Notebook ({}){}",
        FG_YELLOW,
        state.notes_path.display(),
        R
    ));
    sink.println(&format!(
        "{}  ──────────────────────────────────────────{}",
        FG_GRAY, R
    ));
    sink.println("");
    if state.notes.is_empty() {
        sink.println(&format!(
            "  {}(No notes stored — use 'new <title> | <body>' to add one){}",
            FG_GRAY, R
        ));
        sink.println("");
    } else {
        for note in &state.notes {
            let tag_color = match note.priority.as_str() {
                "High" => FG_YELLOW,
                "Normal" => FG_GREEN,
                _ => FG_CYAN,
            };
            sink.println(&format!(
                "  {}[{}] 📌 {}{}",
                FG_WHITE, note.id, note.title, R
            ));
            sink.println(&format!(
                "      {}ট্যাগ: {} • প্রায়োরিটি: {} • তারিখ: {}{}",
                tag_color, note.tag, note.priority, note.date, R
            ));
            sink.println(&format!("      {}{}{}", FG_GRAY, note.body, R));
            sink.println("");
        }
    }
    sink.println(&format!("  {}Commands: 'new <title> | <body>', 'edit <id> <title> | <body>', 'tag <id> <tag>', 'priority <id> <High|Normal|Low>', 'search <q>', 'del <id>', 'home', 'back'{}", FG_YELLOW, R));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── Music App ────────────────────────────────────────────────────────────────
fn draw_music(sink: &mut Sink, state: &AppState) {
    sink.print(CL);
    let audio_st = nilaudio::AudioClient::get_status().unwrap_or(nilprotocol::AudioStatusPayload {
        volume: 75,
        is_muted: false,
        sink_name: "Speaker (ALSA Default)".into(),
    });
    sink.println(&format!(
        "{}  🎵 OnuronOS Music Player — audiod Route: {}  {}{}",
        FG_GREEN,
        audio_st.sink_name,
        simulated::badge(),
        R
    ));
    sink.println(&format!(
        "{}  ──────────────────────────────────────────{}",
        FG_GRAY, R
    ));
    sink.println("");
    let play_icon = if state.music_playing {
        "▶ PLAYING"
    } else {
        "⏸ PAUSED"
    };

    if state.music_tracks.is_empty() {
        sink.println(&format!(
            "  {}No tracks in playlist (/data/music).{}",
            FG_GRAY, R
        ));
        sink.println(&format!(
            "  {}Use 'add <title> | <artist> | <path>' or 'scan' to add tracks.{}",
            FG_YELLOW, R
        ));
    } else {
        let idx = state
            .music_track
            .min(state.music_tracks.len().saturating_sub(1));
        let (cur_title, cur_artist, cur_path) = &state.music_tracks[idx];
        sink.println(&format!("       {}╭─────────────╮{}", FG_CYAN, R));
        sink.println(&format!(
            "       {}│   ◎ ◉ ◎     │{}   Track:  {}{}{}",
            FG_CYAN, R, FG_WHITE, cur_title, R
        ));
        sink.println(&format!(
            "       {}│  AUDIO PCM  │{}   Artist: {}{}{}",
            FG_CYAN, R, FG_YELLOW, cur_artist, R
        ));
        sink.println(&format!(
            "       {}│   ◉ ◎ ◉     │{}   Status: {}{}{}  [Vol: {}%{}]",
            FG_CYAN,
            R,
            FG_GREEN,
            play_icon,
            R,
            audio_st.volume,
            if audio_st.is_muted { " MUTED" } else { "" }
        ));
        sink.println(&format!(
            "       {}╰─────────────╯{}   Source: {}",
            FG_CYAN, R, cur_path
        ));
        sink.println("");
        sink.println("  Playlist Queue:");
        for (i, (t, a, d)) in state.music_tracks.iter().enumerate() {
            let mark = if i == idx { "▶" } else { " " };
            sink.println(&format!(
                "   {} {} [{}] {} — {} ({}){}",
                FG_WHITE,
                mark,
                i + 1,
                t,
                a,
                d,
                R
            ));
        }
    }
    sink.println("");
    sink.println(&format!("  {}Commands: 'play', 'pause', 'next', 'prev', 'vol <0-100>', 'mute', 'route <speaker|headset|bt>', 'add <title> | <artist> | <path>', 'scan', 'home', 'back'{}", FG_YELLOW, R));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── Camera App ───────────────────────────────────────────────────────────────
fn draw_camera(sink: &mut Sink, state: &AppState) {
    sink.print(CL);
    let info =
        nilcam::CameraClient::get_info(state.camera_id).unwrap_or(nilprotocol::CameraInfoPayload {
            camera_id: state.camera_id,
            facing: if state.camera_id == 0 {
                "back".into()
            } else {
                "front".into()
            },
            resolution: "1920x1080".into(),
            torch_active: state.camera_torch,
            preview_active: true,
            supported_formats: vec!["JPEG".into(), "RGBA8888".into()],
        });
    sink.println(&format!(
        "{}  📷 OnuronOS Camera Subsystem — camerad HAL  {}{}",
        FG_MAGENTA,
        simulated::badge(),
        R
    ));
    sink.println(&format!(
        "{}  ──────────────────────────────────────────{}",
        FG_GRAY, R
    ));
    sink.println("");
    sink.println("  ┌────────────────────────────────────────────────────────┐");
    sink.println("  │                      VIEWFINDER                        │");
    sink.println("  │                                                        │");
    sink.println(&format!(
        "  │         [ Camera #{} ({}) • {} ]           │",
        info.camera_id,
        info.facing.to_uppercase(),
        info.resolution
    ));
    sink.println(&format!(
        "  │         Torch: {:<6} • Captures: {:<5}                │",
        if state.camera_torch { "ON" } else { "OFF" },
        state.camera_captures
    ));
    sink.println("  │                                                        │");
    sink.println("  │                       [  ⦿ CAPTURE  ]                  │");
    sink.println("  └────────────────────────────────────────────────────────┘");
    sink.println("");
    sink.println(&format!(
        "  • Active Sensor: Camera #{} ({}) — {}",
        info.camera_id, info.facing, info.resolution
    ));
    sink.println(&format!("  • Formats:       {:?}", info.supported_formats));
    sink.println(&format!("  • Last Action:   {}", state.camera_last_status));
    sink.println("");
    sink.println(&format!(
        "  {}Commands: 'capture', 'torch', 'switch', 'list', 'home', 'back'{}",
        FG_YELLOW, R
    ));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── Notification Shade ──────────────────────────────────────────────────────
fn draw_notifications(sink: &mut Sink, state: &AppState) {
    sink.print(CL);
    sink.println(&format!(
        "{}  🔔 OnuronOS Notification Center (/data/notifications){}",
        FG_YELLOW, R
    ));
    sink.println(&format!(
        "{}  ──────────────────────────────────────────{}",
        FG_GRAY, R
    ));
    sink.println("");
    if Path::new("/run/onuron/data_corrupt").exists() {
        sink.println(&format!("  {}⚠ STORAGE ALERT — nilinit{}", FG_RED, R));
        sink.println(
            "     Persistent block device failed ext4/ext2 mount; running on emergency tmpfs.",
        );
        sink.println("");
    }
    let store_path = nilnotify::NotificationClient::default_store_path();
    let items = nilnotify::NotificationClient::load_from(&store_path);
    if items.is_empty() && state.sms_threads.is_empty() {
        sink.println(&format!("  {}(No active notifications){}", FG_GRAY, R));
        sink.println("");
    } else {
        for item in &items {
            sink.println(&format!(
                "  {}[#{}] 🔔 [{}] {}{}",
                FG_CYAN, item.id, item.app, item.title, R
            ));
            sink.println(&format!("     {}", item.body));
            sink.println("");
        }
        for (contact, msg, time) in &state.sms_threads {
            sink.println(&format!("  {}💬 SMS: {}{}", FG_YELLOW, contact, R));
            sink.println(&format!("     {} — {}", msg, time));
            sink.println("");
        }
    }
    sink.println(&format!(
        "  {}Commands: 'post <title> | <body>', 'dismiss <id>', 'clear', 'home', 'back'{}",
        FG_YELLOW, R
    ));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── Main Loop ────────────────────────────────────────────────────────────────
fn main() {
    let _ = nilsd::notify_ready("nilshell", None);
    let mut sink = Sink::new();
    let mut state = AppState::load();
    let mut settings_in_section: Option<usize> = None;

    // Determine start screen
    let oobe_done = Path::new(OOBE_DONE).exists();
    let mut screen = if !oobe_done {
        Screen::OobeWelcome
    } else {
        Screen::Lockscreen
    };

    let mut lock_error = false;

    loop {
        // Render current screen
        match &screen {
            Screen::OobeWelcome => draw_oobe_welcome(&mut sink),
            Screen::OobeName => draw_oobe_name(&mut sink),
            Screen::OobePin => draw_oobe_pin(&mut sink),
            Screen::OobeConfirmPin => draw_oobe_confirm_pin(&mut sink),
            Screen::OobeDone => draw_oobe_done(&mut sink, &state.user_name),
            Screen::Lockscreen => draw_lockscreen(&mut sink, &state, lock_error),
            Screen::Home => draw_home(&mut sink, &state),
            Screen::AppPhone => draw_phone(&mut sink, &state),
            Screen::AppMessages => draw_messages(&mut sink, &state),
            Screen::AppFiles => draw_files(&mut sink, &state),
            Screen::AppSettings => {
                if let Some(sec) = settings_in_section {
                    draw_settings_section(&mut sink, sec, &state);
                } else {
                    draw_settings(&mut sink);
                }
            }
            Screen::AppNilPkg => draw_nilpkg(&mut sink),
            Screen::AppSoftBus => draw_softbus(&mut sink),
            Screen::AppAndroid => draw_android(&mut sink),
            Screen::AppBrowser => draw_browser(&mut sink, &state),
            Screen::AppTerminal => draw_terminal(&mut sink, &state),
            Screen::AppCalculator => draw_calculator(&mut sink, &state),
            Screen::AppNotes => draw_notes(&mut sink, &state),
            Screen::AppMusic => draw_music(&mut sink, &state),
            Screen::AppCamera => draw_camera(&mut sink, &state),
            Screen::NotificationShade => draw_notifications(&mut sink, &state),
        }

        // Stamp a visible SIMULATED badge on screens from the registry
        // (shell/src/simulated.rs) before reading input.
        simulated::render_badge(&mut sink, simulated::screen_key(&screen));

        let cmd = read_line();

        // ── OOBE flow ─────────────────────────────────────────────────────────
        match screen.clone() {
            Screen::OobeWelcome => {
                screen = Screen::OobeName;
            }
            Screen::OobeName => {
                if !cmd.is_empty() {
                    state.user_name = cmd.clone();
                    state.save_user();
                }
                screen = Screen::OobePin;
            }
            Screen::OobePin => {
                if pin::valid_pin(&cmd) {
                    state.pending_pin = cmd.clone();
                    screen = Screen::OobeConfirmPin;
                } else {
                    sink.println(&format!(
                        "{}  PIN must be 4–8 digits. Try again.{}",
                        FG_RED, R
                    ));
                    let _ = io::stdin().lock().read_line(&mut String::new());
                }
            }
            Screen::OobeConfirmPin => {
                if cmd == state.pending_pin {
                    match pin::create_record(&cmd) {
                        Ok(record) => {
                            state.pin = record;
                            state.pending_pin.clear();
                            state.save_pin();
                            state.mark_oobe_done();
                            screen = Screen::OobeDone;
                        }
                        Err(e) => {
                            sink.println(&format!(
                                "{}  Invalid PIN: {}. Try again.{}",
                                FG_RED, e, R
                            ));
                            let _ = io::stdin().lock().read_line(&mut String::new());
                            screen = Screen::OobePin;
                        }
                    }
                } else {
                    sink.println(&format!(
                        "{}  PINs don't match. Let's try again.{}",
                        FG_RED, R
                    ));
                    let _ = io::stdin().lock().read_line(&mut String::new());
                    screen = Screen::OobePin;
                }
            }
            Screen::OobeDone => {
                screen = Screen::Lockscreen;
                lock_error = false;
            }

            // ── Lock Screen ───────────────────────────────────────────────────
            Screen::Lockscreen => match pin::check_pin(&state.pin, &cmd) {
                pin::CheckResult::Verified => {
                    lock_error = false;
                    screen = Screen::Home;
                }
                pin::CheckResult::VerifiedNeedsMigration => {
                    if let Ok(record) = pin::create_record(&cmd) {
                        state.pin = record;
                        state.save_pin();
                    }
                    lock_error = false;
                    screen = Screen::Home;
                }
                pin::CheckResult::Rejected => {
                    lock_error = true;
                }
            },

            // ── Home Launcher ─────────────────────────────────────────────────
            Screen::Home => {
                settings_in_section = None;
                if cmd.starts_with("run ") {
                    let app_id = cmd.trim_start_matches("run ").trim();
                    launch_app(&mut sink, app_id);
                } else if cmd.starts_with("search ") || cmd.starts_with("/ ") {
                    let query = if cmd.starts_with("search ") {
                        cmd.trim_start_matches("search ").trim()
                    } else {
                        cmd.trim_start_matches("/ ").trim()
                    };
                    let q_low = query.to_lowercase();
                    sink.println(&format!(
                        "\n  {}🔍 Search results for '{}':{}",
                        FG_CYAN, query, R
                    ));
                    for (id, desc) in get_installed_packages() {
                        if id.to_lowercase().contains(&q_low)
                            || desc.to_lowercase().contains(&q_low)
                        {
                            sink.println(&format!("    📦 Package: {} ({})", id, desc));
                        }
                    }
                    for note in notes::search_notes(&state.notes, query) {
                        sink.println(&format!(
                            "    📝 Note #{}: {} [{}]",
                            note.id, note.title, note.tag
                        ));
                    }
                    for (name, num) in &state.contacts {
                        if name.to_lowercase().contains(&q_low) || num.contains(&q_low) {
                            sink.println(&format!("    📞 Contact: {} — {}", name, num));
                        }
                    }
                    sink.print("  Press Enter...");
                    read_line();
                } else {
                    match cmd.as_str() {
                        "1" | "phone" => {
                            screen = Screen::AppPhone;
                            state.call_number.clear();
                        }
                        "2" | "messages" => screen = Screen::AppMessages,
                        "3" | "files" => {
                            screen = Screen::AppFiles;
                            state.files_path = "/data".into();
                        }
                        "4" | "settings" => screen = Screen::AppSettings,
                        "5" | "pkg" => screen = Screen::AppNilPkg,
                        "6" | "softbus" => screen = Screen::AppSoftBus,
                        "7" | "android" => screen = Screen::AppAndroid,
                        "8" | "terminal" => screen = Screen::AppTerminal,
                        "9" | "hello" => launch_app(&mut sink, "org.onuron.hello"),
                        "10" | "calc" | "calculator" => screen = Screen::AppCalculator,
                        "11" | "notes" => screen = Screen::AppNotes,
                        "12" | "music" => screen = Screen::AppMusic,
                        "13" | "camera" => screen = Screen::AppCamera,
                        "14" | "browser" | "web" => screen = Screen::AppBrowser,
                        "n" | "notif" => screen = Screen::NotificationShade,
                        "l" | "lock" => {
                            screen = Screen::Lockscreen;
                            lock_error = false;
                        }
                        _ => {}
                    }
                }
            }

            // ── Phone App ─────────────────────────────────────────────────────
            Screen::AppPhone => {
                if cmd == "home" || cmd == "back" {
                    screen = Screen::Home;
                } else if cmd.starts_with("add ") {
                    let arg = cmd.trim_start_matches("add ").trim();
                    if let Some((name, num)) = arg.split_once('|') {
                        let name_s = name.trim().to_string();
                        let num_s = num.trim().to_string();
                        if !name_s.is_empty() && !num_s.is_empty() {
                            state.contacts.push((name_s.clone(), num_s.clone()));
                            state.save_contacts();
                            sink.println(&format!(
                                "\n  {}✅ Contact saved: {} — {}{}",
                                FG_GREEN, name_s, num_s, R
                            ));
                            read_line();
                        }
                    }
                } else if cmd.starts_with("del ") {
                    let target = cmd.trim_start_matches("del ").trim().to_lowercase();
                    let prev = state.contacts.len();
                    state.contacts.retain(|(n, _)| n.to_lowercase() != target);
                    if state.contacts.len() < prev {
                        state.save_contacts();
                        sink.println(&format!("\n  {}🗑️ Contact deleted.{}", FG_YELLOW, R));
                        read_line();
                    }
                } else if cmd == "clear" {
                    state.call_log.clear();
                    state.save_call_log();
                } else if cmd.starts_with("call ") {
                    let query = cmd.trim_start_matches("call ").trim().to_string();
                    let (peer_name, number) = state
                        .contacts
                        .iter()
                        .find(|(n, _)| n.eq_ignore_ascii_case(&query))
                        .cloned()
                        .unwrap_or_else(|| (query.clone(), query));
                    state.call_number = number.clone();
                    let (time_12h, _, _) = live_clock_and_date();
                    state
                        .call_log
                        .insert(0, (peer_name.clone(), number.clone(), time_12h));
                    state.save_call_log();
                    sink.println(&format!(
                        "\n  {}📞 Calling {} ({})...  {}{}",
                        FG_GREEN,
                        peer_name,
                        number,
                        simulated::badge(),
                        R
                    ));
                    sink.println("  Press Enter to hang up.");
                    read_line();
                    state.call_number.clear();
                } else if !cmd.is_empty()
                    && cmd
                        .chars()
                        .all(|c| c.is_ascii_digit() || c == '+' || c == '-')
                {
                    state.call_number = cmd.clone();
                }
            }

            // ── Messages App ──────────────────────────────────────────────────
            Screen::AppMessages => {
                if cmd == "home" || cmd == "back" {
                    screen = Screen::Home;
                } else if cmd == "new" {
                    sink.print(&format!("\n  {}To (name or number): {}", FG_YELLOW, R));
                    state.compose_to = read_line();
                    sink.print(&format!("  {}Message: {}", FG_YELLOW, R));
                    state.compose_body = read_line();
                    let (time_12h, _, _) = live_clock_and_date();
                    let _ = fs::create_dir_all(SMS_DIR);
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let fname = format!("{}/{}.txt", SMS_DIR, ts);
                    let _ = fs::write(
                        &fname,
                        format!("{}\n{}\n{}", state.compose_to, state.compose_body, time_12h),
                    );
                    state.sms_threads.insert(
                        0,
                        (
                            state.compose_to.clone(),
                            state.compose_body.clone(),
                            time_12h,
                        ),
                    );
                    state.save_sms_threads();
                    state.compose_to.clear();
                    state.compose_body.clear();
                    sink.println(&format!("  {}✅ Message sent & persisted!{}", FG_GREEN, R));
                    read_line();
                } else if cmd.starts_with("del ") {
                    if let Ok(idx) = cmd.trim_start_matches("del ").trim().parse::<usize>() {
                        if idx > 0 && idx <= state.sms_threads.len() {
                            state.sms_threads.remove(idx - 1);
                            state.save_sms_threads();
                        }
                    }
                } else {
                    let idx_opt = if cmd.starts_with("read ") {
                        cmd.trim_start_matches("read ").trim().parse::<usize>().ok()
                    } else {
                        cmd.parse::<usize>().ok()
                    };
                    if let Some(idx) = idx_opt {
                        if idx > 0 && idx <= state.sms_threads.len() {
                            let (contact, msg, time) = state.sms_threads[idx - 1].clone();
                            sink.print(CL);
                            sink.println(&format!("  {}💬 Thread: {}{}", FG_YELLOW, contact, R));
                            sink.println(&format!("  {} — {}", time, msg));
                            sink.println("");
                            sink.print(&format!(
                                "  {}Reply (or press Enter to go back): {}",
                                FG_YELLOW, R
                            ));
                            let reply = read_line();
                            if !reply.is_empty() {
                                let (time_12h, _, _) = live_clock_and_date();
                                state.sms_threads[idx - 1] = (contact, reply, time_12h);
                                state.save_sms_threads();
                            }
                        }
                    }
                }
            }

            // ── Files App ─────────────────────────────────────────────────────
            Screen::AppFiles => {
                if cmd == "home" {
                    screen = Screen::Home;
                } else if cmd == "back" {
                    let p = std::path::Path::new(&state.files_path);
                    if let Some(parent) = p.parent() {
                        state.files_path = parent.to_string_lossy().into();
                    } else {
                        screen = Screen::Home;
                    }
                } else if cmd.starts_with("cd ") {
                    let new_path = cmd.trim_start_matches("cd ").trim();
                    if std::path::Path::new(new_path).is_dir() {
                        state.files_path = new_path.to_string();
                    }
                } else if cmd == "1" {
                    state.files_path = "/data".into();
                } else if cmd == "2" {
                    state.files_path = "/data/app".into();
                } else if cmd == "3" {
                    state.files_path = "/etc".into();
                } else if cmd == "4" {
                    state.files_path = "/tmp".into();
                }
            }

            // ── Settings App ──────────────────────────────────────────────────
            Screen::AppSettings => {
                if let Some(_sec) = settings_in_section {
                    if cmd == "back" {
                        settings_in_section = None;
                    } else if cmd == "home" {
                        screen = Screen::Home;
                        settings_in_section = None;
                    } else if cmd == "toggle wifi" || cmd == "wifi" {
                        state.wifi_enabled = !state.wifi_enabled;
                    } else if cmd == "toggle data" || cmd == "data" {
                        state.cellular_enabled = !state.cellular_enabled;
                    } else if cmd.starts_with("signal ") {
                        if let Ok(b) = cmd.trim_start_matches("signal ").trim().parse::<u8>() {
                            state.signal_bars = b.min(4);
                        }
                    } else if cmd.starts_with("vol ") {
                        if let Ok(v) = cmd.trim_start_matches("vol ").trim().parse::<u8>() {
                            let _ = nilaudio::AudioClient::set_volume(v.min(100));
                        }
                    } else if cmd == "mute" {
                        if let Ok(st) = nilaudio::AudioClient::get_status() {
                            let _ = nilaudio::AudioClient::set_mute(!st.is_muted);
                        }
                    } else if cmd.starts_with("route ") {
                        let r = cmd.trim_start_matches("route ").trim();
                        let _ = nilaudio::AudioClient::set_route(r);
                    } else if cmd == "120hz" {
                        state.high_refresh_rate = true;
                    } else if cmd == "60hz" {
                        state.high_refresh_rate = false;
                    } else if cmd.starts_with("bright ") {
                        if let Ok(b) = cmd.trim_start_matches("bright ").trim().parse::<u8>() {
                            state.brightness = b.clamp(10, 100);
                        }
                    } else if cmd.starts_with("pin ") {
                        let new_pin = cmd.trim_start_matches("pin ").trim();
                        if let Ok(record) = pin::create_record(new_pin) {
                            state.pin = record;
                            state.save_pin();
                            sink.println(&format!(
                                "\n  {}✅ PIN updated and salted-hash persisted!{}",
                                FG_GREEN, R
                            ));
                            read_line();
                        } else {
                            sink.println(&format!(
                                "\n  {}❌ PIN must be 4–8 ASCII digits.{}",
                                FG_RED, R
                            ));
                            read_line();
                        }
                    } else if cmd.starts_with("profile ") {
                        let prof = cmd.trim_start_matches("profile ").trim().to_lowercase();
                        if ["balanced", "powersave", "performance"].contains(&prof.as_str()) {
                            state.power_profile = prof.clone();
                            let _ = fs::create_dir_all("/data/nilos");
                            let _ = fs::write("/data/nilos/power_profile", &prof);
                        }
                    } else if cmd.starts_with("lang ") {
                        state.locale =
                            i18n::Locale::from_code(cmd.trim_start_matches("lang ").trim());
                    } else if cmd == "sr on" {
                        state.screen_reader = true;
                    } else if cmd == "sr off" {
                        state.screen_reader = false;
                    }
                } else {
                    if cmd == "home" || cmd == "back" {
                        screen = Screen::Home;
                    } else if let Ok(n) = cmd.parse::<usize>() {
                        if n >= 1 && n <= 8 {
                            settings_in_section = Some(n - 1);
                        }
                    }
                }
            }

            // ── NilPkg ────────────────────────────────────────────────────────
            Screen::AppNilPkg => {
                if cmd == "home" || cmd == "back" {
                    screen = Screen::Home;
                } else if cmd.starts_with("install ") {
                    let path_str = cmd.trim_start_matches("install ").trim();
                    let path = Path::new(path_str);
                    sink.println(&format!(
                        "\n  {}📦 Installing package from {}...{}",
                        FG_GREEN, path_str, R
                    ));
                    match nilpkg::install_local(
                        path,
                        &nilpkg::get_app_dir(),
                        &nilpkg::get_key_dir(),
                    ) {
                        Ok(()) => {
                            sink.println(&format!(
                                "  {}✅ Successfully installed package: {}!{}",
                                FG_GREEN, path_str, R
                            ));
                        }
                        Err(e) => {
                            sink.println(&format!(
                                "  {}❌ Installation failed: {}{}",
                                FG_RED, e, R
                            ));
                        }
                    }
                    sink.print("  Press Enter...");
                    read_line();
                } else if cmd.starts_with("upgrade ") {
                    let path_str = cmd.trim_start_matches("upgrade ").trim();
                    let path = Path::new(path_str);
                    match nilpkg::upgrade_local(
                        path,
                        &nilpkg::get_app_dir(),
                        &nilpkg::get_key_dir(),
                        false,
                    ) {
                        Ok(()) => sink.println(&format!(
                            "\n  {}✅ Upgraded package from {}!{}",
                            FG_GREEN, path_str, R
                        )),
                        Err(e) => {
                            sink.println(&format!("\n  {}❌ Upgrade failed: {}{}", FG_RED, e, R))
                        }
                    }
                    sink.print("  Press Enter...");
                    read_line();
                } else if cmd.starts_with("rollback ") {
                    let app_id = cmd.trim_start_matches("rollback ").trim();
                    match nilpkg::rollback_package(app_id, &nilpkg::get_app_dir()) {
                        Ok(()) => sink
                            .println(&format!("\n  {}✅ Rolled back {}!{}", FG_GREEN, app_id, R)),
                        Err(e) => {
                            sink.println(&format!("\n  {}❌ Rollback failed: {}{}", FG_RED, e, R))
                        }
                    }
                    sink.print("  Press Enter...");
                    read_line();
                } else if cmd.starts_with("uninstall ") || cmd.starts_with("remove ") {
                    let app_id = if cmd.starts_with("uninstall ") {
                        cmd.trim_start_matches("uninstall ").trim()
                    } else {
                        cmd.trim_start_matches("remove ").trim()
                    };
                    match nilpkg::remove_package(app_id) {
                        Ok(()) => sink
                            .println(&format!("\n  {}🗑️ Uninstalled {}!{}", FG_YELLOW, app_id, R)),
                        Err(e) => {
                            sink.println(&format!("\n  {}❌ Uninstall failed: {}{}", FG_RED, e, R))
                        }
                    }
                    sink.print("  Press Enter...");
                    read_line();
                } else if cmd.starts_with("trust ") {
                    let pub_path = cmd.trim_start_matches("trust ").trim();
                    match nilpkg::trust_command(pub_path) {
                        Ok(()) => sink.println(&format!(
                            "\n  {}✅ Trusted publisher key ({}){}",
                            FG_GREEN, pub_path, R
                        )),
                        Err(e) => {
                            sink.println(&format!("\n  {}❌ Trust failed: {}{}", FG_RED, e, R))
                        }
                    }
                    sink.print("  Press Enter...");
                    read_line();
                } else if cmd.starts_with("revoke ") {
                    let key_name = cmd.trim_start_matches("revoke ").trim();
                    match nilpkg::revoke_command(key_name) {
                        Ok(()) => sink.println(&format!(
                            "\n  {}✅ Revoked key '{}'{}",
                            FG_YELLOW, key_name, R
                        )),
                        Err(e) => {
                            sink.println(&format!("\n  {}❌ Revoke failed: {}{}", FG_RED, e, R))
                        }
                    }
                    sink.print("  Press Enter...");
                    read_line();
                } else if cmd.starts_with("verify ") {
                    let app_id = cmd.trim_start_matches("verify ").trim();
                    sink.println(&format!(
                        "\n  🔍 Verifying package integrity: {}...",
                        app_id
                    ));
                    match nilpkg::verify_installed(
                        app_id,
                        &nilpkg::get_app_dir(),
                        &nilpkg::get_key_dir(),
                    ) {
                        Ok(()) => {
                            sink.println(&format!(
                                "  {}✅ Verification PASSED: {} is authentic and verified!{}",
                                FG_GREEN, app_id, R
                            ));
                        }
                        Err(e) => {
                            sink.println(&format!(
                                "  {}❌ Verification failed: {}{}",
                                FG_RED, e, R
                            ));
                        }
                    }
                    sink.print("  Press Enter...");
                    read_line();
                } else if cmd.starts_with("run ") {
                    let app_id = cmd.trim_start_matches("run ").trim();
                    launch_app(&mut sink, app_id);
                } else if cmd == "list" {
                    sink.println(&format!(
                        "\n  {}Packages in {}:{}",
                        FG_WHITE,
                        nilpkg::get_app_dir().display(),
                        R
                    ));
                    let pkgs = get_installed_packages();
                    if pkgs.is_empty() {
                        sink.println("    No packages installed yet.");
                    } else {
                        for (id, desc) in pkgs {
                            sink.println(&format!("    • {} — {}", id, desc));
                        }
                    }
                    sink.print("  Press Enter...");
                    read_line();
                }
            }

            // ── SoftBus ───────────────────────────────────────────────────────
            Screen::AppSoftBus => {
                if cmd == "home" || cmd == "back" {
                    screen = Screen::Home;
                } else if cmd == "scan" {
                    match nilbus_client::SoftBusClient::get_nearby_peers() {
                        Ok(peers) => {
                            sink.println(&format!(
                                "\n  {}Discovered {} peer(s).{}",
                                FG_GREEN,
                                peers.len(),
                                R
                            ));
                        }
                        Err(e) => {
                            sink.println(&format!("\n  {}Scan error: {}{}", FG_RED, e, R));
                        }
                    }
                    sink.print("  Press Enter...");
                    read_line();
                } else if cmd.starts_with("pair ") {
                    let parts: Vec<&str> =
                        cmd.trim_start_matches("pair ").split_whitespace().collect();
                    if let Some(&id) = parts.first() {
                        let addr_port = parts.get(1).copied().unwrap_or("192.168.1.50:7979");
                        let (addr, port_s) =
                            addr_port.rsplit_once(':').unwrap_or((addr_port, "7979"));
                        let port = port_s.parse::<u16>().unwrap_or(7979);
                        let peers_path = nilbus_client::SoftBusClient::default_peers_path();
                        match nilbus_client::SoftBusClient::pair_peer_to(
                            &peers_path,
                            id,
                            addr,
                            port,
                        ) {
                            Ok(p) => sink.println(&format!(
                                "\n  {}✅ Paired peer {} ({}:{}){}",
                                FG_GREEN, p.device_id, p.addr, p.quic_port, R
                            )),
                            Err(e) => {
                                sink.println(&format!("\n  {}❌ Pair failed: {}{}", FG_RED, e, R))
                            }
                        }
                        sink.print("  Press Enter...");
                        read_line();
                    }
                } else if cmd.starts_with("unpair ") {
                    let id = cmd.trim_start_matches("unpair ").trim();
                    let peers_path = nilbus_client::SoftBusClient::default_peers_path();
                    let _ = nilbus_client::SoftBusClient::unpair_peer_from(&peers_path, id);
                } else if cmd.starts_with("ping ") {
                    let id = cmd.trim_start_matches("ping ").trim();
                    let reachable = nilbus_client::SoftBusClient::ping(id);
                    sink.println(&format!(
                        "\n  Peer '{}': {}",
                        id,
                        if reachable {
                            "REACHABLE (LIVE QUIC)"
                        } else {
                            "UNREACHABLE (daemon offline or peer down)"
                        }
                    ));
                    sink.print("  Press Enter...");
                    read_line();
                } else if cmd.starts_with("handoff ") {
                    let parts: Vec<&str> =
                        cmd.trim_start_matches("handoff ").splitn(3, ' ').collect();
                    if parts.len() == 3 {
                        match nilbus_client::SoftBusClient::send_handoff(
                            parts[0],
                            parts[1],
                            parts[2].as_bytes(),
                        ) {
                            Ok(resp) => sink.println(&format!("\n  {}✅ {}{}", FG_GREEN, resp, R)),
                            Err(e) => sink.println(&format!("\n  {}⚠ {}{}", FG_YELLOW, e, R)),
                        }
                        sink.print("  Press Enter...");
                        read_line();
                    }
                }
            }

            // ── Browser ───────────────────────────────────────────────────────
            Screen::AppBrowser => {
                if cmd == "home" || cmd == "back" {
                    screen = Screen::Home;
                } else if cmd == "d" || cmd == "down" {
                    state.browser.scroll_down(12);
                } else if cmd == "u" || cmd == "up" {
                    state.browser.scroll_up(12);
                } else if cmd == "top" {
                    state.browser.scroll = 0;
                } else if !cmd.is_empty() {
                    // A real network fetch happens here; it blocks the UI thread
                    // until the request finishes or times out.
                    state.browser.navigate(&cmd);
                }
            }

            // ── Android Compatibility ─────────────────────────────────────────
            Screen::AppAndroid => {
                if cmd == "home" || cmd == "back" {
                    screen = Screen::Home;
                } else if cmd == "status" || cmd.is_empty() {
                    match nilandroid_client::AndroidBridge::status() {
                        Ok(st) => sink.println(&format!("\n  {}", st.status_line)),
                        Err(e) => sink.println(&format!("\n  bridge unavailable: {}", e)),
                    }
                    sink.print("  Press Enter...");
                    read_line();
                } else if cmd == "start" {
                    sink.println("\n  Starting Android container...");
                    match nilandroid_client::AndroidBridge::start_container() {
                        Ok(pid) => sink.println(&format!(
                            "  {}Container started (init PID {}).{}",
                            FG_GREEN, pid, R
                        )),
                        Err(e) => sink.println(&format!("  {}start failed: {}{}", FG_RED, e, R)),
                    }
                    sink.print("  Press Enter...");
                    read_line();
                } else if cmd == "stop" {
                    match nilandroid_client::AndroidBridge::stop_container() {
                        Ok(()) => sink.println(&format!("\n  {}Container stopped.{}", FG_GREEN, R)),
                        Err(e) => sink.println(&format!("\n  {}stop failed: {}{}", FG_RED, e, R)),
                    }
                    sink.print("  Press Enter...");
                    read_line();
                } else if let Some(uri) = cmd.strip_prefix("open ") {
                    match nilandroid_client::AndroidBridge::start_activity(
                        "android.intent.action.VIEW",
                        uri.trim(),
                    ) {
                        Ok(()) => sink.println(&format!(
                            "\n  {}Intent delivered to container.{}",
                            FG_GREEN, R
                        )),
                        Err(e) => sink.println(&format!("\n  {}intent failed: {}{}", FG_RED, e, R)),
                    }
                    sink.print("  Press Enter...");
                    read_line();
                }
            }

            // ── Notifications ─────────────────────────────────────────────────
            Screen::NotificationShade => {
                if cmd == "home" || cmd == "back" || cmd.is_empty() {
                    screen = Screen::Home;
                } else if cmd.starts_with("post ") {
                    let raw = cmd.trim_start_matches("post ").trim();
                    let (title, body) = raw
                        .split_once('|')
                        .map(|(t, b)| (t.trim(), b.trim()))
                        .unwrap_or(("User Alert", raw));
                    let _ = nilnotify::NotificationClient::post("nilshell", title, body);
                } else if cmd.starts_with("dismiss ") || cmd.starts_with("del ") {
                    let id_s = cmd
                        .trim_start_matches("dismiss ")
                        .trim_start_matches("del ")
                        .trim();
                    if let Ok(id) = id_s.parse::<u64>() {
                        let store_path = nilnotify::NotificationClient::default_store_path();
                        let _ = nilnotify::NotificationClient::dismiss_from(&store_path, id);
                    }
                } else if cmd == "clear" {
                    let store_path = nilnotify::NotificationClient::default_store_path();
                    let _ = nilnotify::NotificationClient::clear_all(&store_path);
                }
            }

            // ── Terminal ──────────────────────────────────────────────────────
            Screen::AppTerminal => {
                if cmd == "home" || cmd == "exit" {
                    screen = Screen::Home;
                } else if cmd == "reboot" {
                    sink.println(
                        "\n  [nilinit] Requesting system reboot via /run/onuron/power_action...",
                    );
                    let _ = fs::create_dir_all("/run/onuron");
                    let _ = fs::write("/run/onuron/power_action", "reboot");
                    screen = Screen::Lockscreen;
                    lock_error = false;
                } else {
                    handle_terminal_cmd(&mut sink, &mut state, &cmd);
                    draw_terminal(&mut sink, &state);
                    continue;
                }
            }

            // ── Calculator App ────────────────────────────────────────────────
            Screen::AppCalculator => {
                if cmd == "home" || cmd == "back" {
                    screen = Screen::Home;
                } else if cmd == "c" || cmd == "C" || cmd == "clear" {
                    state.calc_val = "0".into();
                    state.calc_history.clear();
                    state.save_calc_history();
                } else if !cmd.is_empty() {
                    if let Ok(res) = eval_simple_math(&cmd) {
                        state.calc_val = res.clone();
                        state.calc_history.push((cmd.clone(), res));
                        if state.calc_history.len() > 20 {
                            state.calc_history.remove(0);
                        }
                        state.save_calc_history();
                    } else {
                        state.calc_val = "Error".into();
                    }
                }
            }

            // ── Notes App ─────────────────────────────────────────────────────
            Screen::AppNotes => {
                if cmd == "home" || cmd == "back" {
                    screen = Screen::Home;
                } else if cmd.starts_with("new ") || cmd.starts_with("add ") {
                    let content = if cmd.starts_with("new ") {
                        cmd.trim_start_matches("new ").trim()
                    } else {
                        cmd.trim_start_matches("add ").trim()
                    };
                    let (title, body) = if let Some((t, b)) = content.split_once('|') {
                        (t.trim().to_string(), b.trim().to_string())
                    } else {
                        (content.to_string(), "সংক্ষিপ্ত নোট".to_string())
                    };
                    let new_id = notes::add_note(&mut state.notes, title, body, "ব্যক্তিগত".into());
                    let _ = notes::save_notes_atomic(&state.notes_path, &state.notes);
                    sink.println(&format!(
                        "\n  {}✅ Note #{} saved to disk storage.{}",
                        FG_GREEN, new_id, R
                    ));
                } else if cmd.starts_with("edit ") {
                    let rest = cmd.trim_start_matches("edit ").trim();
                    if let Some((id_str, content)) = rest.split_once(' ') {
                        if let Ok(id) = id_str.trim().parse::<usize>() {
                            let (title_opt, body_opt) =
                                if let Some((t, b)) = content.split_once('|') {
                                    (Some(t.trim().to_string()), Some(b.trim().to_string()))
                                } else {
                                    (None, Some(content.trim().to_string()))
                                };
                            if notes::edit_note(&mut state.notes, id, title_opt, body_opt) {
                                let _ = notes::save_notes_atomic(&state.notes_path, &state.notes);
                            }
                        }
                    }
                } else if cmd.starts_with("tag ") {
                    let rest = cmd.trim_start_matches("tag ").trim();
                    if let Some((id_str, tag)) = rest.split_once(' ') {
                        if let Ok(id) = id_str.trim().parse::<usize>() {
                            if notes::update_note_meta(
                                &mut state.notes,
                                id,
                                Some(tag.trim().to_string()),
                                None,
                            ) {
                                let _ = notes::save_notes_atomic(&state.notes_path, &state.notes);
                            }
                        }
                    }
                } else if cmd.starts_with("priority ") {
                    let rest = cmd.trim_start_matches("priority ").trim();
                    if let Some((id_str, prio)) = rest.split_once(' ') {
                        if let Ok(id) = id_str.trim().parse::<usize>() {
                            if notes::update_note_meta(
                                &mut state.notes,
                                id,
                                None,
                                Some(prio.trim().to_string()),
                            ) {
                                let _ = notes::save_notes_atomic(&state.notes_path, &state.notes);
                            }
                        }
                    }
                } else if cmd.starts_with("search ") {
                    let q = cmd.trim_start_matches("search ").trim();
                    let matches = notes::search_notes(&state.notes, q);
                    sink.println(&format!(
                        "\n  {}Found {} matching note(s):{}",
                        FG_CYAN,
                        matches.len(),
                        R
                    ));
                    for n in matches {
                        sink.println(&format!(
                            "    [{}] {} — {} ({})",
                            n.id, n.title, n.body, n.tag
                        ));
                    }
                    sink.print("  Press Enter...");
                    read_line();
                } else if cmd.starts_with("del ") || cmd.starts_with("delete ") {
                    let id_str = if cmd.starts_with("del ") {
                        cmd.trim_start_matches("del ").trim()
                    } else {
                        cmd.trim_start_matches("delete ").trim()
                    };
                    if let Ok(id) = id_str.parse::<usize>() {
                        if notes::delete_note(&mut state.notes, id) {
                            let _ = notes::save_notes_atomic(&state.notes_path, &state.notes);
                            sink.println(&format!(
                                "\n  {}🗑️ Note #{} deleted.{}",
                                FG_YELLOW, id, R
                            ));
                        } else {
                            sink.println(&format!("\n  {}Note #{} not found.{}", FG_RED, id, R));
                        }
                    }
                }
            }

            // ── Music App ─────────────────────────────────────────────────────
            Screen::AppMusic => {
                if cmd == "home" || cmd == "back" {
                    screen = Screen::Home;
                } else if cmd == "play" || cmd == "p" || cmd == "pause" {
                    state.music_playing = !state.music_playing;
                } else if cmd == "next" || cmd == "n" {
                    if !state.music_tracks.is_empty() {
                        state.music_track = (state.music_track + 1) % state.music_tracks.len();
                        state.music_playing = true;
                    }
                } else if cmd == "prev" {
                    if !state.music_tracks.is_empty() {
                        state.music_track = if state.music_track == 0 {
                            state.music_tracks.len() - 1
                        } else {
                            state.music_track - 1
                        };
                        state.music_playing = true;
                    }
                } else if cmd.starts_with("vol ") {
                    if let Ok(v) = cmd.trim_start_matches("vol ").trim().parse::<u8>() {
                        let _ = nilaudio::AudioClient::set_volume(v.min(100));
                    }
                } else if cmd == "mute" {
                    if let Ok(st) = nilaudio::AudioClient::get_status() {
                        let _ = nilaudio::AudioClient::set_mute(!st.is_muted);
                    }
                } else if cmd.starts_with("route ") {
                    let _ =
                        nilaudio::AudioClient::set_route(cmd.trim_start_matches("route ").trim());
                } else if cmd.starts_with("add ") {
                    let parts: Vec<&str> = cmd
                        .trim_start_matches("add ")
                        .split('|')
                        .map(|s| s.trim())
                        .collect();
                    if let Some(&title) = parts.first() {
                        if !title.is_empty() {
                            let artist = parts.get(1).copied().unwrap_or("Local Artist");
                            let path = parts.get(2).copied().unwrap_or("/data/music/track.wav");
                            state
                                .music_tracks
                                .push((title.into(), artist.into(), path.into()));
                            state.save_playlist();
                        }
                    }
                } else if cmd == "scan" {
                    state.music_tracks = AppState::discover_music_tracks();
                }
            }

            // ── Camera App ────────────────────────────────────────────────────
            Screen::AppCamera => {
                if cmd == "home" || cmd == "back" {
                    screen = Screen::Home;
                } else if cmd == "capture" {
                    match nilcam::CameraClient::capture_frame(state.camera_id) {
                        Ok(frame) => {
                            state.camera_captures += 1;
                            let _ = fs::create_dir_all(DCIM_DIR);
                            let fname =
                                format!("{}/IMG_{:04}.frame", DCIM_DIR, state.camera_captures);
                            let _ = fs::write(
                                &fname,
                                format!(
                                    "camera_id={}\nwidth={}\nheight={}\nformat={}\ndata={}\n",
                                    frame.camera_id,
                                    frame.width,
                                    frame.height,
                                    frame.format,
                                    frame.data_base64
                                ),
                            );
                            state.camera_last_status = format!(
                                "Saved {} ({}x{} {})",
                                fname, frame.width, frame.height, frame.format
                            );
                        }
                        Err(e) => {
                            state.camera_last_status = format!("Capture error: {}", e);
                        }
                    }
                } else if cmd == "torch" {
                    state.camera_torch = !state.camera_torch;
                    let _ = nilcam::CameraClient::set_torch(state.camera_id, state.camera_torch);
                    state.camera_last_status = format!(
                        "Torch {}",
                        if state.camera_torch {
                            "ENABLED"
                        } else {
                            "DISABLED"
                        }
                    );
                } else if cmd == "switch" {
                    state.camera_id = if state.camera_id == 0 { 1 } else { 0 };
                    state.camera_last_status = format!("Switched to camera #{}", state.camera_id);
                } else if cmd == "list" {
                    let (count, bytes) = directory_usage_summary(Path::new(DCIM_DIR));
                    state.camera_last_status =
                        format!("{} photo(s) in {} ({} bytes)", count, DCIM_DIR, bytes);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_state(user_name: &str) -> AppState {
        AppState {
            user_name: user_name.into(),
            pin: String::new(),
            pending_pin: String::new(),
            pin_input: String::new(),
            sms_threads: Vec::new(),
            contacts: Vec::new(),
            call_log: Vec::new(),
            compose_to: String::new(),
            compose_body: String::new(),
            terminal_history: Vec::new(),
            term: term::TermSession::new("/data".into(), "/data".into()),
            browser: browser::BrowserState::default(),
            settings_cursor: 0,
            files_path: "/data".into(),
            pkg_cursor: 0,
            call_number: String::new(),
            wifi_enabled: true,
            cellular_enabled: true,
            signal_bars: 4,
            term_cwd: "/data".into(),
            calc_val: "0".into(),
            calc_history: Vec::new(),
            music_playing: false,
            music_track: 0,
            music_tracks: Vec::new(),
            notes: notes::default_notes(),
            notes_path: std::path::PathBuf::from("/tmp/onuron_test_notes.json"),
            camera_id: 0,
            camera_torch: false,
            camera_captures: 0,
            camera_last_status: "Ready".into(),
            locale: i18n::Locale::Bn,
            screen_reader: false,
            power_profile: "balanced".into(),
            high_refresh_rate: true,
            brightness: 80,
        }
    }

    #[test]
    fn test_live_clock_and_date_non_empty() {
        let (t12, t24, date) = live_clock_and_date();
        assert!(t12.ends_with("AM") || t12.ends_with("PM"));
        assert_eq!(t24.len(), 5);
        assert!(date.contains("202"));
    }

    #[test]
    fn test_network_display_defaults() {
        let state = create_test_state("TestUser");

        let (wifi, cell) = get_network_display(&state);
        assert!(!wifi.is_empty());
        assert!(!cell.is_empty());

        let bar = status_bar(&state);
        assert!(bar.contains("TestUser"));
    }

    #[test]
    fn test_network_display_airplane_mode() {
        // The guard is "netd reports no real link", not "/sys/class/net is
        // absent": the shell no longer inspects sysfs itself, so a test that
        // checked for the directory would be testing the old implementation.
        let mut state = create_test_state("OfflineUser");
        state.wifi_enabled = false;
        state.cellular_enabled = false;
        state.signal_bars = 0;

        let (wifi, cell) = get_network_display(&state);
        if !nilnet::NetClient::is_daemon_active() {
            assert_eq!(cell, "✈️ Offline");
            assert_eq!(wifi, "✕ Wi-Fi");
        }
    }

    #[test]
    fn status_bar_uses_the_service_boundary_for_the_battery() {
        // The badge must come from a payload, and a simulated payload must be
        // labelled so a placeholder is never read as a real reading.
        let shown = get_battery_display();
        let info = get_battery_info();
        assert!(
            shown.contains(&info.capacity.to_string()),
            "badge {shown:?} does not reflect capacity {}",
            info.capacity
        );
        assert_eq!(
            shown.contains(simulated::badge()),
            info.is_simulated,
            "badge simulation marker disagrees with the payload flag"
        );
    }

    #[test]
    fn test_eval_simple_math_precedence_and_parentheses() {
        assert_eq!(eval_simple_math("2 + 3 * 4").unwrap(), "14");
        assert_eq!(eval_simple_math("(2 + 3) * 4").unwrap(), "20");
        assert_eq!(eval_simple_math("100 / 4 - 5").unwrap(), "20");
        assert_eq!(eval_simple_math("2.5 * 4").unwrap(), "10");
        assert_eq!(eval_simple_math("-5 + 15").unwrap(), "10");
        assert_eq!(eval_simple_math("(10 - 2) / (2 + 2)").unwrap(), "2");
        assert_eq!(eval_simple_math("2 ^ 10").unwrap(), "1024");
        assert_eq!(eval_simple_math("17 % 5").unwrap(), "2");
        assert_eq!(eval_simple_math("sqrt(144) + abs(-8)").unwrap(), "20");
        assert_eq!(eval_simple_math("log(1000)").unwrap(), "3");
    }

    #[test]
    fn test_eval_simple_math_bengali_digits() {
        assert_eq!(eval_simple_math("২৫ * ৪").unwrap(), "100");
        assert_eq!(eval_simple_math("১০০ / ৫").unwrap(), "20");
        assert_eq!(eval_simple_math("১০ + ৫ × ২").unwrap(), "20");
        assert_eq!(eval_simple_math("(১২ − ২) / ৫").unwrap(), "2");
    }

    #[test]
    fn test_eval_simple_math_negative_cases() {
        assert!(
            eval_simple_math("10 / 0").is_err(),
            "Division by zero must fail"
        );
        assert!(eval_simple_math("").is_err(), "Empty expression must fail");
        assert!(
            eval_simple_math("abc + 2").is_err(),
            "Malformed input must fail"
        );
        assert!(
            eval_simple_math("((2 + 3)").is_err(),
            "Unbalanced parentheses must fail"
        );
    }

    #[test]
    fn test_terminal_runs_real_builtins_and_propagates_failure() {
        let mut sink = Sink::new();
        let mut state = create_test_state("Tester");

        // A builtin runs in-process and its output reaches the scrollback.
        handle_terminal_cmd(&mut sink, &mut state, "echo real-output-marker");
        assert!(state
            .terminal_history
            .iter()
            .any(|h| h.contains("real-output-marker")));

        // A missing command reports 127 and says so instead of inventing output.
        state.term.env.push(("PATH".into(), "/nonexistent".into()));
        handle_terminal_cmd(&mut sink, &mut state, "no-such-binary-onuron-xyz");
        assert!(state
            .terminal_history
            .iter()
            .any(|h| h.contains("command not found")));
        assert!(state
            .terminal_history
            .iter()
            .any(|h| h.contains("[exit status 127]")));
    }

    #[test]
    fn test_terminal_executes_a_real_child_process() {
        let echo = ["/bin/echo", "/usr/bin/echo"]
            .iter()
            .find(|p| Path::new(p).exists());
        let Some(echo) = echo else { return };

        let mut sink = Sink::new();
        let mut state = create_test_state("Tester");
        handle_terminal_cmd(
            &mut sink,
            &mut state,
            &format!("{echo} child-process-marker"),
        );
        assert!(state
            .terminal_history
            .iter()
            .any(|h| h.contains("child-process-marker")));
    }

    #[test]
    fn test_terminal_cd_path_traversal_and_validation() {
        let mut sink = Sink::new();
        let mut state = create_test_state("Tester");

        // Use a real, existing directory as $HOME so `cd ~` is testable off-device.
        let home = term::normalize_logical(&std::env::temp_dir());
        state.term = term::TermSession::new(home.clone(), home.clone());
        state.term_cwd = home.display().to_string();

        // cd .. resolves lexically to the parent of the temp dir.
        handle_terminal_cmd(&mut sink, &mut state, "cd ..");
        let parent = home.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        assert_eq!(state.term_cwd, parent.display().to_string());

        // Repeated .. clamps at root and never escapes it.
        handle_terminal_cmd(&mut sink, &mut state, "cd ../../../../../../..");
        let expected_root = if cfg!(windows) {
            parent
                .ancestors()
                .last()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "/".into())
        } else {
            "/".to_string()
        };
        assert_eq!(state.term_cwd, expected_root);

        // cd to a nonexistent directory fails and does NOT change term_cwd.
        handle_terminal_cmd(&mut sink, &mut state, "cd /nonexistent_system_dir_xyz");
        assert!(state.terminal_history.iter().any(|h| h.contains("cd:")));
        assert_eq!(state.term_cwd, expected_root);

        // cd ~ returns to the session home.
        handle_terminal_cmd(&mut sink, &mut state, "cd ~");
        assert_eq!(state.term_cwd, home.display().to_string());
    }

    #[test]
    fn test_terminal_cat_validation() {
        let mut sink = Sink::new();
        let mut state = create_test_state("Tester");

        handle_terminal_cmd(&mut sink, &mut state, "cat nonexistent_missing_file.log");
        // `cat` is a real binary; a missing file makes it fail with a nonzero
        // status, which the terminal surfaces verbatim.
        assert!(state
            .terminal_history
            .iter()
            .any(|h| h.contains("exit status") || h.contains("No such file")));
    }

    #[test]
    fn test_settings_selinux_truthfulness() {
        let status = get_selinux_status();
        if !Path::new("/sys/fs/selinux/enforce").exists() {
            assert!(status.contains("[SIMULATED]"));
            assert!(status.contains("Unverified"));
        }
    }

    #[test]
    fn test_notes_interactive_creation_and_atomic_save() {
        let temp_dir = std::env::temp_dir().join("onuron_shell_notes_test");
        let notes_file = temp_dir.join("notes.json");
        let _ = fs::remove_dir_all(&temp_dir);

        let mut state = create_test_state("NotesUser");
        state.notes_path = notes_file.clone();

        let new_id = notes::add_note(
            &mut state.notes,
            "পরীক্ষামূলক নোট".into(),
            "শেল ইউজার ইন্টারফেস থেকে সংরক্ষিত।".into(),
            "সিস্টেম".into(),
        );
        assert_eq!(new_id, 4);

        notes::save_notes_atomic(&state.notes_path, &state.notes).expect("Save must succeed");
        assert!(notes_file.exists());

        let loaded = notes::load_notes(&notes_file);
        assert_eq!(loaded.len(), 4);
        assert_eq!(loaded[3].title, "পরীক্ষামূলক নোট");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
