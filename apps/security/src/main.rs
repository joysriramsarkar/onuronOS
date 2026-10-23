// apps/security/src/main.rs — OnuronOS Security & Privacy
// Using the official OnuronOS Design System (dark/light themes)
use nilui::design::{DesignSystem, Palette};
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityState {
    pub pin_changed: bool,
    pub lock_screen_pin: String,
    pub lock_screen_timeout: u8,
    pub camera_permission: bool,
    pub mic_permission: bool,
    pub file_permission: bool,
    pub data_encryption: bool,
}

impl Default for SecurityState {
    fn default() -> Self {
        Self {
            pin_changed: false,
            lock_screen_pin: String::new(),
            lock_screen_timeout: 30,
            camera_permission: true,
            mic_permission: true,
            file_permission: true,
            data_encryption: false,
        }
    }
}

pub fn update(state: &mut SecurityState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.pin_changed = true;
        }
        Ev::Click(2) => {
            state.lock_screen_timeout = (state.lock_screen_timeout as i32 + 10).min(120) as u8;
        }
        Ev::Click(3) => {
            state.lock_screen_timeout = (state.lock_screen_timeout as i32 - 10).max(0) as u8;
        }
        Ev::Click(4) => {
            state.camera_permission = !state.camera_permission;
        }
        Ev::Click(5) => {
            state.mic_permission = !state.mic_permission;
        }
        Ev::Click(6) => {
            state.file_permission = !state.file_permission;
        }
        Ev::Click(7) => {
            state.data_encryption = !state.data_encryption;
        }
        _ => {}
    }
}

pub fn view(state: &SecurityState) -> Element {
    let ds = DesignSystem::new();
    let palette = Palette::dark();

    Element::Column {
        children: vec![
            Element::Text {
                content: "🛡️ Security".to_string(),
            },
            Element::Text {
                content: "PIN Change".to_string(),
            },
            Element::Text {
                content: "Lock Screen PIN".to_string(),
            },
            Element::Text {
                content: "Screen Lock Timeout".to_string(),
            },
            Element::Text {
                content: "Camera Permission".to_string(),
            },
            Element::Text {
                content: "Microphone Permission".to_string(),
            },
            Element::Text {
                content: "File Permission".to_string(),
            },
            Element::Text {
                content: "Data Encryption".to_string(),
            },
        ],
    }
}

fn main() {
    let app = App {
        state: SecurityState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS Security Manager");
}
