p = "C:/Users/joysr/Documents/programming language/onuronOS/services/camerad/src/main.rs"
s = open(p, encoding="utf-8", newline="").read()
s = s.replace("\r\n", "\n")
print("start, capture present:", "fn capture_v4l2_frame(node: &str)" in s)
print("handle present:", "pub fn handle_ipc_request" in s)

new_capture = '''fn capture_v4l2_frame(node: &str) -> Result<Vec<u8>, String> {
    const V4L2_BUF_TYPE_VIDEO_CAPTURE: u32 = 1;
    const V4L2_MEMORY_MMAP: u32 = 1;

    #[repr(C)]
    struct v4l2_buffer {
        index: u32,
        r#type: u32,
        memory: u32,
        bytesused: u32,
        flags: u32,
        sequence: u32,
        timestamp: u64,
        timetv: [u64; 2],
        reserved: [u64; 2],
        m: v4l2_mmap,
    }
    #[repr(C)]
    struct v4l2_mmap {
        start: *mut std::ffi::c_void,
        length: usize,
        offset: i32,
        fd: i32,
    }
    #[repr(C)]
    struct v4l2_frmsize_discrete {
        width: u32,
        height: u32,
    }
    #[repr(C)]
    struct v4l2_pix_format {
        width: u32,
        height: u32,
        pixelformat: u32,
        flags: u32,
        field: u32,
        chromakey: u32,
        bounds: v4l2_frmsize_discrete,
        quantization: u8,
        ycbcr: [u32; 4],
        private: u32,
    }
    #[repr(C)]
    struct v4l2_requestbuffers {
        count: u32,
        r#type: u32,
        memory: u32,
        buffers: [u32; 1],
    }
    #[repr(C)]
    struct v4l2_capability {
        driver: [u8; 16],
        card: [u8; 32],
        bus: u32,
        version: u32,
    }

    let fd = unsafe { libc::open(node.as_ptr() as *const libc::c_char, libc::O_RDWR) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }

    // VIDIOC_QUERYCAP
    let mut cap = v4l2_capability {
        driver: [0; 16],
        card: [0; 32],
        bus: 0,
        version: 0,
    };
    if unsafe { libc::ioctl(fd, 0x40044800, &mut cap as *mut _) } != 0 {
        let _ = unsafe { libc::close(fd) };
        return Err("VIDIOC_QUERYCAP failed".to_string());
    }
    // VIDIOC_S_FMT
    let mut pf = v4l2_pix_format {
        width: 1920,
        height: 1080,
        pixelformat: 0x52563634, // 'RV64'
        flags: 0,
        field: 0,
        chromakey: 0,
        bounds: v4l2_frmsize_discrete { width: 1920, height: 1080 },
        quantization: 0,
        ycbcr: [0; 4],
        private: 0,
    };
    if unsafe { libc::ioctl(fd, 0x40044010, &mut pf as *mut _) } != 0 {
        let _ = unsafe { libc::close(fd) };
        return Err("VIDIOC_S_FMT failed".to_string());
    }
    // VIDIOC_REQBU
    let mut rb = v4l2_requestbuffers {
        count: 4,
        r#type: V4L2_BUF_TYPE_VIDEO_CAPTURE,
        memory: V4L2_MEMORY_MMAP,
        buffers: [0; 1],
    };
    if unsafe { libc::ioctl(fd, 0x40044806, &mut rb as *mut _) } != 0 {
        let _ = unsafe { libc::close(fd) };
        return Err("VIDIOC_REQBU failed".to_string());
    }

    // Map buffers.
    let mut base: *mut std::ffi::c_void = std::ptr::null_mut();
    let mut bufs = std::array::from_fn(|i| {
        let mut b = v4l2_buffer {
            index: i as u32,
            r#type: V4L2_BUF_TYPE_VIDEO_CAPTURE,
            memory: V4L2_MEMORY_MMAP,
            bytesused: 0,
            flags: 0,
            sequence: 0,
            timestamp: 0,
            timetv: [0; 2],
            reserved: [0; 2],
            m: v4l2_mmap { start: std::ptr::null_mut(), length: 0, offset: 0, fd: 0 },
        };
        if unsafe { libc::ioctl(fd, 0x40044803, &mut b as *mut _) } != 0 {
            let _ = unsafe { libc::close(fd) };
            return Err("VIDIOC_QUERYBUF failed".to_string());
        }
        b
    });

    // VIDIOC_STREAMON
    {
        let mut so = v4l2_requestbuffers {
            count: 4,
            r#type: V4L2_BUF_TYPE_VIDEO_CAPTURE,
            memory: V4L2_MEMORY_MMAP,
            buffers: [0; 1],
        };
        if unsafe { libc::ioctl(fd, 0x40044807, &mut so as *mut _) } != 0 {
            let _ = unsafe { libc::munmap(base, 1) };
            let _ = unsafe { libc::close(fd) };
            return Err("VIDIOC_STREAMON failed".to_string());
        }
    }

    // Capture one frame (DQBUF).
    {
        let mut b = v4l2_buffer {
            index: 0,
            r#type: V4L2_BUF_TYPE_VIDEO_CAPTURE,
            memory: V4L2_MEMORY_MMAP,
            bytesused: 0,
            flags: 0,
            sequence: 0,
            timestamp: 0,
            timetv: [0; 2],
            reserved: [0; 2],
            m: v4l2_mmap { start: std::ptr::null_mut(), length: 0, offset: 0, fd: 0 },
        };
        if unsafe { libc::ioctl(fd, 0x40044809, &mut b as *mut _) } != 0 {
            let _ = unsafe { libc::ioctl(fd, 0x40044808, &mut rb as *mut _) };
            let _ = unsafe { libc::munmap(base, 1) };
            let _ = unsafe { libc::close(fd) };
            return Err("VIDIOC_DQBUF failed".to_string());
        }
    }

    let slice = unsafe {
        std::slice::from_raw_parts_mut(
            base as *mut u8,
            bufs[0].m.length.max(1),
        )
    };
    let _ = unsafe { libc::ioctl(fd, 0x40044808, &mut rb as *mut _) }; // VIDIOC_STREAMOFF
    let _ = unsafe { libc::munmap(base, 1) };
    let _ = unsafe { libc::close(fd) };
    Ok(slice.to_vec())
}
'''

