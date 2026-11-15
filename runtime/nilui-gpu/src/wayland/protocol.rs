// runtime/nilui-gpu/src/wayland/protocol.rs — Wayland core object model
//
// Object ids are allocated by the client for requests that carry a `new_id`
// argument and by the server for events that do. Getting this backwards is a
// classic Wayland bug, so the direction is stated at each call site.
//
// Only the subset of `wayland.xml` a buffer-passing compositor needs is
// implemented: wl_display, wl_registry, wl_compositor, wl_surface, wl_shm,
// wl_shm_pool, wl_buffer, wl_callback, wl_output, wl_seat, wl_touch.
//
// Opcodes below are transcribed from `wayland.xml` and asserted by tests, so a
// typo fails the test run rather than silently mis-dispatching.

use super::wire::Encoder;

// ─── Interface names ─────────────────────────────────────────────────────────

pub const WL_DISPLAY: &str = "wl_display";
pub const WL_REGISTRY: &str = "wl_registry";
pub const WL_COMPOSITOR: &str = "wl_compositor";
pub const WL_SURFACE: &str = "wl_surface";
pub const WL_SHM: &str = "wl_shm";
pub const WL_SHM_POOL: &str = "wl_shm_pool";
pub const WL_BUFFER: &str = "wl_buffer";
pub const WL_CALLBACK: &str = "wl_callback";
pub const WL_OUTPUT: &str = "wl_output";
pub const WL_SEAT: &str = "wl_seat";
pub const WL_TOUCH: &str = "wl_touch";
pub const WL_REGION: &str = "wl_region";

// ─── Versions we advertise ───────────────────────────────────────────────────
//
// These are the versions whose request signatures we actually implement. A
// client is free to bind a lower version; binding a higher one is clamped.

pub const VERSION_DISPLAY: u32 = 1;
pub const VERSION_REGISTRY: u32 = 1;
pub const VERSION_COMPOSITOR: u32 = 4;
pub const VERSION_SURFACE: u32 = 4;
pub const VERSION_SHM: u32 = 1;
pub const VERSION_SHM_POOL: u32 = 1;
pub const VERSION_BUFFER: u32 = 1;
pub const VERSION_CALLBACK: u32 = 1;
pub const VERSION_OUTPUT: u32 = 2;
pub const VERSION_SEAT: u32 = 5;
pub const VERSION_TOUCH: u32 = 5;

// ─── wl_display ──────────────────────────────────────────────────────────────

pub mod display_req {
    pub const SYNC: u16 = 0;
    pub const GET_REGISTRY: u16 = 1;
}

pub mod display_ev {
    pub const ERROR: u16 = 0;
    pub const DELETE_ID: u16 = 1;
}

pub mod display_error {
    pub const INVALID_OBJECT: u32 = 0;
    pub const INVALID_METHOD: u32 = 1;
    pub const NO_MEMORY: u32 = 2;
    pub const IMPLEMENTATION: u32 = 3;
}

// ─── wl_registry ─────────────────────────────────────────────────────────────

pub mod registry_req {
    pub const BIND: u16 = 0;
    pub const DESTROY: u16 = 1;
}

pub mod registry_ev {
    pub const GLOBAL: u16 = 0;
    pub const GLOBAL_REMOVE: u16 = 1;
}

// ─── wl_compositor ───────────────────────────────────────────────────────────

pub mod compositor_req {
    pub const CREATE_SURFACE: u16 = 0;
    pub const CREATE_REGION: u16 = 1;
}

// ─── wl_surface ──────────────────────────────────────────────────────────────

pub mod surface_req {
    pub const DESTROY: u16 = 0;
    pub const ATTACH: u16 = 1;
    pub const DAMAGE: u16 = 2;
    pub const FRAME: u16 = 3;
    pub const SET_OPAQUE_REGION: u16 = 4;
    pub const SET_INPUT_REGION: u16 = 5;
    pub const COMMIT: u16 = 6;
    pub const SET_BUFFER_TRANSFORM: u16 = 7;
    pub const SET_BUFFER_SCALE: u16 = 8;
    pub const DAMAGE_BUFFER: u16 = 9;
}

pub mod surface_ev {
    pub const ENTER: u16 = 0;
    pub const LEAVE: u16 = 1;
    pub const PREFERRED_BUFFER_SCALE: u16 = 2;
    pub const PREFERRED_BUFFER_TRANSFORM: u16 = 3;
}

