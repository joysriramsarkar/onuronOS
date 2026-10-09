// apps/camdemo/src/main.rs — Camera Preview & Capture Demo Application
use nilcam::CameraClient;
use nilui::{App, Element, Ev};

#[derive(Clone, Debug)]
struct CameraState {
    camera_id: u32,
    facing: String,
    resolution: String,
    torch_on: bool,
    captured_count: u32,
    last_frame_info: String,
    status_msg: String,
}

impl Default for CameraState {
    fn default() -> Self {
        let (id, facing, res) = match CameraClient::get_info(0) {
            Ok(info) => (info.camera_id, info.facing, info.resolution),
            Err(_) => (0, "back".to_string(), "1920x1080".to_string()),
        };
        Self {
            camera_id: id,
            facing,
            resolution: res,
            torch_on: false,
            captured_count: 0,
            last_frame_info: "No frames captured yet".to_string(),
            status_msg: "Viewfinder Active (120Hz Pipeline)".to_string(),
        }
    }
}

fn update(state: &mut CameraState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            // Capture Photo
            match CameraClient::capture_frame(state.camera_id) {
                Ok(frame) => {
                    state.captured_count += 1;
                    state.last_frame_info = format!(
                        "Photo #{} saved: {}x{} ({})",
                        state.captured_count, frame.width, frame.height, frame.format
                    );
                    state.status_msg = "Frame captured successfully.".to_string();
                }
                Err(e) => {
                    state.status_msg = format!("Capture failed: {e}");
                }
            }
        }
        Ev::Click(2) => {
            // Toggle Torch
            state.torch_on = !state.torch_on;
            let _ = CameraClient::set_torch(state.camera_id, state.torch_on);
            state.status_msg = format!(
                "Torch {}",
                if state.torch_on { "Enabled" } else { "Disabled" }
            );
        }
        Ev::Click(3) => {
            // Switch Camera (Back <-> Front)
            state.camera_id = if state.camera_id == 0 { 1 } else { 0 };
            if let Ok(info) = CameraClient::get_info(state.camera_id) {
                state.facing = info.facing;
                state.resolution = info.resolution;
            } else {
                state.facing = if state.camera_id == 0 { "back".into() } else { "front".into() };
            }
            state.status_msg = format!("Switched to {} camera", state.facing);
        }
        _ => {}
    }
}

fn view(state: &CameraState) -> Element {
    Element::Column {
        children: vec![
            Element::Text {
                content: format!("📷 Onuron Camera Viewfinder — {} ({})", state.facing.to_uppercase(), state.resolution),
            },
            Element::Text {
                content: format!("Status: {}", state.status_msg),
            },
            Element::Text {
                content: format!("Last: {}", state.last_frame_info),
            },
            Element::Row {
                children: vec![
                    Element::Button {
                        label: "📸 Capture Photo".into(),
                        on_click_id: 1,
                    },
                    Element::Button {
                        label: format!("🔦 Torch: {}", if state.torch_on { "ON" } else { "OFF" }),
                        on_click_id: 2,
                    },
                    Element::Button {
                        label: "🔄 Switch Camera".into(),
                        on_click_id: 3,
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
    app.run("Onuron Camera");
}
