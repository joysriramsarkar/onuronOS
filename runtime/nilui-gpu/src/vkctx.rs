// runtime/nilui-gpu/src/vkctx.rs — Vulkan Context & Device Helper
pub struct VulkanContext {
    pub instance_name: String,
}

impl VulkanContext {
    pub fn new() -> Result<Self, String> {
        // STUB/SIMULATED previously: printed "initialized" and returned Ok
        // without creating any Vulkan instance/device/queue. Callers
        // (`Renderer2D::new` via `?`, and `nilgpu_init` which maps Err to
        // null) already handle failure, so report it honestly.
        Err(
            "Vulkan not implemented: nilui-gpu creates no Vulkan instance/device/queue (STUB/SIMULATED); use the software PixelBuffer/DRM present path"
                .to_string(),
        )
    }
}
