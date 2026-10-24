// runtime/nilui-gpu/src/lib.rs — Onuron OS Direct DRM/KMS Compositor & GPU Engine
pub mod atlas;
pub mod compositor;
pub mod drm;
pub mod drm_uapi;
pub mod ioctl;
pub mod present;
pub mod renderer;
pub mod touch;
pub mod vkctx;
pub mod wayland;

pub use compositor::{PixelBuffer, Rect};
pub use drm::{connector_type_name, DisplayInfo, DrmBackend, DrmDevice, DrmMode, DumbBuffer};
pub use present::{KmsPresenter, PresentationStats};
pub use renderer::Renderer2D;
pub use touch::{AccessibilityRole, MobileScreen, SwipeGesture, TouchCompositor, TouchTarget};

#[no_mangle]
pub extern "C" fn nilgpu_init() -> *mut Renderer2D {
    match Renderer2D::new() {
        Ok(r) => Box::into_raw(Box::new(r)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// # Safety
/// `r` must be either null or a valid pointer returned by `nilgpu_init`.
#[no_mangle]
pub unsafe extern "C" fn nilgpu_rect(
    r: *mut Renderer2D,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    rad: f32,
    color: u32,
) {
    if !r.is_null() {
        unsafe {
            (*r).draw_rect(x, y, w, h, rad, color);
        }
    }
}

/// # Safety
/// `r` must be either null or a valid pointer returned by `nilgpu_init`.
#[no_mangle]
pub unsafe extern "C" fn nilgpu_flush(r: *mut Renderer2D) {
    if !r.is_null() {
        unsafe {
            (*r).flush();
        }
    }
}
