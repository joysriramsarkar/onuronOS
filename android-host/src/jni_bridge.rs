// android-host/src/jni_bridge.rs — JNI Export layer for Android Host APK
// Implements the native methods declared by org.onuron.mobile.NativeBridge
// Provides shared thread-safe buffers & queues connecting Rust NilHAL to Android Host.

use std::collections::VecDeque;
use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;

use crate::bridge::{GuestToHostCommand, HostToGuestEvent};
use crate::input::AndroidHostInput;

static BRIDGE_RUNNING: AtomicBool = AtomicBool::new(false);
static SURFACE_WIDTH: AtomicU32 = AtomicU32::new(1080);
static SURFACE_HEIGHT: AtomicU32 = AtomicU32::new(2340);
static FRAME_COUNTER: AtomicU64 = AtomicU64::new(0);

static GLOBAL_INPUT: Mutex<Option<AndroidHostInput>> = Mutex::new(None);
static GLOBAL_FRAME_BUFFER: Mutex<Option<Vec<u32>>> = Mutex::new(None);
static GLOBAL_COMMAND_QUEUE: Mutex<VecDeque<GuestToHostCommand>> = Mutex::new(VecDeque::new());
static GLOBAL_EVENT_QUEUE: Mutex<VecDeque<HostToGuestEvent>> = Mutex::new(VecDeque::new());
static GLOBAL_CAMERA_FRAMES: Mutex<VecDeque<Vec<u8>>> = Mutex::new(VecDeque::new());
static GLOBAL_AUDIO_PLAYBACK: Mutex<VecDeque<i16>> = Mutex::new(VecDeque::new());
static GLOBAL_AUDIO_RECORD: Mutex<VecDeque<i16>> = Mutex::new(VecDeque::new());

// ─── Display Frame API ────────────────────────────────────────────────────────
pub fn publish_display_frame(pixels: &[u32], width: u32, height: u32) {
    if width > 0 && height > 0 {
        SURFACE_WIDTH.store(width, Ordering::Relaxed);
        SURFACE_HEIGHT.store(height, Ordering::Relaxed);
    }
    if let Ok(mut lock) = GLOBAL_FRAME_BUFFER.lock() {
        *lock = Some(pixels.to_vec());
    }
    FRAME_COUNTER.fetch_add(1, Ordering::SeqCst);
}

pub fn get_latest_frame() -> Option<Vec<u32>> {
    if let Ok(lock) = GLOBAL_FRAME_BUFFER.lock() {
        lock.clone()
    } else {
        None
    }
}

pub fn copy_latest_frame_to_slice(out: &mut [u32]) -> usize {
    if let Ok(lock) = GLOBAL_FRAME_BUFFER.lock() {
        if let Some(ref frame) = *lock {
            let to_copy = frame.len().min(out.len());
            out[..to_copy].copy_from_slice(&frame[..to_copy]);
            return to_copy;
        }
    }
    0
}

pub fn get_frame_count() -> u64 {
    FRAME_COUNTER.load(Ordering::Relaxed)
}

// ─── Command Queue (Guest -> Host) ────────────────────────────────────────────
pub fn enqueue_guest_command(cmd: GuestToHostCommand) {
    if let Ok(mut lock) = GLOBAL_COMMAND_QUEUE.lock() {
        lock.push_back(cmd);
    }
}

pub fn poll_guest_command() -> Option<GuestToHostCommand> {
    if let Ok(mut lock) = GLOBAL_COMMAND_QUEUE.lock() {
        lock.pop_front()
    } else {
        None
    }
}

pub fn pending_guest_command_count() -> usize {
    if let Ok(lock) = GLOBAL_COMMAND_QUEUE.lock() {
        lock.len()
    } else {
        0
    }
}

// ─── Event Ingestion (Host -> Guest) ──────────────────────────────────────────
pub fn push_host_event(event: HostToGuestEvent) {
    with_global_input(|input| {
        input.ingest_host_event(event.clone());
    });
    if let Ok(mut lock) = GLOBAL_EVENT_QUEUE.lock() {
        lock.push_back(event);
    }
}

pub fn poll_host_event() -> Option<HostToGuestEvent> {
    if let Ok(mut lock) = GLOBAL_EVENT_QUEUE.lock() {
        lock.pop_front()
    } else {
        None
    }
}

// ─── Camera Frame Queue ───────────────────────────────────────────────────────
pub fn push_camera_frame(jpeg_bytes: Vec<u8>) {
    if let Ok(mut lock) = GLOBAL_CAMERA_FRAMES.lock() {
        if lock.len() >= 8 {
            lock.pop_front();
        }
        lock.push_back(jpeg_bytes);
    }
}

pub fn pop_camera_frame() -> Option<Vec<u8>> {
    if let Ok(mut lock) = GLOBAL_CAMERA_FRAMES.lock() {
        lock.pop_front()
    } else {
        None
    }
}

// ─── Audio Ring Buffers ───────────────────────────────────────────────────────
const MAX_AUDIO_QUEUE_SAMPLES: usize = 96_000; // ~2 seconds of 48kHz mono

pub fn push_audio_playback(samples: &[i16]) {
    if let Ok(mut lock) = GLOBAL_AUDIO_PLAYBACK.lock() {
        for &s in samples {
            if lock.len() >= MAX_AUDIO_QUEUE_SAMPLES {
                lock.pop_front();
            }
            lock.push_back(s);
        }
    }
}

pub fn pull_audio_playback(max_samples: usize) -> Vec<i16> {
    if let Ok(mut lock) = GLOBAL_AUDIO_PLAYBACK.lock() {
        let count = lock.len().min(max_samples);
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            if let Some(s) = lock.pop_front() {
                out.push(s);
            }
        }
        out
    } else {
        Vec::new()
    }
}

