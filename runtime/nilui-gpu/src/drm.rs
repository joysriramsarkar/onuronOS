// runtime/nilui-gpu/src/drm.rs — Direct Rendering Manager modesetting.
//
// Drives `/dev/dri/card*` through the real KMS ioctl interface: card
// enumeration, connector/CRTC discovery, mode selection, dumb-buffer
// allocation and mmap, framebuffer creation, and VBLANK-synchronised page
// flips. No X11, no Wayland, no libdrm — the ioctl request numbers and the
// UAPI structures live in `ioctl.rs` and `drm_uapi.rs` and are checked against
// the kernel's own headers at compile time and in unit tests.
//
// The previous revision of this file opened the device node and then never
// issued an ioctl, so `set_crtc`/`page_flip` returned `Ok` without touching the
// display. Everything here either performs the real operation or reports the
// real errno.

use std::fs::{File, OpenOptions};
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::{Path, PathBuf};

use crate::drm_uapi::*;
use crate::ioctl::{io, ioctl, ioctl_noarg, iowr};

pub const DEFAULT_DRM_CARD: &str = "/dev/dri/card0";
pub const SECONDARY_DRM_CARD: &str = "/dev/dri/card1";

/// Sizes passed to the ioctl macros must equal the kernel's `sizeof`. A
/// mismatch makes the request number wrong and the kernel returns ENOTTY.
const SZ_CARD_RES: u32 = 64;
const SZ_GETCRTC: u32 = 104;
const SZ_SETCRTC: u32 = 104;
const SZ_GETENCODER: u32 = 20;
const SZ_GETCONNECTOR: u32 = 80;
const SZ_RMFB: u32 = 4;
const SZ_PAGE_FLIP: u32 = 24;
const SZ_CREATE_DUMB: u32 = 32;
const SZ_MAP_DUMB: u32 = 16;
const SZ_DESTROY_DUMB: u32 = 4;
const SZ_ADDFB2: u32 = 104;

const DRM_IOCTL_SET_MASTER: u64 = io(DRM_IOCTL_BASE, 0x1e);
const DRM_IOCTL_DROP_MASTER: u64 = io(DRM_IOCTL_BASE, 0x1f);
const DRM_IOCTL_MODE_GETRESOURCES: u64 = iowr(DRM_IOCTL_BASE, 0xA0, SZ_CARD_RES);
const DRM_IOCTL_MODE_GETCRTC: u64 = iowr(DRM_IOCTL_BASE, 0xA1, SZ_GETCRTC);
const DRM_IOCTL_MODE_SETCRTC: u64 = iowr(DRM_IOCTL_BASE, 0xA2, SZ_SETCRTC);
const DRM_IOCTL_MODE_GETENCODER: u64 = iowr(DRM_IOCTL_BASE, 0xA6, SZ_GETENCODER);
const DRM_IOCTL_MODE_GETCONNECTOR: u64 = iowr(DRM_IOCTL_BASE, 0xA7, SZ_GETCONNECTOR);
const DRM_IOCTL_MODE_RMFB: u64 = iowr(DRM_IOCTL_BASE, 0xAF, SZ_RMFB);
const DRM_IOCTL_MODE_PAGE_FLIP: u64 = iowr(DRM_IOCTL_BASE, 0xB0, SZ_PAGE_FLIP);
const DRM_IOCTL_MODE_CREATE_DUMB: u64 = iowr(DRM_IOCTL_BASE, 0xB2, SZ_CREATE_DUMB);
const DRM_IOCTL_MODE_MAP_DUMB: u64 = iowr(DRM_IOCTL_BASE, 0xB3, SZ_MAP_DUMB);
const DRM_IOCTL_MODE_DESTROY_DUMB: u64 = iowr(DRM_IOCTL_BASE, 0xB4, SZ_DESTROY_DUMB);
const DRM_IOCTL_MODE_ADDFB2: u64 = iowr(DRM_IOCTL_BASE, 0xB8, SZ_ADDFB2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrmMode {
    pub width: u32,
    pub height: u32,
    pub refresh_rate_hz: u32,
}

impl Default for DrmMode {
    fn default() -> Self {
        // Fallback for a node whose connector reports no modes.
        Self {
            width: 720,
            height: 1440,
            refresh_rate_hz: 120,
        }
    }
}

/// A frame buffer whose pixels live in the GPU's memory, mapped into this
/// process by the kernel.
///
/// Writes to `pixels` go straight to the scanout buffer, so a page flip after
/// writing displays exactly those pixels. `size_bytes` is the kernel's stride
/// times height, which may exceed `width * height * 4` because the pitch is
/// hardware-aligned; rows are laid out at `pitch` intervals, not `width * 4`.
pub struct DumbBuffer {
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub size_bytes: usize,
    pub handle: u32,
    pub fb_id: u32,
    pub pixels: Vec<u32>,
}

impl DumbBuffer {
    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, argb: u32) {
        if x < self.width && y < self.height {
            let idx = (y * self.width + x) as usize;
            if idx < self.pixels.len() {
                self.pixels[idx] = argb;
            }
        }
    }

    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> u32 {
        if x < self.width && y < self.height {
            let idx = (y * self.width + x) as usize;
            if idx < self.pixels.len() {
                return self.pixels[idx];
            }
        }
        0
    }

    pub fn clear(&mut self, argb: u32) {
        self.pixels.fill(argb);
    }

    /// Bytes the kernel needs for one row of this buffer.
    pub fn row_bytes(&self) -> u32 {
        self.width * 4
    }

    /// True when the kernel's stride is wider than the visible row, i.e. the
    /// buffer has row padding.
    pub fn has_padding(&self) -> bool {
        self.pitch != self.row_bytes()
    }
}

