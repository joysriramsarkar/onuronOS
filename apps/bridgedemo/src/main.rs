// apps/bridgedemo/src/main.rs — Isolated Android Bridge Client Inspector
use nilui::{App, Element, Ev};

#[derive(Clone, Debug, Default)]
pub struct BridgeState {
    pub last_intent: String,
    pub status: String,
}

pub fn update(state: &mut BridgeState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.last_intent = "org.onuron.intent.VIEW".into();
            match nilandroid_client::AndroidBridge::start_activity(
                &state.last_intent,
                "onuron://home",
            ) {
                Ok(()) => state.status = format!("Dispatched {}", state.last_intent),
                Err(e) => state.status = format!("Bridge error: {}", e),
            }
        }
        _ => {}
    }
}

pub fn view(state: &BridgeState) -> Element {
    Element::Column {
        children: vec![
            Element::Text {
                content: "OnuronOS Android Isolation Bridge Inspector".into(),
            },
            Element::Text {
                content: format!(
                    "Status: {}",
                    if state.status.is_empty() {
                        "Idle"
                    } else {
                        &state.status
                    }
                ),
            },
            Element::Button {
                label: "Send Test Intent".into(),
                on_click_id: 1,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_dispatch_updates_state() {
        let mut st = BridgeState::default();
        update(&mut st, Ev::Click(1));
        assert_eq!(st.last_intent, "org.onuron.intent.VIEW");
        assert!(!st.status.is_empty());
    }
}
