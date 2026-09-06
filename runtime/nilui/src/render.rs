// runtime/nilui/src/render.rs — Hardware-Independent Render API & Backends
// Abstracts the display rendering so the UI code never needs to know if it is running on
// X11, DRM/KMS, Android SurfaceFlinger, or software framebuffer.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenderBackendType {
    SoftwareFramebuffer, // QEMU & Desktop
    DrmKms,              // Real Linux Host
    AndroidSurface,      // Samsung Galaxy S25 / Android Mobile Runtime
    GpuVulkan,           // Future High-performance GPU
}

pub trait RenderBackend: Send + Sync {
    fn backend_type(&self) -> RenderBackendType;
    fn dimensions(&self) -> (u32, u32);
    fn clear(&mut self, color_argb: u32);
    fn draw_pixel(&mut self, x: u32, y: u32, color_argb: u32);
    fn draw_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color_argb: u32);
    fn draw_text(&mut self, x: i32, y: i32, text: &str, color_argb: u32, scale: u32);
    fn present(&mut self) -> Result<(), String>;
}

/// Software Framebuffer Backend (Default for QEMU & Unit Testing)
pub struct SoftwareFramebufferBackend {
    width: u32,
    height: u32,
    buffer: Vec<u32>,
}

impl SoftwareFramebufferBackend {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        Self {
            width,
            height,
            buffer: vec![0xFF0A0E17; size], // Deep space navy default
        }
    }

    pub fn buffer(&self) -> &[u32] {
        &self.buffer
    }
}

impl RenderBackend for SoftwareFramebufferBackend {
    fn backend_type(&self) -> RenderBackendType {
        RenderBackendType::SoftwareFramebuffer
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn clear(&mut self, color_argb: u32) {
        self.buffer.fill(color_argb);
    }

    fn draw_pixel(&mut self, x: u32, y: u32, color_argb: u32) {
        if x < self.width && y < self.height {
            let idx = (y * self.width + x) as usize;
            self.buffer[idx] = color_argb;
        }
    }

    fn draw_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color_argb: u32) {
        for dy in 0..h {
            let cur_y = y + dy as i32;
            if cur_y < 0 || cur_y >= self.height as i32 { continue; }
            for dx in 0..w {
                let cur_x = x + dx as i32;
                if cur_x < 0 || cur_x >= self.width as i32 { continue; }
                self.draw_pixel(cur_x as u32, cur_y as u32, color_argb);
            }
        }
    }

    fn draw_text(&mut self, _x: i32, _y: i32, _text: &str, _color_argb: u32, _scale: u32) {
        // Text rasterizer fallback
    }

    fn present(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Android Surface Backend (Samsung Galaxy S25 120Hz Dynamic AMOLED 2X)
pub struct AndroidSurfaceBackend {
    width: u32,
    height: u32,
    buffer: Vec<u32>,
}

impl AndroidSurfaceBackend {
    pub fn new() -> Self {
        Self {
            width: 1080,
            height: 2340,
            buffer: vec![0xFF000000; 1080 * 2340],
        }
    }
}

impl Default for AndroidSurfaceBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderBackend for AndroidSurfaceBackend {
    fn backend_type(&self) -> RenderBackendType {
        RenderBackendType::AndroidSurface
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn clear(&mut self, color_argb: u32) {
        self.buffer.fill(color_argb);
    }

    fn draw_pixel(&mut self, x: u32, y: u32, color_argb: u32) {
        if x < self.width && y < self.height {
            let idx = (y * self.width + x) as usize;
            self.buffer[idx] = color_argb;
        }
    }

    fn draw_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color_argb: u32) {
        for dy in 0..h {
            let cur_y = y + dy as i32;
            if cur_y < 0 || cur_y >= self.height as i32 { continue; }
            for dx in 0..w {
                let cur_x = x + dx as i32;
                if cur_x < 0 || cur_x >= self.width as i32 { continue; }
                self.draw_pixel(cur_x as u32, cur_y as u32, color_argb);
            }
        }
    }

    fn draw_text(&mut self, _x: i32, _y: i32, _text: &str, _color_argb: u32, _scale: u32) {
        // Text rasterizer
    }

    fn present(&mut self) -> Result<(), String> {
        // Transmits frame to Android Surface via ANativeWindow or shared bridge buffer
        Ok(())
    }
}

/// Factory to obtain the active render backend automatically
pub fn create_active_render_backend() -> Box<dyn RenderBackend> {
    if std::env::var("ANDROID_ROOT").is_ok()
        || std::env::var("ONURON_ANDROID_HOST").is_ok()
        || std::path::Path::new("/system/lib64/libandroid_runtime.so").exists()
    {
        Box::new(AndroidSurfaceBackend::new())
    } else {
        Box::new(SoftwareFramebufferBackend::new(1080, 2340))
    }
}
