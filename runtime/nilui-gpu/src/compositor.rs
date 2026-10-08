// runtime/nilui-gpu/src/compositor.rs — 2D Pixel Canvas & Direct Screen Compositor
//
// Provides high-performance, memory-safe pixel operations, primitive shape rasterization,
// alpha blending, bitmap text rendering, and surface blitting directly onto DRM dumb buffers.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct PixelBuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>, // ARGB8888 (0xAARRGGBB)
}

impl PixelBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let count = (width * height) as usize;
        Self {
            width,
            height,
            pixels: vec![0xFF000000; count],
        }
    }

    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && (x as u32) < self.width && y >= 0 && (y as u32) < self.height {
            let idx = (y as u32 * self.width + x as u32) as usize;
            self.pixels[idx] = color;
        }
    }

    #[inline]
    pub fn get_pixel(&self, x: i32, y: i32) -> u32 {
        if x >= 0 && (x as u32) < self.width && y >= 0 && (y as u32) < self.height {
            let idx = (y as u32 * self.width + x as u32) as usize;
            self.pixels[idx]
        } else {
            0
        }
    }

    /// Fill a solid rectangle with clipping
    pub fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: u32) {
        let x0 = x.max(0) as u32;
        let y0 = y.max(0) as u32;
        let x1 = ((x + w as i32).min(self.width as i32)).max(0) as u32;
        let y1 = ((y + h as i32).min(self.height as i32)).max(0) as u32;

        for row in y0..y1 {
            let start = (row * self.width + x0) as usize;
            let end = (row * self.width + x1) as usize;
            if start < end && end <= self.pixels.len() {
                self.pixels[start..end].fill(color);
            }
        }
    }

    /// Fill a rounded rectangle
    pub fn fill_rounded_rect(&mut self, x: i32, y: i32, w: u32, h: u32, radius: u32, color: u32) {
        let r = radius.min(w / 2).min(h / 2);
        // Central body
        self.fill_rect(x, y + r as i32, w, h.saturating_sub(r * 2), color);
        // Top and bottom bars
        self.fill_rect(x + r as i32, y, w.saturating_sub(r * 2), r, color);
        self.fill_rect(x + r as i32, y + (h - r) as i32, w.saturating_sub(r * 2), r, color);

        // 4 corner circles
        self.fill_circle_corner(x + r as i32, y + r as i32, r, 0, color);
        self.fill_circle_corner(x + (w - r) as i32, y + r as i32, r, 1, color);
        self.fill_circle_corner(x + r as i32, y + (h - r) as i32, r, 2, color);
        self.fill_circle_corner(x + (w - r) as i32, y + (h - r) as i32, r, 3, color);
    }

    fn fill_circle_corner(&mut self, cx: i32, cy: i32, r: u32, corner: u8, color: u32) {
        let r_i = r as i32;
        for dy in 0..=r_i {
            for dx in 0..=r_i {
                if dx * dx + dy * dy <= r_i * r_i {
                    let (px, py) = match corner {
                        0 => (cx - dx, cy - dy), // Top-left
                        1 => (cx + dx - 1, cy - dy), // Top-right
                        2 => (cx - dx, cy + dy - 1), // Bottom-left
                        _ => (cx + dx - 1, cy + dy - 1), // Bottom-right
                    };
                    self.set_pixel(px, py, color);
                }
            }
        }
    }

    /// Blit another PixelBuffer onto this buffer at (dst_x, dst_y)
    pub fn blit(&mut self, src: &PixelBuffer, dst_x: i32, dst_y: i32) {
        for sy in 0..src.height {
            let dy = dst_y + sy as i32;
            if dy < 0 || dy >= self.height as i32 {
                continue;
            }
            for sx in 0..src.width {
                let dx = dst_x + sx as i32;
                if dx < 0 || dx >= self.width as i32 {
                    continue;
                }
                let src_idx = (sy * src.width + sx) as usize;
                let color = src.pixels[src_idx];
                // Simple alpha test (0x00 is transparent)
                if (color >> 24) != 0 {
                    self.set_pixel(dx, dy, color);
                }
            }
        }
    }

    /// Draw clean 8x8 bitmap text onto the buffer
    pub fn draw_text(&mut self, text: &str, x: i32, y: i32, scale: usize, color: u32) {
        let mut cur_x = x;
        let s = scale.max(1);

        for ch in text.chars() {
            let glyph = get_char_glyph(ch);
            for (row_idx, row_byte) in glyph.iter().enumerate() {
                for col_idx in 0..8 {
                    if (row_byte & (1 << (7 - col_idx))) != 0 {
                        for sy in 0..s {
                            for sx in 0..s {
                                let px = cur_x + (col_idx * s + sx) as i32;
                                let py = y + (row_idx * s + sy) as i32;
                                self.set_pixel(px, py, color);
                            }
                        }
                    }
                }
            }
            cur_x += (8 * s + 1) as i32;
        }
    }
}

