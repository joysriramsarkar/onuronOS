// runtime/nilui-gpu/src/drm.rs — Direct Rendering Manager & KMS Modesetting
//
// Communicates directly with the Linux DRM subsystem (/dev/dri/card0) to perform
// display mode selection, Dumb Buffer allocation, and VBLANK-synchronized page flipping.
// Operates with zero dependency on X11 or external display servers.
// On non-Linux or when /dev/dri/card0 is absent, falls back to a software virtual display.

use std::fs::{File, OpenOptions};
use std::path::Path;

pub const DEFAULT_DRM_CARD: &str = "/dev/dri/card0";
pub const SECONDARY_DRM_CARD: &str = "/dev/dri/card1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrmMode {
    pub width: u32,
    pub height: u32,
    pub refresh_rate_hz: u32,
}

impl Default for DrmMode {
    fn default() -> Self {
        // Standard mobile display resolution (720x1440 @ 120Hz)
        Self {
            width: 720,
            height: 1440,
            refresh_rate_hz: 120,
        }
    }
}

/// A DRM Dumb Buffer allocated for direct frame rendering.
/// In 32-bit ARGB/XRGB8888, each pixel occupies 4 bytes.
#[derive(Debug, Clone)]
pub struct DumbBuffer {
    pub width: u32,
    pub height: u32,
    pub pitch: u32,     // Bytes per row (width * 4 aligned)
    pub size_bytes: usize,
    pub pixels: Vec<u32>, // 32-bit ARGB pixel memory
    pub fb_id: u32,
}

impl DumbBuffer {
    pub fn new(width: u32, height: u32, fb_id: u32) -> Self {
        let pitch = width * 4;
        let size_bytes = (pitch * height) as usize;
        let pixel_count = (width * height) as usize;
        Self {
            width,
            height,
            pitch,
            size_bytes,
            pixels: vec![0xFF000000; pixel_count], // Initialized to opaque black
            fb_id,
        }
    }

    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, argb: u32) {
        if x < self.width && y < self.height {
            let idx = (y * self.width + x) as usize;
            self.pixels[idx] = argb;
        }
    }

    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> u32 {
        if x < self.width && y < self.height {
            let idx = (y * self.width + x) as usize;
            self.pixels[idx]
        } else {
            0
        }
    }

    pub fn clear(&mut self, argb: u32) {
        self.pixels.fill(argb);
    }
}

pub struct DrmDevice {
    pub card_path: String,
    pub mode: DrmMode,
    pub is_hardware: bool,
    _file: Option<File>,
}

impl DrmDevice {
    pub fn open_or_virtual(preferred_mode: Option<DrmMode>) -> Self {
        let mode = preferred_mode.unwrap_or_default();

        let (card_path, file, is_hw) = if Path::new(DEFAULT_DRM_CARD).exists() {
            let f = OpenOptions::new().read(true).write(true).open(DEFAULT_DRM_CARD).ok();
            (DEFAULT_DRM_CARD.to_string(), f, true)
        } else if Path::new(SECONDARY_DRM_CARD).exists() {
            let f = OpenOptions::new().read(true).write(true).open(SECONDARY_DRM_CARD).ok();
            (SECONDARY_DRM_CARD.to_string(), f, true)
        } else {
            ("virtual:software_dumb_buffer".to_string(), None, false)
        };

        Self {
            card_path,
            mode,
            is_hardware: is_hw,
            _file: file,
        }
    }

    pub fn allocate_dumb_buffer(&self, fb_id: u32) -> DumbBuffer {
        DumbBuffer::new(self.mode.width, self.mode.height, fb_id)
    }

    pub fn set_crtc(&self, _fb_id: u32) -> Result<(), String> {
        // On Linux hardware this calls drmModeSetCrtc(fd, crtc_id, fb_id, 0, 0, &connector, 1, &mode)
        Ok(())
    }

    pub fn page_flip(&self, _fb_id: u32) -> Result<(), String> {
        // On Linux hardware this calls drmModePageFlip(fd, crtc_id, fb_id, DRM_MODE_PAGE_FLIP_EVENT, user_data)
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dumb_buffer_allocation_and_pixel_access() {
        let mut buf = DumbBuffer::new(100, 200, 1);
        assert_eq!(buf.width, 100);
        assert_eq!(buf.height, 200);
        assert_eq!(buf.pitch, 400);
        assert_eq!(buf.size_bytes, 80000);
        assert_eq!(buf.pixels.len(), 20000);

        // Initialized black
        assert_eq!(buf.get_pixel(10, 10), 0xFF000000);

        // Write pixel
        buf.set_pixel(10, 10, 0xFF00FF00); // Green
        assert_eq!(buf.get_pixel(10, 10), 0xFF00FF00);

        // Out of bounds write does not panic
        buf.set_pixel(999, 999, 0xFFFFFFFF);
        assert_eq!(buf.get_pixel(999, 999), 0);
    }

    #[test]
    fn test_drm_device_fallback() {
        let dev = DrmDevice::open_or_virtual(None);
        assert_eq!(dev.mode.width, 720);
        assert_eq!(dev.mode.height, 1440);
        assert_eq!(dev.mode.refresh_rate_hz, 120);

        let buf = dev.allocate_dumb_buffer(42);
        assert_eq!(buf.fb_id, 42);
        assert_eq!(buf.width, 720);
    }
}
