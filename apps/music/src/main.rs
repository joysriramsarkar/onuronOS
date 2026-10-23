// apps/music/src/main.rs — OnuronOS Music Player
// Using the official OnuronOS Design System (dark/light themes)
use nilui::{App, Element, Ev};
use nilui::design::{DesignSystem, Palette};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MusicState {
    pub current_track: String,
    pub current_time: f32,
    pub is_playing: bool,
    pub volume: u8,
    pub status: String,
}

impl Default for MusicState {
    fn default() -> Self {
        Self {
            current_track: "আমার সোনার বাংলা".to_string(),
            current_time: 0.0,
            is_playing: true,
            volume: 70,
            status: "Playing".to_string(),
        }
    }
}

pub fn update(state: &mut MusicState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.is_playing = !state.is_playing;
        }
        Ev::Click(2) => {
            state.current_time = 0.0;
        }
        Ev::Click(3) => {
            state.volume = (state.volume as i32 + 10).min(100) as u8;
        }
        Ev::Click(4) => {
            state.volume = (state.volume as i32 - 10).max(0) as u8;
        }
        Ev::Click(5) => {
            state.current_track = "রবীন্দ্র সঙ্গীত".to_string();
        }
        Ev::Click(6) => {
            state.current_track = "আমার সোনার বাংলা".to_string();
        }
        Ev::Click(7) => {
            state.current_track = "বাংলাদেশ".to_string();
        }
        _ => {}
    }
}

pub fn view(state: &MusicState) -> Element {
    let ds = DesignSystem::new();
    let palette = Palette::dark();

    let track_bg = if palette.dark_bg == 0xFF1E1236 { palette.dark_card } else { palette.light_card };
    let text_color = if palette.dark_bg == 0xFF1E1236 { palette.dark_text } else { palette.light_text };
    let accent = if palette.dark_bg == 0xFF1E1236 { palette.dark_accent_green } else { palette.light_accent_green };

fn format_time(t: f32) -> String {
        let s = t as u64;
        format!("{:02}:{:02}", s / 60, s % 60)
    };

    Element::Column {
        children: vec![
            Element::Text { content: "🎵 Music Player".to_string() },
            Element::Text {
                content: state.current_track.clone(),
            },
            Element::Row {
                children: vec![
                    Element::Button { label: "⏮".into(), on_click_id: 2 },
                    if state.is_playing {
                        Element::Button { label: "⏸".into(), on_click_id: 1 }
                    } else {
                        Element::Button { label: "▶".into(), on_click_id: 1 }
                    },
                    Element::Button { label: "⏭".into(), on_click_id: 3 },
                    Element::Button { label: "🔊".into(), on_click_id: 3 },
                ],
            },
            Element::Row {
                children: vec![
                    Element::Button { label: "⏰".into(), on_click_id: 4 },
                    Element::Text { content: format!("{} / 3:20", format_time(state.current_time)) },
                ],
            },
            Element::Text {
                content: format!("Volume: {}%", state.volume),
            },
        ],
    }
}

fn main() {
    let app = App {
        state: MusicState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS Music Player");
}
