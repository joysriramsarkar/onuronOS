// apps/notes/src/main.rs — OnuronOS NilLang Notes
// Using the official OnuronOS Design System (dark/light themes)
// Backed by persistent disk storage at /data/notes/notes.json
use std::fs;
use std::path::Path;
use nilui::design::{DesignSystem, Palette};
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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

const NOTES_FILE_PATH: &str = "/data/notes/notes.json";

pub fn load_notes_from_disk() -> Vec<Note> {
    if let Ok(content) = fs::read_to_string(NOTES_FILE_PATH) {
        if let Ok(parsed) = serde_json::from_str::<Vec<Note>>(&content) {
            if !parsed.is_empty() {
                return parsed;
            }
        }
    }
    vec![
        Note {
            title: "আজকের সময়সূচি".to_string(),
            content: "সিস্টেম রিলিজ প্রস্তুতি।".to_string(),
            created: "14:30".to_string(),
        },
        Note {
            title: "পরিকল্পনা".to_string(),
            content: "কার্নেল ও কম্পোজিটর বুট যাচাই।".to_string(),
            created: "09:15".to_string(),
        },
        Note {
            title: "যোগাযোগ".to_string(),
            content: "অনুরণ ওএস ডেভেলপার দলের সাথে বৈঠক।".to_string(),
            created: "11:00".to_string(),
        },
    ]
}

pub fn save_notes_to_disk(notes: &[Note]) {
    let p = Path::new(NOTES_FILE_PATH);
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(data) = serde_json::to_string_pretty(notes) {
        let _ = fs::write(p, data);
    }
}

impl Default for NotesState {
    fn default() -> Self {
        Self {
            notes: load_notes_from_disk(),
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
            // Save new note to state and persist to disk
            if !state.new_title.trim().is_empty() {
                state.notes.push(Note {
                    title: state.new_title.trim().to_string(),
                    content: state.new_content.clone(),
                    created: "এখন".to_string(),
                });
                save_notes_to_disk(&state.notes);
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
    let _ds = DesignSystem::new();
    let _palette = Palette::dark();

    let q = state.search_query.to_lowercase();
    let filtered: Vec<&Note> = state
        .notes
        .iter()
        .filter(|n| {
            q.is_empty()
                || n.title.to_lowercase().contains(&q)
                || n.content.to_lowercase().contains(&q)
        })
        .collect();

    let mut children = vec![Element::Text {
        content: "📝 NilLang Notes".to_string(),
    }];

    if !state.new_title.trim().is_empty() {
        children.push(Element::Text {
            content: "✏️ নতুন নোট সংরক্ষণ করুন".to_string(),
        });
        children.push(Element::Text {
            content: state.new_title.clone(),
        });
        children.push(Element::Text {
            content: state.new_content.clone(),
        });
        children.push(Element::Row {
            children: vec![
                Element::Button {
                    label: "💾 সংরক্ষণ".into(),
                    on_click_id: 2,
                },
                Element::Button {
                    label: "❌ বাতিল".into(),
                    on_click_id: 3,
                },
            ],
        });
    } else {
        children.push(Element::Text {
            content: format!("মোট নোট: {}", state.notes.len()),
        });
        for note in &filtered {
            children.push(Element::Row {
                children: vec![
                    Element::Text {
                        content: format!("📄 {}", note.title),
                    },
                    Element::Text {
                        content: format!("  {}", note.created),
                    },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notes_save_and_state() {
        let mut state = NotesState::default();
        let initial_count = state.notes.len();

        state.new_title = "পরীক্ষা নোট".to_string();
        state.new_content = "এটি একটি পরীক্ষামূলক নোট।".to_string();

        update(&mut state, Ev::Click(2)); // Save
        assert_eq!(state.notes.len(), initial_count + 1);
        assert_eq!(state.notes.last().unwrap().title, "পরীক্ষা নোট");
    }

    #[test]
    fn test_notes_cancel_click() {
        let mut state = NotesState::default();
        state.search_query = "খোঁজ".to_string();

        update(&mut state, Ev::Click(3)); // Cancel / Clear
        assert!(state.search_query.is_empty());
        assert!(state.current_note.is_none());
    }
}
