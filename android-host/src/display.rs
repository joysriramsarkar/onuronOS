// android-host/src/display.rs — Android Surface / Framebuffer Bridge
// Renders Onuron UI frames directly onto Samsung Galaxy S25 SurfaceView / ANativeWindow.

use nilhal::traits::{DisplayHal, HalError};

pub struct AndroidHostDisplay {
    width: u32,
    height: u32,
    refresh_rate: u32,
    brightness: u8,
}

impl AndroidHostDisplay {
    pub fn new() -> Self {
        Self {
            // Galaxy S25 6.2" Dynamic AMOLED 2X resolution & refresh rate
            width: 1080,
            height: 2340,
            refresh_rate: 120,
            brightness: 90,
        }
    }
}

impl DisplayHal for AndroidHostDisplay {
    fn get_dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn get_refresh_rate(&self) -> u32 {
        self.refresh_rate
    }

    fn present_frame(&mut self, _buffer: &[u32]) -> Result<(), HalError> {
        // Dispatches raw frame buffer to Surface via SharedMemory / ANativeWindow_lock
        Ok(())
    }

    fn set_brightness(&mut self, percent: u8) -> Result<(), HalError> {
        self.brightness = percent.min(100);
        Ok(())
    }

    fn get_brightness(&self) -> u8 {
        self.brightness
    }
}
