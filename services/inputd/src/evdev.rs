// services/inputd/src/evdev.rs — Real Linux evdev input reader
//
// Opens `/dev/input/event*` directly and translates raw `struct input_event`
// records into the tracker's ABS/KEY callbacks. This is the only path that
// makes touch actually work on a physical device: the previous code only
// listed the device nodes and never read from them.
//
// Design notes:
//   * Every device is opened O_RDONLY|O_NONBLOCK, then poll(2)'d, so one thread
//     per device blocks cheaply without a busy loop.
//   * Capability detection uses EVIOCGBIT, not the device name, so a device is
//     classified by what it can report rather than by a vendor string.
//   * Axis ranges come from EVIOCGABS, so a digitizer that reports 0..4095 is
//     scaled correctly instead of being assumed to be pixel-addressed.
//   * Multi-touch Protocol B (ABS_MT_SLOT / ABS_MT_TRACKING_ID) and the legacy
//     single-touch protocol (ABS_X/ABS_Y + BTN_TOUCH) are both handled.

use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::{
    MultiTouchTracker, ABS_MT_POSITION_X, ABS_MT_POSITION_Y, ABS_MT_SLOT, ABS_MT_TRACKING_ID,
    ABS_X, ABS_Y, BTN_TOUCH, EV_ABS, EV_KEY, EV_SYN, SYN_REPORT,
};

// ─── Linux input subsystem constants ─────────────────────────────────────────

const EV_MAX: u16 = 0x1f;
const KEY_MAX: u16 = 0x2ff;
const ABS_MAX: u16 = 0x3f;

const BTN_LEFT: u16 = 0x110;
const BTN_TOOL_FINGER: u16 = 0x145;

// ioctl request encoders (asm-generic, valid for x86_64 and aarch64).
// The request word is `libc::Ioctl`: `c_ulong` on glibc, `c_int` on musl.
const IOC_NRBITS: u32 = 8;
const IOC_TYPEBITS: u32 = 8;
const IOC_SIZEBITS: u32 = 14;
const IOC_NRSHIFT: u32 = 0;
const IOC_TYPESHIFT: u32 = IOC_NRSHIFT + IOC_NRBITS;
const IOC_SIZESHIFT: u32 = IOC_TYPESHIFT + IOC_TYPEBITS;
const IOC_DIRSHIFT: u32 = IOC_SIZESHIFT + IOC_SIZEBITS;
const IOC_READ: u32 = 2;

const fn ioc(dir: u32, ty: u32, nr: u32, size: u32) -> libc::Ioctl {
    ((dir << IOC_DIRSHIFT) | (ty << IOC_TYPESHIFT) | (nr << IOC_NRSHIFT) | (size << IOC_SIZESHIFT))
        as libc::Ioctl
}

/// EVIOCGBIT(ev, len): bitmap of event types (ev == 0) or codes for `ev`.
const fn eviocgbit(ev: u32, len: u32) -> libc::Ioctl {
    ioc(IOC_READ, b'E' as u32, 0x20 + ev, len)
}

/// EVIOCGABS(abs): `struct input_absinfo` for one absolute axis.
const fn eviocgabs(abs: u32) -> libc::Ioctl {
    ioc(
        IOC_READ,
        b'E' as u32,
        0x40 + abs,
        std::mem::size_of::<InputAbsInfo>() as u32,
    )
}

const fn eviocgname(len: u32) -> libc::Ioctl {
    ioc(IOC_READ, b'E' as u32, 0x06, len)
}

/// `struct input_absinfo` — 6 x i32.
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct InputAbsInfo {
    pub value: i32,
    pub minimum: i32,
    pub maximum: i32,
    pub fuzz: i32,
    pub flat: i32,
    pub resolution: i32,
}

/// `struct input_event`. On 64-bit Linux `timeval` is two i64s (16 bytes) so the
/// record is 24 bytes; on 32-bit it is two i32s and the record is 16 bytes.
#[cfg(target_pointer_width = "64")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct InputEventRaw {
    pub tv_sec: i64,
    pub tv_usec: i64,
    pub type_: u16,
    pub code: u16,
    pub value: i32,
}

#[cfg(target_pointer_width = "32")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct InputEventRaw {
    pub tv_sec: i32,
    pub tv_usec: i32,
    pub type_: u16,
    pub code: u16,
    pub value: i32,
}

