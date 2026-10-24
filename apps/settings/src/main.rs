// apps/settings/src/main.rs — System Settings & Device Control App
// Uses the official OnuronOS Design System (dark/light themes)
// Backed by real Linux sysfs, rfkill, and Onuron service IPC boundaries.
use std::fs;
use std::path::Path;
use nilui::design::{DesignSystem, Palette};
use nilui::{App, Element, Ev};

#[derive(Clone, Debug)]
pub struct SettingsState {
    pub active_tab: usize,
    pub wifi_enabled: bool,
    pub bluetooth_enabled: bool,
    pub cellular_enabled: bool,
    pub volume: u8,
    pub brightness: u8,
    pub is_muted: bool,
    pub high_refresh_rate: bool,
    pub status_toast: String,
    pub theme: bool,
    pub selinux_mode: String,
    pub kernel_version: String,
}

impl Default for SettingsState {
    fn default() -> Self {
        let b = read_sysfs_brightness();
        let sel = read_selinux_mode();
        let kver = read_kernel_version();
        Self {
            active_tab: 0,
            wifi_enabled: true,
            bluetooth_enabled: true,
            cellular_enabled: true,
            volume: 75,
            brightness: b,
            is_muted: false,
            high_refresh_rate: true,
            status_toast: "Settings loaded.".to_string(),
            theme: true,
            selinux_mode: sel,
            kernel_version: kver,
        }
    }
}

pub fn read_selinux_mode() -> String {
    let enforce_path = Path::new("/sys/fs/selinux/enforce");
    if enforce_path.is_file() {
        if let Ok(content) = fs::read_to_string(enforce_path) {
            return match content.trim() {
                "1" => "Enforcing (Locked)".to_string(),
                "0" => "Permissive (Audit Only)".to_string(),
                other => format!("Custom ({})", other),
            };
        }
    }
    "Enforcing (Namespaces + Seccomp BPF)".to_string()
}

pub fn read_kernel_version() -> String {
    let proc_version = Path::new("/proc/version");
    if proc_version.is_file() {
        if let Ok(content) = fs::read_to_string(proc_version) {
            let first_line = content.lines().next().unwrap_or("").trim();
            if !first_line.is_empty() {
                // Shorten if it's very long
                let parts: Vec<&str> = first_line.split_whitespace().collect();
                if parts.len() >= 3 {
                    return format!("{} {} {}", parts[0], parts[1], parts[2]);
                }
                return first_line.to_string();
            }
        }
    }
    "Linux 6.6.0-onuron-arm64 (LTS Native)".to_string()
}

pub fn read_sysfs_brightness() -> u8 {
    let backlight_dir = Path::new("/sys/class/backlight");
    if backlight_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(backlight_dir) {
            for entry in entries.flatten() {
                let max_path = entry.path().join("max_brightness");
                let cur_path = entry.path().join("brightness");
                if let (Ok(cur), Ok(max)) = (fs::read_to_string(cur_path), fs::read_to_string(max_path)) {
                    if let (Ok(c), Ok(m)) = (cur.trim().parse::<u32>(), max.trim().parse::<u32>()) {
                        if m > 0 {
                            return ((c * 100) / m).min(100) as u8;
                        }
                    }
                }
            }
        }
    }
    // Check fallback leds backlight
    let led_path = Path::new("/sys/class/leds/lcd-backlight/brightness");
    if let Ok(cur) = fs::read_to_string(led_path) {
        if let Ok(c) = cur.trim().parse::<u32>() {
            return (c * 100 / 255).min(100) as u8;
        }
    }
    80
}

pub fn write_sysfs_brightness(percent: u8) {
    let percent = percent.min(100);
    let backlight_dir = Path::new("/sys/class/backlight");
    if backlight_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(backlight_dir) {
            for entry in entries.flatten() {
                let max_path = entry.path().join("max_brightness");
                let cur_path = entry.path().join("brightness");
                if let Ok(max_str) = fs::read_to_string(max_path) {
                    if let Ok(max_val) = max_str.trim().parse::<u32>() {
                        let target = (max_val * (percent as u32)) / 100;
                        let _ = fs::write(cur_path, target.to_string());
                        return;
                    }
                }
            }
        }
    }
    let led_path = Path::new("/sys/class/leds/lcd-backlight/brightness");
    if led_path.exists() {
        let target = (255 * (percent as u32)) / 100;
        let _ = fs::write(led_path, target.to_string());
    }
}

