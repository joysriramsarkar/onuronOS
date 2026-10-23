// apps/wifi/src/main.rs — OnuronOS WiFi Manager
// Using the official OnuronOS Design System (dark/light themes)
use nilui::{App, Element, Ev};
use nilui::design::{DesignSystem, Palette};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub status: String,
    pub security: String,
    pub hidden: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WifiState {
    pub enabled: bool,
    pub networks: Vec<WifiNetwork>,
    pub selected: Option<String>,
    pub status: String,
    pub scanning: bool,
}

impl Default for WifiState {
    fn default() -> Self {
        Self {
            enabled: true,
            networks: vec![
                WifiNetwork { ssid: "Home Network".to_string(), status: "সংযুক্ত · সুরক্ষিত".to_string().to_string(), security: "WPA3".to_string().to_string(), hidden: false },
                WifiNetwork { ssid: "ক্যাফে-ফ্রি".to_string(), status: "খোলা".to_string(), security: "Open".to_string(), hidden: false },
                WifiNetwork { ssid: "প্রতিবেশী-নেট".to_string(), status: "সুরক্ষিত".to_string(), security: "WPA2".to_string(), hidden: true },
            ],
            selected: None,
            status: "WiFi disabled".to_string(),
            scanning: false,
        }
    }
}

pub fn update(state: &mut WifiState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.enabled = !state.enabled;
            state.scanning = true;
        }
        Ev::Click(2) => {
            state.selected = Some("Home Network".to_string());
        }
        Ev::Click(3) => {
            state.selected = Some("ক্যাফে-ফ্রি".to_string());
        }
        Ev::Click(4) => {
            state.selected = Some("প্রতিবেশী-নেট".to_string().to_string());
        }
        Ev::Click(5) => {
            state.scanning = !state.scanning;
        }
        _ => {}
    }
}

pub fn view(state: &WifiState) -> Element {
    let ds = DesignSystem::new();
    let palette = Palette::dark();

    Element::Column {
        children: vec![
            Element::Text { content: "📶 WiFi".to_string() },
            Element::Text {
                content: format!("WiFi: {}", if state.enabled { "Enabled" } else { "Disabled" }),
            },
            Element::Row {
                children: vec![
                    Element::Button { label: "🔍 স্ক্যান".into(), on_click_id: 5 },
                    Element::Button { label: "📶 নেটওয়ার্ক".into(), on_click_id: 1 },
                ],
            },
            Element::Text { content: "হোম নেটওয়ার্ক".to_string() },
            Element::Text {
                content: format!("📡 {}-{}", if state.enabled { "সংযুক্ত" } else { "খোলা" }, if state.enabled { "সুরক্ষিত" } else { "খোলা" }),
            },
            Element::Text { content: "✓ সুরক্ষিত".to_string() },
            Element::Text { content: "✓ প্রতিবেশী-নেট".to_string() },
            Element::Text { content: "✓ ক্যাফে-ফ্রি".to_string() },
        ],
    }
}

fn main() {
    let app = App {
        state: WifiState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS WiFi Manager");
}
