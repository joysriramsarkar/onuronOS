// runtime/nilui-gpu/src/renderer.rs — Batched 2D GPU & Software Renderer
use crate::compositor::PixelBuffer;
use crate::vkctx::VulkanContext;
use crate::atlas::GlyphAtlas;

pub struct Renderer2D {
    pub ctx: VulkanContext,
    pub atlas: GlyphAtlas,
    pub target: PixelBuffer,
}

impl Renderer2D {
    pub fn new() -> Result<Self, String> {
        let ctx = VulkanContext::new()?;
        let atlas = GlyphAtlas::new(1024, 1024);
        let target = PixelBuffer::new(720, 1440);
        Ok(Self { ctx, atlas, target })
    }

    pub fn draw_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, argb: u32) {
        if radius <= 0.0 {
            self.target.fill_rect(x as i32, y as i32, w.max(0.0) as u32, h.max(0.0) as u32, argb);
        } else {
            self.target.fill_rounded_rect(x as i32, y as i32, w.max(0.0) as u32, h.max(0.0) as u32, radius as u32, argb);
        }
    }

    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, argb: u32) {
        let scale = (size / 8.0).max(1.0) as usize;
        self.target.draw_text(text, x as i32, y as i32, scale, argb);
        self.atlas.shape_and_rasterize_bengali(text);
    }

    pub fn flush(&mut self) {
        // Submit command buffers to Vulkan graphics queue or KMS Dumb Buffer
    }
}
