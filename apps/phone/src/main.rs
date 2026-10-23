// apps/phone/src/main.rs — OnuronOS Phone & Dialer
// Using the official OnuronOS Design System (dark/light themes)
use nilui::design::{DesignSystem, Palette};
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhoneState {
    pub entered: String,
    pub contact: String,
    pub mode: String,
}

impl Default for PhoneState {
    fn default() -> Self {
        Self {
            entered: String::new(),
            contact: "সুমিত্রা".to_string(),
            mode: "মোবাইল".to_string(),
        }
    }
}

pub fn update(state: &mut PhoneState, ev: Ev) {
    match ev {
        Ev::Click(d) if d <= 9 => {
            state.entered.push(char::from_digit(d, 10).unwrap_or('0'));
        }
        Ev::Click(10) => {
            state.entered.pop();
        }
        Ev::Click(11) => {
            if !state.entered.is_empty() {
                state.contact = state.entered.clone();
            }
        }
        Ev::Click(12) => {
            state.entered.push('+');
        }
        Ev::Click(13) => {
            state.entered.push('0');
        }
        _ => {}
    }
}

pub fn view(state: &PhoneState) -> Element {
    let ds = DesignSystem::new();
    let palette = Palette::dark();

    let masked = if state.entered.len() <= 4 {
        "*".repeat(state.entered.len())
    } else {
        format!("{}...", &state.entered[..state.entered.len() - 4])
    };

    Element::Column {
        children: vec![
            Element::Text {
                content: format!("📞 ফোন ({})", state.mode),
            },
            Element::Text {
                content: format!("📞 [{}]", masked),
            },
            Element::Text {
                content: format!("📞 ফোন নম্বর: {}", state.contact),
            },
            Element::Row {
                children: vec![
                    Element::Button {
                        label: "1".into(),
                        on_click_id: 1,
                    },
                    Element::Button {
                        label: "2".into(),
                        on_click_id: 2,
                    },
                    Element::Button {
                        label: "3".into(),
                        on_click_id: 3,
                    },
                ],
            },
            Element::Row {
                children: vec![
                    Element::Button {
                        label: "4".into(),
                        on_click_id: 4,
                    },
                    Element::Button {
                        label: "5".into(),
                        on_click_id: 5,
                    },
                    Element::Button {
                        label: "6".into(),
                        on_click_id: 6,
                    },
                ],
            },
            Element::Row {
                children: vec![
                    Element::Button {
                        label: "7".into(),
                        on_click_id: 7,
                    },
                    Element::Button {
                        label: "8".into(),
                        on_click_id: 8,
                    },
                    Element::Button {
                        label: "9".into(),
                        on_click_id: 9,
                    },
                ],
            },
            Element::Row {
                children: vec![
                    Element::Button {
                        label: "Clear".into(),
                        on_click_id: 10,
                    },
                    Element::Button {
                        label: "0".into(),
                        on_click_id: 13,
                    },
                    Element::Button {
                        label: "Dial".into(),
                        on_click_id: 11,
                    },
                ],
            },
        ],
    }
}

fn main() {
    let app = App {
        state: PhoneState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS Phone & Dialer");
}
