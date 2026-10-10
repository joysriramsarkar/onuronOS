// runtime/nilui-gpu/src/present.rs — 120Hz/60Hz Triple-Buffering DRM/KMS Presenter
//
// Manages three frame buffers (Front, Back, Staging) to prevent tearing, lock contention,
// and frame drops. Synchronizes presentation to display refresh cycles (120Hz = 8.33ms, 60Hz = 16.67ms).

use std::time::{Duration, Instant};
use crate::compositor::PixelBuffer;
use crate::drm::{DrmDevice, DumbBuffer};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PresentationStats {
    pub frame_index: u64,
    pub slot: usize,
    pub frame_time_ms: f32,
    pub fps: f32,
    pub dropped_frames: u64,
    pub is_hardware: bool,
    pub page_flip_success: bool,
    pub backend: &'static str,
}

pub struct KmsPresenter {
    pub width: u32,
    pub height: u32,
    pub refresh_rate_hz: u32,
    pub current_slot: usize,
    pub slots: [PixelBuffer; 3],
    pub drm_buffers: [DumbBuffer; 3],
    pub drm_device: DrmDevice,
    pub frame_counter: u64,
    pub last_frame_time: Instant,
    pub target_frame_duration: Duration,
}

impl KmsPresenter {
    pub fn new() -> Self {
        Self::with_resolution(720, 1440, 120)
    }

    pub fn with_resolution(width: u32, height: u32, refresh_rate_hz: u32) -> Self {
        let drm_device = DrmDevice::open_or_virtual(Some(crate::drm::DrmMode {
            width,
            height,
            refresh_rate_hz,
        }));

        let slots = [
            PixelBuffer::new(width, height),
            PixelBuffer::new(width, height),
            PixelBuffer::new(width, height),
        ];

        let drm_buffers = [
            drm_device.allocate_dumb_buffer(1),
            drm_device.allocate_dumb_buffer(2),
            drm_device.allocate_dumb_buffer(3),
        ];

        let target_frame_duration = Duration::from_nanos((1_000_000_000 / refresh_rate_hz as u64).max(1));

        Self {
            width,
            height,
            refresh_rate_hz,
            current_slot: 0,
            slots,
            drm_buffers,
            drm_device,
            frame_counter: 0,
            last_frame_time: Instant::now(),
            target_frame_duration,
        }
    }

    /// Returns the back buffer slot index
    pub fn acquire_next_slot(&mut self) -> usize {
        self.current_slot = (self.current_slot + 1) % 3;
        self.current_slot
    }

    /// Access the mutable back buffer for rendering
    pub fn back_buffer_mut(&mut self) -> &mut PixelBuffer {
        &mut self.slots[self.current_slot]
    }

    /// Presents the current frame to the DRM/KMS subsystem with structured error propagation.
    /// Validates buffer lengths and propagates page flip failures from hardware ioctl.
    pub fn try_present_frame(&mut self) -> Result<PresentationStats, String> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame_time);
        self.last_frame_time = now;

        // Copy rendered pixels into the hardware DumbBuffer with boundary/stride validation
        let slot = self.current_slot;
        let rendered = &self.slots[slot];
        let hw_buf = &mut self.drm_buffers[slot];

        if rendered.pixels.len() != hw_buf.pixels.len() {
            return Err(format!(
                "Framebuffer size mismatch: rendered buffer has {} pixels, DumbBuffer expects {}",
                rendered.pixels.len(),
                hw_buf.pixels.len()
            ));
        }
        hw_buf.pixels.copy_from_slice(&rendered.pixels);

        // Perform hardware page flip and propagate failure
        self.drm_device.page_flip(hw_buf.fb_id)?;

        self.frame_counter += 1;

        let frame_time_ms = elapsed.as_secs_f32() * 1000.0;
        let fps = if frame_time_ms > 0.0 { 1000.0 / frame_time_ms } else { self.refresh_rate_hz as f32 };

        Ok(PresentationStats {
            frame_index: self.frame_counter,
            slot,
            frame_time_ms,
            fps,
            dropped_frames: if elapsed > self.target_frame_duration * 2 { 1 } else { 0 },
            is_hardware: self.drm_device.is_hardware,
            page_flip_success: true,
            backend: if self.drm_device.is_hardware {
                "drm_kms_hardware"
            } else {
                "virtual_software_fallback"
            },
        })
    }

    /// Presents the current frame to the DRM/KMS subsystem.
    /// Records page flip status and telemetry in the returned stats.
    pub fn present_frame(&mut self) -> PresentationStats {
        match self.try_present_frame() {
            Ok(stats) => stats,
            Err(e) => {
                eprintln!("[KmsPresenter] Frame presentation error: {}", e);
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_frame_time);
                self.last_frame_time = now;
                self.frame_counter += 1;
                let frame_time_ms = elapsed.as_secs_f32() * 1000.0;
                let fps = if frame_time_ms > 0.0 { 1000.0 / frame_time_ms } else { self.refresh_rate_hz as f32 };
                PresentationStats {
                    frame_index: self.frame_counter,
                    slot: self.current_slot,
                    frame_time_ms,
                    fps,
                    dropped_frames: 1,
                    is_hardware: self.drm_device.is_hardware,
                    page_flip_success: false,
                    backend: if self.drm_device.is_hardware {
                        "drm_kms_hardware"
                    } else {
                        "virtual_software_fallback"
                    },
                }
            }
        }
    }
}

impl Default for KmsPresenter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triple_buffering_slot_rotation() {
        let mut presenter = KmsPresenter::with_resolution(100, 100, 60);
        assert_eq!(presenter.current_slot, 0);

        let slot1 = presenter.acquire_next_slot();
        assert_eq!(slot1, 1);

        let slot2 = presenter.acquire_next_slot();
        assert_eq!(slot2, 2);

        let slot3 = presenter.acquire_next_slot();
        assert_eq!(slot3, 0);
    }

    #[test]
    fn test_present_frame_pipeline() {
        let mut presenter = KmsPresenter::with_resolution(100, 100, 120);
        let slot = presenter.acquire_next_slot();
        presenter.back_buffer_mut().fill_rect(10, 10, 20, 20, 0xFF00FF00);

        let stats = presenter.present_frame();
        assert_eq!(stats.slot, slot);
        assert_eq!(stats.frame_index, 1);
        assert!(stats.page_flip_success);
        assert_eq!(stats.backend, "virtual_software_fallback");
        assert_eq!(presenter.drm_buffers[slot].get_pixel(15, 15), 0xFF00FF00);
    }

    #[test]
    fn test_try_present_frame_propagates_page_flip_error() {
        let mut presenter = KmsPresenter::with_resolution(100, 100, 60);
        // Set invalid fb_id to trigger page_flip failure
        presenter.drm_buffers[0].fb_id = 0;
        let res = presenter.try_present_frame();
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Invalid framebuffer ID: 0"));
    }
}