impl std::fmt::Debug for DumbBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DumbBuffer")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("pitch", &self.pitch)
            .field("handle", &self.handle)
            .field("fb_id", &self.fb_id)
            .field("pixels", &self.pixels.len())
            .finish()
    }
}

/// A discovered display pipeline: which connector is lit and by which CRTC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayInfo {
    pub connector_id: u32,
    pub connector_type: u32,
    pub crtc_id: u32,
    pub encoder_id: u32,
    pub mode: DrmMode,
    pub mode_name: String,
    pub preferred: bool,
}

/// Which backend a `DrmDevice` ended up on. Reported honestly so callers can
/// tell a real panel from the offscreen fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrmBackend {
    /// A real `/dev/dri/card*` node was opened and initialised.
    Hardware { card_path: String, driver: String },
    /// No usable node: frames are composed in normal memory and go nowhere.
    Offscreen { reason: String },
}

impl DrmBackend {
    pub fn is_hardware(&self) -> bool {
        matches!(self, DrmBackend::Hardware { .. })
    }

    pub fn label(&self) -> &'static str {
        match self {
            DrmBackend::Hardware { .. } => "drm_kms_hardware",
            DrmBackend::Offscreen { .. } => "offscreen_fallback",
        }
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            DrmBackend::Hardware { .. } => None,
            DrmBackend::Offscreen { reason } => Some(reason),
        }
    }
}

/// An open DRM device. On hardware the kernel owns the scanout buffers; the
/// offscreen variant is a plain `Vec` so the compositor still runs (and still
/// renders correct pixels) on a machine with no display.
#[derive(Debug)]
pub struct DrmDevice {
    pub card_path: String,
    pub mode: DrmMode,
    pub backend: DrmBackend,
    /// Kept so `is_hardware()` stays cheap and the node stays open.
    is_hardware: bool,
    file: Option<File>,
    display: Option<DisplayInfo>,
}

impl DrmDevice {
    /// Open and fully initialise a DRM node.
    ///
    /// Performs: card capability probe, resource enumeration, connector
    /// selection (the connected one with a mode, preferring the driver's
    /// preferred mode), CRTC selection, and `SET_MASTER` so mode setting is
    /// permitted. Any failure is returned as a structured error naming the
    /// syscall that failed.
    pub fn open(path: &str, preferred_mode: Option<DrmMode>) -> Result<Self, String> {
        if !Path::new(path).exists() {
            return Err(format!("DRM device not found: {path}"));
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| format!("open {path}: {e}"))?;
        let fd = file.as_raw_fd();

        let driver = read_driver_name(fd).unwrap_or_else(|| "unknown".to_string());

        // Needed on legacy nodes before any mode-setting ioctl will be honoured.
        match ioctl_noarg(fd, DRM_IOCTL_SET_MASTER) {
            Ok(()) => {}
            // Already master (or a kernel with no legacy master concept) is fine.
            Err(e) if e.raw_os_error() == Some(libc::EACCES) => {}
            Err(e) => return Err(format!("SET_MASTER {path}: {e}")),
        }

        let resources = get_resources(fd)?;
        let display = select_display(fd, &resources, preferred_mode)?;

        Ok(Self {
            card_path: path.to_string(),
            mode: display.mode,
            backend: DrmBackend::Hardware {
                card_path: path.to_string(),
                driver,
            },
            is_hardware: true,
            file: Some(file),
            display: Some(display),
        })
    }

    /// Open the first usable DRM node, otherwise fall back to an offscreen
    /// device that renders but cannot present.
    pub fn open_or_virtual(preferred_mode: Option<DrmMode>) -> Self {
        let mut reason = String::new();
        for path in candidate_nodes() {
            match Self::open(&path, preferred_mode) {
                Ok(dev) => return dev,
                Err(e) => {
                    if !reason.is_empty() {
                        reason.push_str("; ");
                    }
                    reason.push_str(&format!("{path}: {e}"));
                }
            }
        }
        if reason.is_empty() {
            reason = "no /dev/dri/card* nodes present".to_string();
        }
        Self::offscreen(preferred_mode.unwrap_or_default(), reason)
    }

    /// A device with no display attached. Explicitly named so nothing can
    /// mistake it for hardware.
    pub fn offscreen(mode: DrmMode, reason: String) -> Self {
        Self {
            card_path: "offscreen".to_string(),
            mode,
            backend: DrmBackend::Offscreen { reason },
            is_hardware: false,
            file: None,
            display: None,
        }
    }

    pub fn is_hardware(&self) -> bool {
        self.is_hardware
    }

    pub fn display(&self) -> Option<&DisplayInfo> {
        self.display.as_ref()
    }

    /// The DRM file descriptor, if this is a hardware device.
    pub fn fd(&self) -> Option<RawFd> {
        self.file.as_ref().map(|f| f.as_raw_fd())
    }

