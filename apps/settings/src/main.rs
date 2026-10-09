// apps/settings/src/main.rs — System Settings & Device Control App
use nilui::{App, Element, Ev};

#[derive(Clone, Debug)]
struct SettingsState {
    active_tab: usize,
    wifi_enabled: bool,
    bluetooth_enabled: bool,
    cellular_enabled: bool,
    volume: u8,
    is_muted: bool,
    high_refresh_rate: bool,
    status_toast: String,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            active_tab: 0,
            wifi_enabled: true,
            bluetooth_enabled: true,
            cellular_enabled: true,
            volume: 75,
            is_muted: false,
            high_refresh_rate: true,
            status_toast: "Settings loaded.".to_string(),
        }
    }
}

fn update(state: &mut SettingsState, ev: Ev) {
    match ev {
        Ev::Click(100) => {
            state.active_tab = 0; // Overview
            state.status_toast = "System Overview".to_string();
        }
        Ev::Click(101) => {
            state.active_tab = 1; // Connectivity
            state.status_toast = "Wireless & Network".to_string();
        }
        Ev::Click(102) => {
            state.active_tab = 2; // Sound
            state.status_toast = "Audio & Routing".to_string();
        }
        Ev::Click(103) => {
            state.active_tab = 3; // Display
            state.status_toast = "Display & DRM/KMS".to_string();
        }
        Ev::Click(104) => {
            state.active_tab = 4; // Security
            state.status_toast = "Security & Sandboxing".to_string();
        }
        Ev::Click(105) => {
            state.active_tab = 5; // About
            state.status_toast = "About OnuronOS".to_string();
        }

        // Toggles
        Ev::Click(201) => {
            state.wifi_enabled = !state.wifi_enabled;
            state.status_toast = format!("Wi-Fi {}", if state.wifi_enabled { "Connected" } else { "Disconnected" });
        }
        Ev::Click(202) => {
            state.bluetooth_enabled = !state.bluetooth_enabled;
            state.status_toast = format!("Bluetooth {}", if state.bluetooth_enabled { "Enabled" } else { "Disabled" });
        }
        Ev::Click(203) => {
            state.cellular_enabled = !state.cellular_enabled;
            state.status_toast = format!("Mobile Data {}", if state.cellular_enabled { "Active (VoLTE)" } else { "Disabled" });
        }
        Ev::Click(204) => {
            if state.volume >= 10 {
                state.volume -= 10;
            } else {
                state.volume = 0;
            }
            state.status_toast = format!("Volume: {}%", state.volume);
        }
        Ev::Click(205) => {
            if state.volume <= 90 {
                state.volume += 10;
            } else {
                state.volume = 100;
            }
            state.status_toast = format!("Volume: {}%", state.volume);
        }
        Ev::Click(206) => {
            state.is_muted = !state.is_muted;
            state.status_toast = format!("Audio {}", if state.is_muted { "Muted" } else { "Unmuted" });
        }
        Ev::Click(207) => {
            state.high_refresh_rate = !state.high_refresh_rate;
            state.status_toast = format!("Display {}", if state.high_refresh_rate { "120Hz Dynamic" } else { "60Hz Standard" });
        }
        _ => {}
    }
}

