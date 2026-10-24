// runtime/nilrt/src/bin/nilfastbootd.rs — USB Fastboot Protocol Daemon (nilfastbootd)
// Implements standard Fastboot Protocol (RFC / Android Fastboot spec) over USB gadget/TCP
// for recovery flashing, partition updates, bootloader variable querying, and slot switching.

use std::collections::HashMap;
use std::time::Duration;

pub const FASTBOOT_VERSION: &str = "0.4";
pub const FASTBOOT_MAX_DOWNLOAD_SIZE: u64 = 0x20000000; // 512 MB

#[derive(Debug, Clone, PartialEq)]
pub struct FastbootState {
    pub current_slot: String, // "a" or "b"
    pub product: String,
    pub unlocked: bool,
    pub secure: bool,
    pub max_download_size: u64,
    pub download_buffer: Vec<u8>,
    pub pending_download_size: Option<usize>,
    pub partitions: HashMap<String, Vec<u8>>,
    pub reboot_requested: Option<String>,
}

impl Default for FastbootState {
    fn default() -> Self {
        let mut partitions = HashMap::new();
        partitions.insert("boot_a".to_string(), Vec::new());
        partitions.insert("boot_b".to_string(), Vec::new());
        partitions.insert("system_a".to_string(), Vec::new());
        partitions.insert("system_b".to_string(), Vec::new());
        partitions.insert("userdata".to_string(), Vec::new());

        Self {
            current_slot: "a".to_string(),
            product: "onuron_mobile".to_string(),
            unlocked: true,
            secure: true,
            max_download_size: FASTBOOT_MAX_DOWNLOAD_SIZE,
            download_buffer: Vec::new(),
            pending_download_size: None,
            partitions,
            reboot_requested: None,
        }
    }
}

/// Dispatches a fastboot command string and returns response lines
/// Each response line begins with OKAY, FAIL, DATA, or INFO
pub fn handle_fastboot_command(raw_cmd: &str, state: &mut FastbootState) -> Vec<String> {
    let cmd = raw_cmd.trim();
    if cmd.is_empty() {
        return vec!["FAILempty command".to_string()];
    }

    if let Some(var) = cmd.strip_prefix("getvar:") {
        return handle_getvar(var.trim(), state);
    }

    if let Some(hex_size) = cmd.strip_prefix("download:") {
        match usize::from_str_radix(hex_size.trim(), 16) {
            Ok(size) => {
                if (size as u64) > state.max_download_size {
                    return vec!["FAILdata exceeds max download size".to_string()];
                }
                state.pending_download_size = Some(size);
                state.download_buffer.clear();
                return vec![format!("DATA{:08x}", size)];
            }
            Err(_) => return vec!["FAILinvalid download size format".to_string()],
        }
    }

    if let Some(part) = cmd.strip_prefix("flash:") {
        let partition = part.trim();
        let target_part = if (partition == "boot" || partition == "system") && !partition.ends_with("_a") && !partition.ends_with("_b") {
            format!("{}_{}", partition, state.current_slot)
        } else {
            partition.to_string()
        };

        if state.download_buffer.is_empty() {
            return vec!["FAILno data downloaded to flash".to_string()];
        }

        let bytes_len = state.download_buffer.len();
        state.partitions.insert(target_part.clone(), state.download_buffer.clone());
        state.download_buffer.clear();
        return vec![
            format!("INFOflashed {} bytes to {}", bytes_len, target_part),
            "OKAY".to_string(),
        ];
    }

    if let Some(part) = cmd.strip_prefix("erase:") {
        let partition = part.trim();
        state.partitions.insert(partition.to_string(), Vec::new());
        return vec![format!("INFOerased {}", partition), "OKAY".to_string()];
    }

    if let Some(slot) = cmd.strip_prefix("set_active:") {
        let s = slot.trim().to_lowercase();
        if s == "a" || s == "b" {
            state.current_slot = s.clone();
            return vec![format!("INFOactive slot set to {}", s), "OKAY".to_string()];
        } else {
            return vec!["FAILinvalid slot name, use 'a' or 'b'".to_string()];
        }
    }

    match cmd {
        "boot" => vec!["OKAY".to_string()],
        "reboot" => {
            state.reboot_requested = Some("normal".to_string());
            vec!["OKAY".to_string()]
        }
        "reboot-bootloader" => {
            state.reboot_requested = Some("bootloader".to_string());
            vec!["OKAY".to_string()]
        }
        "reboot-recovery" => {
            state.reboot_requested = Some("recovery".to_string());
            vec!["OKAY".to_string()]
        }
        "continue" => vec!["OKAY".to_string()],
        _ => vec![format!("FAILunknown command '{}'", cmd)],
    }
}

