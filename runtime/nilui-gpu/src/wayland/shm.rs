// runtime/nilui-gpu/src/wayland/shm.rs — wl_shm client buffer pools
//
// A Wayland client hands the compositor pixels by creating a shared-memory
// pool, passing the pool's fd in `wl_shm.create_pool`, and then naming
// rectangles inside it with `wl_shm_pool.create_buffer`. The compositor maps
// that fd and reads the pixels directly.
//
// The fd arrives over SCM_RIGHTS, so it is a descriptor into this process's fd
// table pointing at the *same* underlying memory object the client created.
// Mapping it read-only is deliberate: the compositor has no reason to write
// into a client's pool, and a read-only mapping means a bug here cannot corrupt
// a client's buffer.
//
// Validation matters more than it looks. A client controls `width`, `height`,
// `stride`, and `offset` entirely, and a stride smaller than `width * 4` would
// make row `n` read from the middle of row `n-1`. Every one of those fields is
// checked against the pool size before a single pixel is read.

use std::os::fd::RawFd;

/// Bytes per pixel for the formats we advertise.
pub const BYTES_PER_PIXEL: u32 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShmError {
    /// The fd could not be mapped at the requested length.
    MapFailed { len: usize, errno: i32 },
    /// `stride` is smaller than one row of `width` pixels.
    StrideTooSmall { stride: u32, width: u32, min: u32 },
    /// `offset + height * stride` runs past the end of the pool.
    BufferExceedsPool { needed: u64, pool_size: u64 },
    /// Width or height was zero.
    ZeroSize,
    /// The pool size was zero, so it can hold nothing.
    EmptyPool,
    /// The format is not one this compositor advertised.
    UnsupportedFormat(u32),
}

impl std::fmt::Display for ShmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShmError::MapFailed { len, errno } => {
                write!(f, "mmap of {len} bytes failed (errno {errno})")
            }
            ShmError::StrideTooSmall { stride, width, min } => write!(
                f,
                "stride {stride} is smaller than the {min} bytes needed for {width} pixels"
            ),
            ShmError::BufferExceedsPool { needed, pool_size } => write!(
                f,
                "buffer needs {needed} bytes but the pool is {pool_size} bytes"
            ),
            ShmError::ZeroSize => write!(f, "buffer width or height is zero"),
            ShmError::EmptyPool => write!(f, "shm pool size is zero"),
            ShmError::UnsupportedFormat(fmt) => write!(f, "unsupported shm format {fmt}"),
        }
    }
}

impl std::error::Error for ShmError {}

/// A mapped client pool.
///
/// The mapping is owned here and unmapped on drop. `Send`/`Sync` are not
/// derived: a `ShmPool` belongs to the client connection that created it, and
/// sharing it across threads would race with that client's writes.
pub struct ShmPool {
    /// Base address of the mapping.
    base: *mut u8,
    /// Length of the mapping, i.e. the size the client declared.
    len: usize,
}

// The mapping is only ever read, and reads of a shared mapping are safe to
// perform from any thread. Mutation would race with the client, so no `&mut`
// accessor is exposed.
unsafe impl Send for ShmPool {}
unsafe impl Sync for ShmPool {}

