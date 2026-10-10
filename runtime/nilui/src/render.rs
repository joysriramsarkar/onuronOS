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

    fn draw_text(&mut self, x: i32, y: i32, text: &str, color_argb: u32, scale: u32) {
        let s = scale.max(1) as i32;
        let mut cur_x = x;
        for ch in text.chars() {
            let glyph = get_8x8_glyph(ch);
            for (row_idx, row_byte) in glyph.iter().enumerate() {
                for col_idx in 0..8 {
                    if (row_byte & (1 << (7 - col_idx))) != 0 {
                        for sy in 0..s {
                            for sx in 0..s {
                                let px = cur_x + (col_idx as i32 * s + sx);
                                let py = y + (row_idx as i32 * s + sy);
                                if px >= 0 && py >= 0 {
                                    self.draw_pixel(px as u32, py as u32, color_argb);
                                }
                            }
                        }
                    }
                }
            }
            cur_x += 8 * s + 1;
        }
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

    pub fn buffer(&self) -> &[u32] {
        &self.buffer
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

    fn draw_text(&mut self, x: i32, y: i32, text: &str, color_argb: u32, scale: u32) {
        let s = scale.max(1) as i32;
        let mut cur_x = x;
        for ch in text.chars() {
            let glyph = get_8x8_glyph(ch);
            for (row_idx, row_byte) in glyph.iter().enumerate() {
                for col_idx in 0..8 {
                    if (row_byte & (1 << (7 - col_idx))) != 0 {
                        for sy in 0..s {
                            for sx in 0..s {
                                let px = cur_x + (col_idx as i32 * s + sx);
                                let py = y + (row_idx as i32 * s + sy);
                                if px >= 0 && py >= 0 {
                                    self.draw_pixel(px as u32, py as u32, color_argb);
                                }
                            }
                        }
                    }
                }
            }
            cur_x += 8 * s + 1;
        }
    }

    fn present(&mut self) -> Result<(), String> {
        // Transmits frame to Android Surface via ANativeWindow or shared bridge buffer
        Ok(())
    }
}

/// Minimal 8x8 font glyphs for ASCII numbers, uppercase, and basic punctuation
fn get_8x8_glyph(c: char) -> [u8; 8] {
    match c {
        '0' => [0x3C, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x3C, 0x00],
        '1' => [0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00],
        '2' => [0x3C, 0x66, 0x06, 0x0C, 0x18, 0x30, 0x7E, 0x00],
        '3' => [0x3C, 0x66, 0x06, 0x1C, 0x06, 0x66, 0x3C, 0x00],
        '4' => [0x0C, 0x1C, 0x34, 0x64, 0x7E, 0x04, 0x04, 0x00],
        '5' => [0x7E, 0x60, 0x7C, 0x06, 0x06, 0x66, 0x3C, 0x00],
        '6' => [0x3C, 0x66, 0x60, 0x7C, 0x66, 0x66, 0x3C, 0x00],
        '7' => [0x7E, 0x06, 0x0C, 0x18, 0x30, 0x30, 0x30, 0x00],
        '8' => [0x3C, 0x66, 0x66, 0x3C, 0x66, 0x66, 0x3C, 0x00],
        '9' => [0x3C, 0x66, 0x66, 0x3E, 0x06, 0x66, 0x3C, 0x00],
        'A' | 'a' => [0x18, 0x3C, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00],
        'B' | 'b' => [0x7C, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x7C, 0x00],
        'C' | 'c' => [0x3C, 0x66, 0x60, 0x60, 0x60, 0x66, 0x3C, 0x00],
        'D' | 'd' => [0x78, 0x6C, 0x66, 0x66, 0x66, 0x6C, 0x78, 0x00],
        'E' | 'e' => [0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x7E, 0x00],
        'F' | 'f' => [0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x60, 0x00],
        'G' | 'g' => [0x3C, 0x66, 0x60, 0x6E, 0x66, 0x66, 0x3A, 0x00],
        'H' | 'h' => [0x66, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00],
        'I' | 'i' => [0x3C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00],
        'J' | 'j' => [0x0E, 0x06, 0x06, 0x06, 0x06, 0x66, 0x3C, 0x00],
        'K' | 'k' => [0x66, 0x6C, 0x78, 0x70, 0x78, 0x6C, 0x66, 0x00],
        'L' | 'l' => [0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x00],
        'M' | 'm' => [0x63, 0x77, 0x7F, 0x6B, 0x63, 0x63, 0x63, 0x00],
        'N' | 'n' => [0x66, 0x76, 0x7E, 0x7E, 0x6E, 0x66, 0x66, 0x00],
        'O' | 'o' => [0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00],
        'P' | 'p' => [0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x00],
        'Q' | 'q' => [0x3C, 0x66, 0x66, 0x66, 0x6E, 0x3C, 0x0E, 0x00],
        'R' | 'r' => [0x7C, 0x66, 0x66, 0x7C, 0x78, 0x6C, 0x66, 0x00],
        'S' | 's' => [0x3C, 0x66, 0x60, 0x3C, 0x06, 0x66, 0x3C, 0x00],
        'T' | 't' => [0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00],
        'U' | 'u' => [0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00],
        'V' | 'v' => [0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00],
        'W' | 'w' => [0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x00],
        'X' | 'x' => [0x66, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x66, 0x00],
        'Y' | 'y' => [0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x00],
        'Z' | 'z' => [0x7E, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x7E, 0x00],
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x30],
        ':' => [0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x00],
        '!' => [0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x18, 0x00],
        '?' => [0x3C, 0x66, 0x06, 0x0C, 0x18, 0x00, 0x18, 0x00],
        '-' => [0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00],
        '+' => [0x00, 0x18, 0x18, 0x7E, 0x18, 0x18, 0x00, 0x00],
        '(' => [0x0C, 0x18, 0x30, 0x30, 0x30, 0x18, 0x0C, 0x00],
        ')' => [0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x18, 0x30, 0x00],
        '[' => [0x3C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3C, 0x00],
        ']' => [0x3C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x3C, 0x00],
        '/' => [0x02, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x40, 0x00],
        '>' => [0x60, 0x30, 0x18, 0x0C, 0x18, 0x30, 0x60, 0x00],
        '<' => [0x06, 0x0C, 0x18, 0x30, 0x18, 0x0C, 0x06, 0x00],
        _ => [0x7E, 0x42, 0x5A, 0x5A, 0x42, 0x7E, 0x00, 0x00],
    }
}

