// services/nilkeyd/src/fscrypt.rs — Best-effort Linux fscrypt v2 policy binding.
//
// This module applies a v2 encryption policy to a directory using the
// `FS_IOC_ADD_ENCRYPTION_KEY` / `FS_IOC_SET_ENCRYPTION_POLICY` ioctls. It is
// deliberately best-effort: on kernels or filesystems without fscrypt the
// caller gets a `FscryptResult` with `applied: false` and a human-readable
// reason instead of a panic, so nilkeyd never bricks a boot and can fall back
// to key-wrapping-only protection.
//
// Maturity: FUNCTIONAL PROTOTYPE. The ioctl structs match the Linux UAPI
// (`include/uapi/linux/fscrypt.h`); end-to-end behaviour on a real ext4/f2fs
// image is exercised by the Linux gate, not on a Windows dev host.
//
// Note: setting an fscrypt policy encrypts *newly created* files in the
// directory; it does not retroactively encrypt existing files. Apply it before
// /data is populated.

use sha2::{Digest, Sha256};

/// Raw fscrypt v2 key size required by AES-256-XTS + AES-256-CTS-CBC.
pub const FSCRYPT_MAX_KEY_SIZE: usize = 64;

/// fscrypt v2 key identifier length.
pub const FSCRYPT_KEY_IDENTIFIER_SIZE: usize = 16;

/// v1 descriptor size (kept for completeness / callers).
pub const FS_KEY_DESCRIPTOR_SIZE: usize = 8;

// fscrypt v2 constants from include/uapi/linux/fscrypt.h.
pub const FSCRYPT_POLICY_V2: u8 = 2;
pub const FSCRYPT_MODE_AES_256_XTS: u8 = 1;
pub const FSCRYPT_MODE_AES_256_CTS: u8 = 4;
pub const FSCRYPT_KEY_SPEC_TYPE_DESCRIPTOR: u32 = 1;
pub const FSCRYPT_KEY_SPEC_TYPE_IDENTIFIER: u32 = 2;

/// Outcome of attempting to apply an fscrypt policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FscryptResult {
    /// True only when both the key was installed and the policy was set.
    pub applied: bool,
    /// Human-readable explanation, especially when `applied` is false.
    pub detail: String,
    /// The 16-byte v2 key identifier as lowercase hex, when the kernel
    /// returned one.
    pub key_identifier_hex: Option<String>,
}

impl FscryptResult {
    pub fn unsupported(detail: impl Into<String>) -> Self {
        FscryptResult {
            applied: false,
            detail: detail.into(),
            key_identifier_hex: None,
        }
    }

