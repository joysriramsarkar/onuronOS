// apps/busdemo/src/main.rs — SoftBus Cross-Device Mesh Client
use nilbus_client::{Peer, SoftBusClient};
use nilui::{App, Element, Ev};

#[derive(Clone, Debug, Default)]
pub struct BusAppState {
    pub peers: Vec<Peer>,
    pub status: String,
}

pub fn update(state: &mut BusAppState, ev: Ev) {
    match ev {
        Ev::Click(1) => match SoftBusClient::get_nearby_peers() {
            Ok(list) => {
                state.status = format!("Discovered {} peer(s)", list.len());
                state.peers = list;
            }
            Err(e) => {
                state.status = format!("Scan error: {}", e);
            }
        },
        _ => {}
    }
}

pub fn view(state: &BusAppState) -> Element {
    let mut children = vec![
        Element::Text {
            content: "OnuronOS SoftBus Mesh Client".into(),
        },
        Element::Text {
            content: format!(
                "Status: {}",
                if state.status.is_empty() {
                    "Ready"
                } else {
                    &state.status
                }
            ),
        },
        Element::Button {
            label: "Scan Nearby Peers".into(),
            on_click_id: 1,
        },
    ];
    for p in &state.peers {
        children.push(Element::Text {
            content: format!("• {} ({}:{})", p.device_id, p.addr, p.quic_port),
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
    app.run("OnuronOS SoftBus Mesh Client");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bus_scan_updates_status() {
        let mut st = BusAppState::default();
        update(&mut st, Ev::Click(1));
        assert!(st.status.contains("Discovered"));
    }
}