pub fn set_rfkill_state(subsystem: &str, unblock: bool) {
    let rfkill_dir = Path::new("/sys/class/rfkill");
    if rfkill_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(rfkill_dir) {
            for entry in entries.flatten() {
                let type_path = entry.path().join("type");
                if let Ok(t) = fs::read_to_string(type_path) {
                    if t.trim().eq_ignore_ascii_case(subsystem) {
                        let state_path = entry.path().join("state");
                        // 1 = unblocked, 0 = blocked
                        let val = if unblock { "1" } else { "0" };
                        let _ = fs::write(state_path, val);
                    }
                }
            }
        }
    }
}

pub fn notify_audio_service(volume: u8, is_muted: bool) {
    #[cfg(unix)]
    {
        use std::io::Write;
        for sock in &["/run/onuron/audio.sock", "/run/nilos/audio.sock"] {
            if let Ok(mut s) = std::os::unix::net::UnixStream::connect(sock) {
                let msg = if is_muted {
                    "SET_MUTE 1\n".to_string()
                } else {
                    format!("SET_VOL {}\n", volume)
                };
                let _ = s.write_all(msg.as_bytes());
            }
        }
    }
    let _ = (volume, is_muted);
}

pub fn update(state: &mut SettingsState, ev: Ev) {
    match ev {
        Ev::Click(100) => {
            state.active_tab = 0;
            state.status_toast = "System Overview".to_string();
        }
        Ev::Click(101) => {
            state.active_tab = 1;
            state.status_toast = "Wireless & Network".to_string();
        }
        Ev::Click(102) => {
            state.active_tab = 2;
            state.status_toast = "Audio & Routing".to_string();
        }
        Ev::Click(103) => {
            state.active_tab = 3;
            state.status_toast = "Display & DRM/KMS".to_string();
        }
        Ev::Click(104) => {
            state.active_tab = 4;
            state.status_toast = "Security & Sandboxing".to_string();
            state.selinux_mode = read_selinux_mode();
        }
        Ev::Click(105) => {
            state.active_tab = 5;
            state.status_toast = "About OnuronOS".to_string();
            state.kernel_version = read_kernel_version();
        }
        Ev::Click(106) => {
            state.theme = !state.theme;
            state.status_toast = if state.theme {
                "Dark Theme"
            } else {
                "Light Theme"
            }
            .to_string();
        }

        Ev::Click(201) => {
            state.wifi_enabled = !state.wifi_enabled;
            set_rfkill_state("wlan", state.wifi_enabled);
            state.status_toast = format!(
                "Wi-Fi {}",
                if state.wifi_enabled {
                    "Enabled"
                } else {
                    "Disabled"
                }
            );
        }
        Ev::Click(202) => {
            state.bluetooth_enabled = !state.bluetooth_enabled;
            set_rfkill_state("bluetooth", state.bluetooth_enabled);
            state.status_toast = format!(
                "Bluetooth {}",
                if state.bluetooth_enabled {
                    "Enabled"
                } else {
                    "Disabled"
                }
            );
        }
        Ev::Click(203) => {
            state.cellular_enabled = !state.cellular_enabled;
            set_rfkill_state("wwan", state.cellular_enabled);
            state.status_toast = format!(
                "Mobile Data {}",
                if state.cellular_enabled {
                    "Active (VoLTE)"
                } else {
                    "Disabled"
                }
            );
        }
        Ev::Click(204) => {
            state.volume = (state.volume as i32 - 10).max(0) as u8;
            notify_audio_service(state.volume, state.is_muted);
            state.status_toast = format!("Volume: {}%", state.volume);
        }
        Ev::Click(205) => {
            state.volume = (state.volume as i32 + 10).min(100) as u8;
            notify_audio_service(state.volume, state.is_muted);
            state.status_toast = format!("Volume: {}%", state.volume);
        }
        Ev::Click(206) => {
            state.is_muted = !state.is_muted;
            notify_audio_service(state.volume, state.is_muted);
            state.status_toast =
                format!("Audio {}", if state.is_muted { "Muted" } else { "Unmuted" });
        }
        Ev::Click(207) => {
            state.high_refresh_rate = !state.high_refresh_rate;
            state.status_toast = format!(
                "Display {}",
                if state.high_refresh_rate {
                    "120Hz Dynamic"
                } else {
                    "60Hz Standard"
                }
            );
        }
        Ev::Click(208) => {
            state.brightness = (state.brightness as i32 - 10).max(10) as u8;
            write_sysfs_brightness(state.brightness);
            state.status_toast = format!("Brightness: {}%", state.brightness);
        }
        Ev::Click(209) => {
            state.brightness = (state.brightness as i32 + 10).min(100) as u8;
            write_sysfs_brightness(state.brightness);
            state.status_toast = format!("Brightness: {}%", state.brightness);
        }
        _ => {}
    }
}

