// apps/launcher/src/main.rs — OnuronOS Home Grid Launcher
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LauncherState {
    pub current_page: usize,
    pub selected_app: Option<String>,
    pub search_query: String,
    pub installed_apps: Vec<(String, String)>,
}

impl Default for LauncherState {
    fn default() -> Self {
        Self {
            current_page: 0,
            selected_app: None,
            search_query: String::new(),
            installed_apps: vec![
                ("org.onuron.phone".into(), "Phone & Dialer".into()),
                ("org.onuron.messages".into(), "Encrypted SMS".into()),
                ("org.onuron.files".into(), "File Manager".into()),
                ("org.onuron.settings".into(), "System Settings".into()),
                ("org.onuron.notes".into(), "NilLang Notes".into()),
                ("org.onuron.calculator".into(), "Alap Calculator".into()),
                ("org.onuron.camera".into(), "Camera Viewfinder".into()),
                ("org.onuron.softbus".into(), "SoftBus Mesh".into()),
            ],
        }
    }
}

pub fn update(state: &mut LauncherState, ev: Ev) {
    match ev {
        Ev::Click(id) if (1..=8).contains(&id) => {
            if let Some((app_id, _)) = state.installed_apps.get((id - 1) as usize) {
                state.selected_app = Some(app_id.clone());
                let _ = nilui::send_shell_cmd(&format!("LAUNCH {}", app_id));
            }
        }
        Ev::Click(100) => {
            state.current_page = 0;
        }
        Ev::Click(101) => {
            state.current_page = 1;
        }
        Ev::Ime(text) => {
            state.search_query = text;
        }
        _ => {}
    }
}

pub fn view(state: &LauncherState) -> Element {
    let q = state.search_query.to_lowercase();
    let buttons: Vec<Element> = state
        .installed_apps
        .iter()
        .enumerate()
        .filter(|(_, (id, label))| {
            q.is_empty() || id.to_lowercase().contains(&q) || label.to_lowercase().contains(&q)
        })
        .map(|(idx, (_, label))| Element::Button {
            label: label.clone(),
            on_click_id: (idx + 1) as u32,
        })
        .collect();

    Element::Column {
        children: vec![
            Element::Text {
                content: format!("OnuronOS Home Launcher (Page {})", state.current_page + 1),
            },
            Element::Input {
                placeholder: "Search apps...".into(),
                text: state.search_query.clone(),
            },
            Element::Column { children: buttons },
            Element::Text {
                content: match &state.selected_app {
                    Some(app) => format!("Active Launch Target: {}", app),
                    None => "Ready".into(),
                },
            },
        ],
    }
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
    app.run("OnuronOS Home Grid Launcher");
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