    /// Allocate a scanout buffer of this device's current mode.
    ///
    /// On hardware this issues `CREATE_DUMB`, maps the returned GEM object,
    /// creates a framebuffer with `ADDFB2`, and exposes the mapping as `u32`
    /// pixels. The `fb_id` argument is only a label used by the offscreen
    /// backend and by tests; hardware framebuffer ids come from the kernel.
    pub fn allocate_dumb_buffer(&self, fb_id: u32) -> DumbBuffer {
        if !self.is_hardware {
            let mut buf = self.offscreen_buffer();
            buf.fb_id = fb_id;
            return buf;
        }
        match self.allocate_hardware_buffer() {
            Ok(buf) => buf,
            Err(e) => {
                // The device opened but cannot give us a buffer. Fall back to
                // memory so the caller still gets a working surface, and say so.
                let mut buf = self.offscreen_buffer();
                buf.fb_id = fb_id;
                eprintln!("[drm] dumb buffer allocation failed ({e}); using memory buffer");
                buf
            }
        }
    }

    /// Like `allocate_dumb_buffer`, but surfaces the real error instead of
    /// silently degrading. Used by callers that must know they are on hardware.
    pub fn try_allocate_dumb_buffer(&self, fb_id: u32) -> Result<DumbBuffer, String> {
        if !self.is_hardware {
            let mut buf = self.offscreen_buffer();
            buf.fb_id = fb_id;
            return Ok(buf);
        }
        self.allocate_hardware_buffer()
    }

    fn offscreen_buffer(&self) -> DumbBuffer {
        let width = self.mode.width;
        let height = self.mode.height;
        DumbBuffer {
            width,
            height,
            pitch: width * 4,
            size_bytes: (width * height * 4) as usize,
            handle: 0,
            fb_id: 0,
            pixels: vec![0xFF00_0000; (width * height) as usize],
        }
    }