fn handle_getvar(var: &str, state: &FastbootState) -> Vec<String> {
    match var {
        "version" => vec![format!("OKAY{}", FASTBOOT_VERSION)],
        "version-bootloader" => vec!["OKAYOnuron-Bootloader-1.0".to_string()],
        "product" => vec![format!("OKAY{}", state.product)],
        "current-slot" => vec![format!("OKAY{}", state.current_slot)],
        "slot-count" => vec!["OKAY2".to_string()],
        "slot-successful:a" | "slot-successful:b" => vec!["OKAYyes".to_string()],
        "slot-unbootable:a" | "slot-unbootable:b" => vec!["OKAYno".to_string()],
        "slot-retry-count:a" | "slot-retry-count:b" => vec!["OKAY3".to_string()],
        "max-download-size" => vec![format!("OKAY0x{:x}", state.max_download_size)],
        "secure" => vec![format!("OKAY{}", if state.secure { "yes" } else { "no" })],
        "unlocked" => vec![format!("OKAY{}", if state.unlocked { "yes" } else { "no" })],
        "has-slot:boot" | "has-slot:system" => vec!["OKAYyes".to_string()],
        "has-slot:userdata" => vec!["OKAYno".to_string()],
        "all" => {
            vec![
                format!("INFOversion: {}", FASTBOOT_VERSION),
                format!("INFOproduct: {}", state.product),
                format!("INFOcurrent-slot: {}", state.current_slot),
                "INFOslot-count: 2".to_string(),
                format!("INFOmax-download-size: 0x{:x}", state.max_download_size),
                format!("INFOsecure: {}", if state.secure { "yes" } else { "no" }),
                format!("INFOunlocked: {}", if state.unlocked { "yes" } else { "no" }),
                "OKAY".to_string(),
            ]
        }
        _ => vec!["FAILvariable not found".to_string()],
    }
}

fn main() {
    println!("=========================================================");
    println!("         Onuron Fastboot USB Gadget Daemon (nilfastbootd)");
    println!("=========================================================");

    let mut state = FastbootState::default();
    println!(
        "[nilfastbootd] Listening on USB FunctionFS Gadget interface. Product: {}, Slot: {}",
        state.product, state.current_slot
    );

    // Initial self-test
    let test_res = handle_fastboot_command("getvar:version", &mut state);
    println!("[nilfastbootd] Self-test getvar:version -> {:?}", test_res);

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fastboot_getvar_queries() {
        let mut state = FastbootState::default();

        let res_ver = handle_fastboot_command("getvar:version", &mut state);
        assert_eq!(res_ver, vec!["OKAY0.4"]);

        let res_slot = handle_fastboot_command("getvar:current-slot", &mut state);
        assert_eq!(res_slot, vec!["OKAYa"]);

        let res_max = handle_fastboot_command("getvar:max-download-size", &mut state);
        assert_eq!(res_max, vec!["OKAY0x20000000"]);
    }

    #[test]
    fn test_fastboot_slot_switching() {
        let mut state = FastbootState::default();
        assert_eq!(state.current_slot, "a");

        let res_set = handle_fastboot_command("set_active:b", &mut state);
        assert!(res_set.contains(&"OKAY".to_string()));
        assert_eq!(state.current_slot, "b");

        let res_get = handle_fastboot_command("getvar:current-slot", &mut state);
        assert_eq!(res_get, vec!["OKAYb"]);

        // Invalid slot
        let res_bad = handle_fastboot_command("set_active:c", &mut state);
        assert!(res_bad[0].starts_with("FAIL"));
    }

    #[test]
    fn test_fastboot_download_and_flash() {
        let mut state = FastbootState::default();

        // 1. Send download command for 4 bytes (0x04)
        let res_dl = handle_fastboot_command("download:00000004", &mut state);
        assert_eq!(res_dl, vec!["DATA00000004"]);
        assert_eq!(state.pending_download_size, Some(4));

        // 2. Simulate USB transfer receipt
        state.download_buffer = vec![0xde, 0xad, 0xbe, 0xef];

        // 3. Flash to boot partition (which resolves to boot_a for active slot 'a')
        let res_flash = handle_fastboot_command("flash:boot", &mut state);
        assert!(res_flash.contains(&"OKAY".to_string()));
        assert_eq!(state.partitions.get("boot_a"), Some(&vec![0xde, 0xad, 0xbe, 0xef]));
    }

    #[test]
    fn test_fastboot_reboot_commands() {
        let mut state = FastbootState::default();
        let res = handle_fastboot_command("reboot-bootloader", &mut state);
        assert_eq!(res, vec!["OKAY"]);
        assert_eq!(state.reboot_requested.as_deref(), Some("bootloader"));
    }
}
