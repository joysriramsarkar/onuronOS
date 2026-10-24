// apps/sysupdate/src/main.rs — OnuronOS System Update
// Using the official OnuronOS Design System (dark/light themes)
use nilui::design::{DesignSystem, Palette};
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateState {
    pub current_version: String,
    pub available: bool,
    pub downloading: bool,
    pub status: String,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            current_version: "1.7.0".to_string(),
            available: false,
            downloading: false,
            status: "System up to date. No updates available.".to_string(),
        }
    }
}

pub fn update(state: &mut UpdateState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.available = true;
            state.status = format!("Update available! Current: {}", state.current_version);
        }
        Ev::Click(2) => {
            state.downloading = true;
            state.status = format!("Downloading update... {}", state.current_version);
        }
        Ev::Click(3) => {
            state.downloading = false;
            state.status = format!("Updating to new version... {}", state.current_version);
        }
        Ev::Click(4) => {
            state.status = format!("Update complete! Rebooting...");
        }
        _ => {}
    }
}

pub fn view(state: &UpdateState) -> Element {
    let _ds = DesignSystem::new();
    let _palette = Palette::dark();

    Element::Column {
        children: vec![
            Element::Text {
                content: "🔄 System Update".to_string(),
            },
            Element::Text {
                content: format!("Current Version: {}", state.current_version),
            },
            Element::Text {
                content: state.status.clone(),
            },
            Element::Row {
                children: vec![
                    Element::Button {
                        label: "🔄 চেক আপডেট".into(),
                        on_click_id: 1,
                    },
                    Element::Button {
                        label: "⬇️ ডাউনলোড".into(),
                        on_click_id: 2,
                    },
                ],
            },
        ],
    }
}

fn main() {
    let app = App {
        state: UpdateState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS System Update");
}
