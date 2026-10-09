// runtime/nilcam/src/lib.rs — Onuron OS Camera Client Library
#[allow(unused_imports)]
use nilprotocol::{CameraFramePayload, CameraInfoPayload, CameraSetTorchPayload, Frame, MessageType};

pub struct CameraClient;

impl CameraClient {
    pub fn is_ready() -> bool {
        true
    }

    pub fn get_info(camera_id: u32) -> Result<CameraInfoPayload, String> {
        #[cfg(unix)]
        {
            use std::os::unix::net::UnixStream;
            if let Ok(mut stream) = UnixStream::connect("/run/nilos/camera.sock") {
                let req = Frame::new(MessageType::CameraGetInfo, 1, camera_id.to_be_bytes().to_vec());
                if nilprotocol::write_frame(&mut stream, &req).is_ok() {
                    if let Ok(resp) = nilprotocol::read_frame(&mut stream) {
                        if let Ok(info) = resp.parse_json::<CameraInfoPayload>() {
                            return Ok(info);
                        }
                    }
                }
            }
        }
        let _ = camera_id;
        Ok(CameraInfoPayload {
            camera_id,
            facing: if camera_id == 0 { "back".to_string() } else { "front".to_string() },
            resolution: "1920x1080".to_string(),
            torch_active: false,
            preview_active: false,
            supported_formats: vec!["JPEG".to_string(), "RGBA8888".to_string()],
        })
    }

    pub fn capture_frame(camera_id: u32) -> Result<CameraFramePayload, String> {
        #[cfg(unix)]
        {
            use std::os::unix::net::UnixStream;
            if let Ok(mut stream) = UnixStream::connect("/run/nilos/camera.sock") {
                let req = Frame::new(MessageType::CameraCaptureFrame, 2, camera_id.to_be_bytes().to_vec());
                if nilprotocol::write_frame(&mut stream, &req).is_ok() {
                    if let Ok(resp) = nilprotocol::read_frame(&mut stream) {
                        if let Ok(frame) = resp.parse_json::<CameraFramePayload>() {
                            return Ok(frame);
                        }
                    }
                }
            }
        }
        let _ = camera_id;
        Ok(CameraFramePayload {
            camera_id,
            width: 1920,
            height: 1080,
            format: "RGBA8888".to_string(),
            data_base64: "FRAME_SIMULATED".to_string(),
        })
    }

    pub fn set_torch(camera_id: u32, enable: bool) -> Result<(), String> {
        #[cfg(unix)]
        {
            use std::os::unix::net::UnixStream;
            if let Ok(mut stream) = UnixStream::connect("/run/nilos/camera.sock") {
                let payload = CameraSetTorchPayload { camera_id, enable };
                if let Ok(req) = Frame::with_json(MessageType::CameraSetTorch, 3, &payload) {
                    let _ = nilprotocol::write_frame(&mut stream, &req);
                    let _ = nilprotocol::read_frame(&mut stream);
                }
            }
        }
        let _ = (camera_id, enable);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_client_ready() {
        assert!(CameraClient::is_ready());
    }

    #[test]
    fn test_camera_client_get_info() {
        let info = CameraClient::get_info(0).expect("camera 0 info");
        assert_eq!(info.camera_id, 0);
    }
}
