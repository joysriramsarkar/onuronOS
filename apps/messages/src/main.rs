// apps/messages/src/main.rs — OnuronOS Encrypted SMS
// Using the official OnuronOS Design System (dark/light themes)
use nilui::{App, Element, Ev};
use nilui::design::{DesignSystem, Palette};
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
                Message { contact: "সুমিত্রা".to_string(), time: "06:40".to_string(), text: "ছবিগুলো পাঠিয়ে দিলাম".to_string(), new: true },
                Message { contact: "মা".to_string(), time: "05:15".to_string(), text: "খেয়ে নিস, রাত হয়ে গেছে".to_string(), new: true },
                Message { contact: "রাজদীপ".to_string(), time: "গতকাল".to_string(), text: "ঠিক আছে, দেখা হবে".to_string(), new: false },
                Message { contact: "তনুশ্রী".to_string(), time: "গতকাল".to_string(), text: "ধন্যবাদ!".to_string(), new: false },
                Message { contact: "সুমিত্রা".to_string(), time: "০৭:১২".to_string(), text: "কালকের মিটিং কটায়?".to_string(), new: true },
                Message { contact: "মা".to_string(), time: "১৪:০০".to_string(), text: "সকাল".to_string(), new: false },
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
        _ => {}
    }
}

pub fn view(state: &MessagesState) -> Element {
    let ds = DesignSystem::new();
    let palette = Palette::dark();

    let mut list_children = vec![Element::Text { content: "বার্তা · এখন".to_string() }];

    for msg in &state.messages {
        let is_selected = state.selected_contact.as_ref() == Some(&msg.contact);
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
            Element::Text { content: "বার্তা".to_string() },
            Element::Column { children: list_children },
            Element::Input {
                placeholder: "একটি বার্তা লিখুন...".into(),
                text: state.draft.clone(),
            },
            Element::Row {
                children: vec![
                    Element::Button { label: "📷 ছবি যোগ করুন".into(), on_click_id: 1 },
                    Element::Button { label: "🎤 ভয়েস মেসেজ".into(), on_click_id: 2 },
                    Element::Button { label: "📎 ফাইল যোগ করুন".into(), on_click_id: 3 },
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