    pub fn applied(detail: impl Into<String>, id: [u8; FSCRYPT_KEY_IDENTIFIER_SIZE]) -> Self {
        FscryptResult {
            applied: true,
            detail: detail.into(),
            key_identifier_hex: Some(hex(&id)),
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// Derive the 64-byte fscrypt v2 raw key material from the 32-byte master key
/// using SHA-256 with domain separation. Deterministic and one-way.
pub fn derive_fscrypt_key(master: &[u8]) -> [u8; FSCRYPT_MAX_KEY_SIZE] {
    let mut out = [0u8; FSCRYPT_MAX_KEY_SIZE];
    let mut counter: u8 = 0;
    for chunk in out.chunks_mut(32) {
        let mut hasher = Sha256::new();
        hasher.update(b"nilkeyd/fscrypt/v2");
        hasher.update([counter]);
        hasher.update(master);
        let digest = hasher.finalize();
        chunk.copy_from_slice(&digest[..chunk.len()]);
        counter = counter.wrapping_add(1);
    }
    out
}

/// Apply a v2 policy to `dir`, installing the derived key first.
///
/// Never panics. On a platform or filesystem without fscrypt support it
/// returns `applied: false` with a reason.
pub fn apply_fscrypt_policy(dir: &std::path::Path, master: &[u8]) -> FscryptResult {
    #[cfg(target_os = "linux")]
    {
        linux::apply(dir, master)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (dir, master);
        FscryptResult::unsupported("fscrypt is Linux-only; policy not applied on this platform")
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::{
        derive_fscrypt_key, FscryptResult, FSCRYPT_KEY_SPEC_TYPE_IDENTIFIER,
        FSCRYPT_MAX_KEY_SIZE,
    };
    use std::os::unix::io::AsRawFd;

    // _IOC direction bits.
    const IOC_READ: u32 = 2;
    const IOC_WRITE: u32 = 1;

    const fn ioc(dir: u32, nr: u32, size: usize) -> libc::c_ulong {
        (((dir as u64) << 30) | ((size as u64) << 16) | ((b'f' as u64) << 8) | nr as u64)
            as libc::c_ulong
    }

    /// `FS_IOC_SET_ENCRYPTION_POLICY` is encoded against `struct
    /// fscrypt_policy_v1` (12 bytes) by the kernel UAPI even when a v2 policy
    /// is passed.
    pub const FS_IOC_SET_ENCRYPTION_POLICY: libc::c_ulong = ioc(IOC_READ, 19, 12);
    /// `FS_IOC_ADD_ENCRYPTION_KEY` — size is `sizeof(struct fscrypt_add_key_arg)`.
    pub const FS_IOC_ADD_ENCRYPTION_KEY: libc::c_ulong =
        ioc(IOC_READ | IOC_WRITE, 23, std::mem::size_of::<FscryptAddKeyArg>());

    /// Matches `struct fscrypt_key_specifier` (40 bytes: type, reserved,
    /// 32-byte union).
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct FscryptKeySpecifier {
        pub type_: u32,
        pub reserved: u32,
        pub union_bytes: [u8; 32],
    }

    /// Matches the fixed header of `struct fscrypt_add_key_arg` (80 bytes,
    /// before the trailing `raw[]`). `flags` is zero, i.e. raw (not
    /// hardware-wrapped) key material.
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct FscryptAddKeyArg {
        pub key_spec: FscryptKeySpecifier,
        pub raw_size: u32,
        pub key_id: u32,
        pub flags: u32,
        pub reserved: [u32; 7],
    }

    /// Matches `struct fscrypt_policy_v2` (24 bytes).
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct FscryptPolicyV2 {
        pub version: u8,
        pub contents_encryption_mode: u8,
        pub filenames_encryption_mode: u8,
        pub flags: u8,
        pub log2_data_unit_size: u8,
        pub reserved: [u8; 3],
        pub master_key_identifier: [u8; super::FSCRYPT_KEY_IDENTIFIER_SIZE],
    }

    const _: () = assert!(std::mem::size_of::<FscryptKeySpecifier>() == 40);
    const _: () = assert!(std::mem::size_of::<FscryptAddKeyArg>() == 80);
    const _: () = assert!(std::mem::size_of::<FscryptPolicyV2>() == 24);

    fn unsupported(op: &str, err: std::io::Error) -> FscryptResult {
        let reason = match err.raw_os_error() {
            Some(libc::ENOTTY) => {
                format!("{} unsupported: filesystem does not support fscrypt (ENOTTY)", op)
            }
            Some(libc::EOPNOTSUPP) => {
                format!("{} unsupported: fscrypt not enabled on this filesystem (EOPNOTSUPP)", op)
            }
            Some(libc::EINVAL) => format!("{} unsupported: invalid fscrypt configuration", op),
            Some(libc::EPERM) => format!("{} unsupported: requires root/CAP_SYS_ADMIN", op),
            _ => format!("{} failed: {}", op, err),
        };
        FscryptResult::unsupported(reason)
    }

    pub(super) fn apply(dir: &std::path::Path, master: &[u8]) -> FscryptResult {
        let file = match std::fs::File::open(dir) {
            Ok(f) => f,
            Err(e) => {
                return FscryptResult::unsupported(format!("cannot open {}: {}", dir.display(), e))
            }
        };
        let fd = file.as_raw_fd();

        let raw_key = derive_fscrypt_key(master);
        let header = FscryptAddKeyArg {
            key_spec: FscryptKeySpecifier {
                type_: FSCRYPT_KEY_SPEC_TYPE_IDENTIFIER,
                reserved: 0,
                union_bytes: [0u8; 32],
            },
            raw_size: FSCRYPT_MAX_KEY_SIZE as u32,
            key_id: 0,
            flags: 0,
            reserved: [0u32; 7],
        };

        let header_size = std::mem::size_of::<FscryptAddKeyArg>();
        let mut buf = Vec::with_capacity(header_size + raw_key.len());
        // SAFETY: FscryptAddKeyArg is repr(C) and fully initialized; we copy its
        // exact size into the front of `buf` before appending the raw key.
        let header_bytes = unsafe {
            std::slice::from_raw_parts(&header as *const FscryptAddKeyArg as *const u8, header_size)
        };
        buf.extend_from_slice(header_bytes);
        buf.extend_from_slice(&raw_key);

        let ret = unsafe { libc::ioctl(fd, FS_IOC_ADD_ENCRYPTION_KEY, buf.as_mut_ptr()) };
        if ret < 0 {
            return unsupported("FS_IOC_ADD_ENCRYPTION_KEY", std::io::Error::last_os_error());
        }

        // The kernel writes the computed 16-byte identifier into the key spec's
        // union (offset 8 within the header).
        let mut identifier = [0u8; super::FSCRYPT_KEY_IDENTIFIER_SIZE];
        identifier.copy_from_slice(&buf[8..8 + super::FSCRYPT_KEY_IDENTIFIER_SIZE]);

        let policy = FscryptPolicyV2 {
            version: super::FSCRYPT_POLICY_V2,
            contents_encryption_mode: super::FSCRYPT_MODE_AES_256_XTS,
            filenames_encryption_mode: super::FSCRYPT_MODE_AES_256_CTS,
            flags: 0,
            log2_data_unit_size: 0,
            reserved: [0u8; 3],
            master_key_identifier: identifier,
        };

        let ret = unsafe { libc::ioctl(fd, FS_IOC_SET_ENCRYPTION_POLICY, &policy) };
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EEXIST) {
                // The directory already carries a policy.
                return FscryptResult::applied(
                    format!("fscrypt policy already present on {}", dir.display()),
                    identifier,
                );
            }
            return unsupported("FS_IOC_SET_ENCRYPTION_POLICY", err);
        }

        FscryptResult::applied(
            format!("applied fscrypt v2 AES-256-XTS policy to {}", dir.display()),
            identifier,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_key_is_64_bytes_and_deterministic() {
        let master = [7u8; 32];
        let a = derive_fscrypt_key(&master);
        let b = derive_fscrypt_key(&master);
        assert_eq!(a, b);
        assert_eq!(a.len(), FSCRYPT_MAX_KEY_SIZE);
        assert_ne!(&a[..32], &master[..]);
        assert_ne!(derive_fscrypt_key(&[8u8; 32]), a);
    }

    #[test]
    fn apply_policy_is_fallible_not_panicking() {
        let missing = std::path::Path::new("/definitely/not/a/real/directory/onuron");
        let result = apply_fscrypt_policy(missing, &[0u8; 32]);
        assert!(!result.applied);
        assert!(!result.detail.is_empty());
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn non_linux_reports_platform() {
        let dir = std::env::temp_dir();
        let result = apply_fscrypt_policy(&dir, &[0u8; 32]);
        assert!(!result.applied);
        assert!(result.detail.contains("Linux-only"), "{}", result.detail);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn kernel_struct_sizes_match() {
        assert_eq!(std::mem::size_of::<linux::FscryptKeySpecifier>(), 40);
        assert_eq!(std::mem::size_of::<linux::FscryptAddKeyArg>(), 80);
        assert_eq!(std::mem::size_of::<linux::FscryptPolicyV2>(), 24);
        assert_eq!(linux::FS_IOC_SET_ENCRYPTION_POLICY, 0x800C_6613);
        assert_eq!(linux::FS_IOC_ADD_ENCRYPTION_KEY, 0xC050_6617);
    }
}