impl ShmPool {
    /// Map `fd` at `size` bytes, read-only.
    ///
    /// `size` is the client's declared pool size. It is not trusted for
    /// anything except the mapping length; buffer rectangles are validated
    /// against it separately.
    pub fn map(fd: RawFd, size: u32) -> Result<Self, ShmError> {
        if size == 0 {
            return Err(ShmError::EmptyPool);
        }
        let len = size as usize;
        // SAFETY: `fd` is a live descriptor the caller received over SCM_RIGHTS
        // and still owns. mmap with MAP_SHARED and PROT_READ only reads; it
        // cannot modify the object. A failure is reported rather than assumed
        // away, and the returned pointer is only ever used through `as_slice`.
        let base = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        if base == libc::MAP_FAILED {
            return Err(ShmError::MapFailed {
                len,
                errno: std::io::Error::last_os_error().raw_os_error().unwrap_or(0),
            });
        }
        Ok(Self {
            base: base as *mut u8,
            len,
        })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The whole mapping. Only the bytes a validated buffer covers should be
    /// read; the rest belongs to other buffers in the same pool.
    fn as_slice(&self) -> &[u8] {
        // SAFETY: `base` is a valid mapping of `len` bytes for as long as `self`
        // lives, and the borrow ties the slice to that lifetime. The mapping is
        // PROT_READ, so no `&mut` alias can exist through this type.
        unsafe { std::slice::from_raw_parts(self.base, self.len) }
    }

    /// Validate a `wl_shm_pool.create_buffer` request against this pool.
    ///
    /// This is the check that stops a hostile client from making the compositor
    /// read past the mapping or from misinterpreting a stride.
    pub fn validate_buffer(
        &self,
        offset: i32,
        width: i32,
        height: i32,
        stride: i32,
    ) -> Result<BufferLayout, ShmError> {
        if width <= 0 || height <= 0 {
            return Err(ShmError::ZeroSize);
        }
        if offset < 0 || stride <= 0 {
            // The protocol types are i32 but the values are non-negative.
            return Err(ShmError::StrideTooSmall {
                stride: stride.max(0) as u32,
                width: width as u32,
                min: width as u32 * BYTES_PER_PIXEL,
            });
        }
        let width = width as u32;
        let height = height as u32;
        let stride = stride as u32;

        let min_stride = width * BYTES_PER_PIXEL;
        if stride < min_stride {
            return Err(ShmError::StrideTooSmall {
                stride,
                width,
                min: min_stride,
            });
        }

        let needed = offset as u64 + u64::from(stride) * u64::from(height);
        if needed > self.len as u64 {
            return Err(ShmError::BufferExceedsPool {
                needed,
                pool_size: self.len as u64,
            });
        }

        Ok(BufferLayout {
            offset: offset as u32,
            width,
            height,
            stride,
        })
    }

    /// Copy one row of a validated buffer into `out`.
    ///
    /// Rows are `stride` apart, which may exceed `width * 4` when the client
    /// padded them, so the row is read from the correct offset rather than
    /// assumed contiguous.
    pub fn copy_row(&self, layout: &BufferLayout, row: u32, out: &mut Vec<u32>) {
        out.clear();
        if row >= layout.height {
            return;
        }
        let row_start = layout.offset as usize + (row as usize) * (layout.stride as usize);
        let row_bytes = (layout.width as usize) * (BYTES_PER_PIXEL as usize);
        let slice = self.as_slice();
        // The layout was validated, so this cannot run past the mapping; the
        // check is kept as a guard rather than a panic.
        let end = (row_start + row_bytes).min(slice.len());
        if row_start >= end {
            out.resize(layout.width as usize, 0);
            return;
        }
        let bytes = &slice[row_start..end];
        out.reserve(layout.width as usize);
        // Little-endian packed ARGB8888/XRGB8888: B, G, R, A bytes.
        let (chunks, _remainder) = bytes.as_chunks::<4>();
        for chunk in chunks {
            out.push(u32::from_le_bytes(*chunk));
        }
        // If the row was truncated by the guard above, pad so callers can index
        // by x without a length check.
        if out.len() < layout.width as usize {
            out.resize(layout.width as usize, 0);
        }
    }

    /// Read a single pixel, for tests and damage-limited blits.
    pub fn pixel(&self, layout: &BufferLayout, x: u32, y: u32) -> u32 {
        if x >= layout.width || y >= layout.height {
            return 0;
        }
        let idx = layout.offset as usize
            + (y as usize) * (layout.stride as usize)
            + (x as usize) * (BYTES_PER_PIXEL as usize);
        let slice = self.as_slice();
        if idx + 4 > slice.len() {
            return 0;
        }
        u32::from_le_bytes([slice[idx], slice[idx + 1], slice[idx + 2], slice[idx + 3]])
    }
}

impl Drop for ShmPool {
    fn drop(&mut self) {
        // SAFETY: `base`/`len` describe the mapping created in `map`, and this
        // runs once.
        unsafe {
            libc::munmap(self.base as *mut libc::c_void, self.len);
        }
    }
}

/// A validated rectangle inside a pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferLayout {
    pub offset: u32,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

/// Create an anonymous shared-memory file suitable for handing to a client.
///
/// `memfd_create` is used rather than `shm_open` so the object has no name in
/// the filesystem and disappears when the last reference closes.
pub fn create_pool_fd(size: u32) -> Result<RawFd, ShmError> {
    if size == 0 {
        return Err(ShmError::EmptyPool);
    }
    // SAFETY: the name is a static NUL-terminated string and the flags are
    // valid. The returned descriptor is owned by the caller.
    let fd = unsafe {
        libc::memfd_create(
            c"onuron-wayland-pool".as_ptr() as *const libc::c_char,
            libc::MFD_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(ShmError::MapFailed {
            len: size as usize,
            errno: std::io::Error::last_os_error().raw_os_error().unwrap_or(0),
        });
    }
    // SAFETY: `fd` is a live descriptor we just created.
    let rc = unsafe { libc::ftruncate(fd, size as libc::off_t) };
    if rc != 0 {
        let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        // SAFETY: closing a descriptor we own.
        unsafe { libc::close(fd) };
        return Err(ShmError::MapFailed {
            len: size as usize,
            errno,
        });
    }
    Ok(fd)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a real memfd pool with a known pixel pattern, so every test below
    /// exercises a genuine mapping rather than a mock.
    fn pool_with_pixels(width: u32, height: u32) -> (std::fs::File, ShmPool) {
        let size = width * height * BYTES_PER_PIXEL;
        let fd = create_pool_fd(size).expect("memfd");
        let file = unsafe { <std::fs::File as std::os::fd::FromRawFd>::from_raw_fd(fd) };
        // Write a deterministic pattern: pixel (x, y) = 0xFF000000 | (y << 8) | x
        let mut data = Vec::with_capacity(size as usize);
        for y in 0..height {
            for x in 0..width {
                let px = 0xFF00_0000u32 | (y << 8) | x;
                data.extend_from_slice(&px.to_le_bytes());
            }
        }
        use std::io::Write;
        (&file).write_all(&data).expect("write pattern");
        // Map a second, independent descriptor onto the same object, which is
        // exactly what happens when a client passes its fd to the compositor.
        let dup = unsafe { libc::dup(fd) };
        assert!(dup >= 0);
        let pool = ShmPool::map(dup, size).expect("map");
        (file, pool)
    }

    #[test]
    fn maps_a_real_pool_and_reads_the_exact_pixels() {
        let (_file, pool) = pool_with_pixels(8, 4);
        assert_eq!(pool.len(), 8 * 4 * 4);
        let layout = pool.validate_buffer(0, 8, 4, 32).unwrap();
        // The pattern must survive the round trip through shared memory.
        assert_eq!(pool.pixel(&layout, 0, 0), 0xFF00_0000);
        assert_eq!(pool.pixel(&layout, 3, 2), 0xFF00_0203);
        assert_eq!(pool.pixel(&layout, 7, 3), 0xFF00_0307);
    }

    #[test]
    fn reads_rows_through_a_padded_stride() {
        // 2x2 pixels with a stride of 16 bytes: 8 bytes of pixels, 8 of padding.
        let size = 16 * 2;
        let fd = create_pool_fd(size).unwrap();
        let file = unsafe { <std::fs::File as std::os::fd::FromRawFd>::from_raw_fd(fd) };
        let mut data = Vec::new();
        for y in 0..2u32 {
            for x in 0..2u32 {
                data.extend_from_slice(&(0xFF00_0000u32 | (y << 8) | x).to_le_bytes());
            }
            data.extend_from_slice(&[0xAA; 8]); // padding
        }
        use std::io::Write;
        (&file).write_all(&data).unwrap();

        let dup = unsafe { libc::dup(fd) };
        let pool = ShmPool::map(dup, size).unwrap();
        let layout = pool.validate_buffer(0, 2, 2, 16).unwrap();

        // Row 1 must come from offset 16, not offset 8, or the padding would be
        // misread as pixels.
        assert_eq!(pool.pixel(&layout, 0, 0), 0xFF00_0000);
        assert_eq!(pool.pixel(&layout, 1, 0), 0xFF00_0001);
        assert_eq!(pool.pixel(&layout, 0, 1), 0xFF00_0100);
        assert_eq!(pool.pixel(&layout, 1, 1), 0xFF00_0101);

        let mut row = Vec::new();
        pool.copy_row(&layout, 1, &mut row);
        assert_eq!(row, vec![0xFF00_0100, 0xFF00_0101]);
    }

    #[test]
    fn rejects_stride_smaller_than_a_row() {
        let (_file, pool) = pool_with_pixels(8, 4);
        // 8 pixels need 32 bytes; a stride of 16 would overlap rows.
        let err = pool.validate_buffer(0, 8, 4, 16).unwrap_err();
        assert_eq!(
            err,
            ShmError::StrideTooSmall {
                stride: 16,
                width: 8,
                min: 32
            }
        );
    }

    #[test]
    fn rejects_buffer_that_runs_past_the_pool() {
        let (_file, pool) = pool_with_pixels(8, 4); // 128 bytes
                                                    // 8x8 at stride 32 needs 256 bytes.
        let err = pool.validate_buffer(0, 8, 8, 32).unwrap_err();
        assert!(matches!(err, ShmError::BufferExceedsPool { .. }));
    }

    #[test]
    fn rejects_offset_that_runs_past_the_pool() {
        let (_file, pool) = pool_with_pixels(8, 4);
        let err = pool.validate_buffer(120, 8, 1, 32).unwrap_err();
        assert!(matches!(err, ShmError::BufferExceedsPool { .. }));
    }

    #[test]
    fn rejects_zero_sized_buffers() {
        let (_file, pool) = pool_with_pixels(8, 4);
        assert_eq!(pool.validate_buffer(0, 0, 4, 32), Err(ShmError::ZeroSize));
        assert_eq!(pool.validate_buffer(0, 8, 0, 32), Err(ShmError::ZeroSize));
    }

    #[test]
    fn rejects_negative_offset_and_stride() {
        let (_file, pool) = pool_with_pixels(8, 4);
        assert!(pool.validate_buffer(-4, 8, 4, 32).is_err());
        assert!(pool.validate_buffer(0, 8, 4, -32).is_err());
    }

    #[test]
    fn exact_fit_is_accepted() {
        let (_file, pool) = pool_with_pixels(8, 4); // 128 bytes
                                                    // 8 * 4 * 4 == 128, exactly the pool size.
        assert!(pool.validate_buffer(0, 8, 4, 32).is_ok());
    }

    #[test]
    fn rejects_an_empty_pool() {
        assert_eq!(create_pool_fd(0), Err(ShmError::EmptyPool));
        let fd = create_pool_fd(16).unwrap();
        assert!(ShmPool::map(fd, 0).is_err());
        unsafe { libc::close(fd) };
    }

    #[test]
    fn create_pool_fd_yields_a_sized_object() {
        let fd = create_pool_fd(4096).unwrap();
        // SAFETY: fstat on a live descriptor into our own fd table.
        let mut st: libc::stat = unsafe { std::mem::zeroed() };
        assert_eq!(unsafe { libc::fstat(fd, &mut st) }, 0);
        assert_eq!(st.st_size, 4096);
        unsafe { libc::close(fd) };
    }

    #[test]
    fn out_of_range_pixel_reads_are_zero_not_panics() {
        let (_file, pool) = pool_with_pixels(4, 4);
        let layout = pool.validate_buffer(0, 4, 4, 16).unwrap();
        assert_eq!(pool.pixel(&layout, 4, 0), 0);
        assert_eq!(pool.pixel(&layout, 0, 4), 0);
        assert_eq!(pool.pixel(&layout, 999, 999), 0);
    }

    #[test]
    fn pool_fd_round_trips_through_scm_rights() {
        // The whole design depends on a descriptor surviving a socket send.
        // This proves the mechanism against a real socketpair before the server
        // relies on it.
        let mut fds = [0i32; 2];
        // SAFETY: `fds` is a valid 2-element array for socketpair to fill.
        let rc = unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) };
        assert_eq!(rc, 0, "socketpair");
        let (a, b) = (fds[0], fds[1]);

        let pool_fd = create_pool_fd(64).unwrap();

        // Send `pool_fd` as ancillary data with a one-byte payload.
        let byte = [0x7Fu8];
        let mut iov = libc::iovec {
            iov_base: byte.as_ptr() as *mut libc::c_void,
            iov_len: 1,
        };
        let mut cmsg_buf = [0u8; 64];
        let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
        msg.msg_iov = &mut iov;
        msg.msg_iovlen = 1;
        msg.msg_control = cmsg_buf.as_mut_ptr() as *mut libc::c_void;
        msg.msg_controllen = cmsg_buf.len() as _;
        // SAFETY: `msg` is fully initialised above with a control buffer large
        // enough for one fd.
        let cmsg = unsafe { libc::CMSG_FIRSTHDR(&msg) };
        assert!(!cmsg.is_null());
        unsafe {
            (*cmsg).cmsg_level = libc::SOL_SOCKET;
            (*cmsg).cmsg_type = libc::SCM_RIGHTS;
            (*cmsg).cmsg_len = libc::CMSG_LEN(4) as _;
            std::ptr::copy_nonoverlapping(
                (&pool_fd as *const i32) as *const u8,
                libc::CMSG_DATA(cmsg),
                4,
            );
        }
        msg.msg_controllen = unsafe { libc::CMSG_SPACE(4) } as _;

        // SAFETY: `a` is a live socket and `msg` is a valid msghdr.
        let sent = unsafe { libc::sendmsg(a, &msg, 0) };
        assert_eq!(sent, 1, "sendmsg");

        // Receive it on the other end.
        let mut rbyte = [0u8; 1];
        let mut riov = libc::iovec {
            iov_base: rbyte.as_mut_ptr() as *mut libc::c_void,
            iov_len: 1,
        };
        let mut rcmsg = [0u8; 64];
        let mut rmsg: libc::msghdr = unsafe { std::mem::zeroed() };
        rmsg.msg_iov = &mut riov;
        rmsg.msg_iovlen = 1;
        rmsg.msg_control = rcmsg.as_mut_ptr() as *mut libc::c_void;
        rmsg.msg_controllen = rcmsg.len() as _;
        // SAFETY: `b` is a live socket and `rmsg` is initialised.
        let got = unsafe { libc::recvmsg(b, &mut rmsg, 0) };
        assert_eq!(got, 1, "recvmsg");
        assert_eq!(rbyte[0], 0x7F);

        // SAFETY: reading the control header the kernel filled in.
        let received_fd = unsafe {
            let c = libc::CMSG_FIRSTHDR(&rmsg);
            assert!(!c.is_null(), "no control message returned");
            assert_eq!((*c).cmsg_type, libc::SCM_RIGHTS);
            let mut fd = 0i32;
            std::ptr::copy_nonoverlapping(libc::CMSG_DATA(c), (&mut fd as *mut i32) as *mut u8, 4);
            fd
        };
        assert!(received_fd >= 0, "no fd received");

        // The received descriptor must address the same object.
        let mut st: libc::stat = unsafe { std::mem::zeroed() };
        assert_eq!(unsafe { libc::fstat(received_fd, &mut st) }, 0);
        assert_eq!(st.st_size, 64);

        // And it must be mappable, which is what the server actually does.
        let pool = ShmPool::map(received_fd, 64).expect("map received fd");
        assert_eq!(pool.len(), 64);

        unsafe {
            libc::close(a);
            libc::close(b);
            libc::close(pool_fd);
        }
    }
}
