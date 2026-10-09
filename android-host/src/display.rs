// android-host/src/display.rs — Android Surface / Framebuffer Bridge
// Renders Onuron UI frames directly onto Samsung Galaxy S25 SurfaceView / ANativeWindow.

use crate::bridge::GuestToHostCommand;
use crate::jni_bridge;
use nilhal::traits::{DisplayHal, HalError};

pub struct AndroidHostDisplay {
    width: u32,
    height: u32,
    refresh_rate: u32,
    brightness: u8,
    frame_count: u64,
    last_frame: Option<Vec<u32>>,
}

impl AndroidHostDisplay {
    pub fn new() -> Self {
        let (dyn_w, dyn_h) = jni_bridge::get_surface_dimensions();
        Self {
            // Galaxy S25 6.2" Dynamic AMOLED 2X resolution & refresh rate
            width: if dyn_w > 0 { dyn_w } else { 1080 },
            height: if dyn_h > 0 { dyn_h } else { 2340 },
            refresh_rate: 120,
            brightness: 90,
            frame_count: 0,
            last_frame: None,
        }
    }

    pub fn get_frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn get_cached_frame(&self) -> Option<&[u32]> {
        self.last_frame.as_deref()
    }
}

impl Default for AndroidHostDisplay {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayHal for AndroidHostDisplay {
    fn get_dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn get_refresh_rate(&self) -> u32 {
        self.refresh_rate
    }

    fn present_frame(&mut self, buffer: &[u32]) -> Result<(), HalError> {
        let (w, h) = self.get_dimensions();
        let expected_len = (w * h) as usize;

        if !buffer.is_empty() && buffer.len() != expected_len {
            // If dimensions changed, accept buffer and adjust dimensions
            if buffer.len() == (self.width * self.height) as usize {
                // Compatible with internal dimensions
            } else {
                return Err(HalError::UnsupportedOperation(format!(
                    "Buffer pixel count ({}) does not match surface resolution ({}x{} = {})",
                    buffer.len(),
                    w,
                    h,
                    expected_len
                )));
            }
        }

        // 1. Dispatch raw frame buffer to JNI shared memory / Surface lock
        jni_bridge::publish_display_frame(buffer, w, h);

        // 2. Track presentation metrics
        self.frame_count = self.frame_count.saturating_add(1);
        self.last_frame = Some(buffer.to_vec());

        Ok(())
    }

    fn set_brightness(&mut self, percent: u8) -> Result<(), HalError> {
        self.brightness = percent.min(100);
        jni_bridge::enqueue_guest_command(GuestToHostCommand::SetBrightness {
            percent: self.brightness,
        });
        Ok(())
    }

    fn get_brightness(&self) -> u8 {
        self.brightness
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_presentation_pipeline() {
        let mut display = AndroidHostDisplay::new();
        assert_eq!(display.get_refresh_rate(), 120);
        assert_eq!(display.get_frame_count(), 0);

        let (w, h) = display.get_dimensions();
        let pixels = vec![0xFFFFFFFFu32; (w * h) as usize];

        assert!(display.present_frame(&pixels).is_ok());
        assert_eq!(display.get_frame_count(), 1);

        let latest = jni_bridge::get_latest_frame().expect("Frame must be published");
        assert_eq!(latest.len(), pixels.len());
        assert_eq!(latest[0], 0xFFFFFFFF);
    }

    #[test]
    fn test_display_brightness_and_dimensions() {
        let mut display = AndroidHostDisplay::new();
        assert!(display.set_brightness(75).is_ok());
        assert_eq!(display.get_brightness(), 75);

        // Test clamping
        assert!(display.set_brightness(120).is_ok());
        assert_eq!(display.get_brightness(), 100);

        // Check command enqueued
        let cmd = jni_bridge::poll_guest_command();
        assert!(cmd.is_some());
    }

    #[test]
    fn test_mismatched_buffer_rejected() {
        let mut display = AndroidHostDisplay::new();
        let bad_pixels = vec![0u32; 10]; // Too small
        assert!(display.present_frame(&bad_pixels).is_err());
    }
}
