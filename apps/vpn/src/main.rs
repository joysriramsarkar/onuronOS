// apps/vpn/src/main.rs — OnuronOS VPN Manager
// Using the official OnuronOS Design System (dark/light themes)
use nilui::design::{DesignSystem, Palette};
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VpnProfile {
    pub name: String,
    pub server: String,
    pub status: String,
    pub active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VpnState {
    pub enabled: bool,
    pub profiles: Vec<VpnProfile>,
    pub selected: Option<String>,
    pub status: String,
}

impl Default for VpnState {
    fn default() -> Self {
        Self {
            enabled: false,
            profiles: vec![VpnProfile {
                name: "Profile 1".to_string(),
                server: "us-north.nerve.net".to_string(),
                status: "যুক্ত নেই".to_string(),
                active: false,
            }],
            selected: None,
            status: "VPN disabled".to_string(),
        }
    }
}

pub fn update(state: &mut VpnState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.enabled = !state.enabled;
            state.status = if state.enabled {
                "VPN Enabled".to_string()
            } else {
                "VPN Disabled".to_string()
            };
        }
        Ev::Click(2) => {
            state.profiles.push(VpnProfile {
                name: "Profile 2".to_string(),
                server: "eu-west.nerve.net".to_string(),
                status: "যুক্ত নেই".to_string(),
                active: false,
            });
        }
        _ => {}
    }
}

pub fn view(state: &VpnState) -> Element {
    let _ds = DesignSystem::new();
    let _palette = Palette::dark();

    Element::Column {
        children: vec![
            Element::Text {
                content: "🛡️ VPN".to_string(),
            },
            Element::Text {
                content: format!(
                    "VPN: {}",
                    if state.enabled { "Enabled" } else { "Disabled" }
                ),
            },
            Element::Text {
                content: "Profile 1".to_string(),
            },
            Element::Text {
                content: "নেটওয়ার্ক সংযোগ করুন".to_string(),
            },
            Element::Text {
                content: "নতুন প্রোফাইল যোগ করুন".to_string(),
            },
        ],
    }
}

fn main() {
    let app = App {
        state: VpnState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS VPN Manager");
}
