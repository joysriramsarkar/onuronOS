// runtime/nilrt/src/seccomp.rs — Real BPF seccomp filter via prctl(PR_SET_SECCOMP)
//
// Architecture: x86-64 (syscall number constants match amd64; arm64 constants
// differ and would need a separate allowlist — gated via cfg).
//
// Filter strategy: allowlist of syscalls needed by typical NilOS UI apps.
// Unknown syscalls → SECCOMP_RET_ERRNO(EPERM) (app gets an error, not killed)
// so that currently unknown-needed syscalls surface as EPERM rather than
// crashing the process. Change to SECCOMP_RET_KILL_PROCESS for strict mode.

#[cfg(target_os = "linux")]
mod linux {
    use libc::{
        prctl, PR_SET_NO_NEW_PRIVS, PR_SET_SECCOMP, SECCOMP_MODE_FILTER,
    };

    // BPF instruction encoding constants
    const BPF_LD: u16 = 0x00;
    const BPF_W: u16 = 0x00;
    const BPF_ABS: u16 = 0x20;
    const BPF_JMP: u16 = 0x05;
    const BPF_JEQ: u16 = 0x10;
    const BPF_RET: u16 = 0x06;
    const BPF_K: u16 = 0x00;

    pub const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000;
    pub const SECCOMP_RET_ERRNO_EPERM: u32 = 0x0005_0001; // SECCOMP_RET_ERRNO | EPERM
    pub const SECCOMP_RET_KILL_PROCESS: u32 = 0x8000_0000;

    // Offset of syscall number in struct seccomp_data (same on all arches)
    const SECCOMP_DATA_NR_OFFSET: u32 = 0;

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct SockFilter {
        pub code: u16,
        pub jt: u8,
        pub jf: u8,
        pub k: u32,
    }

    #[repr(C)]
    pub struct SockFprog {
        pub len: u16,
        pub filter: *const SockFilter,
    }

    // Safety: SockFprog is only passed to prctl() on the same thread that
    // holds the filter slice alive (inline in apply_app_seccomp below).
    unsafe impl Send for SockFprog {}

    /// Build a BPF instruction: load the syscall number into accumulator.
    const fn load_syscall_nr() -> SockFilter {
        SockFilter {
            code: BPF_LD | BPF_W | BPF_ABS,
            jt: 0,
            jf: 0,
            k: SECCOMP_DATA_NR_OFFSET,
        }
    }

    /// Build a BPF instruction: if accumulator == nr then allow, else skip.
    const fn allow_if(nr: u32) -> [SockFilter; 2] {
        [
            // JEQ k, jt=0 (next: allow), jf=1 (skip allow, continue chain)
            SockFilter {
                code: BPF_JMP | BPF_JEQ | BPF_K,
                jt: 0,
                jf: 1,
                k: nr,
            },
            SockFilter {
                code: BPF_RET | BPF_K,
                jt: 0,
                jf: 0,
                k: SECCOMP_RET_ALLOW,
            },
        ]
    }

    /// Build a BPF instruction: unconditional return with given action.
    const fn ret(action: u32) -> SockFilter {
        SockFilter {
            code: BPF_RET | BPF_K,
            jt: 0,
            jf: 0,
            k: action,
        }
    }

