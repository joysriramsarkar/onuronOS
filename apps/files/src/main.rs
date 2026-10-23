// apps/files/src/main.rs — OnuronOS File Manager
// Using the official OnuronOS Design System (dark/light themes)
use nilui::{App, Element, Ev};
use nilui::design::{DesignSystem, Palette};
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

impl Default for FilesState {
    fn default() -> Self {
        Self {
            path: "/data".to_string(),
            items: vec![
                FileItem { name: "notes".to_string(), size: "3 ফাইল".to_string(), modified: "আজ".to_string(), is_dir: true },
                FileItem { name: "app".to_string(), size: "৫ প্যাকেজ".to_string(), modified: "আজ".to_string(), is_dir: true },
                FileItem { name: "settings.toml".to_string(), size: "2 কেবি".to_string(), modified: "আজ".to_string(), is_dir: false },
                FileItem { name: "recording.wav".to_string(), size: "1.4 এমবি".to_string(), modified: "সোম".to_string(), is_dir: false },
            ],
            selected: None,
            search: String::new(),
        }
    }
}

pub fn update(state: &mut FilesState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.path = "/data".to_string();
            state.items = vec![
                FileItem { name: "notes".to_string(), size: "3 ফাইল".to_string(), modified: "আজ".to_string(), is_dir: true },
                FileItem { name: "app".to_string(), size: "৫ প্যাকেজ".to_string(), modified: "আজ".to_string(), is_dir: true },
            ];
        }
        Ev::Click(2) => {
            state.path = "/etc".to_string();
            state.items = vec![
                FileItem { name: "settings.toml".to_string(), size: "2 কেবি".to_string(), modified: "আজ".to_string(), is_dir: false },
            ];
        }
        Ev::Click(3) => {
            state.path = "/tmp".to_string();
            state.items = vec![FileItem { name: "cache.dat".to_string(), size: "500 কেবি".to_string(), modified: "গতকাল".to_string(), is_dir: false }];
        }
        Ev::Click(4) => {
            state.path = "/data/app".to_string();
            state.items = vec![
                FileItem { name: "com.activision".to_string(), size: "৫০০ মেগা".to_string(), modified: "আজ".to_string(), is_dir: true },
            ];
        }
        Ev::Click(5) => {
            state.path = "/contacts".to_string();
            state.items = vec![
                FileItem { name: "contacts.db".to_string(), size: "১.২ এমবি".to_string(), modified: "আজ".to_string(), is_dir: false },
            ];
        }
        Ev::Click(6) => {
            state.path = "/music".to_string();
            state.items = vec![
                FileItem { name: "song.mp3".to_string(), size: "৩.২ এমবি".to_string(), modified: "আজ".to_string(), is_dir: false },
            ];
        }
        Ev::Ime(text) => {
            state.search = text;
        }
        _ => {}
    }
}

pub fn view(state: &FilesState) -> Element {
    let ds = DesignSystem::new();
    let palette = Palette::dark();

    let mut children = vec![
        Element::Text { content: format!("📁 {}", state.path) },
        Element::Text { content: format!("📂 {:?}", state.selected) },
    ];

    for item in &state.items {
        children.push(Element::Row {
            children: vec![
                if item.is_dir {
                    Element::Text { content: "📂".to_string() }
                } else {
                    Element::Text { content: "📄".to_string() }
                },
                Element::Text {
                    content: format!("{}  {}", item.name, item.size),
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
