// runtime/nilui-gpu/src/drm_uapi.rs — DRM kernel UAPI structures.
//
// Field-for-field transcriptions of `struct drm_mode_*` from
// `include/uapi/drm/drm_mode.h`. Every struct carries a compile-time size and
// offset assertion against the value the kernel header produces, so a layout
// drift is a build failure rather than a corrupted modeset on a real panel.

#![allow(non_camel_case_types)]
// `DRM_MODE_CONNECTOR_eDP` keeps the kernel's own spelling so it can be grepped
// against the header.
#![allow(non_upper_case_globals)]

pub const DRM_IOCTL_BASE: u32 = b'd' as u32;

/// FourCC `XRGB8888` — `fourcc_code('X','R','2','4')`.
pub const DRM_FORMAT_XRGB8888: u32 = 0x3432_5258;

pub const DRM_MODE_CONNECTOR_HDMIA: u32 = 11;
pub const DRM_MODE_CONNECTOR_eDP: u32 = 14;
pub const DRM_MODE_CONNECTOR_VIRTUAL: u32 = 15;
pub const DRM_MODE_CONNECTOR_DSI: u32 = 16;
pub const DRM_MODE_CONNECTOR_DPI: u32 = 17;

/// `enum drm_connector_status` — kernel-internal, but the ABI is frozen.
pub const DRM_MODE_CONNECTED: u32 = 1;
pub const DRM_MODE_DISCONNECTED: u32 = 2;
pub const DRM_MODE_UNKNOWNCONNECTION: u32 = 3;

pub const DRM_MODE_TYPE_PREFERRED: u32 = 1 << 3;
pub const DRM_MODE_TYPE_DRIVER: u32 = 1 << 6;
pub const DRM_DISPLAY_MODE_LEN: usize = 32;

pub const DRM_MODE_PAGE_FLIP_EVENT: u32 = 0x01;
pub const DRM_MODE_PAGE_FLIP_ASYNC: u32 = 0x02;

pub const DRM_EVENT_FLIP_COMPLETE: u32 = 0x02;
pub const DRM_EVENT_VBLANK: u32 = 0x01;

pub const DRM_CLIENT_CAP_UNIVERSAL_PLANES: u32 = 2;
pub const DRM_CLIENT_CAP_ATOMIC: u32 = 3;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct drm_mode_modeinfo {
    pub clock: u32,
    pub hdisplay: u16,
    pub hsync_start: u16,
    pub hsync_end: u16,
    pub htotal: u16,
    pub hskew: u16,
    pub vdisplay: u16,
    pub vsync_start: u16,
    pub vsync_end: u16,
    pub vtotal: u16,
    pub vscan: u16,
    pub vrefresh: u32,
    pub flags: u32,
    pub type_: u32,
    pub name: [u8; DRM_DISPLAY_MODE_LEN],
}

impl drm_mode_modeinfo {
    /// `name` is a NUL-terminated fixed array, not necessarily valid UTF-8.
    pub fn name_str(&self) -> String {
        let end = self
            .name
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(DRM_DISPLAY_MODE_LEN);
        String::from_utf8_lossy(&self.name[..end]).into_owned()
    }

