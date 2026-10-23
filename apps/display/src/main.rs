// apps/display/src/main.rs — OnuronOS Display Manager
// Using the official OnuronOS Design System (dark/light themes)
use nilui::design::{DesignSystem, Palette};
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisplayState {
    pub theme: bool,
    pub brightness: u8,
    pub auto_brightness: bool,
    pub text_size: u8,
    pub refresh_rate: bool,
    pub screen_timeout: u8,
    pub status: String,
}

impl Default for DisplayState {
    fn default() -> Self {
        Self {
            theme: true,
            brightness: 60,
            auto_brightness: true,
            text_size: 50,
            refresh_rate: true,
            screen_timeout: 30,
            status: "Display settings loaded".to_string(),
        }
    }
}

pub fn update(state: &mut DisplayState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.theme = !state.theme;
            state.status = if state.theme {
                "Dark Theme".to_string()
            } else {
                "Light Theme".to_string()
            };
        }
        Ev::Click(2) => {
            state.brightness = (state.brightness as i32 + 5).min(100) as u8;
        }
        Ev::Click(3) => {
            state.brightness = (state.brightness as i32 - 5).max(0) as u8;
        }
        Ev::Click(4) => {
            state.auto_brightness = !state.auto_brightness;
        }
        Ev::Click(5) => {
            state.text_size = (state.text_size as i32 + 5).min(100) as u8;
        }
        Ev::Click(6) => {
            state.text_size = (state.text_size as i32 - 5).max(0) as u8;
        }
        Ev::Click(7) => {
            state.refresh_rate = !state.refresh_rate;
        }
        Ev::Click(8) => {
            state.screen_timeout = (state.screen_timeout as i32 + 5).min(120) as u8;
        }
        _ => {}
    }
}

pub fn view(state: &DisplayState) -> Element {
    let ds = DesignSystem::new();
    let palette = if state.theme {
        Palette::dark()
    } else {
        Palette::light()
    };

    Element::Column {
        children: vec![
            Element::Text {
                content: "🖥️ Display".to_string(),
            },
            Element::Row {
                children: vec![
                    Element::Button {
                        label: "🌙 Dark".into(),
                        on_click_id: 1,
                    },
                    Element::Button {
                        label: "☀️ Light".into(),
                        on_click_id: 1,
                    },
                ],
            },
            Element::Text {
                content: format!("🌞 Brightness: {}%", state.brightness),
            },
            Element::Row {
                children: vec![
                    Element::Button {
                        label: "🔆 +".into(),
                        on_click_id: 2,
                    },
                    Element::Button {
                        label: "🌑 -".into(),
                        on_click_id: 3,
                    },
                ],
            },
            Element::Text {
                content: format!(
                    "📱 Auto Brightness: {}",
                    if state.auto_brightness { "ON" } else { "OFF" }
                ),
            },
            Element::Text {
                content: format!("🔤 Text Size: {}%", state.text_size),
            },
            Element::Row {
                children: vec![
                    Element::Button {
                        label: "A+".into(),
                        on_click_id: 5,
                    },
                    Element::Button {
                        label: "A-".into(),
                        on_click_id: 6,
                    },
                ],
            },
            Element::Text {
                content: format!(
                    "🔄 Refresh Rate: {}Hz",
                    if state.refresh_rate { "120" } else { "60" }
                ),
            },
            Element::Text {
                content: format!("⏱️ Screen Timeout: {}s", state.screen_timeout),
            },
        ],
    }
}

fn main() {
    let app = App {
        state: DisplayState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS Display Manager");
}