pub const INPUT_EVENT_SIZE: usize = std::mem::size_of::<InputEventRaw>();

/// Poll the fd for readability with a millisecond timeout.
fn poll_readable(fd: RawFd, timeout_ms: i32) -> io::Result<bool> {
    let mut pfd = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: pfd is a valid, initialized pollfd for the duration of the call.
    let rc = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
    if rc < 0 {
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::Interrupted {
            return Ok(false);
        }
        return Err(err);
    }
    Ok(rc > 0 && (pfd.revents & (libc::POLLIN | libc::POLLERR | libc::POLLHUP)) != 0)
}

fn read_bits(fd: RawFd, ev: u32, nbits: usize) -> io::Result<Vec<u8>> {
    let mut buf = vec![0u8; nbits.div_ceil(8)];
    // SAFETY: buf is a writable slice of exactly the length encoded in the
    // ioctl request, which is what the kernel expects for EVIOCGBIT.
    let rc = unsafe { libc::ioctl(fd, eviocgbit(ev, buf.len() as u32), buf.as_mut_ptr()) };
    if rc < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(buf)
}

fn bit_set(bits: &[u8], idx: u16) -> bool {
    let byte = (idx / 8) as usize;
    byte < bits.len() && (bits[byte] >> (idx % 8)) & 1 == 1
}

fn read_abs(fd: RawFd, axis: u32) -> io::Result<InputAbsInfo> {
    let mut info = InputAbsInfo::default();
    // SAFETY: info is a properly aligned input_absinfo, matching the ioctl size.
    let rc = unsafe { libc::ioctl(fd, eviocgabs(axis), &mut info) };
    if rc < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(info)
}

fn read_name(fd: RawFd) -> String {
    let mut buf = [0u8; 256];
    // SAFETY: buf is 256 writable bytes and the request length matches.
    let rc = unsafe { libc::ioctl(fd, eviocgname(buf.len() as u32), buf.as_mut_ptr()) };
    if rc <= 0 {
        return "unknown".to_string();
    }
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).trim().to_string()
}

/// What a device node is able to report. Decided from EVIOCGBIT, not its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceClass {
    /// Multi-touch or single-touch digitiser with absolute X/Y.
    Touchscreen,
    /// Relative pointer (mouse, trackpad, virtio tablet in absolute mode is Touchscreen).
    Pointer,
    /// Hardware key / button source (power, volume).
    Keys,
    /// Nothing we consume.
    Ignored,
}

pub fn classify(fd: RawFd) -> io::Result<DeviceClass> {
    let ev_bits = read_bits(fd, 0, (EV_MAX + 1) as usize)?;
    let has_abs = bit_set(&ev_bits, EV_ABS);
    let has_key = bit_set(&ev_bits, EV_KEY);

    if has_abs {
        let abs_bits = read_bits(fd, EV_ABS as u32, (ABS_MAX + 1) as usize)?;
        if bit_set(&abs_bits, ABS_MT_POSITION_X) && bit_set(&abs_bits, ABS_MT_POSITION_Y) {
            return Ok(DeviceClass::Touchscreen);
        }
        if bit_set(&abs_bits, ABS_X) && bit_set(&abs_bits, ABS_Y) {
            return Ok(DeviceClass::Touchscreen);
        }
    }

    if has_key {
        let key_bits = read_bits(fd, EV_KEY as u32, (KEY_MAX + 1) as usize)?;
        if bit_set(&key_bits, BTN_TOUCH)
            || bit_set(&key_bits, BTN_TOOL_FINGER)
            || bit_set(&key_bits, BTN_LEFT)
        {
            return Ok(DeviceClass::Pointer);
        }
        return Ok(DeviceClass::Keys);
    }

    Ok(DeviceClass::Ignored)
}

/// A single open evdev device.
pub struct EvdevDevice {
    pub path: PathBuf,
    pub name: String,
    pub class: DeviceClass,
    pub fd: RawFd,
    _file: File,
    /// Digitizer axis ranges, used to normalise raw coordinates.
    pub x_range: (i32, i32),
    pub y_range: (i32, i32),
    /// True when the device speaks Multi-Touch Protocol B (has ABS_MT_SLOT).
    pub multitouch_b: bool,
    /// Current slot for protocol-B devices.
    current_slot: u32,
    /// Last ABS_X/ABS_Y seen, for legacy single-touch release handling.
    legacy_x: i32,
    legacy_y: i32,
    legacy_touching: bool,
}

