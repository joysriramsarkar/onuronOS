// runtime/nilui-gpu/src/ioctl.rs — Linux ioctl request encoding and invocation.
//
// The `_IOC` encoding is reproduced here rather than taken from a crate so that
// it can be diffed by eye against `/usr/include/asm-generic/ioctl.h`. The tests
// at the bottom pin every request this crate issues to the number the kernel's
// own headers produce (captured by compiling a C program that includes
// <drm/drm.h> and <drm/drm_mode.h>), so a typo in a shift or a size cannot
// silently reach a real display device.

use std::io;
use std::os::unix::io::RawFd;

pub const IOC_NRBITS: u32 = 8;
pub const IOC_TYPEBITS: u32 = 8;
pub const IOC_SIZEBITS: u32 = 14;
pub const IOC_DIRBITS: u32 = 2;

pub const IOC_NRSHIFT: u32 = 0;
pub const IOC_TYPESHIFT: u32 = IOC_NRSHIFT + IOC_NRBITS;
pub const IOC_SIZESHIFT: u32 = IOC_TYPESHIFT + IOC_TYPEBITS;
pub const IOC_DIRSHIFT: u32 = IOC_SIZESHIFT + IOC_SIZEBITS;

pub const IOC_NONE: u32 = 0;
pub const IOC_WRITE: u32 = 1;
pub const IOC_READ: u32 = 2;

#[inline]
pub const fn ioc(dir: u32, ty: u32, nr: u32, size: u32) -> u64 {
    ((dir as u64) << IOC_DIRSHIFT)
        | ((ty as u64) << IOC_TYPESHIFT)
        | ((nr as u64) << IOC_NRSHIFT)
        | ((size as u64) << IOC_SIZESHIFT)
}

/// `_IO(ty, nr)` — no argument.
pub const fn io(ty: u32, nr: u32) -> u64 {
    ioc(IOC_NONE, ty, nr, 0)
}

/// `_IOW(ty, nr, size)` — kernel reads the argument.
pub const fn iow(ty: u32, nr: u32, size: u32) -> u64 {
    ioc(IOC_WRITE, ty, nr, size)
}

/// `_IOR(ty, nr, size)` — kernel writes the argument.
pub const fn ior(ty: u32, nr: u32, size: u32) -> u64 {
    ioc(IOC_READ, ty, nr, size)
}

/// `_IOWR(ty, nr, size)` — kernel reads and writes the argument.
pub const fn iowr(ty: u32, nr: u32, size: u32) -> u64 {
    ioc(IOC_READ | IOC_WRITE, ty, nr, size)
}

/// Issue an ioctl whose argument is a pointer to `T`.
pub fn ioctl<T>(fd: RawFd, request: u64, arg: &mut T) -> io::Result<()> {
    // `as _` rather than `as c_ulong`: glibc declares the request parameter as
    // `unsigned long` while musl declares it `int`. Letting inference pick the
    // declared type keeps this correct on both.
    let rc = unsafe { libc::ioctl(fd, request as _, arg as *mut T) };
    if rc < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Issue an ioctl that takes no argument (`DRM_IOCTL_SET_MASTER`/`DROP_MASTER`).
pub fn ioctl_noarg(fd: RawFd, request: u64) -> io::Result<()> {
    let rc = unsafe { libc::ioctl(fd, request as _) };
    if rc < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Values captured from a C program compiled against the running kernel's
    // UAPI headers. They are the specification this module must match.
    #[test]
    fn drm_ioctl_numbers_match_the_kernel_headers() {
        const DRM: u32 = b'd' as u32;
        // struct sizes, taken from the same C program.
        assert_eq!(iowr(DRM, 0xA0, 64), 0xc040_64a0); // MODE_GETRESOURCES
        assert_eq!(iowr(DRM, 0xA1, 104), 0xc068_64a1); // MODE_GETCRTC
        assert_eq!(iowr(DRM, 0xA2, 104), 0xc068_64a2); // MODE_SETCRTC
        assert_eq!(iowr(DRM, 0xA6, 20), 0xc014_64a6); // MODE_GETENCODER
        assert_eq!(iowr(DRM, 0xA7, 80), 0xc050_64a7); // MODE_GETCONNECTOR
        assert_eq!(iowr(DRM, 0xAF, 4), 0xc004_64af); // MODE_RMFB
        assert_eq!(iowr(DRM, 0xB0, 24), 0xc018_64b0); // MODE_PAGE_FLIP
        assert_eq!(iowr(DRM, 0xB2, 32), 0xc020_64b2); // MODE_CREATE_DUMB
        assert_eq!(iowr(DRM, 0xB3, 16), 0xc010_64b3); // MODE_MAP_DUMB
        assert_eq!(iowr(DRM, 0xB4, 4), 0xc004_64b4); // MODE_DESTROY_DUMB
        assert_eq!(iowr(DRM, 0xB8, 104), 0xc068_64b8); // MODE_ADDFB2
        assert_eq!(io(DRM, 0x1e), 0x0000_641e); // SET_MASTER
        assert_eq!(io(DRM, 0x1f), 0x0000_641f); // DROP_MASTER
    }

    #[test]
    fn direction_bits_land_where_the_kernel_expects() {
        assert_eq!(ioc(IOC_READ, 0, 0, 0) >> 30, 2);
        assert_eq!(ioc(IOC_WRITE, 0, 0, 0) >> 30, 1);
        assert_eq!(ioc(IOC_READ | IOC_WRITE, 0, 0, 0) >> 30, 3);
        // type occupies bits 8..16, nr bits 0..8, size bits 16..30
        assert_eq!(ioc(0, 0xAB, 0, 0) >> 8 & 0xFF, 0xAB);
        assert_eq!(ioc(0, 0, 0xCD, 0) & 0xFF, 0xCD);
        assert_eq!(ioc(0, 0, 0, 0x1234) >> 16 & 0x3FFF, 0x1234);
    }
}