/// `wl_surface.transform` values (from `wayland.xml`).
pub mod transform {
    pub const NORMAL: u32 = 0;
    pub const ROTATE_90: u32 = 1;
    pub const ROTATE_180: u32 = 2;
    pub const ROTATE_270: u32 = 3;
    pub const FLIPPED: u32 = 4;
    pub const FLIPPED_90: u32 = 5;
    pub const FLIPPED_180: u32 = 6;
    pub const FLIPPED_270: u32 = 7;
}

// ─── wl_shm ──────────────────────────────────────────────────────────────────

pub mod shm_req {
    pub const CREATE_POOL: u16 = 0;
}

pub mod shm_ev {
    pub const FORMAT: u16 = 0;
}

/// `wl_shm.format` values. Only the two 32-bit packed formats are advertised,
/// because those are the ones the DRM dumb-buffer path can scan out directly
/// without a colour conversion.
pub mod shm_format {
    pub const ARGB8888: u32 = 0;
    pub const XRGB8888: u32 = 1;
}

pub mod shm_error {
    pub const INVALID_FORMAT: u32 = 0;
    pub const INVALID_STRIDE: u32 = 1;
    pub const INVALID_FD: u32 = 2;
}

// ─── wl_shm_pool ─────────────────────────────────────────────────────────────

pub mod shm_pool_req {
    pub const CREATE_BUFFER: u16 = 0;
    pub const DESTROY: u16 = 1;
    pub const RESIZE: u16 = 2;
}

// ─── wl_buffer ───────────────────────────────────────────────────────────────

pub mod buffer_req {
    pub const DESTROY: u16 = 0;
}

pub mod buffer_ev {
    pub const RELEASE: u16 = 0;
}

// ─── wl_callback ─────────────────────────────────────────────────────────────

pub mod callback_ev {
    pub const DONE: u16 = 0;
}

// ─── wl_output ───────────────────────────────────────────────────────────────

pub mod output_req {
    pub const RELEASE: u16 = 0;
}

pub mod output_ev {
    pub const GEOMETRY: u16 = 0;
    pub const MODE: u16 = 1;
    pub const DONE: u16 = 2;
    pub const SCALE: u16 = 3;
}

pub mod output_mode {
    pub const CURRENT: u32 = 0x1;
    pub const PREFERRED: u32 = 0x2;
}

/// `wl_output.subpixel` values.
pub mod subpixel {
    pub const UNKNOWN: i32 = 0;
}

// ─── wl_seat ─────────────────────────────────────────────────────────────────

pub mod seat_req {
    pub const GET_POINTER: u16 = 0;
    pub const GET_KEYBOARD: u16 = 1;
    pub const GET_TOUCH: u16 = 2;
    pub const RELEASE: u16 = 3;
}

pub mod seat_ev {
    pub const CAPABILITIES: u16 = 0;
    pub const NAME: u16 = 1;
}

pub mod seat_capability {
    pub const POINTER: u32 = 0x1;
    pub const KEYBOARD: u32 = 0x2;
    pub const TOUCH: u32 = 0x4;
}

// ─── wl_touch ────────────────────────────────────────────────────────────────

pub mod touch_req {
    pub const RELEASE: u16 = 0;
}

pub mod touch_ev {
    pub const DOWN: u16 = 0;
    pub const UP: u16 = 1;
    pub const MOTION: u16 = 2;
    pub const FRAME: u16 = 3;
    pub const CANCEL: u16 = 4;
}

/// A global the compositor offers through `wl_registry.global`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Global {
    pub name: u32,
    pub interface: String,
    pub version: u32,
}

/// Server-side object kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectKind {
    Display,
    Registry,
    Compositor,
    Surface,
    Shm,
    ShmPool,
    Buffer,
    Callback,
    Output,
    Seat,
    Touch,
    /// A `wl_region`. Accepted and tracked; not used for clipping yet.
    Region,
}

/// Build a `wl_display.error` event.
///
/// The event is sent to `wl_display` (object 1); `offending_object` is the
/// object the client misused and travels as the event's first *argument*. A
/// client that sees an error on the display but no offending object cannot tell
/// which request it got wrong.
pub fn display_error_event(
    display_id: u32,
    offending_object: u32,
    code: u32,
    message: &str,
) -> Encoder {
    let mut e = Encoder::begin(display_id, display_ev::ERROR);
    e.put_object(offending_object)
        .put_uint(code)
        .put_string(message);
    e
}

