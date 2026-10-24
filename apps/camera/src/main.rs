// apps/camera/src/main.rs — OnuronOS Camera Viewfinder
// Using the official OnuronOS Design System (dark/light themes)
use nilui::design::{DesignSystem, Palette};
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CameraState {
    pub facing: String,
    pub torch: bool,
    pub zoom: f32,
    pub captured: u32,
    pub status: String,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            facing: "back".to_string(),
            torch: false,
            zoom: 1.0,
            captured: 0,
            status: "Ready".to_string(),
        }
    }
}

pub fn update(state: &mut CameraState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.facing = if state.facing == "back" {
                "front".to_string()
            } else {
                "back".to_string()
            };
        }
        Ev::Click(2) => {
            state.torch = !state.torch;
        }
        Ev::Click(3) => {
            state.zoom = (state.zoom + 0.5).min(5.0);
        }
        Ev::Click(4) => {
            state.zoom = (state.zoom - 0.5).max(1.0);
        }
        Ev::Click(5) => {
            state.captured += 1;
            state.torch = false;
            let _ = std::fs::create_dir_all("/data/dcim");
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let img_name = format!("/data/dcim/IMG_{}.jpg", secs);
            state.status = format!("সংরক্ষিত: {}", img_name);

            #[cfg(unix)]
            {
                use std::io::Write;
                if let Ok(mut s) = std::os::unix::net::UnixStream::connect("/run/nilos/camera.sock") {
                    let _ = s.write_all(b"CAPTURE\n");
                }
            }
        }
        _ => {}
    }
}

pub fn view(state: &CameraState) -> Element {
    let _ds = DesignSystem::new();
    let _palette = Palette::dark();

    Element::Column {
        children: vec![
            Element::Text {
                content: format!("📷 Camera ({})", state.facing),
            },
            Element::Text {
                content: format!("🔦 Torch: {}", if state.torch { "ON" } else { "OFF" }),
            },
            Element::Text {
                content: format!("🔍 Zoom: {:.1}x", state.zoom),
            },
            Element::Text {
                content: format!("📸 Captures: {}", state.captured),
            },
            Element::Row {
                children: vec![
                    Element::Button {
                        label: "📸 Capture".into(),
                        on_click_id: 5,
                    },
                    Element::Button {
                        label: "🔄 Flip".into(),
                        on_click_id: 1,
                    },
                    Element::Button {
                        label: "🔦 Torch".into(),
                        on_click_id: 2,
                    },
                ],
            },
        ],
    }
}

fn main() {
    let app = App {
        state: CameraState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS Camera Viewfinder");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_flip_and_capture() {
        let mut state = CameraState::default();
        assert_eq!(state.facing, "back");

        update(&mut state, Ev::Click(1)); // Flip
        assert_eq!(state.facing, "front");

        let initial_captured = state.captured;
        update(&mut state, Ev::Click(5)); // Capture
        assert_eq!(state.captured, initial_captured + 1);
    }
}

