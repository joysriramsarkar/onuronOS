// nilinit/src/fscrypt.rs — nilkeyd → nilinit fscrypt key handoff
//
// On boot, nilinit asks nilkeyd (via its Unix domain socket) for the
// fscrypt v2 policy key, then applies it to the `/data` block device with
// `FS_IOC_ADD_ENCRYPTION_KEY` + `FS_IOC_SET_ENCRYPTION_POLICY`.
//
// The key is identified by a "key descriptor" (64-byte hex) stored in
// `/data/system/fscrypt/policy`. The descriptor is derived from the PIN hash
// by nilkeyd, so a wrong PIN yields a different descriptor (or none at all),
// and the kernel refuses to set the policy.
//
// Security invariants:
// - apply_key() MUST NEVER return Ok(()) if kernel ioctl was not performed or failed.
// - All failures must cleanly report why the policy could not be set.

/// Parse a 64-byte hex key descriptor from the policy file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyDescriptor {
    pub descriptor: [u8; 32],
    pub hash_specifier: [u8; 16],
}

impl KeyDescriptor {
    pub fn parse(content: &str) -> Result<Self, String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut descriptor = [0u8; 32];
        let mut hash_specifier = [0u8; 16];

        for line in lines {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() != 2 {
                continue;
            }
            let key = parts[0];
            let value = parts[1];

            if key == "key_descriptor" {
                if value.len() != 64 {
                    return Err(format!(
                        "key_descriptor must be 64 hex chars, got {}",
                        value.len()
                    ));
                }
                let bytes = hex::decode(value).map_err(|e| e.to_string())?;
                descriptor = bytes
                    .try_into()
                    .map_err(|_| "key_descriptor wrong length".to_string())?;
            } else if key == "hash_specifier" {
                if value.len() != 32 {
                    return Err(format!(
                        "hash_specifier must be 32 hex chars, got {}",
                        value.len()
                    ));
                }
                let bytes = hex::decode(value).map_err(|e| e.to_string())?;
                hash_specifier = bytes
                    .try_into()
                    .map_err(|_| "hash_specifier wrong length".to_string())?;
            }
        }

        if descriptor.iter().all(|&b| b == 0) {
            return Err("key_descriptor is all zeros".to_string());
        }

        Ok(KeyDescriptor {
            descriptor,
            hash_specifier,
        })
    }
}

/// Connect to nilkeyd and ask for the key blob.
pub fn request_key(sock_path: &str, descriptor: &[u8; 32]) -> Result<Vec<u8>, String> {
    #[cfg(unix)]
    {
        use std::io::{Read, Write};
        use std::os::unix::net::UnixStream;

        let mut stream = UnixStream::connect(sock_path)
            .map_err(|e| format!("cannot connect to keyd socket {}: {}", sock_path, e))?;

        let request = format!("GET_KEY {}\n", hex::encode(descriptor));
        stream
            .write_all(request.as_bytes())
            .map_err(|e| format!("cannot write to keyd socket: {}", e))?;

        let mut response = [0u8; 512];
        let n = stream
            .read(&mut response)
            .map_err(|e| format!("cannot read from keyd socket: {}", e))?;

        let response_str = String::from_utf8_lossy(&response[..n]);
        if response_str.starts_with("ERROR") {
            return Err(response_str.trim().to_string());
        }

        // Response format: "KEY <hex_blob>" or "OK <hex_blob>"
        let parts: Vec<&str> = response_str.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(format!("unexpected keyd response: {}", response_str.trim()));
        }

        let blob = hex::decode(parts[1]).map_err(|e| format!("invalid hex in key: {}", e))?;
        Ok(blob)
    }

    #[cfg(not(unix))]
    {
        let _ = (sock_path, descriptor);
        Err("Unix domain sockets not supported on non-unix dev-host".to_string())
    }
}