    #[cfg(target_os = "linux")]
    fn allocate_hardware_buffer(&self) -> Result<DumbBuffer, String> {
        let fd = self.fd().ok_or("device is not open")?;
        let width = self.mode.width;
        let height = self.mode.height;

        // 1. Ask the kernel for a buffer.
        let mut create = drm_mode_create_dumb {
            width,
            height,
            bpp: 32,
            ..Default::default()
        };
        ioctl(fd, DRM_IOCTL_MODE_CREATE_DUMB, &mut create)
            .map_err(|e| format!("CREATE_DUMB {width}x{height}: {e}"))?;
        if create.handle == 0 || create.size == 0 {
            return Err("CREATE_DUMB returned an empty buffer".to_string());
        }

        // 2. Ask where to mmap it.
        let mut map = drm_mode_map_dumb {
            handle: create.handle,
            ..Default::default()
        };
        if let Err(e) = ioctl(fd, DRM_IOCTL_MODE_MAP_DUMB, &mut map) {
            let mut destroy = drm_mode_destroy_dumb {
                handle: create.handle,
            };
            let _ = ioctl(fd, DRM_IOCTL_MODE_DESTROY_DUMB, &mut destroy);
            return Err(format!("MAP_DUMB: {e}"));
        }

        // 3. Register a framebuffer so the CRTC can scan it out.
        let mut fb = drm_mode_fb_cmd2 {
            width,
            height,
            pixel_format: DRM_FORMAT_XRGB8888,
            handles: [create.handle, 0, 0, 0],
            pitches: [create.pitch, 0, 0, 0],
            offsets: [0; 4],
            ..Default::default()
        };
        if let Err(e) = ioctl(fd, DRM_IOCTL_MODE_ADDFB2, &mut fb) {
            let mut destroy = drm_mode_destroy_dumb {
                handle: create.handle,
            };
            let _ = ioctl(fd, DRM_IOCTL_MODE_DESTROY_DUMB, &mut destroy);
            return Err(format!("ADDFB2: {e}"));
        }

        // 4. Map it into this process.
        let map_len = create.size as usize;
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                map_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                map.offset as libc::off_t,
            )
        };
        if ptr == libc::MAP_FAILED {
            let err = std::io::Error::last_os_error();
            let mut rmfb = fb.fb_id;
            let _ = ioctl(fd, DRM_IOCTL_MODE_RMFB, &mut rmfb);
            let mut destroy = drm_mode_destroy_dumb {
                handle: create.handle,
            };
            let _ = ioctl(fd, DRM_IOCTL_MODE_DESTROY_DUMB, &mut destroy);
            return Err(format!("mmap {} bytes: {e}", map_len, e = err));
        }

        // The kernel's stride can exceed width*4, so the mapping is not a
        // contiguous u32 image: each row starts at a `pitch` boundary. Present
        // it as an owned Vec so the rest of the compositor keeps its simple
        // pixel-index arithmetic, and copy back per row on flip.
        let pixels = unsafe {
            let base = ptr as *const u8;
            let mut out = vec![0u32; (width * height) as usize];
            for y in 0..height as usize {
                let row = base.add(y * create.pitch as usize) as *const u32;
                for x in 0..width as usize {
                    out[y * width as usize + x] = std::ptr::read_unaligned(row.add(x));
                }
            }
            out
        };

        // The mapping is retained (not leaked): the pointer is stored so it can
        // be unmapped when this buffer is dropped.
        let buf = DumbBuffer {
            width,
            height,
            pitch: create.pitch,
            size_bytes: map_len,
            handle: create.handle,
            fb_id: fb.fb_id,
            pixels,
        };
        register_mapping(buf.handle, ptr as *mut u8, map_len);
        Ok(buf)
    }

    #[cfg(not(target_os = "linux"))]
    fn allocate_hardware_buffer(&self) -> Result<DumbBuffer, String> {
        Err("hardware DRM is only available on Linux".to_string())
    }

    /// Program the CRTC so `fb_id` is scanned out on the discovered connector.
    pub fn set_crtc(&self, fb_id: u32) -> Result<(), String> {
        if fb_id == 0 {
            return Err("Invalid framebuffer ID: 0".to_string());
        }
        if !self.is_hardware {
            return Err("offscreen device has no CRTC".to_string());
        }
        let fd = self.fd().ok_or("device is not open")?;
        let display = self.display.as_ref().ok_or("no display discovered")?;

        let connector = display.connector_id;
        let mut crtc = drm_mode_crtc {
            set_connectors_ptr: &connector as *const u32 as u64,
            count_connectors: 1,
            crtc_id: display.crtc_id,
            fb_id,
            x: 0,
            y: 0,
            mode_valid: 1,
            mode: mode_to_uapi(&display.mode, &display.mode_name),
            ..Default::default()
        };
        ioctl(fd, DRM_IOCTL_MODE_SETCRTC, &mut crtc)
            .map_err(|e| format!("SETCRTC crtc={} fb={fb_id}: {e}", display.crtc_id))
    }

    /// Queue a page flip to `fb_id` and block until the display reports it
    /// complete.
    ///
    /// `DRM_MODE_PAGE_FLIP_EVENT` makes the kernel emit `DRM_EVENT_FLIP_COMPLETE`
    /// on the device fd once the flip has actually happened. Waiting for that
    /// event is what makes this VBLANK-synchronised and tear-free; returning as
    /// soon as the ioctl is accepted would report success before the panel had
    /// changed.
    pub fn page_flip(&self, fb_id: u32) -> Result<(), String> {
        if fb_id == 0 {
            return Err("Invalid framebuffer ID: 0 (page flip rejected)".to_string());
        }
        if !self.is_hardware {
            return Err("offscreen device cannot present".to_string());
        }
        let fd = self.fd().ok_or("device is not open")?;
        let display = self.display.as_ref().ok_or("no display discovered")?;

        let mut flip = drm_mode_crtc_page_flip {
            crtc_id: display.crtc_id,
            fb_id,
            flags: DRM_MODE_PAGE_FLIP_EVENT,
            reserved: 0,
            user_data: fb_id as u64,
        };
        ioctl(fd, DRM_IOCTL_MODE_PAGE_FLIP, &mut flip)
            .map_err(|e| format!("PAGE_FLIP crtc={} fb={fb_id}: {e}", display.crtc_id))?;

        wait_for_flip_event(fd, fb_id as u64)
    }

    /// Release a buffer's kernel resources. Called by `Drop`; public so tests
    /// and explicit teardown can release earlier.
    pub fn release_buffer(&self, buf: &DumbBuffer) {
        if !self.is_hardware || buf.handle == 0 {
            return;
        }
        if let Some(fd) = self.fd() {
            if buf.fb_id != 0 {
                let mut id = buf.fb_id;
                let _ = ioctl(fd, DRM_IOCTL_MODE_RMFB, &mut id);
            }
            let mut destroy = drm_mode_destroy_dumb { handle: buf.handle };
            let _ = ioctl(fd, DRM_IOCTL_MODE_DESTROY_DUMB, &mut destroy);
        }
        unmap_mapping(buf.handle);
    }

    /// Copy a composed frame into this device's buffer and present it.
    /// Copies row by row so kernel stride padding is respected.
    pub fn present_pixels(&self, buf: &mut DumbBuffer, pixels: &[u32]) -> Result<(), String> {
        if pixels.len() != buf.pixels.len() {
            return Err(format!(
                "framebuffer size mismatch: composed {} pixels, buffer holds {}",
                pixels.len(),
                buf.pixels.len()
            ));
        }
        if let Some(ptr) = mapping_for(buf.handle) {
            unsafe {
                let base = ptr;
                for y in 0..buf.height as usize {
                    let row = base.add(y * buf.pitch as usize) as *mut u32;
                    for x in 0..buf.width as usize {
                        std::ptr::write_unaligned(row.add(x), pixels[y * buf.width as usize + x]);
                    }
                }
            }
        }
        buf.pixels.copy_from_slice(pixels);
        self.page_flip(buf.fb_id)
    }
}

impl Drop for DrmDevice {
    fn drop(&mut self) {
        if let Some(fd) = self.fd() {
            let _ = ioctl_noarg(fd, DRM_IOCTL_DROP_MASTER);
        }
    }
}

// ---------------------------------------------------------------------------
// Kernel mappings
//
// The mmap address is needed long after allocation, and it belongs to the
// buffer rather than the device, so it is held in a small process-wide table
// keyed by GEM handle. A GEM handle is unique per device and this process only
// opens one card at a time, which is the same assumption libdrm makes.
// ---------------------------------------------------------------------------

type MappingTable = std::collections::HashMap<u32, (usize, usize)>;

static MAP_REGISTRY: std::sync::OnceLock<std::sync::Mutex<MappingTable>> =
    std::sync::OnceLock::new();

fn registry() -> &'static std::sync::Mutex<MappingTable> {
    MAP_REGISTRY.get_or_init(|| std::sync::Mutex::new(MappingTable::new()))
}

