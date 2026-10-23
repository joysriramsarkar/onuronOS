// apps/launcher/src/main.rs — OnuronOS Home Launcher (Design System)
// Uses the official OnuronOS Design System (dark/light themes)
use nilui::design::{DesignSystem, Palette, Spacing, Typography};
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LauncherState {
    pub current_page: usize,
    pub selected_app: Option<String>,
    pub search_query: String,
    pub theme: bool,
    pub installed_apps: Vec<(String, String, String)>, // (id, label, icon)
}

impl Default for LauncherState {
    fn default() -> Self {
        Self {
            current_page: 0,
            selected_app: None,
            search_query: String::new(),
            theme: true,
            installed_apps: vec![
                ("phone".to_string(), "Phone".to_string(), "📞".to_string()),
                (
                    "messages".to_string(),
                    "Messages".to_string(),
                    "💬".to_string(),
                ),
                (
                    "browser".to_string(),
                    "Browser".to_string(),
                    "🌐".to_string(),
                ),
                (
                    "settings".to_string(),
                    "Settings".to_string(),
                    "⚙️".to_string(),
                ),
                // Keep launcher IDs aligned with the signed Store manifests so a
                // selected app can be handed directly to nilrt/nilpkg.
                (
                    "org.onuron.notes".to_string(),
                    "Notes".to_string(),
                    "📝".to_string(),
                ),
                (
                    "org.onuron.calculator".to_string(),
                    "Alap Calculator".to_string(),
                    "🧮".to_string(),
                ),
                ("camera".to_string(), "Camera".to_string(), "📷".to_string()),
                ("wifi".to_string(), "WiFi".to_string(), "📡".to_string()),
                (
                    "bluetooth".to_string(),
                    "Bluetooth".to_string(),
                    "🔵".to_string(),
                ),
                ("sound".to_string(), "Sound".to_string(), "🔊".to_string()),
                (
                    "display".to_string(),
                    "Display".to_string(),
                    "🖥️".to_string(),
                ),
                (
                    "security".to_string(),
                    "Security".to_string(),
                    "🔒".to_string(),
                ),
                (
                    "storage".to_string(),
                    "Storage".to_string(),
                    "💾".to_string(),
                ),
                (
                    "language".to_string(),
                    "Language".to_string(),
                    "🌐".to_string(),
                ),
                ("vpn".to_string(), "VPN".to_string(), "🛡️".to_string()),
                ("backup".to_string(), "Backup".to_string(), "💾".to_string()),
            ],
        }
    }
}

pub fn update(state: &mut LauncherState, ev: Ev) {
    match ev {
        Ev::Click(id) if (1..=34).contains(&id) => {
            if let Some((app_id, _, _)) = state.installed_apps.get((id - 1) as usize) {
                state.selected_app = Some(app_id.clone());
            }
        }
        Ev::Click(100) => state.current_page = 0,
        Ev::Click(101) => state.current_page = 1,
        Ev::Click(102) => state.current_page = 2,
        Ev::Ime(text) => state.search_query = text,
        _ => {}
    }
}

pub fn view(state: &LauncherState) -> Element {
    let _ds = DesignSystem::new();
    let _palette = if state.theme {
        Palette::dark()
    } else {
        Palette::light()
    };

    let _q = state.search_query.to_lowercase();

    // Status Bar with design system styling
    let status_bar = Element::Column {
        children: vec![Element::Row {
            children: vec![
                Element::Text {
                    content: "🕐 ১২:৫৯".to_string(),
                },
                Element::Text {
                    content: " 📶 নেটওয়ার্ক সংযোগ | 🌙 অন্ধকার ".to_string(),
                },
                Element::Text {
                    content: "২২ অক্টোবর ২০২৬".to_string(),
                },
            ],
        }],
    };

    // Search Bar
    let search_bar = Element::Input {
        placeholder: "সার্চ করুন...".to_string(),
        text: state.search_query.clone(),
    };

    // App Grid - 3 columns using AppCard-style buttons
    let app_cards: Vec<Element> = state
        .installed_apps
        .iter()
        .filter(|(_, label, icon)| {
            let query = state.search_query.to_lowercase();
            if query.is_empty() {
                return true;
            }
            let label_lower = label.to_lowercase();
            let icon_lower = icon.to_lowercase();
            label_lower.contains(&query) || icon_lower.contains(&query)
        })
        .enumerate()
        .map(|(idx, (_, label, icon))| Element::Button {
            label: format!("  {}  {}\n  📱", icon, label),
            on_click_id: (idx + 1) as u32,
        })
        .collect();

    let mut rows = Vec::new();
    for chunk in app_cards.chunks(3) {
        rows.push(Element::Row {
            children: chunk.to_vec(),
        });
    }

    // Bottom Navigation with design system styling
    let bottom_nav = Element::Row {
        children: vec![
            Element::Button {
                label: "🔙 পেছনে".to_string(),
                on_click_id: 101,
            },
            Element::Button {
                label: "🏠 হোম".to_string(),
                on_click_id: 100,
            },
            Element::Button {
                label: "🌀 সাম্প্রতিক".to_string(),
                on_click_id: 102,
            },
        ],
    };

    Element::Column {
        children: vec![
            status_bar,
            search_bar,
            Element::Column { children: rows },
            bottom_nav,
        ],
    }
}

fn label_contains(query: &str, label: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    label.to_lowercase().contains(&query.to_lowercase())
}

fn main() {
    let app = App {
        state: LauncherState::default(),
        update,
        view,
        on_snapshot: Some(|s| serde_json::to_vec(s).unwrap_or_default()),
        on_restore: Some(|s, bytes| {
            if let Ok(restored) = serde_json::from_slice::<LauncherState>(bytes) {
                *s = restored;
            }
        }),
    };
    app.run("OnuronOS Home Launcher with Design System");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launcher_selection_and_search() {
        let mut st = LauncherState::default();
        update(&mut st, Ev::Click(5));
        assert_eq!(st.selected_app.as_deref(), Some("org.onuron.notes"));

        update(&mut st, Ev::Ime("calc".into()));
        let tree = view(&st);
        let json = serde_json::to_string(&tree).unwrap();
        assert!(json.contains("Alap Calculator"));
    }
}