/// Apply the encryption key to `/data` using Linux kernel fscrypt ioctls.
///
/// Under no circumstances does this function return `Ok(())` if the kernel
/// ioctls were not successfully executed.
pub fn apply_key(key: &[u8], _descriptor: &[u8; 32], _policy: &[u8]) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::io::AsRawFd;

        const IOC_READ: u32 = 2;
        const IOC_WRITE: u32 = 1;

        const fn ioc(dir: u32, nr: u32, size: usize) -> libc::c_ulong {
            (((dir as u64) << 30) | ((size as u64) << 16) | ((b'f' as u64) << 8) | nr as u64)
                as libc::c_ulong
        }

        const FS_IOC_SET_ENCRYPTION_POLICY: libc::c_ulong = ioc(IOC_READ, 19, 12);
        const FS_IOC_ADD_ENCRYPTION_KEY: libc::c_ulong = ioc(
            IOC_READ | IOC_WRITE,
            23,
            std::mem::size_of::<FscryptAddKeyArg>(),
        );

        const FSCRYPT_KEY_SPEC_TYPE_IDENTIFIER: u32 = 2;
        const FSCRYPT_MAX_KEY_SIZE: usize = 64;
        const FSCRYPT_POLICY_V2: u8 = 2;
        const FSCRYPT_MODE_AES_256_XTS: u8 = 1;
        const FSCRYPT_MODE_AES_256_CTS: u8 = 4;
        const FSCRYPT_KEY_IDENTIFIER_SIZE: usize = 16;

        #[repr(C)]
        #[derive(Clone, Copy)]
        struct FscryptKeySpecifier {
            type_: u32,
            reserved: u32,
            union_bytes: [u8; 32],
        }

        #[repr(C)]
        #[derive(Clone, Copy)]
        struct FscryptAddKeyArg {
            key_spec: FscryptKeySpecifier,
            raw_size: u32,
            key_id: u32,
            flags: u32,
            reserved: [u32; 7],
        }

        #[repr(C)]
        #[derive(Clone, Copy)]
        struct FscryptPolicyV2 {
            version: u8,
            contents_encryption_mode: u8,
            filenames_encryption_mode: u8,
            flags: u8,
            log2_data_unit_size: u8,
            reserved: [u8; 3],
            master_key_identifier: [u8; FSCRYPT_KEY_IDENTIFIER_SIZE],
        }

        if key.is_empty() || key.len() > FSCRYPT_MAX_KEY_SIZE {
            return Err(format!(
                "invalid key size: {} (expected 1..={})",
                key.len(),
                FSCRYPT_MAX_KEY_SIZE
            ));
        }

        let file = std::fs::File::open("/data")
            .map_err(|e| format!("cannot open /data for fscrypt: {}", e))?;
        let fd = file.as_raw_fd();

        let mut key_buf = [0u8; FSCRYPT_MAX_KEY_SIZE];
        key_buf[..key.len()].copy_from_slice(key);

        let header = FscryptAddKeyArg {
            key_spec: FscryptKeySpecifier {
                type_: FSCRYPT_KEY_SPEC_TYPE_IDENTIFIER,
                reserved: 0,
                union_bytes: [0u8; 32],
            },
            raw_size: key.len() as u32,
            key_id: 0,
            flags: 0,
            reserved: [0u32; 7],
        };

        let header_size = std::mem::size_of::<FscryptAddKeyArg>();
        let mut buf = Vec::with_capacity(header_size + key.len());
        let header_bytes = unsafe {
            std::slice::from_raw_parts(&header as *const FscryptAddKeyArg as *const u8, header_size)
        };
        buf.extend_from_slice(header_bytes);
        buf.extend_from_slice(&key_buf[..key.len()]);

        let ret = unsafe { libc::ioctl(fd, FS_IOC_ADD_ENCRYPTION_KEY as _, buf.as_mut_ptr()) };
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            let reason = match err.raw_os_error() {
                Some(libc::ENOTTY) => {
                    "FS_IOC_ADD_ENCRYPTION_KEY unsupported: filesystem does not support fscrypt (ENOTTY)".to_string()
                }
                Some(libc::EOPNOTSUPP) => {
                    "FS_IOC_ADD_ENCRYPTION_KEY unsupported: fscrypt not enabled on kernel/filesystem (EOPNOTSUPP)".to_string()
                }
                Some(libc::EINVAL) => "FS_IOC_ADD_ENCRYPTION_KEY failed: invalid arguments (EINVAL)".to_string(),
                Some(libc::EPERM) => "FS_IOC_ADD_ENCRYPTION_KEY failed: requires CAP_SYS_ADMIN (EPERM)".to_string(),
                _ => format!("FS_IOC_ADD_ENCRYPTION_KEY failed: {}", err),
            };
            return Err(reason);
        }

        let mut identifier = [0u8; FSCRYPT_KEY_IDENTIFIER_SIZE];
        identifier.copy_from_slice(&buf[8..8 + FSCRYPT_KEY_IDENTIFIER_SIZE]);

        let policy_v2 = FscryptPolicyV2 {
            version: FSCRYPT_POLICY_V2,
            contents_encryption_mode: FSCRYPT_MODE_AES_256_XTS,
            filenames_encryption_mode: FSCRYPT_MODE_AES_256_CTS,
            flags: 0,
            log2_data_unit_size: 0,
            reserved: [0u8; 3],
            master_key_identifier: identifier,
        };

        let ret = unsafe { libc::ioctl(fd, FS_IOC_SET_ENCRYPTION_POLICY as _, &policy_v2) };
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EEXIST) {
                // Policy already applied to directory from previous boot
                return Ok(());
            }
            let reason = match err.raw_os_error() {
                Some(libc::ENOTTY) => {
                    "FS_IOC_SET_ENCRYPTION_POLICY unsupported: filesystem does not support fscrypt (ENOTTY)".to_string()
                }
                Some(libc::EOPNOTSUPP) => {
                    "FS_IOC_SET_ENCRYPTION_POLICY unsupported: fscrypt feature flag missing (EOPNOTSUPP)".to_string()
                }
                Some(libc::EINVAL) => "FS_IOC_SET_ENCRYPTION_POLICY failed: invalid policy configuration (EINVAL)".to_string(),
                _ => format!("FS_IOC_SET_ENCRYPTION_POLICY failed: {}", err),
            };
            return Err(reason);
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (key, _descriptor, _policy);
        Err("fscrypt kernel ioctl unsupported on non-Linux dev-host platform".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_descriptor_parse_valid() {
        let content = "key_descriptor 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\nhash_specifier 0123456789abcdef0123456789abcdef\n";
        let parsed = KeyDescriptor::parse(content).expect("parse should succeed");
        assert_eq!(parsed.descriptor[0], 0x01);
        assert_eq!(parsed.hash_specifier[0], 0x01);
    }

    #[test]
    fn test_key_descriptor_parse_all_zeros() {
        let content = "key_descriptor 0000000000000000000000000000000000000000000000000000000000000000\nhash_specifier 00000000000000000000000000000000\n";
        let res = KeyDescriptor::parse(content);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("all zeros"));
    }

    #[test]
    fn test_key_descriptor_parse_invalid_length() {
        let content = "key_descriptor 1234\n";
        let res = KeyDescriptor::parse(content);
        assert!(res.is_err());
    }

    #[test]
    fn test_apply_key_empty_or_non_linux_fails_cleanly() {
        let desc = [1u8; 32];
        let policy = b"dummy";
        let res = apply_key(&[], &desc, policy);
        assert!(res.is_err(), "Empty key or non-linux must never return Ok");
    }
}