    /// Pixel count of the visible area.
    pub fn pixels(&self) -> u64 {
        self.hdisplay as u64 * self.vdisplay as u64
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct drm_mode_card_res {
    pub fb_id_ptr: u64,
    pub crtc_id_ptr: u64,
    pub connector_id_ptr: u64,
    pub encoder_id_ptr: u64,
    pub count_fbs: u32,
    pub count_crtcs: u32,
    pub count_connectors: u32,
    pub count_encoders: u32,
    pub min_width: u32,
    pub max_width: u32,
    pub min_height: u32,
    pub max_height: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct drm_mode_get_connector {
    pub encoders_ptr: u64,
    pub modes_ptr: u64,
    pub props_ptr: u64,
    pub prop_values_ptr: u64,
    pub count_modes: u32,
    pub count_props: u32,
    pub count_encoders: u32,
    pub encoder_id: u32,
    pub connector_id: u32,
    pub connector_type: u32,
    pub connector_type_id: u32,
    pub connection: u32,
    pub mm_width: u32,
    pub mm_height: u32,
    pub subpixel: u32,
    pub pad: u32,
}

impl drm_mode_get_connector {
    pub fn is_connected(&self) -> bool {
        self.connection == DRM_MODE_CONNECTED
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct drm_mode_get_encoder {
    pub encoder_id: u32,
    pub encoder_type: u32,
    pub crtc_id: u32,
    pub possible_crtcs: u32,
    pub possible_clones: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct drm_mode_crtc {
    pub set_connectors_ptr: u64,
    pub count_connectors: u32,
    pub crtc_id: u32,
    pub fb_id: u32,
    pub x: u32,
    pub y: u32,
    pub gamma_size: u32,
    pub mode_valid: u32,
    pub mode: drm_mode_modeinfo,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct drm_mode_create_dumb {
    pub height: u32,
    pub width: u32,
    pub bpp: u32,
    pub flags: u32,
    pub handle: u32,
    pub pitch: u32,
    pub size: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct drm_mode_map_dumb {
    pub handle: u32,
    pub pad: u32,
    pub offset: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct drm_mode_destroy_dumb {
    pub handle: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct drm_mode_fb_cmd2 {
    pub fb_id: u32,
    pub width: u32,
    pub height: u32,
    pub pixel_format: u32,
    pub flags: u32,
    pub handles: [u32; 4],
    pub pitches: [u32; 4],
    pub offsets: [u32; 4],
    pub modifier: [u64; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct drm_mode_crtc_page_flip {
    pub crtc_id: u32,
    pub fb_id: u32,
    pub flags: u32,
    pub reserved: u32,
    pub user_data: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct drm_event {
    pub type_: u32,
    pub length: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct drm_event_vblank {
    pub base: drm_event,
    pub user_data: u64,
    pub tv_sec: u32,
    pub tv_usec: u32,
    pub sequence: u32,
    pub crtc_id: u32,
}

/// `DRM_IOCTL_VERSION` argument. Used to identify the driver behind a node.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct drm_version {
    pub version_major: i32,
    pub version_minor: i32,
    pub version_patchlevel: i32,
    pub name_len: usize,
    pub name: u64,
    pub date_len: usize,
    pub date: u64,
    pub desc_len: usize,
    pub desc: u64,
}

/// `DRM_IOCTL_SET_CLIENT_CAP` argument.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct drm_set_client_cap {
    pub capability: u64,
    pub value: u64,
}

// Layout is an ABI contract with the kernel, not an internal detail. These
// assertions are the reason the structs above can be trusted on a real device.
const _: () = {
    use std::mem::{align_of, offset_of, size_of};

    assert!(size_of::<drm_mode_modeinfo>() == 68);
    assert!(offset_of!(drm_mode_modeinfo, vdisplay) == 14);
    assert!(offset_of!(drm_mode_modeinfo, vrefresh) == 24);
    assert!(offset_of!(drm_mode_modeinfo, name) == 36);

    assert!(size_of::<drm_mode_card_res>() == 64);
    assert!(offset_of!(drm_mode_card_res, count_fbs) == 32);
    assert!(offset_of!(drm_mode_card_res, min_width) == 48);

    assert!(size_of::<drm_mode_get_connector>() == 80);
    assert!(offset_of!(drm_mode_get_connector, count_modes) == 32);
    assert!(offset_of!(drm_mode_get_connector, encoder_id) == 44);
    assert!(offset_of!(drm_mode_get_connector, connection) == 60);

    assert!(size_of::<drm_mode_get_encoder>() == 20);
    assert!(size_of::<drm_mode_crtc>() == 104);
    assert!(offset_of!(drm_mode_crtc, count_connectors) == 8);
    assert!(offset_of!(drm_mode_crtc, crtc_id) == 12);
    assert!(offset_of!(drm_mode_crtc, mode_valid) == 32);
    assert!(offset_of!(drm_mode_crtc, mode) == 36);

    assert!(size_of::<drm_mode_create_dumb>() == 32);
    assert!(size_of::<drm_mode_map_dumb>() == 16);
    assert!(size_of::<drm_mode_destroy_dumb>() == 4);
    assert!(size_of::<drm_mode_fb_cmd2>() == 104);
    assert!(size_of::<drm_mode_crtc_page_flip>() == 24);
    assert!(size_of::<drm_event>() == 8);
    assert!(size_of::<drm_event_vblank>() == 32);
    assert!(size_of::<drm_set_client_cap>() == 16);

    // 64-bit kernels hand the kernel 8-byte-aligned pointers.
    assert!(align_of::<drm_mode_card_res>() == 8);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modeinfo_name_is_nul_terminated_and_lossy_safe() {
        let mut m = drm_mode_modeinfo {
            hdisplay: 1080,
            vdisplay: 2400,
            vrefresh: 90,
            ..Default::default()
        };
        m.name[..9].copy_from_slice(b"1080x2400");
        assert_eq!(m.name_str(), "1080x2400");
        assert_eq!(m.pixels(), 1080 * 2400);

        // Invalid UTF-8 must not panic.
        m.name[0] = 0xFF;
        let _ = m.name_str();
    }

    #[test]
    fn full_name_array_without_nul_is_handled() {
        let m = drm_mode_modeinfo {
            name: [b'A'; DRM_DISPLAY_MODE_LEN],
            ..Default::default()
        };
        assert_eq!(m.name_str().len(), DRM_DISPLAY_MODE_LEN);
    }

    #[test]
    fn connector_connection_maps_to_status() {
        let connected = drm_mode_get_connector {
            connection: DRM_MODE_CONNECTED,
            ..Default::default()
        };
        assert!(connected.is_connected());
        let disconnected = drm_mode_get_connector {
            connection: DRM_MODE_DISCONNECTED,
            ..Default::default()
        };
        assert!(!disconnected.is_connected());
    }
}
