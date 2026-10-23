// apps/a11y/src/main.rs — OnuronOS Accessibility
// Using the official OnuronOS Design System (dark/light themes)
use nilui::design::{DesignSystem, Palette};
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct A11yState {
    pub text_size: u8,
    pub high_contrast: bool,
    pub screen_reader: bool,
    pub haptics: bool,
}

impl Default for A11yState {
    fn default() -> Self {
        Self {
            text_size: 50,
            high_contrast: false,
            screen_reader: false,
            haptics: true,
        }
    }
}

pub fn update(state: &mut A11yState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.text_size = (state.text_size as i32 + 5).min(100) as u8;
        }
        Ev::Click(2) => {
            state.text_size = (state.text_size as i32 - 5).max(0) as u8;
        }
        Ev::Click(3) => {
            state.high_contrast = !state.high_contrast;
        }
        Ev::Click(4) => {
            state.screen_reader = !state.screen_reader;
        }
        Ev::Click(5) => {
            state.haptics = !state.haptics;
        }
        _ => {}
    }
}

pub fn view(state: &A11yState) -> Element {
    let ds = DesignSystem::new();
    let palette = Palette::dark();

    Element::Column {
        children: vec![
            Element::Text {
                content: "♿ Accessibility".to_string(),
            },
            Element::Text {
                content: format!("Text Size: {}%", state.text_size),
            },
            Element::Row {
                children: vec![
                    Element::Button {
                        label: "A+".into(),
                        on_click_id: 1,
                    },
                    Element::Button {
                        label: "A-".into(),
                        on_click_id: 2,
                    },
                ],
            },
            Element::Text {
                content: format!(
                    "High Contrast: {}",
                    if state.high_contrast { "ON" } else { "OFF" }
                ),
            },
            Element::Text {
                content: format!(
                    "Screen Reader: {}",
                    if state.screen_reader { "ON" } else { "OFF" }
                ),
            },
            Element::Text {
                content: format!("Haptics: {}", if state.haptics { "ON" } else { "OFF" }),
            },
        ],
    }
}

fn main() {
    let app = App {
        state: A11yState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS Accessibility");
}
