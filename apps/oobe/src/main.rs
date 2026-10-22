// apps/oobe/src/main.rs — OnuronOS First Boot Setup Wizard
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
                        OobeStep::Completed
                    } else {
                        OobeStep::PinSetup
                    }
                }
                OobeStep::Completed => OobeStep::Completed,
            };
        }
        Ev::Click(2) => {
            state.locale = "bn-BD".into();
        }
        Ev::Click(3) => {
            state.locale = "en-US".into();
        }
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
            Element::Row {
                children: vec![
                    Element::Button {
                        label: "বাংলা (bn-BD)".into(),
                        on_click_id: 2,
                    },
                    Element::Button {
                        label: "English (en-US)".into(),
                        on_click_id: 3,
                    },
                    Element::Button {
                        label: "Next →".into(),
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
    app.run("OnuronOS First Boot Wizard");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oobe_flow_progression() {
        let mut st = OobeState::default();
        assert_eq!(st.step, OobeStep::Welcome);
        update(&mut st, Ev::Click(1));
        assert_eq!(st.step, OobeStep::Language);
        update(&mut st, Ev::Click(3));
        assert_eq!(st.locale, "en-US");
        update(&mut st, Ev::Click(1));
        assert_eq!(st.step, OobeStep::UserProfile);
        update(&mut st, Ev::Ime("Joy Sarkar".into()));
        assert_eq!(st.user_name, "Joy Sarkar");
        update(&mut st, Ev::Click(1));
        assert_eq!(st.step, OobeStep::PinSetup);
        // Cannot complete without valid 4-8 digit PIN
        update(&mut st, Ev::Click(1));
        assert_eq!(st.step, OobeStep::PinSetup);
        update(&mut st, Ev::Ime("4826".into()));
        update(&mut st, Ev::Click(1));
        assert_eq!(st.step, OobeStep::Completed);
    }
}
