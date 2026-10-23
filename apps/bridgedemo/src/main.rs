// apps/bridgedemo/src/main.rs — Isolated Android Bridge Client Inspector
// Using the official OnuronOS Design System (dark/light themes)
use nilui::{App, Element, Ev};
use nilui::design::{DesignSystem, Palette};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BridgeState {
    pub last_intent: String,
    pub status: String,
}

impl Default for BridgeState {
    fn default() -> Self {
        Self {
            last_intent: "org.onuron.intent.VIEW".into(),
            status: "Ready".to_string(),
        }
    }
}

pub fn update(state: &mut BridgeState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.last_intent = "org.onuron.intent.VIEW".into();
            state.status = "Dispatched VIEW".to_string();
        }
        Ev::Click(2) => {
            state.last_intent = "org.onuron.intent.EDIT".into();
            state.status = "Dispatched EDIT".to_string();
        }
        Ev::Click(3) => {
            state.last_intent = "org.onuron.intent.SEND".into();
            state.status = "Dispatched SEND".to_string();
        }
        Ev::Click(4) => {
            state.last_intent = "org.onuron.intent.PICK".into();
            state.status = "Dispatched PICK".to_string();
        }
        Ev::Click(5) => {
            state.last_intent = "org.onuron.intent.GET_CONTENT".into();
            state.status = "Dispatched GET_CONTENT".to_string();
        }
        Ev::Click(6) => {
            state.last_intent = "org.onuron.intent.SEARCH".into();
            state.status = "Dispatched SEARCH".to_string();
        }
        _ => {}
    }
}

pub fn view(state: &BridgeState) -> Element {
    let ds = DesignSystem::new();
    let palette = Palette::dark();

    Element::Column {
        children: vec![
            Element::Text {
                content: "🧩 Isolated Android Bridge Client Inspector".into(),
            },
            Element::Text {
                content: format!("Last Intent: {}", state.last_intent),
            },
            Element::Text { content: format!("Status: {}", state.status) },
            Element::Row {
                children: vec![
                    Element::Button { label: "View".into(), on_click_id: 1 },
                    Element::Button { label: "Edit".into(), on_click_id: 2 },
                    Element::Button { label: "Send".into(), on_click_id: 3 },
                    Element::Button { label: "Pick".into(), on_click_id: 4 },
                    Element::Button { label: "Get Content".into(), on_click_id: 5 },
                    Element::Button { label: "Search".into(), on_click_id: 6 },
                ],
            },
        ],
    }
}

fn main() {
    let app = App {
        state: BridgeState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS Android Bridge Inspector");
}
