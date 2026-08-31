// runtime/nilui-gpu/src/renderer.rs — Batched 2D GPU Renderer
use crate::vkctx::VulkanContext;
use crate::atlas::GlyphAtlas;

pub struct Renderer2D {
    ctx: VulkanContext,
    atlas: GlyphAtlas,
}

impl Renderer2D {
    pub fn new() -> Result<Self, String> {
        let ctx = VulkanContext::new()?;
        let atlas = GlyphAtlas::new(1024, 1024);
        Ok(Self { ctx, atlas })
    }

    pub fn draw_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, argb: u32) {
        // Batched rounded box draw command
    }

    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, argb: u32) {
        self.atlas.shape_and_rasterize_bengali(text);
    }

    pub fn flush(&mut self) {
        // Submit command buffers to Vulkan graphics queue
    }
}