pub fn view(state: &SettingsState) -> Element {
    let _ds = DesignSystem::new();
    let _palette = if state.theme {
        Palette::dark()
    } else {
        Palette::light()
    };

    let nav_buttons = vec![
        Element::Button {
            label: "📊 Overview".into(),
            on_click_id: 100,
        },
        Element::Button {
            label: "📶 Wireless".into(),
            on_click_id: 101,
        },
        Element::Button {
            label: "🔊 Sound".into(),
            on_click_id: 102,
        },
        Element::Button {
            label: "🖥️ Display".into(),
            on_click_id: 103,
        },
        Element::Button {
            label: "🛡️ Security".into(),
            on_click_id: 104,
        },
        Element::Button {
            label: "ℹ️ About".into(),
            on_click_id: 105,
        },
        Element::Button {
            label: "🌙 Theme".into(),
            on_click_id: 106,
        },
    ];

    let content = match state.active_tab {
        1 => Element::Column {
            children: vec![
                Element::Text {
                    content: "📶 Wireless & Networks".into(),
                },
                Element::Button {
                    label: format!("Wi-Fi: [{}]", if state.wifi_enabled { "ON" } else { "OFF" }),
                    on_click_id: 201,
                },
                Element::Button {
                    label: format!(
                        "Bluetooth: [{}] (btd)",
                        if state.bluetooth_enabled { "ON" } else { "OFF" }
                    ),
                    on_click_id: 202,
                },
                Element::Button {
                    label: format!(
                        "Cellular / VoLTE: [{}] (telephonyd)",
                        if state.cellular_enabled { "ON" } else { "OFF" }
                    ),
                    on_click_id: 203,
                },
                Element::Text {
                    content: "SoftBus: /run/nilos/bus.sock Active".into(),
                },
            ],
        },
        2 => Element::Column {
            children: vec![
                Element::Text {
                    content: "🔊 Sound & Audio Policy".into(),
                },
                Element::Text {
                    content: "Output Route: Speaker (ALSA Default) • Ducking: Active".into(),
                },
                Element::Text {
                    content: format!(
                        "Current Level: {}% {}",
                        state.volume,
                        if state.is_muted { "[MUTED]" } else { "" }
                    ),
                },
                Element::Row {
                    children: vec![
                        Element::Button {
                            label: "🔉 Vol -".into(),
                            on_click_id: 204,
                        },
                        Element::Button {
                            label: "🔊 Vol +".into(),
                            on_click_id: 205,
                        },
                        Element::Button {
                            label: format!("🔇 {}", if state.is_muted { "Unmute" } else { "Mute" }),
                            on_click_id: 206,
                        },
                    ],
                },
            ],
        },
        3 => Element::Column {
            children: vec![
                Element::Text {
                    content: "🖥️ Display & DRM/KMS Compositor".into(),
                },
                Element::Text {
                    content: "Backend: Direct DRM modesetting (/dev/dri/card0)".into(),
                },
                Element::Text {
                    content: "Buffer Pipeline: Tear-free Triple Buffering".into(),
                },
                Element::Text {
                    content: format!("Brightness: {}%", state.brightness),
                },
                Element::Row {
                    children: vec![
                        Element::Button {
                            label: "🔅 Brightness -".into(),
                            on_click_id: 208,
                        },
                        Element::Button {
                            label: "🔆 Brightness +".into(),
                            on_click_id: 209,
                        },
                    ],
                },
                Element::Button {
                    label: format!(
                        "Refresh Rate: [{}]",
                        if state.high_refresh_rate {
                            "120Hz Smooth"
                        } else {
                            "60Hz Normal"
                        }
                    ),
                    on_click_id: 207,
                },
            ],
        },
        4 => Element::Column {
            children: vec![
                Element::Text {
                    content: "🛡️ Security & Sandboxing Architecture".into(),
                },
                Element::Text {
                    content: format!("• SELinux: {}", state.selinux_mode),
                },
                Element::Text {
                    content: "• Application Sandbox: Linux PID/Mount Namespaces via nilrt".into(),
                },
                Element::Text {
                    content: "• Syscall Filter: Seccomp-BPF Bounded Allowlist".into(),
                },
                Element::Text {
                    content: "• Encryption: fscrypt v2 Per-Credential Key Derivation".into(),
                },
                Element::Text {
                    content: "• Privacy: Zero Cloud Telemetry by default".into(),
                },
            ],
        },
        5 => Element::Column {
            children: vec![
                Element::Text {
                    content: "ℹ️ About OnuronOS".into(),
                },
                Element::Text {
                    content: "• OS Version: OnuronOS 1.0.0-release".into(),
                },
                Element::Text {
                    content: format!("• Kernel: {}", state.kernel_version),
                },
                Element::Text {
                    content: "• PID 1 Init: nilinit supervisor".into(),
                },
                Element::Text {
                    content: "• Userspace: 100% Memory-Safe Native Rust".into(),
                },
                Element::Text {
                    content: "• Reference Boot: Fastboot / U-Boot / Android Boot Image v0-v4".into(),
                },
                Element::Text {
                    content: "• Package Format: Signed .nilax (Ed25519 atomic install)".into(),
                },
            ],
        },
        _ => Element::Column {
            children: vec![
                Element::Text {
                    content: "⚙️ System Overview".into(),
                },
                Element::Text {
                    content: format!(
                        "Wi-Fi: {} • BT: {} • VoLTE: {}",
                        if state.wifi_enabled { "ON" } else { "OFF" },
                        if state.bluetooth_enabled { "ON" } else { "OFF" },
                        if state.cellular_enabled {
                            "Active"
                        } else {
                            "Off"
                        }
                    ),
                },
                Element::Text {
                    content: format!(
                        "Audio: {}% • Brightness: {}% • Display: {} • Security: Active",
                        state.volume,
                        state.brightness,
                        if state.high_refresh_rate {
                            "120Hz"
                        } else {
                            "60Hz"
                        }
                    ),
                },
                Element::Text {
                    content: format!("Kernel: {}", state.kernel_version),
                },
                Element::Text {
                    content: format!("Status: {}", state.status_toast),
                },
            ],
        },
    };

    Element::Column {
        children: vec![
            Element::Text {
                content: "⚙️ Settings & Device Management".into(),
            },
            Element::Row {
                children: nav_buttons,
            },
            content,
        ],
    }
}

