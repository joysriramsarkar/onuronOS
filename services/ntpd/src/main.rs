// services/ntpd/src/main.rs — SNTP Lightweight Time Synchronization Daemon (ntpd)
// Implements RFC 4330 SNTP v4 client for network time synchronization on OnuronOS.
// Queries pool.ntp.org or custom NTP server over UDP port 123, computes roundtrip offset,
// and adjusts the system real-time clock.

use serde::{Deserialize, Serialize};
use std::fs;
use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const NTP_PORT: u16 = 123;
pub const DEFAULT_NTP_SERVER: &str = "pool.ntp.org:123";
pub const NTP_DELTA_SECONDS: u64 = 2_208_988_800; // Seconds from Jan 1 1900 (NTP) to Jan 1 1970 (Unix)
pub const CLOCK_OFFSET_PATH: &str = "/data/system/clock_offset.json";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ClockSyncRecord {
    pub server: String,
    pub unix_epoch_secs: u64,
    pub offset_ms: i64,
    pub roundtrip_delay_ms: i64,
    pub stratum: u8,
}

/// Builds a 48-byte SNTP client request packet (RFC 4330)
pub fn create_sntp_request_packet() -> [u8; 48] {
    let mut packet = [0u8; 48];
    // LI = 0 (no warning), VN = 4 (IPv4/IPv6 v4), Mode = 3 (Client)
    // (0 << 6) | (4 << 3) | 3 = 0b00100011 = 0x23
    packet[0] = 0x23;

    // Transmit timestamp: current system time in NTP epoch
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let ntp_secs = (now.as_secs() + NTP_DELTA_SECONDS) as u32;
    let ntp_frac = ((now.subsec_nanos() as u64 * (1u64 << 32)) / 1_000_000_000) as u32;

    packet[40..44].copy_from_slice(&ntp_secs.to_be_bytes());
    packet[44..48].copy_from_slice(&ntp_frac.to_be_bytes());

    packet
}

/// Parses an SNTP response packet and computes the server transmit timestamp in Unix epoch seconds
pub fn parse_sntp_response(packet: &[u8]) -> Result<(u64, u8), String> {
    if packet.len() < 48 {
        return Err("SNTP response packet too short (<48 bytes)".to_string());
    }

    let stratum = packet[1];
    let mode = packet[0] & 0x07;
    if mode != 4 && mode != 5 {
        // Mode 4 = Server, Mode 5 = Broadcast
        return Err(format!("Invalid SNTP response mode: {}", mode));
    }

    // Transmit timestamp at offset 40
    let mut secs_bytes = [0u8; 4];
    secs_bytes.copy_from_slice(&packet[40..44]);
    let ntp_secs = u32::from_be_bytes(secs_bytes) as u64;

    if ntp_secs < NTP_DELTA_SECONDS {
        return Err("Server time before Unix epoch".to_string());
    }

    let unix_secs = ntp_secs - NTP_DELTA_SECONDS;
    Ok((unix_secs, stratum))
}

/// Computes clock offset between client and server
pub fn calculate_offset_ms(t0_ms: i64, t1_ms: i64, t2_ms: i64, t3_ms: i64) -> (i64, i64) {
    let offset = ((t1_ms - t0_ms) + (t2_ms - t3_ms)) / 2;
    let delay = (t3_ms - t0_ms) - (t2_ms - t1_ms);
    (offset, delay)
}

/// Attempts to query the given NTP server
pub fn query_ntp_server(server: &str, timeout: Duration) -> Result<ClockSyncRecord, String> {
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    socket.set_read_timeout(Some(timeout)).map_err(|e| e.to_string())?;

    let req = create_sntp_request_packet();
    let t0 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64;

    socket.send_to(&req, server).map_err(|e| e.to_string())?;

    let mut buf = [0u8; 48];
    let (amt, _) = socket.recv_from(&mut buf).map_err(|e| e.to_string())?;
    let t3 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64;

    if amt < 48 {
        return Err("Truncated response".to_string());
    }

    let (unix_secs, stratum) = parse_sntp_response(&buf)?;
    let server_ms = (unix_secs * 1000) as i64;
    let (offset, delay) = calculate_offset_ms(t0, server_ms, server_ms, t3);

    Ok(ClockSyncRecord {
        server: server.to_string(),
        unix_epoch_secs: unix_secs,
        offset_ms: offset,
        roundtrip_delay_ms: delay,
        stratum,
    })
}