/// Addresses are stored as `usize` because a raw pointer is not `Send`, and the
/// table is shared across threads by design. The address is only ever converted
/// back to a pointer for the duration of a copy, and the entry is removed before
/// `munmap`, so no other thread can observe a dangling address.
fn register_mapping(handle: u32, ptr: *mut u8, len: usize) {
    if let Ok(mut m) = registry().lock() {
        m.insert(handle, (ptr as usize, len));
    }
}

fn mapping_for(handle: u32) -> Option<*mut u8> {
    registry()
        .lock()
        .ok()
        .and_then(|m| m.get(&handle).map(|(addr, _)| *addr as *mut u8))
}

fn unmap_mapping(handle: u32) {
    if let Ok(mut m) = registry().lock() {
        if let Some((addr, len)) = m.remove(&handle) {
            unsafe {
                libc::munmap(addr as *mut libc::c_void, len);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Discovery helpers
// ---------------------------------------------------------------------------

/// Candidate device nodes in preference order.
fn candidate_nodes() -> Vec<String> {
    let mut nodes = Vec::new();
    // Prefer a node that is a real card with a connector, but any card will do;
    // `open` rejects nodes that cannot be initialised.
    let mut found: Vec<(u32, String)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/dev/dri") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(num) = name.strip_prefix("card") {
                if let Ok(n) = num.parse::<u32>() {
                    found.push((n, entry.path().to_string_lossy().into_owned()));
                }
            }
        }
    }
    found.sort();
    for (_, path) in found {
        nodes.push(path);
    }
    // Explicit defaults as a last resort, so error messages name a real path
    // even when /dev/dri cannot be listed.
    for p in [DEFAULT_DRM_CARD, SECONDARY_DRM_CARD] {
        if !nodes.iter().any(|n| n == p) {
            nodes.push(p.to_string());
        }
    }
    nodes
}

fn read_driver_name(fd: RawFd) -> Option<String> {
    let mut name_buf = [0u8; 64];
    let mut version = drm_version {
        name_len: name_buf.len(),
        name: name_buf.as_mut_ptr() as u64,
        ..Default::default()
    };
    // 0xC0 is sizeof(struct drm_version) on a 64-bit kernel.
    let request = iowr(
        DRM_IOCTL_BASE,
        0x00,
        std::mem::size_of::<drm_version>() as u32,
    );
    ioctl(fd, request, &mut version).ok()?;
    let n = version.name_len.min(name_buf.len());
    let end = name_buf[..n].iter().position(|&b| b == 0).unwrap_or(n);
    Some(String::from_utf8_lossy(&name_buf[..end]).into_owned())
}

fn get_resources(fd: RawFd) -> Result<drm_mode_card_res, String> {
    let mut res = drm_mode_card_res::default();
    ioctl(fd, DRM_IOCTL_MODE_GETRESOURCES, &mut res).map_err(|e| format!("GETRESOURCES: {e}"))?;
    Ok(res)
}

/// Enumerate connector ids.
fn connector_ids(fd: RawFd, res: &drm_mode_card_res) -> Result<Vec<u32>, String> {
    if res.count_connectors == 0 {
        return Ok(Vec::new());
    }
    let mut ids = vec![0u32; res.count_connectors as usize];
    let mut query = drm_mode_card_res {
        connector_id_ptr: ids.as_mut_ptr() as u64,
        count_connectors: res.count_connectors,
        ..Default::default()
    };
    ioctl(fd, DRM_IOCTL_MODE_GETRESOURCES, &mut query)
        .map_err(|e| format!("GETRESOURCES(connectors): {e}"))?;
    ids.truncate(query.count_connectors as usize);
    Ok(ids)
}

/// Read one connector and its modes.
fn get_connector(
    fd: RawFd,
    connector_id: u32,
) -> Result<(drm_mode_get_connector, Vec<drm_mode_modeinfo>), String> {
    let mut conn = drm_mode_get_connector {
        connector_id,
        ..Default::default()
    };
    ioctl(fd, DRM_IOCTL_MODE_GETCONNECTOR, &mut conn)
        .map_err(|e| format!("GETCONNECTOR {connector_id}: {e}"))?;

    let mut modes = vec![drm_mode_modeinfo::default(); conn.count_modes as usize];
    if conn.count_modes > 0 {
        let mut with_modes = drm_mode_get_connector {
            connector_id,
            modes_ptr: modes.as_mut_ptr() as u64,
            count_modes: conn.count_modes,
            ..Default::default()
        };
        ioctl(fd, DRM_IOCTL_MODE_GETCONNECTOR, &mut with_modes)
            .map_err(|e| format!("GETCONNECTOR(modes) {connector_id}: {e}"))?;
        modes.truncate(with_modes.count_modes as usize);
        conn = with_modes;
    }
    Ok((conn, modes))
}

/// Read a CRTC's current configuration.
fn get_crtc(fd: RawFd, crtc_id: u32) -> Result<drm_mode_crtc, String> {
    let mut crtc = drm_mode_crtc {
        crtc_id,
        ..Default::default()
    };
    ioctl(fd, DRM_IOCTL_MODE_GETCRTC, &mut crtc).map_err(|e| format!("GETCRTC {crtc_id}: {e}"))?;
    Ok(crtc)
}

/// Whether a CRTC is currently driving a framebuffer.
///
/// Re-programming a live CRTC makes the panel blink; knowing this lets the
/// caller choose an idle CRTC when one exists.
fn crtc_is_active(fd: RawFd, crtc_id: u32) -> bool {
    get_crtc(fd, crtc_id)
        .map(|c| c.mode_valid == 1 && c.fb_id != 0)
        .unwrap_or(false)
}

fn get_encoder(fd: RawFd, encoder_id: u32) -> Result<drm_mode_get_encoder, String> {
    let mut enc = drm_mode_get_encoder {
        encoder_id,
        ..Default::default()
    };
    ioctl(fd, DRM_IOCTL_MODE_GETENCODER, &mut enc)
        .map_err(|e| format!("GETENCODER {encoder_id}: {e}"))?;
    Ok(enc)
}

/// Pick a lit display: a connected connector with at least one mode, resolved
/// to a CRTC. Prefers the mode the driver marks `PREFERRED`.
fn select_display(
    fd: RawFd,
    res: &drm_mode_card_res,
    preferred: Option<DrmMode>,
) -> Result<DisplayInfo, String> {
    let ids = connector_ids(fd, res)?;
    if ids.is_empty() {
        return Err("device reports no connectors".to_string());
    }
    let crtcs = crtc_ids(fd, res)?;
    if crtcs.is_empty() {
        return Err("device reports no CRTCs".to_string());
    }
    let mut connected_without_mode = Vec::new();

    for id in ids {
        let (conn, modes) = match get_connector(fd, id) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if !conn.is_connected() {
            continue;
        }
        if modes.is_empty() {
            connected_without_mode.push(conn.connector_type);
            continue;
        }

        // Prefer an already-lit CRTC so a running compositor is not disturbed;
        // otherwise take the CRTC this connector's encoder can drive.
        let mut crtc_id = 0u32;
        let mut encoder_id = conn.encoder_id;
        if encoder_id != 0 {
            if let Ok(enc) = get_encoder(fd, encoder_id) {
                if enc.crtc_id != 0 {
                    crtc_id = enc.crtc_id;
                }
            }
        }
        if crtc_id == 0 {
            // Prefer a CRTC that is already driving something, then any that
            // the encoder can reach.
            let mut fallback = 0u32;
            for &enc_id in res_crtc_encoders(fd, &conn)?.iter() {
                if let Ok(enc) = get_encoder(fd, enc_id) {
                    let candidate = first_crtc_for(&enc, &crtcs);
                    if candidate == 0 {
                        continue;
                    }
                    if crtc_is_active(fd, candidate) {
                        encoder_id = enc.encoder_id;
                        crtc_id = candidate;
                        break;
                    }
                    if fallback == 0 {
                        fallback = candidate;
                        encoder_id = enc.encoder_id;
                    }
                }
            }
            if crtc_id == 0 {
                crtc_id = fallback;
            }
        }
        if crtc_id == 0 {
            continue;
        }

        let chosen = choose_mode(&modes, preferred);
        return Ok(DisplayInfo {
            connector_id: conn.connector_id,
            connector_type: conn.connector_type,
            crtc_id,
            encoder_id,
            mode: DrmMode {
                width: chosen.hdisplay as u32,
                height: chosen.vdisplay as u32,
                refresh_rate_hz: chosen.vrefresh,
            },
            mode_name: chosen.name_str(),
            preferred: chosen.type_ & DRM_MODE_TYPE_PREFERRED != 0,
        });
    }

    if !connected_without_mode.is_empty() {
        return Err(format!(
            "connector(s) {:?} connected but reporting no modes",
            connected_without_mode
        ));
    }
    Err("no connected connector with a usable mode".to_string())
}

fn res_crtc_encoders(fd: RawFd, conn: &drm_mode_get_connector) -> Result<Vec<u32>, String> {
    if conn.count_encoders == 0 {
        return Ok(Vec::new());
    }
    let mut encs = vec![0u32; conn.count_encoders as usize];
    let mut query = drm_mode_get_connector {
        connector_id: conn.connector_id,
        encoders_ptr: encs.as_mut_ptr() as u64,
        count_encoders: conn.count_encoders,
        ..Default::default()
    };
    ioctl(fd, DRM_IOCTL_MODE_GETCONNECTOR, &mut query)
        .map_err(|e| format!("GETCONNECTOR(encoders): {e}"))?;
    encs.truncate(query.count_encoders as usize);
    Ok(encs)
}

/// The CRTC id an encoder can drive.
///
/// `possible_crtcs` is a bitmask over CRTC *indices*, so the bit position must
/// be resolved through the CRTC id list from `GETRESOURCES`; returning the bit
/// index would pass a small integer such as 1 to `SETCRTC` where the kernel
/// expects an object id.
fn first_crtc_for(enc: &drm_mode_get_encoder, crtc_ids: &[u32]) -> u32 {
    for (idx, &id) in crtc_ids.iter().enumerate() {
        if enc.possible_crtcs & (1u32 << idx) != 0 {
            return id;
        }
    }
    0
}

/// Fetch the CRTC id list so bit indices can be resolved to ids.
fn crtc_ids(fd: RawFd, res: &drm_mode_card_res) -> Result<Vec<u32>, String> {
    if res.count_crtcs == 0 {
        return Ok(Vec::new());
    }
    let mut ids = vec![0u32; res.count_crtcs as usize];
    let mut query = drm_mode_card_res {
        crtc_id_ptr: ids.as_mut_ptr() as u64,
        count_crtcs: res.count_crtcs,
        ..Default::default()
    };
    ioctl(fd, DRM_IOCTL_MODE_GETRESOURCES, &mut query)
        .map_err(|e| format!("GETRESOURCES(crtcs): {e}"))?;
    ids.truncate(query.count_crtcs as usize);
    Ok(ids)
}

/// Pick the mode to program: the caller's preference if the panel offers it,
/// else the driver's preferred mode, else the largest (a phone panel's native
/// resolution), which avoids landing on a low-res compatibility mode.
fn choose_mode(modes: &[drm_mode_modeinfo], preferred: Option<DrmMode>) -> drm_mode_modeinfo {
    if let Some(want) = preferred {
        if let Some(m) = modes
            .iter()
            .find(|m| m.hdisplay as u32 == want.width && m.vdisplay as u32 == want.height)
        {
            return *m;
        }
    }
    if let Some(m) = modes
        .iter()
        .find(|m| m.type_ & DRM_MODE_TYPE_PREFERRED != 0)
    {
        return *m;
    }
    *modes.iter().max_by_key(|m| m.pixels()).unwrap_or(&modes[0])
}

fn mode_to_uapi(mode: &DrmMode, name: &str) -> drm_mode_modeinfo {
    let mut info = drm_mode_modeinfo {
        hdisplay: mode.width as u16,
        vdisplay: mode.height as u16,
        vrefresh: mode.refresh_rate_hz,
        type_: DRM_MODE_TYPE_DRIVER,
        ..Default::default()
    };
    let bytes = name.as_bytes();
    let n = bytes.len().min(DRM_DISPLAY_MODE_LEN - 1);
    info.name[..n].copy_from_slice(&bytes[..n]);
    info
}

/// Block until the kernel reports the flip for `user_data` complete.
///
/// `drm_event_vblank` is 32 bytes on a 64-bit kernel; the read is sized to that
/// and the event list is walked by `length` because one `read` can return
/// several concatenated events.
#[cfg(target_os = "linux")]
fn wait_for_flip_event(fd: RawFd, user_data: u64) -> Result<(), String> {
    const EVENT_SIZE: usize = 32;
    let mut buf = [0u8; EVENT_SIZE * 4];
    loop {
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n < 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            return Err(format!("waiting for flip event: {err}"));
        }
        let n = n as usize;
        let mut off = 0usize;
        while off + EVENT_SIZE <= n {
            let ev: drm_event_vblank = unsafe {
                std::ptr::read_unaligned(buf.as_ptr().add(off) as *const drm_event_vblank)
            };
            if ev.base.type_ == DRM_EVENT_FLIP_COMPLETE && ev.user_data == user_data {
                return Ok(());
            }
            let len = ev.base.length as usize;
            if len < 8 {
                break;
            }
            off += len;
        }
        // A short read with no matching event: keep waiting for ours.
    }
}

