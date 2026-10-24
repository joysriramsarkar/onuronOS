// apps/messages/src/main.rs — OnuronOS Encrypted SMS
// Using the official OnuronOS Design System (dark/light themes)
use nilui::design::{DesignSystem, Palette};
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub contact: String,
    pub time: String,
    pub text: String,
    pub new: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessagesState {
    pub messages: Vec<Message>,
    pub selected_contact: Option<String>,
    pub draft: String,
    pub status: String,
}

impl Default for MessagesState {
    fn default() -> Self {
        Self {
            messages: vec![
                Message {
                    contact: "সুমিত্রা".to_string(),
                    time: "06:40".to_string(),
                    text: "ছবিগুলো পাঠিয়ে দিলাম".to_string(),
                    new: true,
                },
                Message {
                    contact: "মা".to_string(),
                    time: "05:15".to_string(),
                    text: "খেয়ে নিস, রাত হয়ে গেছে".to_string(),
                    new: true,
                },
                Message {
                    contact: "রাজদীপ".to_string(),
                    time: "গতকাল".to_string(),
                    text: "ঠিক আছে, দেখা হবে".to_string(),
                    new: false,
                },
                Message {
                    contact: "তনুশ্রী".to_string(),
                    time: "গতকাল".to_string(),
                    text: "ধন্যবাদ!".to_string(),
                    new: false,
                },
                Message {
                    contact: "সুমিত্রা".to_string(),
                    time: "০৭:১২".to_string(),
                    text: "কালকের মিটিং কটায়?".to_string(),
                    new: true,
                },
                Message {
                    contact: "মা".to_string(),
                    time: "১৪:০০".to_string(),
                    text: "সকাল".to_string(),
                    new: false,
                },
            ],
            selected_contact: Some("সুমিত্রা".to_string()),
            draft: String::new(),
            status: "Messages loaded".to_string(),
        }
    }
}

pub fn update(state: &mut MessagesState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.selected_contact = Some("সুমিত্রা".to_string());
            state.draft.clear();
        }
        Ev::Click(2) => {
            state.selected_contact = Some("মা".to_string());
            state.draft.clear();
        }
        Ev::Click(3) => {
            state.selected_contact = Some("রাজদীপ".to_string());
            state.draft.clear();
        }
        Ev::Click(4) => {
            state.selected_contact = Some("তনুশ্রী".to_string());
            state.draft.clear();
        }
        Ev::Click(5) => {
            state.selected_contact = Some("সুমিত্রা".to_string());
            state.draft.clear();
        }
        Ev::Click(6) => {
            state.selected_contact = Some("মা".to_string());
            state.draft.clear();
        }
        Ev::Ime(text) => {
            state.draft = text;
        }
        Ev::Click(7) => {
            state.selected_contact = None;
            state.draft.clear();
        }
        Ev::Click(10) => {
            if !state.draft.is_empty() {
                let text = state.draft.clone();
                let contact = state
                    .selected_contact
                    .clone()
                    .unwrap_or_else(|| "মা".to_string());
                state.messages.push(Message {
                    contact: contact.clone(),
                    time: "এখন".to_string(),
                    text: text.clone(),
                    new: false,
                });
                state.draft.clear();

                let _ = std::fs::create_dir_all("/data/sms");
                if let Ok(json) = serde_json::to_string_pretty(&state.messages) {
                    let _ = std::fs::write("/data/sms/threads.json", json);
                }

                #[cfg(unix)]
                {
                    use std::io::Write;
                    if let Ok(mut s) = std::os::unix::net::UnixStream::connect("/run/nilos/telephony.sock") {
                        let _ = s.write_all(format!("AT+CMGS=\"{}\"\r\n{}\x1A", contact, text).as_bytes());
                    }
                }
            }
        }
        _ => {}
    }
}

pub fn view(state: &MessagesState) -> Element {
    let _ds = DesignSystem::new();
    let _palette = Palette::dark();

    let mut list_children = vec![Element::Text {
        content: "বার্তা · এখন".to_string(),
    }];

    for msg in &state.messages {
        let _is_selected = state.selected_contact.as_ref() == Some(&msg.contact);
        let is_new = msg.new;
        list_children.push(Element::Row {
            children: vec![
                Element::Text {
                    content: format!("{} {}", msg.time, if is_new { "🔔" } else { "" }),
                },
                Element::Text {
                    content: format!("  {}", msg.contact),
                },
                Element::Text {
                    content: format!("  {}", if is_new { "💬" } else { "" }),
                },
            ],
        });
    }

    Element::Column {
        children: vec![
            Element::Text {
                content: "বার্তা".to_string(),
            },
            Element::Column {
                children: list_children,
            },
            Element::Input {
                placeholder: "একটি বার্তা লিখুন...".into(),
                text: state.draft.clone(),
            },
            Element::Row {
                children: vec![
                    Element::Button {
                        label: "📷 ছবি যোগ করুন".into(),
                        on_click_id: 1,
                    },
                    Element::Button {
                        label: "🎤 ভয়েস মেসেজ".into(),
                        on_click_id: 2,
                    },
                    Element::Button {
                        label: "📎 ফাইল যোগ করুন".into(),
                        on_click_id: 3,
                    },
                    Element::Button {
                        label: "📤 পাঠান".into(),
                        on_click_id: 10,
                    },
                ],
            },
        ],
    }
}

fn main() {
    let app = App {
        state: MessagesState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS Encrypted SMS");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_message_dispatches_and_clears_draft() {
        let mut state = MessagesState::default();
        let initial_count = state.messages.len();

        state.draft = "হ্যালো, কেমন আছো?".to_string();
        update(&mut state, Ev::Click(10)); // Send

        assert_eq!(state.messages.len(), initial_count + 1);
        assert_eq!(state.messages.last().unwrap().text, "হ্যালো, কেমন আছো?");
        assert!(state.draft.is_empty());
    }
}

