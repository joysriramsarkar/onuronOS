// runtime/nilui/src/share.rs — Reusable System Share Sheet Component
use crate::Element;

pub struct ShareSheet {
    pub title: String,
    pub file_path: String,
    pub nearby_peers: Vec<String>,
}

impl ShareSheet {
    pub fn new(title: &str, path: &str, peers: Vec<String>) -> Self {
        Self {
            title: title.to_string(),
            file_path: path.to_string(),
            nearby_peers: peers,
        }
    }

    pub fn view(&self) -> Element {
        Element::Column {
            children: vec![
                Element::Text { content: format!("Share: {}", self.title) },
                Element::Text { content: format!("Path: {}", self.file_path) },
            ],
        }
    }
}