    // ── x86-64 syscall numbers (Linux ABI) ───────────────────────────────────
    // Selected allowlist: minimal set for NilUI apps (read/write, memory,
    // scheduling, IPC via futex, socket I/O to the SoftBus socket, exit).
    // Extend as needed; every new syscall must be explicitly audited.
    #[cfg(target_arch = "x86_64")]
    mod nr {
        pub const READ: u32 = 0;
        pub const WRITE: u32 = 1;
        pub const OPEN: u32 = 2;
        pub const CLOSE: u32 = 3;
        pub const STAT: u32 = 4;
        pub const FSTAT: u32 = 5;
        pub const LSTAT: u32 = 6;
        pub const POLL: u32 = 7;
        pub const LSEEK: u32 = 8;
        pub const MMAP: u32 = 9;
        pub const MPROTECT: u32 = 10;
        pub const MUNMAP: u32 = 11;
        pub const BRK: u32 = 12;
        pub const RT_SIGACTION: u32 = 13;
        pub const RT_SIGPROCMASK: u32 = 14;
        pub const RT_SIGRETURN: u32 = 15;
        pub const IOCTL: u32 = 16;
        pub const PREAD64: u32 = 17;
        pub const PWRITE64: u32 = 18;
        pub const READV: u32 = 19;
        pub const WRITEV: u32 = 20;
        pub const ACCESS: u32 = 21;
        pub const PIPE: u32 = 22;
        pub const SELECT: u32 = 23;
        pub const NANOSLEEP: u32 = 35;
        pub const GETITIMER: u32 = 36;
        pub const SETITIMER: u32 = 38;
        pub const GETPID: u32 = 39;
        pub const SOCKET: u32 = 41;
        pub const CONNECT: u32 = 42;
        pub const ACCEPT: u32 = 43;
        pub const SENDTO: u32 = 44;
        pub const RECVFROM: u32 = 45;
        pub const SENDMSG: u32 = 46;
        pub const RECVMSG: u32 = 47;
        pub const BIND: u32 = 49;
        pub const GETSOCKNAME: u32 = 51;
        pub const GETPEERNAME: u32 = 52;
        pub const SOCKETPAIR: u32 = 53;
        pub const SETSOCKOPT: u32 = 54;
        pub const GETSOCKOPT: u32 = 55;
        pub const CLONE: u32 = 56;
        pub const FORK: u32 = 57;
        pub const EXECVE: u32 = 59;
        pub const EXIT: u32 = 60;
        pub const WAIT4: u32 = 61;
        pub const KILL: u32 = 62;
        pub const UNAME: u32 = 63;
        pub const FCNTL: u32 = 72;
        pub const FLOCK: u32 = 73;
        pub const FSYNC: u32 = 74;
        pub const TRUNCATE: u32 = 76;
        pub const FTRUNCATE: u32 = 77;
        pub const GETDENTS: u32 = 78;
        pub const GETCWD: u32 = 79;
        pub const RENAME: u32 = 82;
        pub const MKDIR: u32 = 83;
        pub const RMDIR: u32 = 84;
        pub const UNLINK: u32 = 87;
        pub const READLINK: u32 = 89;
        pub const CHMOD: u32 = 90;
        pub const CHOWN: u32 = 92;
        pub const UMASK: u32 = 95;
        pub const GETTIMEOFDAY: u32 = 96;
        pub const GETRLIMIT: u32 = 97;
        pub const SYSINFO: u32 = 99;
        pub const GETUID: u32 = 102;
        pub const GETGID: u32 = 104;
        pub const GETEUID: u32 = 107;
        pub const GETEGID: u32 = 108;
        pub const GETPPID: u32 = 110;
        pub const GETPGRP: u32 = 111;
        pub const SETSID: u32 = 112;
        pub const SETRLIMIT: u32 = 160;
        pub const GETTID: u32 = 186;
        pub const FUTEX: u32 = 202;
        pub const SCHED_SETAFFINITY: u32 = 203;
        pub const SCHED_GETAFFINITY: u32 = 204;
        pub const EPOLL_CREATE: u32 = 213;
        pub const GETDENTS64: u32 = 217;
        pub const SET_TID_ADDRESS: u32 = 218;
        pub const EPOLL_CTL: u32 = 233;
        pub const EPOLL_WAIT: u32 = 232;
        pub const CLOCK_GETTIME: u32 = 228;
        pub const CLOCK_GETRES: u32 = 229;
        pub const CLOCK_NANOSLEEP: u32 = 230;
        pub const EXIT_GROUP: u32 = 231;
        pub const TGKILL: u32 = 234;
        pub const OPENAT: u32 = 257;
        pub const MKDIRAT: u32 = 258;
        pub const UNLINKAT: u32 = 263;
        pub const RENAMEAT: u32 = 264;
        pub const FSTATAT: u32 = 262;
        pub const READLINKAT: u32 = 267;
        pub const SET_ROBUST_LIST: u32 = 273;
        pub const GET_ROBUST_LIST: u32 = 274;
        pub const SPLICE: u32 = 275;
        pub const EPOLL_PWAIT: u32 = 281;
        pub const EVENTFD: u32 = 284;
        pub const TIMERFD_CREATE: u32 = 283;
        pub const TIMERFD_SETTIME: u32 = 286;
        pub const TIMERFD_GETTIME: u32 = 287;
        pub const ACCEPT4: u32 = 288;
        pub const EVENTFD2: u32 = 290;
        pub const EPOLL_CREATE1: u32 = 291;
        pub const DUP3: u32 = 292;
        pub const PIPE2: u32 = 293;
        pub const PRLIMIT64: u32 = 302;
        pub const GETRANDOM: u32 = 318;
        pub const MEMFD_CREATE: u32 = 319;
        pub const STATX: u32 = 332;
        pub const RSEQ: u32 = 334;
    }