impl EvdevDevice {
    /// Open `path` and describe it. Returns `Ok(None)` when the node exists but
    /// carries nothing this daemon consumes.
    pub fn open(path: &Path) -> io::Result<Option<Self>> {
        let file = OpenOptions::new().read(true).open(path)?;
        let fd = file.as_raw_fd();

        // Non-blocking so a slow device never stalls the polling loop.
        // SAFETY: fd is a valid open descriptor owned by `file`.
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFL);
            if flags >= 0 {
                libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
        }

        let class = classify(fd).unwrap_or(DeviceClass::Ignored);
        if class == DeviceClass::Ignored {
            return Ok(None);
        }

        let name = read_name(fd);

        let abs_bits = read_bits(fd, EV_ABS as u32, (ABS_MAX + 1) as usize).unwrap_or_default();
        let multitouch_b = bit_set(&abs_bits, ABS_MT_SLOT);

        let mut x_range = (0, 0);
        let mut y_range = (0, 0);
        if class == DeviceClass::Touchscreen {
            let x_axis = if multitouch_b {
                ABS_MT_POSITION_X as u32
            } else {
                ABS_X as u32
            };
            let y_axis = if multitouch_b {
                ABS_MT_POSITION_Y as u32
            } else {
                ABS_Y as u32
            };
            if let Ok(info) = read_abs(fd, x_axis) {
                x_range = (info.minimum, info.maximum);
            }
            if let Ok(info) = read_abs(fd, y_axis) {
                y_range = (info.minimum, info.maximum);
            }
        }

        Ok(Some(Self {
            path: path.to_path_buf(),
            name,
            class,
            fd,
            _file: file,
            x_range,
            y_range,
            multitouch_b,
            current_slot: 0,
            legacy_x: 0,
            legacy_y: 0,
            legacy_touching: false,
        }))
    }

    /// Read every pending record and feed it to `tracker`.
    ///
    /// Returns the number of records consumed. `Ok(0)` means nothing was
    /// pending; a device that was unplugged surfaces as an `Err`.
    pub fn drain(&mut self, tracker: &Arc<Mutex<MultiTouchTracker>>) -> io::Result<usize> {
        let mut consumed = 0usize;
        let mut raw = [0u8; 64];
        assert!(
            INPUT_EVENT_SIZE <= raw.len(),
            "input_event must fit in the read buffer"
        );

        loop {
            // SAFETY: raw is a valid writable buffer; the kernel writes at most
            // INPUT_EVENT_SIZE bytes for a complete record.
            let n = unsafe {
                libc::read(
                    self.fd,
                    raw.as_mut_ptr() as *mut libc::c_void,
                    INPUT_EVENT_SIZE,
                )
            };

            if n < 0 {
                let err = io::Error::last_os_error();
                match err.kind() {
                    io::ErrorKind::WouldBlock => break,
                    io::ErrorKind::Interrupted => continue,
                    _ => return Err(err),
                }
            }
            if n == 0 {
                break;
            }
            if (n as usize) < INPUT_EVENT_SIZE {
                // A partial record can only happen at device removal.
                break;
            }

            // SAFETY: the kernel wrote exactly INPUT_EVENT_SIZE bytes into raw,
            // and InputEventRaw is repr(C) with that exact size and no padding.
            let ev: InputEventRaw =
                unsafe { std::ptr::read_unaligned(raw.as_ptr() as *const InputEventRaw) };
            consumed += 1;
            self.dispatch(ev, tracker);
        }

        Ok(consumed)
    }

    fn dispatch(&mut self, ev: InputEventRaw, tracker: &Arc<Mutex<MultiTouchTracker>>) {
        match ev.type_ {
            t if t == EV_ABS => {
                if self.class != DeviceClass::Touchscreen {
                    return;
                }
                let mut guard = match tracker.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                if ev.code == ABS_MT_SLOT {
                    self.current_slot = ev.value.max(0) as u32;
                } else if !self.multitouch_b && (ev.code == ABS_X || ev.code == ABS_Y) {
                    if ev.code == ABS_X {
                        self.legacy_x = ev.value;
                    } else {
                        self.legacy_y = ev.value;
                    }
                }
                // The tracker owns slot bookkeeping; give it the raw record.
                let _ = guard.handle_abs(ev.code, ev.value);
            }
            t if t == EV_KEY => {
                if ev.code == BTN_TOUCH && !self.multitouch_b {
                    // Legacy protocol: BTN_TOUCH 1 = down, 0 = up.
                    let mut guard = match tracker.lock() {
                        Ok(g) => g,
                        Err(p) => p.into_inner(),
                    };
                    if ev.value == 1 {
                        self.legacy_touching = true;
                        let _ = guard.handle_abs(ABS_X, self.legacy_x);
                        let _ = guard.handle_abs(ABS_Y, self.legacy_y);
                        let _ = guard.handle_abs(ABS_MT_TRACKING_ID, 0);
                    } else {
                        self.legacy_touching = false;
                        let _ = guard.handle_abs(ABS_MT_TRACKING_ID, -1);
                    }
                    return;
                }
                let mut guard = match tracker.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                let _ = guard.handle_key(ev.code, ev.value);
            }
            t if t == EV_SYN => {
                if ev.code != SYN_REPORT {
                    return;
                }
                let mut guard = match tracker.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                let _ = guard.handle_syn();
            }
            _ => {}
        }
    }
}

