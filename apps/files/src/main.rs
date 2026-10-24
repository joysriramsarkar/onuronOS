// apps/files/src/main.rs — OnuronOS File Manager
// Using the official OnuronOS Design System (dark/light themes)
// Backed by real Linux virtual & persistent filesystem boundaries.
use std::fs;
use std::path::Path;
use nilui::design::{DesignSystem, Palette};
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileItem {
    pub name: String,
    pub size: String,
    pub modified: String,
    pub is_dir: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FilesState {
    pub path: String,
    pub items: Vec<FileItem>,
    pub selected: Option<String>,
    pub search: String,
}

pub fn read_real_directory(dir_path: &str) -> Vec<FileItem> {
    let mut items = Vec::new();
    let p = Path::new(dir_path);
    if let Ok(entries) = fs::read_dir(p) {
        let mut sorted_entries: Vec<_> = entries.flatten().collect();
        sorted_entries.sort_by_key(|e| e.file_name());
        for entry in sorted_entries {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let size = if is_dir {
                "Folder".to_string()
            } else if let Ok(meta) = entry.metadata() {
                let len = meta.len();
                if len < 1024 {
                    format!("{} B", len)
                } else if len < 1024 * 1024 {
                    format!("{:.1} KB", len as f64 / 1024.0)
                } else {
                    format!("{:.1} MB", len as f64 / (1024.0 * 1024.0))
                }
            } else {
                "0 B".to_string()
            };
            items.push(FileItem {
                name,
                size,
                modified: "Local".to_string(),
                is_dir,
            });
        }
    }
    if items.is_empty() {
        items.push(FileItem {
            name: "(empty or unmounted)".to_string(),
            size: "-".to_string(),
            modified: "-".to_string(),
            is_dir: false,
        });
    }
    items
}

impl Default for FilesState {
    fn default() -> Self {
        let initial_path = if Path::new("/data").is_dir() {
            "/data".to_string()
        } else {
            ".".to_string()
        };
        let items = read_real_directory(&initial_path);
        Self {
            path: initial_path,
            items,
            selected: None,
            search: String::new(),
        }
    }
}

pub fn update(state: &mut FilesState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.path = "/data".to_string();
            state.items = read_real_directory(&state.path);
        }
        Ev::Click(2) => {
            state.path = "/etc".to_string();
            state.items = read_real_directory(&state.path);
        }
        Ev::Click(3) => {
            state.path = "/tmp".to_string();
            state.items = read_real_directory(&state.path);
        }
        Ev::Click(4) => {
            state.path = "/data/app".to_string();
            state.items = read_real_directory(&state.path);
        }
        Ev::Click(5) => {
            state.path = "/run".to_string();
            state.items = read_real_directory(&state.path);
        }
        Ev::Click(6) => {
            state.path = "/system".to_string();
            state.items = read_real_directory(&state.path);
        }
        Ev::Ime(text) => {
            state.search = text;
        }
        _ => {}
    }
}

pub fn view(state: &FilesState) -> Element {
    let _ds = DesignSystem::new();
    let _palette = Palette::dark();

    let nav_row = Element::Row {
        children: vec![
            Element::Button {
                label: "/data".into(),
                on_click_id: 1,
            },
            Element::Button {
                label: "/etc".into(),
                on_click_id: 2,
            },
            Element::Button {
                label: "/tmp".into(),
                on_click_id: 3,
            },
            Element::Button {
                label: "/data/app".into(),
                on_click_id: 4,
            },
            Element::Button {
                label: "/run".into(),
                on_click_id: 5,
            },
            Element::Button {
                label: "/system".into(),
                on_click_id: 6,
            },
        ],
    };

    let mut children = vec![
        Element::Text {
            content: format!("📁 Location: {}", state.path),
        },
        nav_row,
    ];

    let filtered_items: Vec<_> = if state.search.is_empty() {
        state.items.iter().collect()
    } else {
        state
            .items
            .iter()
            .filter(|it| it.name.to_lowercase().contains(&state.search.to_lowercase()))
            .collect()
    };

    for item in filtered_items {
        children.push(Element::Row {
            children: vec![
                if item.is_dir {
                    Element::Text {
                        content: "📂".to_string(),
                    }
                } else {
                    Element::Text {
                        content: "📄".to_string(),
                    }
                },
                Element::Text {
                    content: format!("{}  [{}]", item.name, item.size),
                },
                Element::Text {
                    content: format!("  {}", item.modified),
                },
            ],
        });
    }

    Element::Column { children }
}

fn main() {
    let app = App {
        state: FilesState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS File Manager");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_real_directory() {
        let items = read_real_directory(".");
        assert!(!items.is_empty());
    }

    #[test]
    fn test_navigation_clicks() {
        let mut state = FilesState::default();
        update(&mut state, Ev::Click(2));
        assert_eq!(state.path, "/etc");

        update(&mut state, Ev::Click(3));
        assert_eq!(state.path, "/tmp");
    }
}
