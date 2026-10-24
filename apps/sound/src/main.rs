// apps/sound/src/main.rs — OnuronOS Sound & Audio Manager
// Using the official OnuronOS Design System (dark/light themes)
use nilui::design::{DesignSystem, Palette};
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SoundState {
    pub ringtone: String,
    pub ringtone_volume: u8,
    pub media_volume: u8,
    pub alarm_volume: u8,
    pub haptics: bool,
}

impl Default for SoundState {
    fn default() -> Self {
        Self {
            ringtone: "Default".to_string(),
            ringtone_volume: 70,
            media_volume: 50,
            alarm_volume: 80,
            haptics: true,
        }
    }
}

pub fn update(state: &mut SoundState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.ringtone = "Default".to_string();
        }
        Ev::Click(2) => {
            state.ringtone = "Custom Ringtone".to_string();
        }
        Ev::Click(3) => {
            state.ringtone_volume = (state.ringtone_volume as i32 + 10).min(100) as u8;
        }
        Ev::Click(4) => {
            state.ringtone_volume = (state.ringtone_volume as i32 - 10).max(0) as u8;
        }
        Ev::Click(5) => {
            state.media_volume = (state.media_volume as i32 + 10).min(100) as u8;
        }
        Ev::Click(6) => {
            state.media_volume = (state.media_volume as i32 - 10).max(0) as u8;
        }
        Ev::Click(7) => {
            state.alarm_volume = (state.alarm_volume as i32 + 10).min(100) as u8;
        }
        Ev::Click(8) => {
            state.alarm_volume = (state.alarm_volume as i32 - 10).max(0) as u8;
        }
        Ev::Click(9) => {
            state.haptics = !state.haptics;
        }
        _ => {}
    }
}

pub fn view(state: &SoundState) -> Element {
    let _ds = DesignSystem::new();
    let _palette = Palette::dark();

    Element::Column {
        children: vec![
            Element::Text {
                content: "🔊 Sound".to_string(),
            },
            Element::Text {
                content: format!("🔔 Ringtone: {}", state.ringtone),
            },
            Element::Text {
                content: format!("📢 Ringtone Volume: {}%", state.ringtone_volume),
            },
            Element::Text {
                content: format!("🎵 Media Volume: {}%", state.media_volume),
            },
            Element::Text {
                content: format!("⏰ Alarm Volume: {}%", state.alarm_volume),
            },
            Element::Text {
                content: format!("📳 Haptics: {}", if state.haptics { "ON" } else { "OFF" }),
            },
        ],
    }
}

fn main() {
    let app = App {
        state: SoundState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS Sound Manager");
}