/// Enumerate `/dev/input` and open every device this daemon can consume.
pub fn open_all(dir: &Path) -> Vec<EvdevDevice> {
    let mut devices = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return devices,
    };

    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("event"))
                .unwrap_or(false)
        })
        .collect();
    paths.sort();

    for path in paths {
        match EvdevDevice::open(&path) {
            Ok(Some(dev)) => devices.push(dev),
            Ok(None) => {}
            Err(_) => {}
        }
    }
    devices
}

/// Blocking-per-device read loop. One thread per device.
pub fn spawn_reader(dev: EvdevDevice, tracker: Arc<Mutex<MultiTouchTracker>>) {
    std::thread::spawn(move || {
        let mut dev = dev;
        loop {
            match poll_readable(dev.fd, 1000) {
                Ok(true) => {
                    if dev.drain(&tracker).is_err() {
                        break;
                    }
                }
                Ok(false) => continue,
                Err(_) => break,
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_event_layout_matches_kernel_abi() {
        // The kernel ABI for struct input_event is 24 bytes on 64-bit Linux and
        // 16 bytes on 32-bit. Getting this wrong silently misparses every event.
        #[cfg(target_pointer_width = "64")]
        assert_eq!(INPUT_EVENT_SIZE, 24);
        #[cfg(target_pointer_width = "32")]
        assert_eq!(INPUT_EVENT_SIZE, 16);
    }

    #[test]
    fn ioctl_requests_match_linux_uapi() {
        // Cross-checked against <linux/input.h> on a 64-bit build:
        //   EVIOCGBIT(0, 8)  == 0x80084520
        //   EVIOCGABS(0)     == 0x80184540
        //   EVIOCGNAME(256)  == 0x81004506
        // Compare the 32-bit request word: the C type is `c_ulong` on glibc
        // and `c_int` on musl, but the encoded value is identical.
        assert_eq!(eviocgbit(0, 8) as u32, 0x80084520);
        assert_eq!(eviocgabs(0) as u32, 0x80184540);
        assert_eq!(eviocgname(256) as u32, 0x81004506);
    }

    #[test]
    fn bit_set_reads_little_endian_bitmap() {
        // ABS_MT_POSITION_X == 0x35 -> byte 6, bit 5.
        let mut bits = vec![0u8; 8];
        bits[6] = 1 << 5;
        assert!(bit_set(&bits, ABS_MT_POSITION_X));
        assert!(!bit_set(&bits, ABS_MT_POSITION_Y));
        // Out-of-range indices must not panic.
        assert!(!bit_set(&bits, 0x3ff));
    }

    #[test]
    fn open_all_returns_empty_for_missing_directory() {
        let devices = open_all(Path::new("/definitely/not/a/directory"));
        assert!(devices.is_empty());
    }
}