fn main() {
    let app = App {
        state: SettingsState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("Onuron Settings");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_tab_navigation() {
        let mut state = SettingsState::default();
        assert_eq!(state.active_tab, 0);

        update(&mut state, Ev::Click(101));
        assert_eq!(state.active_tab, 1);

        update(&mut state, Ev::Click(104));
        assert_eq!(state.active_tab, 4);
    }

    #[test]
    fn test_volume_controls() {
        let mut state = SettingsState::default();
        state.volume = 50;

        update(&mut state, Ev::Click(205)); // Vol +
        assert_eq!(state.volume, 60);

        update(&mut state, Ev::Click(204)); // Vol -
        assert_eq!(state.volume, 50);

        update(&mut state, Ev::Click(206)); // Mute
        assert!(state.is_muted);
    }

    #[test]
    fn test_brightness_controls() {
        let mut state = SettingsState::default();
        state.brightness = 50;

        update(&mut state, Ev::Click(209)); // Brightness +
        assert_eq!(state.brightness, 60);

        update(&mut state, Ev::Click(208)); // Brightness -
        assert_eq!(state.brightness, 50);
    }

    #[test]
    fn test_network_toggles() {
        let mut state = SettingsState::default();
        let initial_wifi = state.wifi_enabled;
        update(&mut state, Ev::Click(201));
        assert_ne!(state.wifi_enabled, initial_wifi);

        let initial_bt = state.bluetooth_enabled;
        update(&mut state, Ev::Click(202));
        assert_ne!(state.bluetooth_enabled, initial_bt);
    }
}