pub fn push_audio_record(samples: &[i16]) {
    if let Ok(mut lock) = GLOBAL_AUDIO_RECORD.lock() {
        for &s in samples {
            if lock.len() >= MAX_AUDIO_QUEUE_SAMPLES {
                lock.pop_front();
            }
            lock.push_back(s);
        }
    }
}

pub fn pull_audio_record(buf: &mut [i16]) -> usize {
    if let Ok(mut lock) = GLOBAL_AUDIO_RECORD.lock() {
        let count = lock.len().min(buf.len());
        for i in 0..count {
            if let Some(s) = lock.pop_front() {
                buf[i] = s;
            }
        }
        count
    } else {
        0
    }
}

// ─── Global Input Access ──────────────────────────────────────────────────────
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

// ─── Exported JNI Functions ───────────────────────────────────────────────────

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

    push_host_event(event);
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

    push_host_event(event);
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

#[no_mangle]
pub unsafe extern "C" fn Java_org_onuron_mobile_NativeBridge_nativePollGuestCommand(
    _env: *mut c_void,
    _class: *mut c_void,
) -> *const c_char {
    if let Some(cmd) = poll_guest_command() {
        if let Ok(json) = serde_json::to_string(&cmd) {
            if let Ok(c_str) = CString::new(json) {
                return c_str.into_raw();
            }
        }
    }
    std::ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn Java_org_onuron_mobile_NativeBridge_nativeFreeCommandString(
    _env: *mut c_void,
    _class: *mut c_void,
    ptr: *mut c_char,
) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

#[no_mangle]
pub unsafe extern "C" fn Java_org_onuron_mobile_NativeBridge_nativePushCameraFrame(
    _env: *mut c_void,
    _class: *mut c_void,
    buffer: *const u8,
    len: i32,
) {
    if !buffer.is_null() && len > 0 {
        let slice = std::slice::from_raw_parts(buffer, len as usize);
        push_camera_frame(slice.to_vec());
    }
}

#[no_mangle]
pub unsafe extern "C" fn Java_org_onuron_mobile_NativeBridge_nativePushAudioSamples(
    _env: *mut c_void,
    _class: *mut c_void,
    buffer: *const i16,
    len: i32,
) {
    if !buffer.is_null() && len > 0 {
        let slice = std::slice::from_raw_parts(buffer, len as usize);
        push_audio_record(slice);
    }
}

#[no_mangle]
pub unsafe extern "C" fn Java_org_onuron_mobile_NativeBridge_nativePullAudioSamples(
    _env: *mut c_void,
    _class: *mut c_void,
    out_buffer: *mut i16,
    max_len: i32,
) -> i32 {
    if !out_buffer.is_null() && max_len > 0 {
        let slice = std::slice::from_raw_parts_mut(out_buffer, max_len as usize);
        let samples = pull_audio_playback(max_len as usize);
        let count = samples.len().min(slice.len());
        slice[..count].copy_from_slice(&samples[..count]);
        count as i32
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn Java_org_onuron_mobile_NativeBridge_nativeGetLatestFrame(
    _env: *mut c_void,
    _class: *mut c_void,
    out_pixels: *mut u32,
    max_len: i32,
) -> i32 {
    if !out_pixels.is_null() && max_len > 0 {
        let slice = std::slice::from_raw_parts_mut(out_pixels, max_len as usize);
        copy_latest_frame_to_slice(slice) as i32
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn Java_org_onuron_mobile_NativeBridge_nativePushHostEventJson(
    _env: *mut c_void,
    _class: *mut c_void,
    json_ptr: *const c_char,
) -> bool {
    if json_ptr.is_null() {
        return false;
    }
    if let Ok(c_str) = CStr::from_ptr(json_ptr).to_str() {
        if let Ok(event) = serde_json::from_str::<HostToGuestEvent>(c_str) {
            push_host_event(event);
            return true;
        }
    }
    false
}

#[cfg(test)]
pub(crate) static TEST_BRIDGE_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub(crate) fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_BRIDGE_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jni_touch_lifecycle() {
        let _guard = test_lock();
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

            // Reset test dimensions
            SURFACE_WIDTH.store(1080, Ordering::Relaxed);
            SURFACE_HEIGHT.store(2340, Ordering::Relaxed);
        }
    }

    #[test]
    fn test_command_and_audio_queues() {
        let _guard = test_lock();

        // Drain any existing leftover commands to ensure strict test isolation
        while poll_guest_command().is_some() {}

        let cmd = GuestToHostCommand::DialNumber {
            number: "1234567890".into(),
        };
        enqueue_guest_command(cmd.clone());
        assert_eq!(pending_guest_command_count(), 1);
        let popped = poll_guest_command().expect("Expected command");
        match popped {
            GuestToHostCommand::DialNumber { number } => {
                assert_eq!(number, "1234567890");
            }
            other => panic!("Expected DialNumber, got {:?}", other),
        }
        assert_eq!(pending_guest_command_count(), 0);

        // Drain any previous audio playback samples
        let _ = pull_audio_playback(100_000);
        let pcm = [100i16, 200, 300, 400];
        push_audio_playback(&pcm);
        let pulled = pull_audio_playback(100);
        assert_eq!(pulled, vec![100, 200, 300, 400]);

        // Drain any previous audio record samples
        let mut discard = [0i16; 512];
        while pull_audio_record(&mut discard) > 0 {}
        push_audio_record(&[50, 60]);
        let mut rec_buf = [0i16; 64];
        let n = pull_audio_record(&mut rec_buf);
        assert_eq!(n, 2);
        assert_eq!(&rec_buf[..n], &[50, 60]);
    }
}
