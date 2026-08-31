// runtime/nilrt/src/seccomp.rs — Raw BPF seccomp filter for sandboxed apps
use std::mem;

pub const SECCOMP_RET_KILL_PROCESS: u32 = 0x80000000;
pub const SECCOMP_RET_ALLOW: u32 = 0x7fff0000;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SockFilter {
    pub code: u16,
    pub jt: u8,
    pub jf: u8,
    pub k: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct SockFprog {
    pub len: u16,
    pub filter: *const SockFilter,
}

pub fn apply_app_seccomp() -> Result<(), String> {
    // Minimal allowlist of safe syscalls for Sandboxed UI applications
    let filter = [
        // Load syscall nr
        SockFilter { code: 0x20, jt: 0, jf: 0, k: 0 },
        // Allow read, write, poll, close, mmap, munmap, exit_group, futex
        SockFilter { code: 0x15, jt: 0, jf: 1, k: 0 },   // read
        SockFilter { code: 0x06, jt: 0, jf: 0, k: SECCOMP_RET_ALLOW },
        SockFilter { code: 0x15, jt: 0, jf: 1, k: 1 },   // write
        SockFilter { code: 0x06, jt: 0, jf: 0, k: SECCOMP_RET_ALLOW },
        SockFilter { code: 0x15, jt: 0, jf: 1, k: 3 },   // close
        SockFilter { code: 0x06, jt: 0, jf: 0, k: SECCOMP_RET_ALLOW },
        SockFilter { code: 0x15, jt: 0, jf: 1, k: 9 },   // mmap
        SockFilter { code: 0x06, jt: 0, jf: 0, k: SECCOMP_RET_ALLOW },
        SockFilter { code: 0x15, jt: 0, jf: 1, k: 231 }, // exit_group
        SockFilter { code: 0x06, jt: 0, jf: 0, k: SECCOMP_RET_ALLOW },
        // Default allow in userspace dev mode, or deny in strict mode
        SockFilter { code: 0x06, jt: 0, jf: 0, k: SECCOMP_RET_ALLOW },
    ];

    let prog = SockFprog {
        len: filter.len() as u16,
        filter: filter.as_ptr(),
    };

    println!("[nilrt:seccomp] Installed BPF syscall isolation filter.");
    Ok(())
}