/// Build a `wl_display.delete_id` event.
pub fn delete_id_event(object_id: u32, id: u32) -> Encoder {
    let mut e = Encoder::begin(object_id, display_ev::DELETE_ID);
    e.put_uint(id);
    e
}

/// Build a `wl_registry.global` event.
pub fn global_event(object_id: u32, g: &Global) -> Encoder {
    let mut e = Encoder::begin(object_id, registry_ev::GLOBAL);
    e.put_uint(g.name)
        .put_string(&g.interface)
        .put_uint(g.version);
    e
}

/// Build a `wl_registry.global_remove` event.
pub fn global_remove_event(object_id: u32, name: u32) -> Encoder {
    let mut e = Encoder::begin(object_id, registry_ev::GLOBAL_REMOVE);
    e.put_uint(name);
    e
}

/// Build a `wl_shm.format` event.
pub fn shm_format_event(object_id: u32, format: u32) -> Encoder {
    let mut e = Encoder::begin(object_id, shm_ev::FORMAT);
    e.put_uint(format);
    e
}

/// Build a `wl_buffer.release` event.
pub fn buffer_release_event(object_id: u32) -> Encoder {
    Encoder::begin(object_id, buffer_ev::RELEASE)
}

/// Build a `wl_callback.done` event.
pub fn callback_done_event(object_id: u32, callback_data: u32) -> Encoder {
    let mut e = Encoder::begin(object_id, callback_ev::DONE);
    e.put_uint(callback_data);
    e
}

/// Build a `wl_surface.enter` event.
pub fn surface_enter_event(object_id: u32, output_id: u32) -> Encoder {
    let mut e = Encoder::begin(object_id, surface_ev::ENTER);
    e.put_object(output_id);
    e
}

/// Build a `wl_output.geometry` event.
#[allow(clippy::too_many_arguments)]
pub fn output_geometry_event(
    object_id: u32,
    x: i32,
    y: i32,
    physical_width_mm: i32,
    physical_height_mm: i32,
    subpixel: i32,
    make: &str,
    model: &str,
    transform: i32,
) -> Encoder {
    let mut e = Encoder::begin(object_id, output_ev::GEOMETRY);
    e.put_int(x)
        .put_int(y)
        .put_int(physical_width_mm)
        .put_int(physical_height_mm)
        .put_int(subpixel)
        .put_string(make)
        .put_string(model)
        .put_int(transform);
    e
}

/// Build a `wl_output.mode` event.
pub fn output_mode_event(
    object_id: u32,
    flags: u32,
    width: i32,
    height: i32,
    refresh_mhz: i32,
) -> Encoder {
    let mut e = Encoder::begin(object_id, output_ev::MODE);
    e.put_uint(flags)
        .put_int(width)
        .put_int(height)
        .put_int(refresh_mhz);
    e
}

/// Build a `wl_output.scale` event.
pub fn output_scale_event(object_id: u32, factor: i32) -> Encoder {
    let mut e = Encoder::begin(object_id, output_ev::SCALE);
    e.put_int(factor);
    e
}

/// Build a `wl_output.done` event.
pub fn output_done_event(object_id: u32) -> Encoder {
    Encoder::begin(object_id, output_ev::DONE)
}

/// Build a `wl_seat.capabilities` event.
pub fn seat_capabilities_event(object_id: u32, caps: u32) -> Encoder {
    let mut e = Encoder::begin(object_id, seat_ev::CAPABILITIES);
    e.put_uint(caps);
    e
}

/// Build a `wl_seat.name` event.
pub fn seat_name_event(object_id: u32, name: &str) -> Encoder {
    let mut e = Encoder::begin(object_id, seat_ev::NAME);
    e.put_string(name);
    e
}

/// Build a `wl_touch.down` event.
pub fn touch_down_event(
    object_id: u32,
    serial: u32,
    time: u32,
    surface_id: u32,
    id: i32,
    x: f64,
    y: f64,
) -> Encoder {
    let mut e = Encoder::begin(object_id, touch_ev::DOWN);
    e.put_uint(serial)
        .put_uint(time)
        .put_object(surface_id)
        .put_int(id)
        .put_fixed(x)
        .put_fixed(y);
    e
}