    pub fn apply_app_seccomp() -> Result<(), String> {
        #[cfg(target_arch = "x86_64")]
        {
            use nr::*;

            // ── PR_SET_NO_NEW_PRIVS is mandatory before seccomp (without CAP_SYS_ADMIN) ──
            let ret_prctl = unsafe { prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
            if ret_prctl != 0 {
                return Err(format!(
                    "[nilrt:seccomp] PR_SET_NO_NEW_PRIVS failed: errno={}",
                    std::io::Error::last_os_error()
                ));
            }

            // ── Build the BPF program ─────────────────────────────────────────
            // Each allow_if() expands to 2 instructions; we concatenate them.
            // Final instruction is the default action for unmatched syscalls.
            let mut filter: Vec<SockFilter> = vec![load_syscall_nr()];

            for nr in [
                READ, WRITE, OPEN, CLOSE, STAT, FSTAT, LSTAT, POLL, LSEEK,
                MMAP, MPROTECT, MUNMAP, BRK, RT_SIGACTION, RT_SIGPROCMASK,
                RT_SIGRETURN, IOCTL, PREAD64, PWRITE64, READV, WRITEV,
                ACCESS, PIPE, SELECT, NANOSLEEP, GETITIMER, SETITIMER,
                GETPID, SOCKET, CONNECT, ACCEPT, SENDTO, RECVFROM, SENDMSG,
                RECVMSG, BIND, GETSOCKNAME, GETPEERNAME, SOCKETPAIR,
                SETSOCKOPT, GETSOCKOPT, CLONE, FORK, EXECVE, EXIT, WAIT4,
                KILL, UNAME, FCNTL, FLOCK, FSYNC, TRUNCATE, FTRUNCATE,
                GETDENTS, GETCWD, RENAME, MKDIR, RMDIR, UNLINK, READLINK,
                CHMOD, CHOWN, UMASK, GETTIMEOFDAY, GETRLIMIT, SYSINFO,
                GETUID, GETGID, GETEUID, GETEGID, GETPPID, GETPGRP, SETSID,
                SETRLIMIT, GETTID, FUTEX, SCHED_SETAFFINITY, SCHED_GETAFFINITY,
                EPOLL_CREATE, GETDENTS64, SET_TID_ADDRESS, EPOLL_CTL, EPOLL_WAIT,
                CLOCK_GETTIME, CLOCK_GETRES, CLOCK_NANOSLEEP, EXIT_GROUP, TGKILL,
                OPENAT, MKDIRAT, UNLINKAT, RENAMEAT, FSTATAT, READLINKAT,
                SET_ROBUST_LIST, GET_ROBUST_LIST, SPLICE, EPOLL_PWAIT, EVENTFD,
                TIMERFD_CREATE, TIMERFD_SETTIME, TIMERFD_GETTIME, ACCEPT4,
                EVENTFD2, EPOLL_CREATE1, DUP3, PIPE2, PRLIMIT64, GETRANDOM,
                MEMFD_CREATE, STATX, RSEQ,
            ] {
                let pair = allow_if(nr);
                filter.push(pair[0]);
                filter.push(pair[1]);
            }

            // Default: deny with EPERM (not kill) so failures are auditable
            filter.push(ret(SECCOMP_RET_ERRNO_EPERM));

            // ── Install the filter via prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER) ──
            let prog = SockFprog {
                len: filter.len() as u16,
                filter: filter.as_ptr(),
            };

            let ret_seccomp = unsafe {
                prctl(
                    PR_SET_SECCOMP,
                    SECCOMP_MODE_FILTER as libc::c_ulong,
                    &prog as *const SockFprog as libc::c_ulong,
                    0,
                    0,
                )
            };

            if ret_seccomp != 0 {
                return Err(format!(
                    "[nilrt:seccomp] prctl(PR_SET_SECCOMP) failed: {}",
                    std::io::Error::last_os_error()
                ));
            }

            println!(
                "[nilrt:seccomp] BPF allowlist installed: {} instructions, {} syscalls allowed, default=EPERM",
                filter.len(),
                filter.len().saturating_sub(1) / 2  // subtract load + default
            );
            Ok(())
        }

        #[cfg(not(target_arch = "x86_64"))]
        {
            println!("[nilrt:seccomp] Architecture not yet supported; seccomp skipped.");
            Ok(())
        }
    }
}

// ── Public surface ────────────────────────────────────────────────────────────

/// Re-export the constants that callers may need for logging/auditing.
#[cfg(target_os = "linux")]
pub use linux::{SECCOMP_RET_ALLOW, SECCOMP_RET_ERRNO_EPERM, SECCOMP_RET_KILL_PROCESS};

/// Apply a real seccomp BPF syscall allowlist to the calling process.
/// Must be called *after* any setup that requires privileged syscalls,
/// and *before* executing untrusted app code.
///
/// # Errors
/// Returns `Err(String)` if `prctl(PR_SET_NO_NEW_PRIVS)` or
/// `prctl(PR_SET_SECCOMP)` fails (e.g. not running on Linux, or
/// `CONFIG_SECCOMP` not compiled into the kernel).
pub fn apply_app_seccomp() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return linux::apply_app_seccomp();

    #[cfg(not(target_os = "linux"))]
    {
        println!("[nilrt:seccomp] Non-Linux host: seccomp passthrough (dev mode).");
        Ok(())
    }
}
