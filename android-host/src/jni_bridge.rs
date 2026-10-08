// android-host/src/jni_bridge.rs — JNI Export layer for Android Host APK
// Implements the native methods declared by org.onuron.mobile.NativeBridge

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

use crate::bridge::HostToGuestEvent;
use crate::input::AndroidHostInput;

static BRIDGE_RUNNING: AtomicBool = AtomicBool::new(false);
static SURFACE_WIDTH: AtomicU32 = AtomicU32::new(1080);
static SURFACE_HEIGHT: AtomicU32 = AtomicU32::new(2340);

static GLOBAL_INPUT: Mutex<Option<AndroidHostInput>> = Mutex::new(None);

pub fn with_global_input<F, R>(f: F) -> R
where
    F: FnOnce(&mut AndroidHostInput) -> R,
{
    let mut lock = GLOBAL_INPUT.lock().unwrap();
    if lock.is_none() {
        *lock = Some(AndroidHostInput::new());
    }
    f(lock.as_mut().unwrap())
}

pub fn is_bridge_active() -> bool {
    BRIDGE_RUNNING.load(Ordering::SeqCst)
}

pub fn get_surface_dimensions() -> (u32, u32) {
    (
        SURFACE_WIDTH.load(Ordering::Relaxed),
        SURFACE_HEIGHT.load(Ordering::Relaxed),
    )
}

#[no_mangle]
pub unsafe extern "C" fn Java_org_onuron_mobile_NativeBridge_nativeSurfaceCreated(
    _env: *mut c_void,
    _class: *mut c_void,
    _surface: *mut c_void,
) {
    BRIDGE_RUNNING.store(true, Ordering::SeqCst);
    println!("[native_bridge] Surface created in host Android runtime");
}

#[no_mangle]
pub unsafe extern "C" fn Java_org_onuron_mobile_NativeBridge_nativeSurfaceChanged(
    _env: *mut c_void,
    _class: *mut c_void,
    _surface: *mut c_void,
    width: i32,
    height: i32,
) {
    if width > 0 && height > 0 {
        SURFACE_WIDTH.store(width as u32, Ordering::Relaxed);
        SURFACE_HEIGHT.store(height as u32, Ordering::Relaxed);
    }
    println!("[native_bridge] Surface resized to {}x{}", width, height);
}

#[no_mangle]
pub unsafe extern "C" fn Java_org_onuron_mobile_NativeBridge_nativeSurfaceDestroyed(
    _env: *mut c_void,
    _class: *mut c_void,
) {
    BRIDGE_RUNNING.store(false, Ordering::SeqCst);
    println!("[native_bridge] Surface destroyed in host Android runtime");
}

#[no_mangle]
pub unsafe extern "C" fn Java_org_onuron_mobile_NativeBridge_nativeHostTouchEvent(
    _env: *mut c_void,
    _class: *mut c_void,
    action: i32,
    pointer_id: i32,
    x: f32,
    y: f32,
    pressure: f32,
) {
    let action_str = match action {
        0 => "DOWN",
        1 => "UP",
        2 => "MOVE",
        3 => "CANCEL",
        _ => "OTHER",
    };

    let event = HostToGuestEvent::TouchEvent {
        action: action_str.to_string(),
        pointer_id: pointer_id as u32,
        x,
        y,
        pressure,
    };

    with_global_input(|input| {
        input.ingest_host_event(event);
    });
}

#[no_mangle]
pub unsafe extern "C" fn Java_org_onuron_mobile_NativeBridge_nativeHostKeyEvent(
    _env: *mut c_void,
    _class: *mut c_void,
    action: i32,
    keycode: i32,
    character: u16,
) {
    let action_str = match action {
        0 => "DOWN",
        _ => "UP",
    };

    let ch = if character != 0 {
        char::from_u32(character as u32)
    } else {
        None
    };

    let event = HostToGuestEvent::KeyEvent {
        action: action_str.to_string(),
        keycode: keycode as u32,
        character: ch,
    };

    with_global_input(|input| {
        input.ingest_host_event(event);
    });
}

#[no_mangle]
pub unsafe extern "C" fn Java_org_onuron_mobile_NativeBridge_nativeStartBridgeServer(
    _env: *mut c_void,
    _class: *mut c_void,
    _files_dir_path: *mut c_void,
) {
    BRIDGE_RUNNING.store(true, Ordering::SeqCst);
    println!("[native_bridge] Bridge server initialized and listening for IPC");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jni_touch_lifecycle() {
        unsafe {
            Java_org_onuron_mobile_NativeBridge_nativeSurfaceCreated(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            assert!(is_bridge_active());

            Java_org_onuron_mobile_NativeBridge_nativeSurfaceChanged(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1080,
                2400,
            );
            assert_eq!(get_surface_dimensions(), (1080, 2400));

            Java_org_onuron_mobile_NativeBridge_nativeHostTouchEvent(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0, // DOWN
                1,
                300.0,
                600.0,
                1.0,
            );

            with_global_input(|input| {
                use nilhal::traits::InputHal;
                let events = input.poll_events();
                assert_eq!(events.len(), 1);
            });

            Java_org_onuron_mobile_NativeBridge_nativeSurfaceDestroyed(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            assert!(!is_bridge_active());
        }
    }
}