fn view(state: &SettingsState) -> Element {
    let nav_buttons = vec![
        Element::Button { label: "📊 Overview".into(), on_click_id: 100 },
        Element::Button { label: "📶 Wireless".into(), on_click_id: 101 },
        Element::Button { label: "🔊 Sound".into(), on_click_id: 102 },
        Element::Button { label: "🖥️ Display".into(), on_click_id: 103 },
        Element::Button { label: "🛡️ Security".into(), on_click_id: 104 },
        Element::Button { label: "ℹ️ About".into(), on_click_id: 105 },
    ];

    let content = match state.active_tab {
        1 => Element::Column {
            children: vec![
                Element::Text { content: "📶 Wireless & Networks".into() },
                Element::Button {
                    label: format!("Wi-Fi: [{}]", if state.wifi_enabled { "ON" } else { "OFF" }),
                    on_click_id: 201,
                },
                Element::Button {
                    label: format!("Bluetooth: [{}] (btd)", if state.bluetooth_enabled { "ON" } else { "OFF" }),
                    on_click_id: 202,
                },
                Element::Button {
                    label: format!("Cellular / VoLTE: [{}] (telephonyd)", if state.cellular_enabled { "ON" } else { "OFF" }),
                    on_click_id: 203,
                },
                Element::Text { content: "SoftBus: /run/nilos/bus.sock Active".into() },
            ],
        },
        2 => Element::Column {
            children: vec![
                Element::Text { content: "🔊 Sound & Audio Policy".into() },
                Element::Text { content: format!("Output Route: Speaker (ALSA Default) • Ducking: Active") },
                Element::Text { content: format!("Current Level: {}% {}", state.volume, if state.is_muted { "[MUTED]" } else { "" }) },
                Element::Row {
                    children: vec![
                        Element::Button { label: "🔉 Vol -".into(), on_click_id: 204 },
                        Element::Button { label: "🔊 Vol +".into(), on_click_id: 205 },
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
                Element::Text { content: "🖥️ Display & DRM/KMS Compositor".into() },
                Element::Text { content: "Backend: Direct DRM modesetting (/dev/dri/card0)".into() },
                Element::Text { content: "Buffer Pipeline: Tear-free Triple Buffering".into() },
                Element::Button {
                    label: format!("Refresh Rate: [{}]", if state.high_refresh_rate { "120Hz Smooth" } else { "60Hz Normal" }),
                    on_click_id: 207,
                },
            ],
        },
        4 => Element::Column {
            children: vec![
                Element::Text { content: "🛡️ Security & Sandboxing Architecture".into() },
                Element::Text { content: "• SELinux: Enforcing (Mobile Security Profile 33)".into() },
                Element::Text { content: "• Application Sandbox: Linux PID/Mount Namespaces via nilrt".into() },
                Element::Text { content: "• Syscall Filter: Seccomp-BPF Bounded Allowlist".into() },
                Element::Text { content: "• Encryption: fscrypt v2 Per-Credential Key Derivation".into() },
                Element::Text { content: "• Privacy: Zero Cloud Telemetry by default".into() },
            ],
        },
        5 => Element::Column {
            children: vec![
                Element::Text { content: "ℹ️ About OnuronOS".into() },
                Element::Text { content: "• OS Version: OnuronOS 1.0.0-alpha".into() },
                Element::Text { content: "• Kernel: Linux LTS 6.6 (Defconfig Minimal Mobile)".into() },
                Element::Text { content: "• PID 1 Init: nilinit supervisor".into() },
                Element::Text { content: "• Userspace: 100% Memory-Safe Native Rust".into() },
                Element::Text { content: "• Reference Boot: Fastboot / U-Boot / Android Boot Image v0-v4".into() },
                Element::Text { content: "• Package Format: Signed .nilax (Ed25519 atomic install)".into() },
            ],
        },
        _ => Element::Column {
            children: vec![
                Element::Text { content: "⚙️ System Overview".into() },
                Element::Text { content: format!("Wi-Fi: {} • BT: {} • VoLTE: {}", if state.wifi_enabled { "ON" } else { "OFF" }, if state.bluetooth_enabled { "ON" } else { "OFF" }, if state.cellular_enabled { "Active" } else { "Off" }) },
                Element::Text { content: format!("Audio: {}% • Display: {} • Security: Enforcing", state.volume, if state.high_refresh_rate { "120Hz" } else { "60Hz" }) },
                Element::Text { content: format!("Status: {}", state.status_toast) },
            ],
        },
    };

    Element::Column {
        children: vec![
            Element::Text { content: "⚙️ Settings & Device Management".into() },
            Element::Row { children: nav_buttons },
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
