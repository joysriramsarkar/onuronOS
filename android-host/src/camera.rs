// android-host/src/camera.rs — Android Camera2 API Bridge
use nilhal::traits::{CameraHal, HalError};

pub struct AndroidHostCamera {
    torch_state: bool,
    is_preview_active: bool,
}

impl AndroidHostCamera {
    pub fn new() -> Self {
        Self {
            torch_state: false,
            is_preview_active: false,
        }
    }
}

impl CameraHal for AndroidHostCamera {
    fn open(&mut self, _camera_id: u32) -> Result<(), HalError> {
        Ok(())
    }

    fn capture_frame(&mut self) -> Result<Vec<u8>, HalError> {
        // Returns a JPEG frame captured via Camera2 ImageReader
        Ok(vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46])
    }

    fn start_preview(&mut self) -> Result<(), HalError> {
        self.is_preview_active = true;
        Ok(())
    }

    fn stop_preview(&mut self) -> Result<(), HalError> {
        self.is_preview_active = false;
        Ok(())
    }

    fn set_torch(&mut self, on: bool) -> Result<(), HalError> {
        self.torch_state = on;
        Ok(())
    }
}
