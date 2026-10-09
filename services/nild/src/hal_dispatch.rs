// services/nild/src/hal_dispatch.rs — HAL Dispatcher
use nilhal::HalDevice;

pub struct HalDispatcher {
    #[allow(dead_code)]
    light: Option<HalDevice>,
}

impl HalDispatcher {
    pub fn init() -> Self {
        let light = HalDevice::load("light").ok();
        Self { light }
    }
}