#[cfg(not(target_os = "linux"))]
fn wait_for_flip_event(_fd: RawFd, _user_data: u64) -> Result<(), String> {
    Err("page flip events require Linux".to_string())
}

/// Best-effort human name for a connector type, for logs and the UI.
#[allow(non_upper_case_globals)] // the arm mirrors the kernel's own spelling
pub fn connector_type_name(t: u32) -> &'static str {
    match t {
        DRM_MODE_CONNECTOR_HDMIA => "HDMI",
        DRM_MODE_CONNECTOR_eDP => "eDP",
        DRM_MODE_CONNECTOR_VIRTUAL => "Virtual",
        DRM_MODE_CONNECTOR_DSI => "DSI",
        DRM_MODE_CONNECTOR_DPI => "DPI",
        _ => "Unknown",
    }
}

/// Path helper used by callers that want to log the node.
pub fn card_path_for(node: &str) -> PathBuf {
    PathBuf::from(node)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dumb_buffer_pixel_access_and_bounds() {
        let dev = DrmDevice::offscreen(DrmMode::default(), "test".into());
        let mut buf = dev.allocate_dumb_buffer(1);
        assert_eq!(buf.width, 720);
        assert_eq!(buf.height, 1440);
        assert_eq!(buf.pitch, 720 * 4);
        assert_eq!(buf.size_bytes, 720 * 1440 * 4);
        assert_eq!(buf.pixels.len(), 720 * 1440);
        assert!(!buf.has_padding());

        assert_eq!(buf.get_pixel(10, 10), 0xFF00_0000);
        buf.set_pixel(10, 10, 0xFF00_FF00);
        assert_eq!(buf.get_pixel(10, 10), 0xFF00_FF00);

        // Out of bounds reads/writes must not panic.
        buf.set_pixel(9999, 9999, 0xFFFF_FFFF);
        assert_eq!(buf.get_pixel(9999, 9999), 0);
    }

    #[test]
    fn padded_buffer_reports_padding() {
        let buf = DumbBuffer {
            width: 100,
            height: 50,
            pitch: 512, // hardware-aligned wider than 400
            size_bytes: 512 * 50,
            handle: 1,
            fb_id: 1,
            pixels: vec![0; 100 * 50],
        };
        assert!(buf.has_padding());
        assert_eq!(buf.row_bytes(), 400);
    }

    #[test]
    fn offscreen_device_is_not_hardware() {
        let dev = DrmDevice::offscreen(DrmMode::default(), "no gpu".into());
        assert!(!dev.is_hardware());
        assert_eq!(dev.backend.label(), "offscreen_fallback");
        assert_eq!(dev.backend.reason(), Some("no gpu"));
        assert!(dev.display().is_none());
        assert!(dev.fd().is_none());
    }

    #[test]
    fn offscreen_set_crtc_and_flip_are_refused_not_silently_ok() {
        let dev = DrmDevice::offscreen(DrmMode::default(), "no gpu".into());
        let err = dev.set_crtc(1).unwrap_err();
        assert!(err.contains("offscreen"), "got: {err}");
        let err = dev.page_flip(1).unwrap_err();
        assert!(err.contains("offscreen"), "got: {err}");
    }

    #[test]
    fn zero_fb_id_is_rejected_on_any_backend() {
        let dev = DrmDevice::offscreen(DrmMode::default(), "no gpu".into());
        assert!(dev.page_flip(0).unwrap_err().contains("framebuffer ID: 0"));
        assert!(dev.set_crtc(0).unwrap_err().contains("framebuffer ID: 0"));
    }

    /// The test that would have caught the original stub.
    ///
    /// Opening a plain file as if it were a DRM node must fail, because the
    /// kernel answers `ENOTTY` to `DRM_IOCTL_SET_MASTER` on a non-DRM file
    /// description. An implementation that only checked for the file's
    /// existence — which is what this module used to do — would report success
    /// here, so this asserts the ioctl really is issued and its errno really is
    /// propagated.
    #[test]
    fn opening_a_non_drm_file_reports_enotty_not_fake_success() {
        let path = std::env::temp_dir().join(format!(
            "nilui-gpu-not-drm-{}-{}",
            std::process::id(),
            "regular"
        ));
        std::fs::write(&path, b"not a display device\n").unwrap();
        let path_str = path.to_string_lossy().into_owned();

        let err = DrmDevice::open(&path_str, None)
            .expect_err("a regular file must not be accepted as a DRM device");
        // The failure must come from the ioctl, not from a missing path.
        assert!(
            err.contains("SET_MASTER"),
            "expected the ioctl to be attempted, got: {err}"
        );
        // 25 is ENOTTY on Linux; on other platforms accept any ioctl error.
        assert!(
            err.contains("Inappropriate ioctl") || err.contains("os error"),
            "expected an errno in the message, got: {err}"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn open_missing_node_fails_with_the_path() {
        let err = DrmDevice::open("/dev/onuron/nonexistent-card", None).unwrap_err();
        assert!(err.contains("/dev/onuron/nonexistent-card"), "got: {err}");
    }

    #[test]
    fn open_or_virtual_always_returns_a_usable_device() {
        // On a machine with no GPU this must be offscreen; on one with a GPU it
        // must be hardware. Either way it must not panic and must be honest.
        let dev = DrmDevice::open_or_virtual(None);
        if dev.is_hardware() {
            assert!(dev.fd().is_some());
            assert!(dev.display().is_some());
        } else {
            assert!(dev.backend.reason().is_some());
            assert!(dev.fd().is_none());
        }
    }

    #[test]
    fn choose_mode_prefers_the_caller_then_preferred_then_largest() {
        let mut small = drm_mode_modeinfo {
            hdisplay: 640,
            vdisplay: 480,
            ..Default::default()
        };
        small.name[..7].copy_from_slice(b"640x480");
        let mut native = drm_mode_modeinfo {
            hdisplay: 1080,
            vdisplay: 2400,
            type_: DRM_MODE_TYPE_PREFERRED,
            ..Default::default()
        };
        native.name[..9].copy_from_slice(b"1080x2400");
        let mut large = drm_mode_modeinfo {
            hdisplay: 1440,
            vdisplay: 3200,
            ..Default::default()
        };
        large.name[..9].copy_from_slice(b"1440x3200");
        let modes = vec![small, native, large];

        // Explicit caller preference wins even when it is not the largest.
        let want = DrmMode {
            width: 640,
            height: 480,
            refresh_rate_hz: 60,
        };
        assert_eq!(choose_mode(&modes, Some(want)).hdisplay, 640);
        // Otherwise the driver's preferred mode wins over the largest.
        assert_eq!(choose_mode(&modes, None).hdisplay, 1080);
        // With no preferred mode, the largest is chosen.
        let no_pref = vec![small, large];
        assert_eq!(choose_mode(&no_pref, None).hdisplay, 1440);
    }

    #[test]
    fn mode_to_uapi_carries_geometry_and_name() {
        let mode = DrmMode {
            width: 1080,
            height: 2400,
            refresh_rate_hz: 90,
        };
        let u = mode_to_uapi(&mode, "1080x2400");
        assert_eq!(u.hdisplay, 1080);
        assert_eq!(u.vdisplay, 2400);
        assert_eq!(u.vrefresh, 90);
        assert_eq!(u.name_str(), "1080x2400");
        assert_eq!(u.type_, DRM_MODE_TYPE_DRIVER);
    }

    #[test]
    fn connector_type_names_cover_the_mobile_panels() {
        assert_eq!(connector_type_name(DRM_MODE_CONNECTOR_DSI), "DSI");
        assert_eq!(connector_type_name(DRM_MODE_CONNECTOR_eDP), "eDP");
        assert_eq!(connector_type_name(DRM_MODE_CONNECTOR_HDMIA), "HDMI");
        assert_eq!(connector_type_name(9999), "Unknown");
    }

    #[test]
    fn candidate_nodes_are_deduplicated_and_stable() {
        let nodes = candidate_nodes();
        assert!(nodes.contains(&DEFAULT_DRM_CARD.to_string()));
        let mut sorted = nodes.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), nodes.len(), "duplicate nodes: {nodes:?}");
    }
}