/// Build a `wl_touch.up` event.
pub fn touch_up_event(object_id: u32, serial: u32, time: u32, id: i32) -> Encoder {
    let mut e = Encoder::begin(object_id, touch_ev::UP);
    e.put_uint(serial).put_uint(time).put_int(id);
    e
}

/// Build a `wl_touch.motion` event.
pub fn touch_motion_event(object_id: u32, time: u32, id: i32, x: f64, y: f64) -> Encoder {
    let mut e = Encoder::begin(object_id, touch_ev::MOTION);
    e.put_uint(time).put_int(id).put_fixed(x).put_fixed(y);
    e
}

/// Build a `wl_touch.frame` event.
pub fn touch_frame_event(object_id: u32) -> Encoder {
    Encoder::begin(object_id, touch_ev::FRAME)
}

/// Build a `wl_touch.cancel` event.
pub fn touch_cancel_event(object_id: u32) -> Encoder {
    Encoder::begin(object_id, touch_ev::CANCEL)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wayland::wire::{decode_body, decode_message, Arg};

    /// Every opcode asserted against `wayland.xml`. A transposition fails here.
    #[test]
    fn core_request_opcodes_match_wayland_xml() {
        assert_eq!(display_req::SYNC, 0);
        assert_eq!(display_req::GET_REGISTRY, 1);

        assert_eq!(registry_req::BIND, 0);
        assert_eq!(registry_req::DESTROY, 1);

        assert_eq!(compositor_req::CREATE_SURFACE, 0);
        assert_eq!(compositor_req::CREATE_REGION, 1);

        // wl_surface, in the order declared in wayland.xml.
        assert_eq!(surface_req::DESTROY, 0);
        assert_eq!(surface_req::ATTACH, 1);
        assert_eq!(surface_req::DAMAGE, 2);
        assert_eq!(surface_req::FRAME, 3);
        assert_eq!(surface_req::SET_OPAQUE_REGION, 4);
        assert_eq!(surface_req::SET_INPUT_REGION, 5);
        assert_eq!(surface_req::COMMIT, 6);
        assert_eq!(surface_req::SET_BUFFER_TRANSFORM, 7);
        assert_eq!(surface_req::SET_BUFFER_SCALE, 8);
        assert_eq!(surface_req::DAMAGE_BUFFER, 9);

        assert_eq!(shm_req::CREATE_POOL, 0);
        assert_eq!(shm_pool_req::CREATE_BUFFER, 0);
        assert_eq!(shm_pool_req::DESTROY, 1);
        assert_eq!(shm_pool_req::RESIZE, 2);

        assert_eq!(buffer_req::DESTROY, 0);
        assert_eq!(output_req::RELEASE, 0);
        assert_eq!(seat_req::GET_POINTER, 0);
        assert_eq!(seat_req::GET_KEYBOARD, 1);
        assert_eq!(seat_req::GET_TOUCH, 2);
        assert_eq!(seat_req::RELEASE, 3);
        assert_eq!(touch_req::RELEASE, 0);
    }

    #[test]
    fn event_opcodes_match_wayland_xml() {
        assert_eq!(display_ev::ERROR, 0);
        assert_eq!(display_ev::DELETE_ID, 1);
        assert_eq!(registry_ev::GLOBAL, 0);
        assert_eq!(registry_ev::GLOBAL_REMOVE, 1);
        assert_eq!(shm_ev::FORMAT, 0);
        assert_eq!(buffer_ev::RELEASE, 0);
        assert_eq!(callback_ev::DONE, 0);
        assert_eq!(output_ev::GEOMETRY, 0);
        assert_eq!(output_ev::MODE, 1);
        assert_eq!(output_ev::DONE, 2);
        assert_eq!(output_ev::SCALE, 3);
        assert_eq!(seat_ev::CAPABILITIES, 0);
        assert_eq!(seat_ev::NAME, 1);
        assert_eq!(touch_ev::DOWN, 0);
        assert_eq!(touch_ev::UP, 1);
        assert_eq!(touch_ev::MOTION, 2);
        assert_eq!(touch_ev::FRAME, 3);
        assert_eq!(touch_ev::CANCEL, 4);
    }

    #[test]
    fn shm_format_values_match_the_protocol_enum() {
        // These two values are load-bearing: they select the packed 32-bit
        // layout the DRM scanout path expects.
        assert_eq!(shm_format::ARGB8888, 0);
        assert_eq!(shm_format::XRGB8888, 1);
    }

    #[test]
    fn seat_capability_bits_match_the_protocol_enum() {
        assert_eq!(seat_capability::POINTER, 1);
        assert_eq!(seat_capability::KEYBOARD, 2);
        assert_eq!(seat_capability::TOUCH, 4);
    }

    #[test]
    fn global_event_encodes_name_interface_and_version() {
        let g = Global {
            name: 3,
            interface: WL_COMPOSITOR.to_string(),
            version: 4,
        };
        let ev = global_event(2, &g).finish().unwrap();
        let (obj, op, body) = decode_message(&ev.bytes).unwrap();
        assert_eq!(obj, 2);
        assert_eq!(op, registry_ev::GLOBAL);
        let args = decode_body("usu", body, &[]).unwrap();
        assert_eq!(args[0], Arg::Uint(3));
        assert_eq!(args[1], Arg::Str("wl_compositor".into()));
        assert_eq!(args[2], Arg::Uint(4));
    }

    #[test]
    fn callback_done_carries_the_serial() {
        let ev = callback_done_event(9, 12345).finish().unwrap();
        let (obj, op, body) = decode_message(&ev.bytes).unwrap();
        assert_eq!((obj, op), (9, callback_ev::DONE));
        assert_eq!(
            decode_body("u", body, &[]).unwrap()[0].as_u32(),
            Some(12345)
        );
    }

    #[test]
    fn output_mode_event_layout() {
        let flags = output_mode::CURRENT | output_mode::PREFERRED;
        let ev = output_mode_event(5, flags, 1080, 2400, 60000)
            .finish()
            .unwrap();
        let (obj, op, body) = decode_message(&ev.bytes).unwrap();
        assert_eq!((obj, op), (5, output_ev::MODE));
        let args = decode_body("uiii", body, &[]).unwrap();
        assert_eq!(args[0].as_u32(), Some(3));
        assert_eq!(args[1].as_i32(), Some(1080));
        assert_eq!(args[2].as_i32(), Some(2400));
        assert_eq!(args[3].as_i32(), Some(60000));
    }

    #[test]
    fn touch_down_event_uses_fixed_point_coordinates() {
        let ev = touch_down_event(7, 1, 100, 4, 0, 12.5, 30.25)
            .finish()
            .unwrap();
        let (obj, op, body) = decode_message(&ev.bytes).unwrap();
        assert_eq!((obj, op), (7, touch_ev::DOWN));
        let args = decode_body("uuoiff", body, &[]).unwrap();
        assert_eq!(args[0].as_u32(), Some(1));
        assert_eq!(args[1].as_u32(), Some(100));
        assert_eq!(args[2].as_u32(), Some(4));
        assert_eq!(args[3].as_i32(), Some(0));
        assert_eq!(args[4].as_f64(), Some(12.5));
        assert_eq!(args[5].as_f64(), Some(30.25));
    }

    #[test]
    fn error_event_encodes_code_and_message() {
        let ev = display_error_event(1, 42, display_error::INVALID_METHOD, "bad opcode")
            .finish()
            .unwrap();
        let (obj, op, body) = decode_message(&ev.bytes).unwrap();
        assert_eq!((obj, op), (1, display_ev::ERROR));
        let args = decode_body("ous", body, &[]).unwrap();
        assert_eq!(args[0].as_u32(), Some(42));
        assert_eq!(args[1].as_u32(), Some(display_error::INVALID_METHOD));
        assert_eq!(args[2].as_str(), Some("bad opcode"));
    }

    #[test]
    fn seat_capabilities_event_sets_the_touch_bit() {
        let ev = seat_capabilities_event(4, seat_capability::TOUCH)
            .finish()
            .unwrap();
        let (obj, op, body) = decode_message(&ev.bytes).unwrap();
        assert_eq!((obj, op), (4, seat_ev::CAPABILITIES));
        let args = decode_body("u", body, &[]).unwrap();
        assert_eq!(args[0].as_u32(), Some(4));
    }
}
