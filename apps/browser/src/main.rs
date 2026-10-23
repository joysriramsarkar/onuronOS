// apps/browser/src/main.rs — OnuronOS NilZar Browser
// Using the official OnuronOS Design System (dark/light themes)
use nilui::{App, Element, Ev};
use nilui::design::{DesignSystem, Palette};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BrowserState {
    pub url: String,
    pub title: String,
    pub loading: bool,
    pub progress: f32,
    pub status: String,
}

impl Default for BrowserState {
    fn default() -> Self {
        Self {
            url: "https://nilos.dev".to_string(),
            title: "NilZar Browser".to_string(),
            loading: false,
            progress: 0.0,
            status: "Ready".to_string(),
        }
    }
}

pub fn update(state: &mut BrowserState, ev: Ev) {
    match ev {
        Ev::Ime(text) => {
            let url = text.clone();
            state.url = url;
            state.progress = 0.0;
            state.loading = true;
            state.title = format!("Loading {}", text);
        }
        Ev::Click(1) => {
            state.url = "https://nilos.dev".to_string();
            state.title = "NilZar Browser".to_string();
            state.loading = false;
        }
        Ev::Click(2) => {
            state.url = "https://github.com".to_string();
            state.title = "GitHub".to_string();
            state.loading = false;
        }
        Ev::Click(3) => {
            state.url = "https://bsky.app".to_string();
            state.title = "Bluesky".to_string();
            state.loading = false;
        }
        _ => {}
    }
}

pub fn view(state: &BrowserState) -> Element {
    let ds = DesignSystem::new();
    let palette = Palette::dark();

    Element::Column {
        children: vec![
            Element::Text { content: state.title.clone() },
            Element::Input {
                placeholder: "ঠিকানা লিখুন বা খুঁজুন...".into(),
                text: state.url.clone(),
            },
            Element::Row {
                children: vec![
                    Element::Button { label: "📖 বুকমার্ক".into(), on_click_id: 1 },
                    Element::Button { label: "🔍 সন্ধান".into(), on_click_id: 2 },
                    Element::Button { label: "🗺️ মানচিত্র".into(), on_click_id: 3 },
                ],
            },
            Element::Text {
                content: format!("📊 প্রগ্রেস: {:.0}%", state.progress * 100.0),
            },
        ],
    }
}

fn main() {
    let app = App {
        state: BrowserState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS NilZar Browser");
}
