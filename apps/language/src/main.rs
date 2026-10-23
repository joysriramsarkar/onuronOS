// apps/language/src/main.rs — OnuronOS Language & Keyboard
// Using the official OnuronOS Design System (dark/light themes)
use nilui::design::{DesignSystem, Palette};
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LanguageState {
    pub language: String,
    pub keyboard_layout: String,
    pub auto_correct: bool,
    pub second_keyboard: String,
}

impl Default for LanguageState {
    fn default() -> Self {
        Self {
            language: "বাংলা".to_string(),
            keyboard_layout: "বাংলা (ফোনেটিক)".to_string(),
            auto_correct: true,
            second_keyboard: "English".to_string(),
        }
    }
}

pub fn update(state: &mut LanguageState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.language = "বাংলা".to_string();
            state.keyboard_layout = "বাংলা (ফোনেটিক)".to_string();
        }
        Ev::Click(2) => {
            state.language = "English".to_string();
            state.keyboard_layout = "QWERTY".to_string();
        }
        Ev::Click(3) => {
            state.keyboard_layout = "বাংলা (ফোনেটিক)".to_string();
        }
        Ev::Click(4) => {
            state.keyboard_layout = "English (QWERTY)".to_string();
        }
        Ev::Click(5) => {
            state.auto_correct = !state.auto_correct;
        }
        _ => {}
    }
}

pub fn view(state: &LanguageState) -> Element {
    let ds = DesignSystem::new();
    let palette = Palette::dark();

    Element::Column {
        children: vec![
            Element::Text {
                content: "🌐 Language & Keyboard".to_string(),
            },
            Element::Text {
                content: format!("Language: {}", state.language),
            },
            Element::Text {
                content: format!("Keyboard Layout: {}", state.keyboard_layout),
            },
            Element::Text {
                content: format!(
                    "Auto-correct: {}",
                    if state.auto_correct { "ON" } else { "OFF" }
                ),
            },
            Element::Text {
                content: format!("Second Keyboard: {}", state.second_keyboard),
            },
        ],
    }
}

fn main() {
    let app = App {
        state: LanguageState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS Language & Keyboard");
}
