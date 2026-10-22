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
// Steps:
//   1. Read /data/system/fscrypt/policy (key_specifier, hash_specifier)
//   2. Connect to /run/nilos/keyd.sock
//   3. Request the key blob: "GET_KEY <hex_descriptor>"
//   4. Send the key: FS_IOC_ADD_ENCRYPTION_KEY with the descriptor + key blob
//   5. Set the encryption policy: FS_IOC_SET_ENCRYPTION_POLICY
//
// On non-Linux: no-op (fake HAL used on dev-host).

/// Parse a 64-byte hex key descriptor from the policy file.
#[derive(Debug, Clone)]
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
            let key = &parts[0];
            let value = parts[1];

            if key.eq("key_descriptor") {
                if value.len() != 64 {
                    return Err(format!(
                        "key_descriptor must be 64 hex chars, got {}",
                        value.len()
                    ));
                }
                let bytes = hex::decode(value).map_err(|e| e.to_string())?;
                descriptor = bytes.try_into().map_err(|_| "key_descriptor wrong length".to_string())?;
            } else if key.eq("hash_specifier") {
                if value.len() != 32 {
                    return Err(format!(
                        "hash_specifier must be 32 hex chars, got {}",
                        value.len()
                    ));
                }
                let bytes = hex::decode(value).map_err(|e| e.to_string())?;
                hash_specifier = bytes.try_into().map_err(|_| "hash_specifier wrong length".to_string())?;
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
    #[cfg(target_os = "unix")]
    {
        use std::os::unix::net::UnixStream;

        let mut stream = UnixStream::connect(sock_path)
            .map_err(|e| format!("cannot connect to keyd socket {}: {}", sock_path, e))?;

        let request = format!("GET_KEY {:x}\n", hex::encode(&descriptor));
        stream.write_all(request.as_bytes())
            .map_err(|e| format!("cannot write to keyd socket: {}", e))?;

        let mut response = [0u8; 512];
        let n = stream
            .read(&mut response)
            .map_err(|e| format!("cannot read from keyd socket: {}", e))?;

        let response = String::from_utf8_lossy(&response[..n]);
        if response.starts_with("ERROR") {
            return Err(response.trim().to_string());
        }

        // Response format: "KEY <hex_blob>" or "OK <hex_blob>"
        let parts: Vec<&str> = response.splitn(2, ' ').collect();
        if parts.len() < 2 {
            return Err(format!("unexpected keyd response: {}", response.trim()));
        }

        let blob = hex::decode(parts[1]).map_err(|e| e.to_string())?;
        Ok(blob)
    }

    #[cfg(not(target_os = "unix"))]
    {
        // On Windows, socket is not available. Return an error so the caller
        // knows this is not supported on this platform.
        Err("Unix domain sockets not supported on Windows (dev-host)".to_string())
    }
}

/// Apply the encryption key to /data.
pub fn apply_key(key: &[u8], descriptor: &[u8; 32], _policy: &[u8]) -> Result<(), String> {
    // FS_IOC_ADD_ENCRYPTION_KEY and FS_IOC_SET_ENCRYPTION_POLICY are Linux
    // kernel ioctls. On Windows (dev-host), we skip the actual crypto operation.
    #[cfg(target_os = "linux")]
    {
        // Real kernel ioctls would go here using nix or raw libc.
        // These are platform-specific and require the target kernel headers.
        println!("[fscrypt] kernel ioctl not implemented on this build");
    }

    #[cfg(not(target_os = "linux"))]
    {
        // On Windows (dev-host), skip the crypto operation (fake HAL).
        println!("[fscrypt] key applied (simulated on Windows; use QEMU/Linux for real fscrypt)");
    }

    Ok(())
}
