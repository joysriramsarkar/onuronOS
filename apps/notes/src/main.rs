// apps/notes/src/main.rs — OnuronOS NilLang Notes
// Using the official OnuronOS Design System (dark/light themes)
use nilui::{App, Element, Ev};
use nilui::design::{DesignSystem, Palette};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Note {
    pub title: String,
    pub content: String,
    pub created: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotesState {
    pub notes: Vec<Note>,
    pub current_note: Option<Note>,
    pub new_title: String,
    pub new_content: String,
    pub search_query: String,
}

impl Default for NotesState {
    fn default() -> Self {
        Self {
            notes: vec![
                Note {
                    title: "আজকের সময়সূচি".to_string(),
                    content: "ব্যবসা করতে হবে।".to_string(),
                    created: "14:30".to_string(),
                },
                Note {
                    title: "পরিকল্পনা".to_string(),
                    content: "কাজ করতে হবে।".to_string(),
                    created: "09:15".to_string(),
                },
                Note {
                    title: "যোগাযোগ".to_string(),
                    content: "মানুষের সাথে যোগাযোগ করতে হবে।".to_string(),
                    created: "11:00".to_string(),
                },
            ],
            current_note: None,
            new_title: String::new(),
            new_content: String::new(),
            search_query: String::new(),
        }
    }
}

pub fn update(state: &mut NotesState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.current_note = Some(Note {
                title: "নতুন নোট".to_string(),
                content: String::new(),
                created: "এখন".to_string(),
            });
        }
        Ev::Click(2) => {
            // Save new note
            if !state.new_title.trim().is_empty() {
                state.notes.push(Note {
                    title: state.new_title.trim().to_string(),
                    content: state.new_content.clone(),
                    created: "এখন".to_string(),
                });
                state.new_title.clear();
                state.new_content.clear();
                state.current_note = None;
            }
        }
        Ev::Ime(text) => {
            if state.current_note.is_some() {
                if let Some(note) = state.current_note.as_mut() {
                    if note.title.is_empty() {
                        note.title = text.clone();
                    } else {
                        note.content = text;
                    }
                }
            } else {
                state.new_title = text;
            }
        }
        Ev::Click(3) => {
            state.search_query = String::new();
            state.current_note = None;
        }
        _ => {}
    }
}

pub fn view(state: &NotesState) -> Element {
    let ds = DesignSystem::new();
    let palette = Palette::dark();

    let q = state.search_query.to_lowercase();
    let filtered: Vec<&Note> = state.notes.iter().filter(|n| {
        q.is_empty() || n.title.to_lowercase().contains(&q) || n.content.to_lowercase().contains(&q)
    }).collect();

    let mut children = vec![
        Element::Text { content: "📝 NilLang Notes".to_string() },
    ];

    if !state.new_title.trim().is_empty() {
        children.push(Element::Text { content: "✏️ নতুন নোট সংরক্ষণ করুন".to_string() });
        children.push(Element::Text { content: state.new_title.clone() });
        children.push(Element::Text { content: state.new_content.clone() });
        children.push(Element::Row {
            children: vec![
                Element::Button { label: "💾 সংরক্ষণ".into(), on_click_id: 2 },
                Element::Button { label: "❌ বাতিল".into(), on_click_id: 3 },
            ],
        });
    } else {
        children.push(Element::Text { content: format!("নোট {}: {}", state.notes.len(), "")});
        for note in &filtered {
            children.push(Element::Row {
                children: vec![
                    Element::Text { content: format!("📄 {}", note.title) },
                    Element::Text { content: format!("  {}", note.created) },
                ],
            });
        }
    }

    Element::Column { children }
}

fn main() {
    let app = App {
        state: NotesState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS NilLang Notes");
}
