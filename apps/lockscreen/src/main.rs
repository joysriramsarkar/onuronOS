// apps/lockscreen/src/main.rs — OnuronOS PIN & Swipe-to-Unlock Lockscreen
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LockscreenState {
    pub entered_pin: String,
    pub expected_pin: String,
    pub unlocked: bool,
    pub failed_attempts: u32,
    pub drag_offset_y: i32,
}

impl Default for LockscreenState {
    fn default() -> Self {
        Self {
            entered_pin: String::new(),
            expected_pin: "1234".into(),
            unlocked: false,
            failed_attempts: 0,
            drag_offset_y: 0,
        }
    }
}

pub fn update(state: &mut LockscreenState, ev: Ev) {
    match ev {
        Ev::Click(d) if d <= 9 => {
            if state.entered_pin.len() < 8 {
                state
                    .entered_pin
                    .push(char::from_digit(d, 10).unwrap_or('0'));
            }
        }
        Ev::Click(10) => {
            state.entered_pin.pop();
        }
        Ev::Click(11) => {
            if state.entered_pin == state.expected_pin {
                state.unlocked = true;
                state.failed_attempts = 0;
                state.entered_pin.clear();
                let _ = nilui::send_shell_cmd("UNLOCK");
            } else {
                state.unlocked = false;
                state.failed_attempts += 1;
                state.entered_pin.clear();
            }
        }
        Ev::Drag(dy) => {
            state.drag_offset_y = dy;
        }
        _ => {}
    }
}

pub fn view(state: &LockscreenState) -> Element {
    let masked = "*".repeat(state.entered_pin.len());
    let status = if state.unlocked {
        "Unlocked".to_string()
    } else if state.failed_attempts > 0 {
        format!("Wrong PIN ({} failed attempt(s))", state.failed_attempts)
    } else {
        "Locked — Enter PIN".to_string()
    };

    Element::Column {
        children: vec![
            Element::Text {
                content: "OnuronOS Lockscreen".into(),
            },
            Element::Text { content: status },
            Element::Text {
                content: format!("PIN: [{}]", masked),
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
                        on_click_id: 0,
                    },
                    Element::Button {
                        label: "Unlock".into(),
                        on_click_id: 11,
                    },
                ],
            },
        ],
    }
}

fn main() {
    let app = App {
        state: LockscreenState::default(),
        update,
        view,
        on_snapshot: Some(|s| serde_json::to_vec(s).unwrap_or_default()),
        on_restore: Some(|s, bytes| {
            if let Ok(restored) = serde_json::from_slice::<LockscreenState>(bytes) {
                *s = restored;
            }
        }),
    };
    app.run("OnuronOS Lockscreen");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pin_unlock_and_rejection() {
        let mut st = LockscreenState::default();
        update(&mut st, Ev::Click(9));
        update(&mut st, Ev::Click(9));
        update(&mut st, Ev::Click(11));
        assert!(!st.unlocked);
        assert_eq!(st.failed_attempts, 1);

        for d in [1, 2, 3, 4] {
            update(&mut st, Ev::Click(d));
        }
        update(&mut st, Ev::Click(11));
        assert!(st.unlocked);
        assert_eq!(st.failed_attempts, 0);
    }
}
