// apps/browser/src/main.rs — OnuronOS NilZar Browser
// Using the official OnuronOS Design System (dark/light themes)
use nilui::design::{DesignSystem, Palette};
use nilui::{App, Element, Ev};
use nilweb::BrowserState as Engine;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BrowserState {
    pub url: String,
    pub title: String,
    pub loading: bool,
    pub progress: f32,
    pub status: String,
    /// Result of the last real fetch, rendered as text lines.
    pub lines: Vec<String>,
}

impl Default for BrowserState {
    fn default() -> Self {
        Self {
            url: "https://nilos.dev".to_string(),
            title: "NilZar Browser".to_string(),
            loading: false,
            progress: 0.0,
            status: "Ready".to_string(),
            lines: Vec::new(),
        }
    }
}

/// Fetch `url` through the shared native engine and fold the result into the
/// UI state. A failure is reported as a failure; nothing is faked as loaded.
fn load(state: &mut BrowserState, url: &str) {
    state.url = url.to_string();
    state.loading = true;
    state.progress = 0.0;
    state.title = format!("Loading {}", url);

    let mut engine = Engine::default();
    engine.navigate(url);

    match engine.page {
        Some(page) => {
            state.title = if page.title.is_empty() {
                page.final_url.clone()
            } else {
                page.title.clone()
            };
            state.status = format!("{} {}", page.status, page.content_type);
            state.lines = page.lines.clone();
            state.loading = false;
            state.progress = 1.0;
        }
        None => {
            state.status = engine.error.unwrap_or_else(|| "Failed".to_string());
            state.title = format!("Failed: {}", url);
            state.lines = Vec::new();
            state.loading = false;
            state.progress = 0.0;
        }
    }
}

pub fn update(state: &mut BrowserState, ev: Ev) {
    match ev {
        Ev::Ime(text) => {
            state.url = text.clone();
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
            let url = state.url.clone();
            load(state, &url);
        }
        Ev::Click(3) => {
            load(state, "https://maps.google.com");
        }
        _ => {}
    }
}

pub fn view(state: &BrowserState) -> Element {
    let _ds = DesignSystem::new();
    let _palette = Palette::dark();

    Element::Column {
        children: vec![
            Element::Text {
                content: state.title.clone(),
            },
            Element::Input {
                placeholder: "ঠিকানা লিখুন বা খুঁজুন...".into(),
                text: state.url.clone(),
            },
            Element::Row {
                children: vec![
                    Element::Button {
                        label: "📖 বুকমার্ক".into(),
                        on_click_id: 1,
                    },
                    Element::Button {
                        label: "🔍 সন্ধান".into(),
                        on_click_id: 2,
                    },
                    Element::Button {
                        label: "🗺️ মানচিত্র".into(),
                        on_click_id: 3,
                    },
                ],
            },
            Element::Text {
                content: format!("📊 প্রগ্রেস: {:.0}%", state.progress * 100.0),
            },
            Element::Text {
                content: state.status.clone(),
            },
            Element::Column {
                children: state
                    .lines
                    .iter()
                    .map(|l| Element::Text { content: l.clone() })
                    .collect(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_url_input_and_title() {
        let mut state = BrowserState::default();
        update(&mut state, Ev::Ime("https://onuron.org".into()));
        assert_eq!(state.url, "https://onuron.org");

        update(&mut state, Ev::Click(1)); // Bookmark
        assert_eq!(state.url, "https://nilos.dev");
    }

    /// A failed fetch must be reported as a failure, not shown as a loaded page.
    #[test]
    fn failed_fetch_is_reported_honestly() {
        let mut state = BrowserState::default();
        load(&mut state, "http://127.0.0.1:1/");
        assert!(state.lines.is_empty(), "no content may be invented");
        assert!(!state.loading);
        assert!(
            state.title.starts_with("Failed:"),
            "title must say it failed: {}",
            state.title
        );
    }

    /// `about:blank` needs no network and must render something real.
    #[test]
    fn about_blank_renders_without_network() {
        let mut state = BrowserState::default();
        load(&mut state, "about:blank");
        assert!(!state.lines.is_empty(), "about:blank must produce lines");
        assert_eq!(state.status, "200 text/plain");
    }
}