/// Minimal 8x8 font glyphs for ASCII numbers, uppercase, and basic punctuation
fn get_char_glyph(c: char) -> [u8; 8] {
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
        'K' | 'k' => [0x66, 0x6C, 0x78, 0x70, 0x78, 0x6C, 0x66, 0x00],
        'L' | 'l' => [0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x00],
        'M' | 'm' => [0x63, 0x77, 0x7F, 0x6B, 0x63, 0x63, 0x63, 0x00],
        'N' | 'n' => [0x66, 0x76, 0x7E, 0x7E, 0x6E, 0x66, 0x66, 0x00],
        'O' | 'o' => [0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00],
        'P' | 'p' => [0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x00],
        'R' | 'r' => [0x7C, 0x66, 0x66, 0x7C, 0x78, 0x6C, 0x66, 0x00],
        'S' | 's' => [0x3C, 0x66, 0x60, 0x3C, 0x06, 0x66, 0x3C, 0x00],
        'T' | 't' => [0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00],
        'U' | 'u' => [0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00],
        'V' | 'v' => [0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00],
        'W' | 'w' => [0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x00],
        'X' | 'x' => [0x66, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x66, 0x00],
        'Y' | 'y' => [0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x00],
        'Z' | 'z' => [0x7E, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x7E, 0x00],
        ':' => [0x00, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00],
        '%' => [0x62, 0x64, 0x08, 0x10, 0x20, 0x26, 0x46, 0x00],
        '-' => [0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00],
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        _   => [0x7E, 0x42, 0x42, 0x42, 0x42, 0x42, 0x7E, 0x00], // Box fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_buffer_fill_and_clipping() {
        let mut buf = PixelBuffer::new(20, 20);
        buf.fill_rect(5, 5, 10, 10, 0xFFFF0000); // Red box

        assert_eq!(buf.get_pixel(4, 4), 0xFF000000);
        assert_eq!(buf.get_pixel(5, 5), 0xFFFF0000);
        assert_eq!(buf.get_pixel(14, 14), 0xFFFF0000);
        assert_eq!(buf.get_pixel(15, 15), 0xFF000000);
    }

    #[test]
    fn test_pixel_buffer_blit() {
        let mut dest = PixelBuffer::new(40, 40);
        let mut src = PixelBuffer::new(10, 10);
        src.fill_rect(0, 0, 10, 10, 0xFF00FF00); // Green

        dest.blit(&src, 15, 15);
        assert_eq!(dest.get_pixel(14, 14), 0xFF000000);
        assert_eq!(dest.get_pixel(15, 15), 0xFF00FF00);
        assert_eq!(dest.get_pixel(24, 24), 0xFF00FF00);
        assert_eq!(dest.get_pixel(25, 25), 0xFF000000);
    }

    #[test]
    fn test_draw_text() {
        let mut buf = PixelBuffer::new(50, 20);
        buf.draw_text("12:45", 2, 2, 1, 0xFFFFFFFF);
        // Assert that some pixels were set to white
        let non_black_count = buf.pixels.iter().filter(|&&p| p == 0xFFFFFFFF).count();
        assert!(non_black_count > 20);
    }
}
