// apps/bluetooth/src/main.rs — OnuronOS Bluetooth Manager
// Using the official OnuronOS Design System (dark/light themes)
use nilui::design::{DesignSystem, Palette};
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BluetoothDevice {
    pub name: String,
    pub status: String,
    pub paired: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BluetoothState {
    pub enabled: bool,
    pub devices: Vec<BluetoothDevice>,
    pub selected: Option<String>,
    pub status: String,
}

impl Default for BluetoothState {
    fn default() -> Self {
        Self {
            enabled: true,
            devices: vec![
                BluetoothDevice {
                    name: "যৌথ ডিভাইস".to_string(),
                    status: "সংযুক্ত".to_string(),
                    paired: true,
                },
                BluetoothDevice {
                    name: "হেডফোন".to_string(),
                    status: "ডিভাইস খুঁজছে".to_string(),
                    paired: false,
                },
            ],
            selected: None,
            status: "Bluetooth Disabled".to_string(),
        }
    }
}

pub fn update(state: &mut BluetoothState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.enabled = !state.enabled;
        }
        Ev::Click(2) => {
            state.selected = Some("যৌথ ডিভাইস".to_string());
        }
        Ev::Click(3) => {
            state.selected = Some("হেডফোন".to_string());
        }
        Ev::Click(4) => {
            state.devices.push(BluetoothDevice {
                name: "নতুন ডিভাইস".to_string(),
                status: "সন্ধান করছে".to_string(),
                paired: false,
            });
        }
        _ => {}
    }
}

pub fn view(state: &BluetoothState) -> Element {
    let ds = DesignSystem::new();
    let palette = Palette::dark();

    Element::Column {
        children: vec![
            Element::Text {
                content: "🔵 Bluetooth".to_string(),
            },
            Element::Text {
                content: format!(
                    "Bluetooth: {}",
                    if state.enabled { "Enabled" } else { "Disabled" }
                ),
            },
            Element::Row {
                children: vec![
                    Element::Button {
                        label: "➕ নতুন ডিভাইস খুঁজুন".into(),
                        on_click_id: 4,
                    },
                    Element::Button {
                        label: "📱 জোড়া যোগ করুন".into(),
                        on_click_id: 1,
                    },
                ],
            },
            Element::Text {
                content: "যৌথ ডিভাইস".to_string(),
            },
            Element::Text {
                content: "✓ সংযুক্ত".to_string(),
            },
            Element::Text {
                content: "হেডফোন".to_string(),
            },
            Element::Text {
                content: "🔄 ডিভাইস খুঁজছে".to_string(),
            },
        ],
    }
}

fn main() {
    let app = App {
        state: BluetoothState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS Bluetooth Manager");
}
