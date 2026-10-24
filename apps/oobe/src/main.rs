// apps/oobe/src/main.rs — OnuronOS First Boot Setup Wizard
// Using the official OnuronOS Design System (dark/light themes)
use nilui::design::{DesignSystem, Palette};
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OobeStep {
    Welcome,
    Language,
    UserProfile,
    PinSetup,
    Completed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OobeState {
    pub step: OobeStep,
    pub locale: String,
    pub user_name: String,
    pub pin: String,
}

impl Default for OobeState {
    fn default() -> Self {
        Self {
            step: OobeStep::Welcome,
            locale: "bn-BD".into(),
            user_name: "Onuron User".into(),
            pin: String::new(),
        }
    }
}

pub fn update(state: &mut OobeState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.step = match state.step {
                OobeStep::Welcome => OobeStep::Language,
                OobeStep::Language => OobeStep::UserProfile,
                OobeStep::UserProfile => OobeStep::PinSetup,
                OobeStep::PinSetup => {
                    if state.pin.len() >= 4 {
                        let p = std::path::Path::new("/data/system/user.json");
                        if let Some(parent) = p.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let config = serde_json::json!({
                            "user_name": state.user_name,
                            "locale": state.locale,
                            "pin_configured": true,
                        });
                        let _ = std::fs::write(p, config.to_string());
                        OobeStep::Completed
                    } else {
                        OobeStep::PinSetup
                    }
                }
                OobeStep::Completed => OobeStep::Completed,
            };
        }
        Ev::Click(2) => state.locale = "bn-BD".into(),
        Ev::Click(3) => state.locale = "en-US".into(),
        Ev::Ime(val) => match state.step {
            OobeStep::UserProfile => {
                if !val.trim().is_empty() {
                    state.user_name = val.trim().to_string();
                }
            }
            OobeStep::PinSetup => {
                if val.chars().all(|c| c.is_ascii_digit()) && (4..=8).contains(&val.len()) {
                    state.pin = val;
                }
            }
            _ => {}
        },
        _ => {}
    }
}

pub fn view(state: &OobeState) -> Element {
    let _ds = DesignSystem::new();
    let _palette = Palette::dark();

    let step_title = match state.step {
        OobeStep::Welcome => "Step 1/4: Welcome to OnuronOS",
        OobeStep::Language => "Step 2/4: Select Locale",
        OobeStep::UserProfile => "Step 3/4: Enter Your Name",
        OobeStep::PinSetup => "Step 4/4: Configure 4-8 Digit PIN",
        OobeStep::Completed => "Setup Complete — Device Ready",
    };

    Element::Column {
        children: vec![
            Element::Text {
                content: step_title.into(),
            },
            Element::Text {
                content: format!("Locale: {} • User: {}", state.locale, state.user_name),
            },
            match state.step {
                OobeStep::Language => Element::Row {
                    children: vec![
                        Element::Button {
                            label: "বাংলা (bn-BD)".into(),
                            on_click_id: 2,
                        },
                        Element::Button {
                            label: "English (en-US)".into(),
                            on_click_id: 3,
                        },
                    ],
                },
                OobeStep::UserProfile => Element::Input {
                    placeholder: "Enter your name...".into(),
                    text: state.user_name.clone(),
                },
                OobeStep::PinSetup => Element::Input {
                    placeholder: "Enter 4-8 digit PIN".into(),
                    text: state.pin.clone(),
                },
                _ => Element::Column {
                    children: vec![Element::Text {
                        content: "🎉 Setup complete!".into(),
                    }],
                },
            },
            Element::Row {
                children: vec![
                    Element::Button {
                        label: "Back".into(),
                        on_click_id: 0,
                    },
                    Element::Button {
                        label: "Next".into(),
                        on_click_id: 1,
                    },
                ],
            },
        ],
    }
}

fn main() {
    let app = App {
        state: OobeState::default(),
        update,
        view,
        on_snapshot: Some(|s| serde_json::to_vec(s).unwrap_or_default()),
        on_restore: Some(|s, bytes| {
            if let Ok(restored) = serde_json::from_slice::<OobeState>(bytes) {
                *s = restored;
            }
        }),
    };
    app.run("OnuronOS First Boot Setup");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oobe_wizard_steps() {
        let mut st = OobeState::default();
        update(&mut st, Ev::Click(1));
        assert_eq!(st.step, OobeStep::Language);
        update(&mut st, Ev::Click(2));
        assert_eq!(st.locale, "bn-BD");
        update(&mut st, Ev::Click(1));
        assert_eq!(st.step, OobeStep::UserProfile);
        update(&mut st, Ev::Ime("Test User".into()));
        assert_eq!(st.user_name, "Test User");
        update(&mut st, Ev::Click(1));
        assert_eq!(st.step, OobeStep::PinSetup);
        update(&mut st, Ev::Ime("1234".into()));
        update(&mut st, Ev::Click(1));
        assert_eq!(st.step, OobeStep::Completed);
    }
}
