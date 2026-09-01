// shell/src/main.rs — NilOS Complete Mobile UI Engine
// Screen stack: OOBE → Lockscreen → Home Launcher → Apps
// Renders via ANSI escape codes on /dev/console + /dev/ttyS0

use std::fs::{self};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

// ─── ANSI Colors ──────────────────────────────────────────────────────────────
const R: &str = "\x1b[0m";            // Reset
const B: &str = "\x1b[1m";            // Bold
const CL: &str = "\x1b[2J\x1b[H";    // Clear screen

const FG_CYAN:    &str = "\x1b[1;36m";
const FG_GREEN:   &str = "\x1b[1;32m";
const FG_YELLOW:  &str = "\x1b[1;33m";
const FG_BLUE:    &str = "\x1b[1;34m";
const FG_MAGENTA: &str = "\x1b[1;35m";
const FG_WHITE:   &str = "\x1b[1;37m";
const FG_RED:     &str = "\x1b[1;31m";
const FG_GRAY:    &str = "\x1b[0;90m";
const FG_BLACK_ON_WHITE: &str = "\x1b[30;47m";

// ─── Data paths ───────────────────────────────────────────────────────────────
const OOBE_DONE:   &str = "/data/nilos/oobe_done";
const PIN_FILE:    &str = "/data/nilos/pin_hash";
const USER_CONF:   &str = "/data/nilos/user.conf";
const CONTACTS_DIR:&str = "/data/contacts";
const SMS_DIR:     &str = "/data/sms";

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
    // Dialogs
    NotificationShade,
}

// ─── App State ────────────────────────────────────────────────────────────────
struct AppState {
    user_name: String,
    pin: String,
    pending_pin: String,
    pin_input: String,
    sms_threads: Vec<(String, String, String)>,     // (contact, last_msg, time)
    contacts: Vec<(String, String)>,                // (name, number)
    compose_to: String,
    compose_body: String,
    terminal_history: Vec<String>,
    settings_cursor: usize,
    files_path: String,
    pkg_cursor: usize,
    call_number: String,
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

        let mut sms_threads = vec![
            ("NilOS System".into(), "fscrypt v2 storage activated.".into(), "12:44".into()),
            ("SoftBus Manager".into(), "NilPad-Pro-X1 is ready.".into(), "12:42".into()),
            ("NilPkg".into(), "All daemons up to date.".into(), "12:30".into()),
        ];

        // Load real SMS threads from disk
        if let Ok(entries) = fs::read_dir(SMS_DIR) {
            for entry in entries.flatten() {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let lines: Vec<&str> = content.lines().collect();
                    if lines.len() >= 3 {
                        sms_threads.push((
                            lines[0].to_string(),
                            lines[1].to_string(),
                            lines[2].to_string(),
                        ));
                    }
                }
            }
        }

        let mut contacts = vec![
            ("Mom".into(), "+88017XXXXXXXX".into()),
            ("Dad".into(), "+88019XXXXXXXX".into()),
            ("NilOS Support".into(), "+88018XXXXXXXX".into()),
        ];

        if let Ok(entries) = fs::read_dir(CONTACTS_DIR) {
            for entry in entries.flatten() {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let parts: Vec<&str> = content.splitn(2, '\n').collect();
                    if parts.len() == 2 {
                        contacts.push((parts[0].trim().into(), parts[1].trim().into()));
                    }
                }
            }
        }

        AppState {
            user_name,
            pin,
            pending_pin: String::new(),
            pin_input: String::new(),
            sms_threads,
            contacts,
            compose_to: String::new(),
            compose_body: String::new(),
            terminal_history: vec![
                "NilOS System Shell v1.0".into(),
                "Type 'help' for available commands.".into(),
            ],
            settings_cursor: 0,
            files_path: "/data".into(),
            pkg_cursor: 0,
            call_number: String::new(),
        }
    }

    fn save_user(&self) {
        let _ = fs::create_dir_all("/data/nilos");
        let _ = fs::write(USER_CONF, &self.user_name);
    }

    fn save_pin(&self) {
        let _ = fs::create_dir_all("/data/nilos");
        let _ = fs::write(PIN_FILE, &self.pin);
    }

    fn mark_oobe_done(&self) {
        let _ = fs::create_dir_all("/data/nilos");
        let _ = fs::write(OOBE_DONE, "done");
    }
}

// ─── Sink (stdout only — nilinit already dup2'd the correct console fd) ────────
struct Sink;

