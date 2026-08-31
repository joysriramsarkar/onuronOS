// runtime/nilui-gpu/src/lib.rs — Public C-ABI exports for nilshell
pub mod vkctx;
pub mod atlas;
pub mod renderer;
pub mod present;

pub use renderer::Renderer2D;
pub use present::KmsPresenter;

#[no_mangle]
pub extern "C" fn nilgpu_init() -> *mut Renderer2D {
    match Renderer2D::new() {
        Ok(r) => Box::into_raw(Box::new(r)),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn nilgpu_rect(r: *mut Renderer2D, x: f32, y: f32, w: f32, h: f32, rad: f32, color: u32) {
    if !r.is_null() {
        unsafe { (*r).draw_rect(x, y, w, h, rad, color); }
    }
}

#[no_mangle]
pub extern "C" fn nilgpu_flush(r: *mut Renderer2D) {
    if !r.is_null() {
        unsafe { (*r).flush(); }
    }
}