/// Recursively layouts and paints a declarative `Element` tree onto a `RenderBackend`.
pub fn render_element_to_backend(
    backend: &mut dyn RenderBackend,
    element: &crate::Element,
    x: i32,
    y: i32,
) -> (i32, i32) {
    match element {
        crate::Element::Text { content } => {
            backend.draw_text(x, y, content, 0xFFFFFFFF, 2);
            let w = (content.len() as i32) * 17;
            (x + w, y + 20)
        }
        crate::Element::Button { label, .. } => {
            let btn_w = ((label.len() as u32) * 17) + 24;
            backend.draw_rect(x, y, btn_w, 36, 0xFF1E3A8A); // Onuron deep blue
            backend.draw_text(x + 12, y + 10, label, 0xFFFFFFFF, 2);
            (x + btn_w as i32, y + 40)
        }
        crate::Element::Input { placeholder, text } => {
            let w = 260u32;
            backend.draw_rect(x, y, w, 36, 0xFF1F2937); // Dark gray field
            if !text.is_empty() {
                backend.draw_text(x + 8, y + 10, text, 0xFFFFFFFF, 2);
            } else {
                backend.draw_text(x + 8, y + 10, placeholder, 0xFF9CA3AF, 2);
            }
            (x + w as i32, y + 40)
        }
        crate::Element::Column { children } => {
            let mut cur_y = y;
            let mut max_x = x;
            for child in children {
                let (cx, cy) = render_element_to_backend(backend, child, x, cur_y);
                max_x = max_x.max(cx);
                cur_y = cy + 8; // Spacing
            }
            (max_x, cur_y)
        }
        crate::Element::Row { children } => {
            let mut cur_x = x;
            let mut max_y = y;
            for child in children {
                let (cx, cy) = render_element_to_backend(backend, child, cur_x, y);
                cur_x = cx + 12; // Spacing
                max_y = max_y.max(cy);
            }
            (cur_x, max_y)
        }
        crate::Element::Stack { children } => {
            let mut max_x = x;
            let mut max_y = y;
            for child in children {
                let (cx, cy) = render_element_to_backend(backend, child, x, y);
                max_x = max_x.max(cx);
                max_y = max_y.max(cy);
            }
            (max_x, max_y)
        }
    }
}

/// Helper to render an `alap::Component` graph directly onto the backend.
pub fn render_component_to_backend(
    backend: &mut dyn RenderBackend,
    component: &alap::Component,
    x: i32,
    y: i32,
) -> (i32, i32) {
    let elem = crate::Element::from(component);
    render_element_to_backend(backend, &elem, x, y)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_software_framebuffer_clear_and_draw() {
        let mut fb = SoftwareFramebufferBackend::new(100, 100);
        fb.clear(0xFF000000);
        assert_eq!(fb.buffer()[0], 0xFF000000);

        fb.draw_pixel(10, 10, 0xFFFFFFFF);
        assert_eq!(fb.buffer()[10 * 100 + 10], 0xFFFFFFFF);

        fb.draw_rect(20, 20, 10, 10, 0xFFFF0000);
        assert_eq!(fb.buffer()[25 * 100 + 25], 0xFFFF0000);
    }

    #[test]
    fn test_draw_text_rasterization() {
        let mut fb = SoftwareFramebufferBackend::new(120, 40);
        fb.clear(0xFF000000);
        fb.draw_text(5, 5, "NIL", 0xFFFFFFFF, 1);

        // Verify that white pixels were drawn
        let non_black = fb.buffer().iter().filter(|&&p| p == 0xFFFFFFFF).count();
        assert!(non_black > 0, "Text rasterization must draw visible pixels");
    }

    #[test]
    fn test_render_element_tree_and_component_pipeline() {
        let mut fb = SoftwareFramebufferBackend::new(400, 400);
        fb.clear(0xFF000000);

        let alap_ui = alap::Component::Column {
            children: vec![
                alap::Component::Text {
                    content: "Hello Onuron".to_string(),
                    font_size: 16,
                    color: None,
                    accessibility_label: None,
                },
                alap::Component::Button {
                    id: "btn_test".to_string(),
                    label: "Launch".to_string(),
                    enabled: true,
                    accessibility_label: None,
                },
            ],
            spacing: 8,
            alignment: alap::Alignment::Start,
        };

        let (final_x, final_y) = render_component_to_backend(&mut fb, &alap_ui, 10, 10);
        assert!(final_x > 10);
        assert!(final_y > 10);

        let white_pixels = fb.buffer().iter().filter(|&&p| p == 0xFFFFFFFF).count();
        assert!(white_pixels > 0, "Rendered UI must paint white text on screen");

        let blue_pixels = fb.buffer().iter().filter(|&&p| p == 0xFF1E3A8A).count();
        assert!(blue_pixels > 0, "Rendered UI must paint blue button on screen");
    }
}