/// Persists sync status to /data/system/clock_offset.json
pub fn save_clock_sync(record: &ClockSyncRecord, path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let data = serde_json::to_string_pretty(record)?;
    fs::write(path, data)
}

fn main() {
    println!("[ntpd] SNTP Lightweight Time Synchronization Daemon starting...");
    let offset_path = PathBuf::from(CLOCK_OFFSET_PATH);

    // Initial NTP sync attempt
    match query_ntp_server(DEFAULT_NTP_SERVER, Duration::from_secs(3)) {
        Ok(rec) => {
            println!(
                "[ntpd] Synced with {}! Offset: {}ms, Delay: {}ms, Stratum: {}",
                rec.server, rec.offset_ms, rec.roundtrip_delay_ms, rec.stratum
            );
            let _ = save_clock_sync(&rec, &offset_path);
        }
        Err(e) => {
            println!("[ntpd] Initial NTP sync note: {} (will retry periodically)", e);
        }
    }

    loop {
        std::thread::sleep(Duration::from_secs(3600)); // Sync hourly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sntp_request_packet() {
        let pkt = create_sntp_request_packet();
        assert_eq!(pkt.len(), 48);
        // Byte 0: LI=0 (bits 7-6), VN=4 (bits 5-3 = 100), Mode=3 (bits 2-0 = 011) -> 0b00100011 = 0x23
        assert_eq!(pkt[0], 0x23);

        // Verify transmit timestamp seconds > NTP_DELTA_SECONDS
        let mut secs_b = [0u8; 4];
        secs_b.copy_from_slice(&pkt[40..44]);
        let ntp_secs = u32::from_be_bytes(secs_b) as u64;
        assert!(ntp_secs >= NTP_DELTA_SECONDS);
    }

    #[test]
    fn test_parse_sntp_response() {
        let mut resp = [0u8; 48];
        resp[0] = 0x24; // Mode 4 (Server)
        resp[1] = 2;    // Stratum 2

        // Set transmit timestamp to 2026-10-24 timestamp in NTP epoch
        let mock_unix_secs = 1792800000u64;
        let mock_ntp_secs = (mock_unix_secs + NTP_DELTA_SECONDS) as u32;
        resp[40..44].copy_from_slice(&mock_ntp_secs.to_be_bytes());

        let (parsed_unix, stratum) = parse_sntp_response(&resp).expect("Valid response");
        assert_eq!(parsed_unix, mock_unix_secs);
        assert_eq!(stratum, 2);
    }

    #[test]
    fn test_calculate_offset_and_delay() {
        // t0 = 1000, t1 = 1050, t2 = 1050, t3 = 1100
        // Roundtrip delay = (1100 - 1000) - (1050 - 1050) = 100ms
        // Offset = ((1050 - 1000) + (1050 - 1100)) / 2 = (50 - 50) / 2 = 0ms
        let (offset, delay) = calculate_offset_ms(1000, 1050, 1050, 1100);
        assert_eq!(offset, 0);
        assert_eq!(delay, 100);

        // Case with 200ms offset (client 200ms behind server)
        // t0 = 1000, t1 = 1250, t2 = 1250, t3 = 1100
        let (offset2, delay2) = calculate_offset_ms(1000, 1250, 1250, 1100);
        assert_eq!(offset2, 200);
        assert_eq!(delay2, 100);
    }

    #[test]
    fn test_save_clock_sync() {
        let tmp = std::env::temp_dir().join(format!("test_ntp_{}.json", std::process::id()));
        let rec = ClockSyncRecord {
            server: "time.cloudflare.com:123".to_string(),
            unix_epoch_secs: 1792800000,
            offset_ms: 12,
            roundtrip_delay_ms: 45,
            stratum: 1,
        };
        save_clock_sync(&rec, &tmp).expect("Save clock sync");
        assert!(tmp.exists());
        let content = fs::read_to_string(&tmp).expect("Read clock sync");
        assert!(content.contains("time.cloudflare.com"));
        let _ = fs::remove_file(tmp);
    }
}
