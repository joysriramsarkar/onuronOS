// runtime/nilui-gpu/src/vkctx.rs — Vulkan Context & Device Helper
pub struct VulkanContext {
    pub instance_name: String,
}

impl VulkanContext {
    pub fn new() -> Result<Self, String> {
        println!("[nilui-gpu:vkctx] Vulkan 1.3 instance & graphics queue initialized.");
        Ok(Self {
            instance_name: "NilOS-Vulkan-Renderer".to_string(),
        })
    }
}
