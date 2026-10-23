// apps/busdemo/src/main.rs — SoftBus Cross-Device Mesh Client
// Using the official OnuronOS Design System (dark/light themes)
use nilui::{App, Element, Ev};
use nilui::design::{DesignSystem, Palette};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BusAppState {
    pub peers: Vec<String>,
    pub status: String,
}

impl Default for BusAppState {
    fn default() -> Self {
        Self {
            peers: vec!["peer1.onuron.local".to_string(), "peer2.onuron.local".to_string()],
            status: "Connected to 2 peers".to_string(),
        }
    }
}

pub fn update(state: &mut BusAppState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.peers = vec!["peer3.onuron.local".to_string(), "peer4.onuron.local".to_string()];
            state.status = "Discovered 2 new peers".to_string();
        }
        Ev::Click(2) => {
            state.peers.clear();
            state.status = "Peer scan cleared".to_string();
        }
        Ev::Click(3) => {
            state.status = "Broadcasting mesh request...".to_string();
        }
        _ => {}
    }
}

pub fn view(state: &BusAppState) -> Element {
    let ds = DesignSystem::new();
    let palette = Palette::dark();

    let mut children = vec![
        Element::Text {
            content: "🔄 SoftBus Cross-Device Mesh Client".into(),
        },
        Element::Text { content: format!("Status: {}", state.status) },
        Element::Text { content: format!("Peers: {} detected", state.peers.len()) },
        Element::Row {
            children: vec![
                Element::Button { label: "Scan Peers".into(), on_click_id: 1 },
                Element::Button { label: "Clear List".into(), on_click_id: 2 },
                Element::Button { label: "Broadcast".into(), on_click_id: 3 },
            ],
        },
    ];

    for peer in &state.peers {
        children.push(Element::Text {
            content: format!("  • {}", peer),
        });
    }

    Element::Column { children }
}

fn main() {
    let app = App {
        state: BusAppState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS SoftBus Mesh");
}