start = s.index("fn capture_v4l2_frame(node: &str) -> Result<Vec<u8>, String> {")
end = s.index("pub fn handle_ipc_request", start)
s = s[:start] + new_capture + s[end:]

fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity((bytes.len() + 2) / 3 * 4);
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = bytes.get(i + 1).copied().unwrap_or(0) as u32;
        let b2 = bytes.get(i + 2).copied().unwrap_or(0) as u32;
        out.push(TABLE[(b0 >> 2) & 0x3f] as u8);
        out.push(TABLE[((b0 << 4) | (b1 >> 4)) & 0x3f] as u8);
        out.push(TABLE[((b1 << 2) | (b2 >> 6)) & 0x3f] as u8);
        out.push(TABLE[b2 & 0x3f] as u8);
        i += 3;
    }
    let rem = bytes.len() % 3;
    if rem == 1 {
        let b0 = bytes[bytes.len() - 1] as u32;
        out.push(TABLE[(b0 >> 2) & 0x3f] as u8);
        out.push(TABLE[(b0 << 4) & 0x3f] as u8);
        out.push(b'=' as u8);
        out.push(b'=' as u8);
    } else if rem == 2 {
        let b0 = bytes[bytes.len() - 2] as u32;
        let b1 = bytes[bytes.len() - 1] as u32;
        out.push(TABLE[(b0 >> 2) & 0x3f] as u8);
        out.push(TABLE[((b0 << 4) | (b1 >> 4)) & 0x3f] as u8);
        out.push(TABLE[(b1 << 2) & 0x3f] as u8);
        out.push(b'=' as u8);
    }
    String::from_utf8(out).unwrap()
}

start = s.index("fn capture_v4l2_frame(node: &str) -> Result<Vec<u8>, String> {")
end = s.index("pub fn handle_ipc_request", start)
s = s[:start] + new_capture + s[end:]

open(p, "w", encoding="utf-8", newline="\n").write(s)
print("written; total lines:", s.count("\n") + 1)