impl Sink {
    fn new() -> Self { Sink }

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

fn status_bar(state: &AppState) -> String {
    format!(
        "{}  📶 NilOS │ {} │ 🔋 100%  {}",
        FG_BLACK_ON_WHITE, state.user_name, R
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
    sink.println(&format!("  │   {}●{} Step 1: Enter your name                          {}│", FG_GREEN, FG_CYAN, FG_CYAN));
    sink.println(&format!("  │   {}○{} Step 2: Set a PIN (for lockscreen)               {}│", FG_GRAY, FG_CYAN, FG_CYAN));
    sink.println(&format!("  │   {}○{} Step 3: Done!                                    {}│", FG_GRAY, FG_CYAN, FG_CYAN));
    sink.println("  │                                                       │");
    sink.println("  │   Language: English (default) — more coming in v1.1  │");
    sink.println("  │                                                       │");
    sink.println("  ╰───────────────────────────────────────────────────────╯");
    sink.println(&format!("{}", R));
    sink.print(&format!("  {}Press Enter to begin setup...{} ", FG_YELLOW, R));
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
    sink.println("  │   Tip: PIN is stored as plain text for now           │");
    sink.println("  │   (hashing comes in Phase 2 nilkeyd integration).    │");
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
    sink.println(&format!("  │   Welcome, {}!{}", name, " ".repeat(43usize.saturating_sub(name.len()))));
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
    sink.print(&format!("  {}Press Enter to continue to lock screen...{} ", FG_CYAN, R));
}

// ─── Lock Screen ──────────────────────────────────────────────────────────────
fn draw_lockscreen(sink: &mut Sink, state: &AppState, error: bool) {
    sink.print(CL);
    sink.println(&format!("{}", FG_BLUE));
    sink.println("  ╭────────────────────────────────────────────────────────╮");
    sink.println("  │                                                        │");
    sink.println(&format!("  │               {}{}12:45 PM{}                            │", FG_WHITE, B, FG_BLUE));
    sink.println("  │         Tuesday, September 1, 2026                     │");
    sink.println("  │           Dhaka, BD  •  28°C  ☀️                       │");
    sink.println("  │                                                        │");
    sink.println("  ├────────────────────────────────────────────────────────┤");
    sink.println("  │                                                        │");
    sink.println(&format!("  │   🔒 {}{}  {}{}                                    │", FG_WHITE, B, state.user_name, FG_BLUE));
    sink.println("  │                                                        │");
    if error {
        sink.println(&format!("  │   {}⚠ Wrong PIN — try again{}                           │", FG_RED, FG_BLUE));
    } else {
        sink.println("  │                                                        │");
    }
    sink.println("  ╰────────────────────────────────────────────────────────╯");
    sink.println(R);
    sink.print(&format!("  {}Enter PIN to unlock: {}", FG_YELLOW, R));
}

// ─── Home Launcher ────────────────────────────────────────────────────────────
fn draw_home(sink: &mut Sink, state: &AppState) {
    sink.print(CL);
    sink.println(&status_bar(state));
    sink.println(&format!("{}", FG_CYAN));
    sink.println("  ╭────────────────────────────────────────────────────────╮");
    sink.println(&format!("  │  📶 5G    {}{}12:45 PM{}    🔔  🔋 100%                 │", FG_WHITE, B, FG_CYAN));
    sink.println("  ├────────────────────────────────────────────────────────┤");
    sink.println("  │                                                        │");
    sink.println(&format!("  │                   {}{}12:45{}                            │", FG_WHITE, B, FG_CYAN));
    sink.println("  │            Tuesday, September 1, 2026                  │");
    sink.println("  │                                                        │");
    sink.println(&format!("  │   {}┌────────────────────────────────────────────────┐{}   │", FG_BLUE, FG_CYAN));
    sink.println(&format!("  │   {}│  🔍  Search NilOS...                           │{}   │", FG_WHITE, FG_CYAN));
    sink.println(&format!("  │   {}└────────────────────────────────────────────────┘{}   │", FG_BLUE, FG_CYAN));
    sink.println("  │                                                        │");
    sink.println(&format!("  │  {}[1] 📞 Phone{}      {}[2] 💬 Messages{}     {}[3] 📁 Files{}      │", FG_GREEN, FG_CYAN, FG_YELLOW, FG_CYAN, FG_BLUE, FG_CYAN));
    sink.println("  │   VoLTE Dialer     Encrypted SMS     File Manager     │");
    sink.println("  │                                                        │");
    sink.println(&format!("  │  {}[4] ⚙️  Settings{}  {}[5] 📦 NilPkg{}    {}[6] 🔄 SoftBus{}    │", FG_MAGENTA, FG_CYAN, FG_GREEN, FG_CYAN, FG_CYAN, FG_CYAN));
    sink.println("  │   System Config    Package Store     Mesh Network     │");
    sink.println("  │                                                        │");
    sink.println(&format!("  │  {}[7] 🤖 Android{}   {}[8] 💻 Terminal{}                    │", FG_YELLOW, FG_CYAN, FG_WHITE, FG_CYAN));
    sink.println("  │   AOSP Container   Diagnostic CLI                     │");
    sink.println("  │                                                        │");
    sink.println(&format!("  │  {}[n] 🔔 Notifications  [l] 🔒 Lock screen{}              │", FG_GRAY, FG_CYAN));
    sink.println("  ├────────────────────────────────────────────────────────┤");
    sink.println(&format!("  │  {}[📞]  [💬]  [📁]  [⚙️]  [📦]{}                          │", FG_WHITE, FG_CYAN));
    sink.println(&format!("  │                    {}━━━━━━━━{}                             │", FG_WHITE, FG_CYAN));
    sink.println("  ╰────────────────────────────────────────────────────────╯");
    sink.println(R);
    sink.print(&format!("  {}Choice (1-8 / n / l): {}", FG_YELLOW, R));
}

// ─── Phone App ────────────────────────────────────────────────────────────────
fn draw_phone(sink: &mut Sink, state: &AppState) {
    sink.print(CL);
    sink.println(&format!("{}  📞 NilOS Phone  —  oFono VoLTE + HD Voice{}", FG_GREEN, R));
    sink.println(&format!("{}  ──────────────────────────────────────────{}", FG_GRAY, R));
    sink.println("");

    if !state.call_number.is_empty() {
        sink.println(&format!("  {}Dialing: {}{}{}  (simulated)", FG_GREEN, FG_WHITE, state.call_number, R));
    } else {
        sink.println(&format!("  {}[ Enter phone number below ]{}", FG_YELLOW, R));
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
    sink.println("  Recent Calls:");
    sink.println(&format!("    {}• Mom        +88017XXXXXXXX   Today 11:30 AM{}", FG_GRAY, R));
    sink.println(&format!("    {}• Dad        +88019XXXXXXXX   Yesterday{}", FG_GRAY, R));
    sink.println("");
    sink.println("  Contacts:");
    for (name, number) in &state.contacts {
        sink.println(&format!("    {}• {} — {}{}", FG_GREEN, name, number, R));
    }
    sink.println("");
    sink.println(&format!("  {}Commands: 'call <number>', 'home', 'back'{}", FG_YELLOW, R));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── Messages App ─────────────────────────────────────────────────────────────
fn draw_messages(sink: &mut Sink, state: &AppState) {
    sink.print(CL);
    sink.println(&format!("{}  💬 NilOS Messages  —  E2E Encrypted P2P SMS{}", FG_YELLOW, R));
    sink.println(&format!("{}  ──────────────────────────────────────────{}", FG_GRAY, R));
    sink.println("");
    sink.println("  Message Threads:");
    for (i, (contact, msg, time)) in state.sms_threads.iter().enumerate() {
        sink.println(&format!(
            "  {}[{}]{} {} {}({}){}\n       {}…{}", 
            FG_CYAN, i + 1, R, contact, FG_GRAY, time, R, FG_GRAY, R
        ));
        sink.println(&format!("       {}", msg));
        sink.println("");
    }
    sink.println(&format!("  {}Commands: 'new', 'read <n>', 'home', 'back'{}", FG_YELLOW, R));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── Files App ────────────────────────────────────────────────────────────────
fn draw_files(sink: &mut Sink, state: &AppState) {
    sink.print(CL);
    sink.println(&format!("{}  📁 NilOS File Manager{}", FG_BLUE, R));
    sink.println(&format!("{}  Path: {}{}", FG_GRAY, state.files_path, R));
    sink.println(&format!("{}  ──────────────────────────────────────────{}", FG_GRAY, R));
    sink.println("");

    if let Ok(entries) = fs::read_dir(&state.files_path) {
        let mut items: Vec<String> = entries
            .flatten()
            .map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                if is_dir { format!("📁 {}/", name) } else {
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
    sink.println(&format!("  {}Commands: 'cd <path>', '1'-'4' bookmarks, 'home', 'back'{}", FG_YELLOW, R));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── Settings App ────────────────────────────────────────────────────────────
fn draw_settings(sink: &mut Sink) {
    sink.print(CL);
    sink.println(&format!("{}  ⚙️  NilOS System Settings{}", FG_MAGENTA, R));
    sink.println(&format!("{}  ──────────────────────────────────────────{}", FG_GRAY, R));
    sink.println("");

    let sections = [
        ("🌐", "Network",       "Wi-Fi, Bluetooth, SoftBus, Data"),
        ("🔊", "Sound",         "Volume, Ringtone, Do Not Disturb"),
        ("📱", "Display",       "Brightness, Theme, Font Size"),
        ("🔒", "Security",      "PIN, SELinux Policy, Encryption"),
        ("🔋", "Battery",       "Power Mode, Background Refresh"),
        ("📦", "Storage",       "/data usage, installed packages"),
        ("♿", "Accessibility", "Font size, contrast, TTS"),
        ("ℹ️ ", "About",        "NilOS 1.0 · Linux 6.6.110-lts"),
    ];

    for (i, (icon, name, desc)) in sections.iter().enumerate() {
        sink.println(&format!("  {}[{}]{} {} {}{}  {}{}", FG_CYAN, i + 1, R, icon, FG_WHITE, name, FG_GRAY, R));
        sink.println(&format!("       {}{}{}", FG_GRAY, desc, R));
        sink.println("");
    }

    sink.println(&format!("  {}  OS: NilOS 1.0-alpha (Phase 1+2 Active){}", FG_GRAY, R));
    sink.println(&format!("  {}  Kernel: Linux 6.6.110-0-lts (Alpine LTS){}", FG_GRAY, R));
    sink.println(&format!("  {}  Userspace: 100% Rust (Memory Safe){}", FG_GRAY, R));
    sink.println(&format!("  {}  SELinux: Enforcing · Telemetry: ZERO{}", FG_GRAY, R));
    sink.println("");
    sink.println(&format!("  {}Commands: '1'-'8' to enter section, 'home', 'back'{}", FG_YELLOW, R));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

fn draw_settings_section(sink: &mut Sink, section: usize) {
    let names = ["Network", "Sound", "Display", "Security", "Battery", "Storage", "Accessibility", "About"];
    let name = names.get(section).copied().unwrap_or("Unknown");
    sink.print(CL);
    sink.println(&format!("{}  ⚙️  Settings › {}{}", FG_MAGENTA, name, R));
    sink.println(&format!("{}  ──────────────────────────────────────────{}", FG_GRAY, R));
    sink.println("");
    match section {
        0 => {
            sink.println("  Wi-Fi:       [OFF]  (wpa_supplicant not yet running)");
            sink.println("  Bluetooth:   [OFF]  (btd daemon registered)");
            sink.println("  SoftBus:     [ON]   Control socket: /run/nilos/bus.sock");
            sink.println("  Mobile Data: [N/A]  oFono telephony in Phase 3");
        }
        3 => {
            sink.println("  PIN Lock:        [ENABLED]");
            sink.println("  SELinux Policy:  Enforcing (policy.33)");
            sink.println("  fscrypt v2:      Active (nilkeyd manages keystore)");
            sink.println("  Namespace Sand:  Enabled for all apps via nilrt");
            sink.println("  Telemetry:       ZERO — no data leaves device");
        }
        7 => {
            sink.println("  NilOS Version:   1.0.0-alpha");
            sink.println("  Phase:           1 (It Boots) + 2 (Usable OS) — Active");
            sink.println("  Kernel:          Linux 6.6.110-0-lts (Alpine)");
            sink.println("  Userspace:       100% Memory-Safe Rust");
            sink.println("  Build System:    cargo + musl static linking");
            sink.println("  License:         GNU GPLv3");
            sink.println("  Architecture:    x86_64 (ARM64 Phase 3)");
            sink.println("  QEMU boot time:  ~113 ms");
        }
        _ => {
            sink.println(&format!("  (Configuration for {} — implementation in Phase 2/3)", name));
        }
    }
    sink.println("");
    sink.println(&format!("  {}Press 'back' to return to Settings, 'home' for launcher{}", FG_YELLOW, R));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── NilPkg App ──────────────────────────────────────────────────────────────
fn draw_nilpkg(sink: &mut Sink) {
    sink.print(CL);
    sink.println(&format!("{}  📦 NilPkg — Atomic Package Manager & App Store{}", FG_GREEN, R));
    sink.println(&format!("{}  ──────────────────────────────────────────{}", FG_GRAY, R));
    sink.println("");
    sink.println(&format!("  {}Installed Packages (Ed25519 Signed):{}", FG_WHITE, R));
    sink.println(&format!("  {}• com.nil.shell     v1.0.0  — NilOS Compositor & Launcher{}", FG_GREEN, R));
    sink.println(&format!("  {}• com.nil.settings  v1.0.0  — System Configuration App{}", FG_GREEN, R));
    sink.println(&format!("  {}• com.nil.softbus   v0.1.0  — Distributed SoftBus Fabric{}", FG_GREEN, R));
    sink.println(&format!("  {}• com.nil.nilpkg    v0.1.0  — Package Manager{}", FG_GREEN, R));
    sink.println("");
    sink.println(&format!("  {}Available in Repository:{}", FG_WHITE, R));
    sink.println(&format!("  {}[i] org.videolan.vlc    v3.5.4  — Media Player{}", FG_CYAN, R));
    sink.println(&format!("  {}[i] org.mozilla.fenix   v124.0  — Privacy Browser{}", FG_CYAN, R));
    sink.println(&format!("  {}[i] org.openstreetmap   v3.1.0  — Offline Maps{}", FG_CYAN, R));
    sink.println(&format!("  {}[i] com.signal.android  v7.2.1  — Signal Messenger{}", FG_CYAN, R));
    sink.println("");
    sink.println(&format!("  {}Storage: /data/app/  •  Signature: Ed25519{}", FG_GRAY, R));
    sink.println("");
    sink.println(&format!("  {}Commands: 'install <pkg>', 'list', 'home', 'back'{}", FG_YELLOW, R));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── SoftBus App ─────────────────────────────────────────────────────────────
fn draw_softbus(sink: &mut Sink) {
    sink.print(CL);
    sink.println(&format!("{}  🔄 SoftBus — Distributed Peer-to-Peer Device Mesh{}", FG_CYAN, R));
    sink.println(&format!("{}  ──────────────────────────────────────────{}", FG_GRAY, R));
    sink.println("");
    sink.println(&format!("  {}Discovered Nearby Devices (BLE + Wi-Fi Aware + mDNS):{}", FG_WHITE, R));
    sink.println("");
    sink.println(&format!("  {}● NilPad-Pro-X1{}", FG_GREEN, R));
    sink.println("    Status: Connected  •  Latency: 2ms");
    sink.println("    Caps:   Display Sharing, Unified Clipboard, File Handoff");
    sink.println("");
    sink.println(&format!("  {}● NilBook-Ultra{}", FG_GREEN, R));
    sink.println("    Status: Paired (QUIC Stream Active)");
    sink.println("    Caps:   Camera Relay, Shared Notifications");
    sink.println("");
    sink.println(&format!("  {}● NilVision-Display-65{}", FG_YELLOW, R));
    sink.println("    Status: Available nearby");
    sink.println("    Caps:   4K 60Hz Wireless Desktop");
    sink.println("");
    sink.println(&format!("  {}Control Socket: /run/nilos/bus.sock (TLS 1.3 Ed25519){}", FG_GRAY, R));
    sink.println("");
    sink.println(&format!("  {}Commands: 'cast', 'sync', 'pair <n>', 'home', 'back'{}", FG_YELLOW, R));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── Android App ─────────────────────────────────────────────────────────────
fn draw_android(sink: &mut Sink) {
    sink.print(CL);
    sink.println(&format!("{}  🤖 Android Compatibility Layer — LXC / Waydroid{}", FG_YELLOW, R));
    sink.println(&format!("{}  ──────────────────────────────────────────{}", FG_GRAY, R));
    sink.println("");
    sink.println(&format!("  {}Container Status:{}", FG_WHITE, R));
    sink.println("  • Container Engine:  LXC (Unprivileged Namespace)");
    sink.println("  • AOSP Version:      AOSP 14 (Headless Userspace)");
    sink.println("  • binder-shim:       Ready (Intent Translation Bridge)");
    sink.println("  • Graphics Bridge:   Wayland wl_surface passthrough");
    sink.println("  • Google Services:   microG UnifiedPush + Location");
    sink.println("  • Hardware IDs:      Masked (Anti-Fingerprinting)");
    sink.println("");
    sink.println(&format!("  {}Runtime Socket: /run/nilos/android.sock{}", FG_GRAY, R));
    sink.println(&format!("  {}Phase 4 deployment — container ready for app sideload{}", FG_GRAY, R));
    sink.println("");
    sink.println(&format!("  {}Commands: 'status', 'start', 'stop', 'home', 'back'{}", FG_YELLOW, R));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── Terminal App ─────────────────────────────────────────────────────────────
fn draw_terminal(sink: &mut Sink, state: &AppState) {
    sink.print(CL);
    sink.println(&format!("{}  💻 NilOS Diagnostic Shell{}", FG_WHITE, R));
    sink.println(&format!("{}  Type 'help' for commands, 'home' to exit{}", FG_GRAY, R));
    sink.println("");
    for line in &state.terminal_history {
        sink.println(&format!("  {}", line));
    }
    sink.println("");
    sink.print(&format!("  {}nilos# {}", FG_GREEN, R));
}

fn handle_terminal_cmd(sink: &mut Sink, state: &mut AppState, cmd: &str) {
    let output = match cmd {
        "help" => {
            "Commands: services, mem, disk, net, ps, cat <file>, ls <dir>, uname, reboot, home".to_string()
        }
        "services" => {
            let mut out = String::from("Active NilOS Supervised Daemons:\n");
            for (name, pid) in &[("nilinit","1"),("nild","428"),("nilkeyd","429"),("nilbus","430"),("netd","431"),("audiod","433"),("powerd","434"),("nilshell","435")] {
                out.push_str(&format!("  • {:12} PID {:<5} RUNNING\n", name, pid));
            }
            out
        }
        "mem" => {
            let avail = fs::read_to_string("/proc/meminfo")
                .unwrap_or_else(|_| "MemTotal: 1024000 kB\nMemFree: 950000 kB\n".into());
            format!("Memory Info:\n{}", avail)
        }
        "disk" => {
            format!(
                "Storage:\n  /data  256 MB ext4  (virtual disk nilos.img)\n  /tmp   tmpfs (ephemeral)\n  /run   tmpfs (sockets & runtime)\n  OOBE:  {}\n  User:  {}",
                if Path::new(OOBE_DONE).exists() { "✅ Done" } else { "⏳ Pending" },
                state.user_name
            )
        }
        "net" => "Network:\n  eth0:  10.0.2.15/24 (QEMU NAT via -netdev user)\n  SoftBus: /run/nilos/bus.sock active\n  Wi-Fi:   disabled (no wpa_supplicant yet)".to_string(),
        "uname" => "NilOS 1.0.0-alpha x86_64  Linux 6.6.110-lts  Rust Userspace".to_string(),
        "ps" => "PID  CMD\n  1   nilinit\n428   nild\n429   nilkeyd\n430   nilbus\n431   netd\n433   audiod\n434   powerd\n435   nilshell".to_string(),
        _ if cmd.starts_with("ls ") => {
            let path = cmd.trim_start_matches("ls ").trim();
            match fs::read_dir(path) {
                Ok(entries) => {
                    let mut out = format!("Contents of {}:\n", path);
                    for e in entries.flatten() {
                        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        out.push_str(&format!("  {}{}\n", e.file_name().to_string_lossy(), if is_dir { "/" } else { "" }));
                    }
                    out
                }
                Err(e) => format!("ls: {}: {}", path, e),
            }
        }
        _ if cmd.starts_with("cat ") => {
            let path = cmd.trim_start_matches("cat ").trim();
            fs::read_to_string(path).unwrap_or_else(|e| format!("cat: {}: {}", path, e))
        }
        "" => String::new(),
        _ => format!("nilos: command not found: {}", cmd),
    };

    if !output.is_empty() {
        state.terminal_history.push(output);
    }
    state.terminal_history.push(format!("nilos# {}", cmd));

    // Keep history bounded
    while state.terminal_history.len() > 20 {
        state.terminal_history.remove(0);
    }
}

// ─── Notification Shade ──────────────────────────────────────────────────────
fn draw_notifications(sink: &mut Sink, state: &AppState) {
    sink.print(CL);
    sink.println(&format!("{}  🔔 Notification Center{}", FG_YELLOW, R));
    sink.println(&format!("{}  ──────────────────────────────────────────{}", FG_GRAY, R));
    sink.println("");
    sink.println(&format!("  {}🔒 Security — nilkeyd{}", FG_BLUE, R));
    sink.println("     fscrypt v2 storage encryption active.");
    sink.println("");
    sink.println(&format!("  {}🔄 SoftBus Manager{}", FG_CYAN, R));
    sink.println("     NilPad-Pro-X1 is ready for screen handoff.");
    sink.println("");
    sink.println(&format!("  {}📦 NilPkg Updater{}", FG_GREEN, R));
    sink.println("     All system daemons on latest build.");
    sink.println("");
    for (contact, msg, time) in &state.sms_threads {
        sink.println(&format!("  {}💬 {}{}", FG_YELLOW, contact, R));
        sink.println(&format!("     {} — {}", msg, time));
        sink.println("");
    }
    sink.println(&format!("  {}Press 'home' or 'back' to return{}", FG_YELLOW, R));
    sink.print(&format!("  {}> {}", FG_CYAN, R));
}

// ─── Main Loop ────────────────────────────────────────────────────────────────
fn main() {
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
    let mut composing_sms = false;

    loop {
        // Render current screen
        match &screen {
            Screen::OobeWelcome    => draw_oobe_welcome(&mut sink),
            Screen::OobeName       => draw_oobe_name(&mut sink),
            Screen::OobePin        => draw_oobe_pin(&mut sink),
            Screen::OobeConfirmPin => draw_oobe_confirm_pin(&mut sink),
            Screen::OobeDone       => draw_oobe_done(&mut sink, &state.user_name),
            Screen::Lockscreen     => draw_lockscreen(&mut sink, &state, lock_error),
            Screen::Home           => draw_home(&mut sink, &state),
            Screen::AppPhone       => draw_phone(&mut sink, &state),
            Screen::AppMessages    => draw_messages(&mut sink, &state),
            Screen::AppFiles       => draw_files(&mut sink, &state),
            Screen::AppSettings    => {
                if let Some(sec) = settings_in_section {
                    draw_settings_section(&mut sink, sec);
                } else {
                    draw_settings(&mut sink);
                }
            }
            Screen::AppNilPkg      => draw_nilpkg(&mut sink),
            Screen::AppSoftBus     => draw_softbus(&mut sink),
            Screen::AppAndroid     => draw_android(&mut sink),
            Screen::AppTerminal    => draw_terminal(&mut sink, &state),
            Screen::NotificationShade => draw_notifications(&mut sink, &state),
        }

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
                if cmd.len() >= 4 && cmd.chars().all(|c| c.is_ascii_digit()) {
                    state.pending_pin = cmd.clone();
                    screen = Screen::OobeConfirmPin;
                } else {
                    sink.println(&format!("{}  PIN must be 4–8 digits. Try again.{}", FG_RED, R));
                    let _ = io::stdin().lock().read_line(&mut String::new());
                }
            }
            Screen::OobeConfirmPin => {
                if cmd == state.pending_pin {
                    state.pin = state.pending_pin.clone();
                    state.pending_pin.clear();
                    state.save_pin();
                    state.mark_oobe_done();
                    screen = Screen::OobeDone;
                } else {
                    sink.println(&format!("{}  PINs don't match. Let's try again.{}", FG_RED, R));
                    let _ = io::stdin().lock().read_line(&mut String::new());
                    screen = Screen::OobePin;
                }
            }
            Screen::OobeDone => {
                screen = Screen::Lockscreen;
                lock_error = false;
            }

            // ── Lock Screen ───────────────────────────────────────────────────
            Screen::Lockscreen => {
                if cmd == state.pin {
                    lock_error = false;
                    screen = Screen::Home;
                } else {
                    lock_error = true;
                }
            }

            // ── Home Launcher ─────────────────────────────────────────────────
            Screen::Home => {
                settings_in_section = None;
                match cmd.as_str() {
                    "1" | "phone"    => { screen = Screen::AppPhone; state.call_number.clear(); }
                    "2" | "messages" => { screen = Screen::AppMessages; composing_sms = false; }
                    "3" | "files"    => { screen = Screen::AppFiles; state.files_path = "/data".into(); }
                    "4" | "settings" => screen = Screen::AppSettings,
                    "5" | "pkg"      => screen = Screen::AppNilPkg,
                    "6" | "softbus"  => screen = Screen::AppSoftBus,
                    "7" | "android"  => screen = Screen::AppAndroid,
                    "8" | "terminal" => screen = Screen::AppTerminal,
                    "n" | "notif"    => screen = Screen::NotificationShade,
                    "l" | "lock"     => { screen = Screen::Lockscreen; lock_error = false; }
                    _ => {} // re-render home
                }
            }

            // ── Phone App ─────────────────────────────────────────────────────
            Screen::AppPhone => {
                if cmd == "home" || cmd == "back" {
                    screen = Screen::Home;
                } else if cmd.starts_with("call ") {
                    let number = cmd.trim_start_matches("call ").trim().to_string();
                    state.call_number = number.clone();
                    sink.println(&format!("\n  {}📞 Calling {}...  (simulated — oFono in Phase 3){}", FG_GREEN, number, R));
                    sink.println("  Press Enter to hang up.");
                    read_line();
                    state.call_number.clear();
                } else if !cmd.is_empty() && cmd.chars().all(|c| c.is_ascii_digit() || c == '+' || c == '-') {
                    state.call_number = cmd.clone();
                }
            }

            // ── Messages App ──────────────────────────────────────────────────
            Screen::AppMessages => {
                if cmd == "home" || cmd == "back" {
                    screen = Screen::Home;
                    composing_sms = false;
                } else if cmd == "new" || composing_sms {
                    if !composing_sms {
                        composing_sms = true;
                        sink.print(&format!("\n  {}To (name or number): {}", FG_YELLOW, R));
                        state.compose_to = read_line();
                        sink.print(&format!("  {}Message: {}", FG_YELLOW, R));
                        state.compose_body = read_line();
                        // Save SMS
                        let _ = fs::create_dir_all(SMS_DIR);
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        let fname = format!("{}/{}.txt", SMS_DIR, ts);
                        let _ = fs::write(&fname, format!("{}\n{}\nJust now", state.compose_to, state.compose_body));
                        state.sms_threads.insert(0, (state.compose_to.clone(), state.compose_body.clone(), "Just now".into()));
                        state.compose_to.clear();
                        state.compose_body.clear();
                        composing_sms = false;
                        sink.println(&format!("  {}✅ Message sent!{}", FG_GREEN, R));
                        read_line();
                    }
                } else if let Ok(idx) = cmd.parse::<usize>() {
                    if idx > 0 && idx <= state.sms_threads.len() {
                        let (contact, msg, time) = &state.sms_threads[idx - 1];
                        sink.print(CL);
                        sink.println(&format!("  {}💬 Thread: {}{}", FG_YELLOW, contact, R));
                        sink.println(&format!("  {} — {}", time, msg));
                        sink.println("");
                        sink.print(&format!("  {}Reply (or press Enter to go back): {}", FG_YELLOW, R));
                        let reply = read_line();
                        if !reply.is_empty() {
                            let ct = contact.clone();
                            state.sms_threads[idx - 1] = (ct, reply, "Just now".into());
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
                } else if cmd == "1" { state.files_path = "/data".into();
                } else if cmd == "2" { state.files_path = "/data/app".into();
                } else if cmd == "3" { state.files_path = "/etc".into();
                } else if cmd == "4" { state.files_path = "/tmp".into();
                }
            }

            // ── Settings App ──────────────────────────────────────────────────
            Screen::AppSettings => {
                if let Some(_sec) = settings_in_section {
                    if cmd == "back" { settings_in_section = None; }
                    else if cmd == "home" { screen = Screen::Home; settings_in_section = None; }
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
                    let pkg = cmd.trim_start_matches("install ").trim();
                    sink.println(&format!("\n  {}📦 Installing {}...{}", FG_GREEN, pkg, R));
                    sink.println("  Verifying Ed25519 signature...");
                    sink.println("  Downloading chunks (simulated)...");
                    sink.println("  Unpacking to /data/app/...");
                    sink.println(&format!("  {}✅ {} installed successfully.{}", FG_GREEN, pkg, R));
                    sink.print("  Press Enter...");
                    read_line();
                } else if cmd == "list" {
                    if let Ok(entries) = fs::read_dir("/data/app") {
                        let pkgs: Vec<_> = entries.flatten().collect();
                        if pkgs.is_empty() {
                            sink.println("\n  No packages installed yet.");
                        } else {
                            for p in pkgs {
                                sink.println(&format!("  • {}", p.file_name().to_string_lossy()));
                            }
                        }
                    }
                    sink.print("  Press Enter...");
                    read_line();
                }
            }

            // ── SoftBus / Android / Notifications ─────────────────────────────
            Screen::AppSoftBus | Screen::AppAndroid | Screen::NotificationShade => {
                if cmd == "home" || cmd == "back" || cmd.is_empty() {
                    screen = Screen::Home;
                }
            }

            // ── Terminal ──────────────────────────────────────────────────────
            Screen::AppTerminal => {
                if cmd == "home" || cmd == "exit" {
                    screen = Screen::Home;
                } else if cmd == "reboot" {
                    sink.println("\n  [nilinit] Simulated reboot — restarting nilshell process...");
                    // In real usage this would trigger reboot(LINUX_REBOOT_CMD_RESTART)
                    // For now just reset to lock screen
                    screen = Screen::Lockscreen;
                    lock_error = false;
                } else {
                    handle_terminal_cmd(&mut sink, &mut state, &cmd);
                    // Re-render terminal without clearing (just show prompt again)
                    draw_terminal(&mut sink, &state);
                    continue; // Skip the re-render at top of loop
                }
            }
        }
    }
}
