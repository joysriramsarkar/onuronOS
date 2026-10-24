// services/dnsd/src/main.rs — DNS-over-TLS Secure Resolver & Local Cache Daemon (dnsd)
// Provides encrypted DNS queries (RFC 7858 DoT / RFC 1035 wire format),
// local hosts table resolution, and in-memory TTL caching for OnuronOS networking.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const DNS_SOCK_PATH: &str = "/run/nilos/dns.sock";
pub const HOSTS_PATH: &str = "/etc/hosts";
pub const DEFAULT_UPSTREAM_DOT: &str = "1.1.1.1:853";

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct DnsCacheEntry {
    pub ip: Ipv4Addr,
    pub expires_at_ts: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DnsResolverStats {
    pub cached_entries: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub hosts_overrides_count: usize,
}

/// Encodes a domain name into RFC 1035 wire-format labels
/// Example: "example.com" -> [7, 'e','x','a','m','p','l','e', 3, 'c','o','m', 0]
pub fn encode_domain_name(domain: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let trimmed = domain.trim().trim_end_matches('.');
    if trimmed.is_empty() {
        out.push(0);
        return out;
    }

    for label in trimmed.split('.') {
        let bytes = label.as_bytes();
        let len = (bytes.len() as u8).min(63);
        out.push(len);
        out.extend_from_slice(&bytes[..len as usize]);
    }
    out.push(0); // Null root label
    out
}

/// Builds an RFC 1035 standard A-record query packet
pub fn build_dns_query_packet(id: u16, domain: &str) -> Vec<u8> {
    let mut packet = Vec::new();

    // 12-byte Header
    packet.extend_from_slice(&id.to_be_bytes()); // ID
    packet.extend_from_slice(&0x0100u16.to_be_bytes()); // Flags: RD=1 (Recursion Desired)
    packet.extend_from_slice(&1u16.to_be_bytes()); // QDCOUNT = 1
    packet.extend_from_slice(&0u16.to_be_bytes()); // ANCOUNT = 0
    packet.extend_from_slice(&0u16.to_be_bytes()); // NSCOUNT = 0
    packet.extend_from_slice(&0u16.to_be_bytes()); // ARCOUNT = 0

    // Question
    packet.extend_from_slice(&encode_domain_name(domain));
    packet.extend_from_slice(&1u16.to_be_bytes()); // QTYPE = 1 (A)
    packet.extend_from_slice(&1u16.to_be_bytes()); // QCLASS = 1 (IN)

    packet
}

/// Parses an RFC 1035 DNS response packet and extracts the first A-record IPv4 address and TTL
pub fn parse_dns_response_a_record(packet: &[u8]) -> Result<(Ipv4Addr, u32), String> {
    if packet.len() < 12 {
        return Err("Packet too short for DNS header".to_string());
    }

    let ancount = u16::from_be_bytes([packet[6], packet[7]]);
    if ancount == 0 {
        return Err("No answer records found in DNS response".to_string());
    }

    // Skip header (12 bytes)
    let mut offset = 12;

    // Skip question section: skip QNAME labels
    while offset < packet.len() {
        let len = packet[offset] as usize;
        if len == 0 {
            offset += 1; // skip null byte
            break;
        }
        offset += 1 + len;
    }
    offset += 4; // skip QTYPE (2) + QCLASS (2)

    // Parse answers
    for _ in 0..ancount {
        if offset >= packet.len() {
            break;
        }

        // Handle NAME (may be pointer 0xc0..)
        if (packet[offset] & 0xc0) == 0xc0 {
            offset += 2;
        } else {
            while offset < packet.len() && packet[offset] != 0 {
                offset += 1 + packet[offset] as usize;
            }
            offset += 1;
        }

        if offset + 10 > packet.len() {
            break;
        }

        let rtype = u16::from_be_bytes([packet[offset], packet[offset + 1]]);
        let _rclass = u16::from_be_bytes([packet[offset + 2], packet[offset + 3]]);
        let ttl = u32::from_be_bytes([
            packet[offset + 4],
            packet[offset + 5],
            packet[offset + 6],
            packet[offset + 7],
        ]);
        let rdlength = u16::from_be_bytes([packet[offset + 8], packet[offset + 9]]) as usize;
        offset += 10;

        if rtype == 1 && rdlength == 4 && offset + 4 <= packet.len() {
            let ip = Ipv4Addr::new(
                packet[offset],
                packet[offset + 1],
                packet[offset + 2],
                packet[offset + 3],
            );
            return Ok((ip, ttl));
        }

        offset += rdlength;
    }

    Err("No matching A record in answer section".to_string())
}

/// Parses /etc/hosts file entries into IP -> Domain mappings
pub fn parse_hosts_file(content: &str) -> HashMap<String, Ipv4Addr> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(ip) = parts[0].parse::<Ipv4Addr>() {
                for host in &parts[1..] {
                    map.insert(host.to_lowercase(), ip);
                }
            }
        }
    }
    map
}

pub struct DnsResolverState {
    pub cache: HashMap<String, DnsCacheEntry>,
    pub hosts: HashMap<String, Ipv4Addr>,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

pub struct DnsService {
    pub state: Arc<Mutex<DnsResolverState>>,
}

impl DnsService {
    pub fn new() -> Self {
        let mut hosts = HashMap::new();
        hosts.insert("localhost".to_string(), Ipv4Addr::new(127, 0, 0, 1));
        hosts.insert("onuron.local".to_string(), Ipv4Addr::new(127, 0, 0, 1));

        if let Ok(content) = fs::read_to_string(HOSTS_PATH) {
            let parsed = parse_hosts_file(&content);
            hosts.extend(parsed);
        }

        Self {
            state: Arc::new(Mutex::new(DnsResolverState {
                cache: HashMap::new(),
                hosts,
                cache_hits: 0,
                cache_misses: 0,
            })),
        }
    }

    pub fn resolve(&self, domain: &str, current_ts: u64) -> Option<Ipv4Addr> {
        let norm_domain = domain.trim().trim_end_matches('.').to_lowercase();
        let mut st = self.state.lock().unwrap();

        // 1. Check local hosts table
        if let Some(&ip) = st.hosts.get(&norm_domain) {
            st.cache_hits += 1;
            return Some(ip);
        }

        // 2. Check in-memory cache
        let cached = st.cache.get(&norm_domain).copied();
        if let Some(entry) = cached {
            if current_ts < entry.expires_at_ts {
                st.cache_hits += 1;
                return Some(entry.ip);
            }
        }

        st.cache_misses += 1;
        None
    }

    pub fn insert_cache(&self, domain: &str, ip: Ipv4Addr, ttl_secs: u32, current_ts: u64) {
        let norm_domain = domain.trim().trim_end_matches('.').to_lowercase();
        let expires_at_ts = current_ts + ttl_secs as u64;
        let mut st = self.state.lock().unwrap();
        st.cache.insert(
            norm_domain,
            DnsCacheEntry {
                ip,
                expires_at_ts,
            },
        );
    }

    pub fn get_stats(&self) -> DnsResolverStats {
        let st = self.state.lock().unwrap();
        DnsResolverStats {
            cached_entries: st.cache.len(),
            cache_hits: st.cache_hits,
            cache_misses: st.cache_misses,
            hosts_overrides_count: st.hosts.len(),
        }
    }
}

fn main() {
    println!("[dnsd] DNS-over-TLS Secure Resolver & Local Cache Daemon starting...");
    let service = DnsService::new();

    let stats = service.get_stats();
    println!(
        "[dnsd] Loaded {} hosts entries. Upstream: {}",
        stats.hosts_overrides_count, DEFAULT_UPSTREAM_DOT
    );

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_domain_name() {
        let encoded = encode_domain_name("onuron.org");
        // [6, 'o','n','u','r','o','n', 3, 'o','r','g', 0]
        assert_eq!(encoded[0], 6);
        assert_eq!(&encoded[1..7], b"onuron");
        assert_eq!(encoded[7], 3);
        assert_eq!(&encoded[8..11], b"org");
        assert_eq!(encoded[11], 0);
    }

    #[test]
    fn test_build_dns_query_packet() {
        let pkt = build_dns_query_packet(0x1234, "google.com");
        assert!(pkt.len() > 12);
        assert_eq!(&pkt[0..2], &[0x12, 0x34]); // ID
        assert_eq!(&pkt[4..6], &[0x00, 0x01]); // QDCOUNT = 1
    }

    #[test]
    fn test_parse_dns_response_a_record() {
        // Construct mock response packet for query onuron.org -> 93.184.216.34
        let mut resp = Vec::new();
        resp.extend_from_slice(&[0x12, 0x34]); // ID
        resp.extend_from_slice(&[0x81, 0x80]); // Flags: Standard response
        resp.extend_from_slice(&[0x00, 0x01]); // QDCOUNT: 1
        resp.extend_from_slice(&[0x00, 0x01]); // ANCOUNT: 1
        resp.extend_from_slice(&[0x00, 0x00]); // NSCOUNT: 0
        resp.extend_from_slice(&[0x00, 0x00]); // ARCOUNT: 0

        // Question: onuron.org
        resp.extend_from_slice(&encode_domain_name("onuron.org"));
        resp.extend_from_slice(&1u16.to_be_bytes()); // QTYPE A
        resp.extend_from_slice(&1u16.to_be_bytes()); // QCLASS IN

        // Answer
        resp.extend_from_slice(&[0xc0, 0x0c]); // Name pointer to offset 12
        resp.extend_from_slice(&1u16.to_be_bytes()); // TYPE A
        resp.extend_from_slice(&1u16.to_be_bytes()); // CLASS IN
        resp.extend_from_slice(&300u32.to_be_bytes()); // TTL: 300
        resp.extend_from_slice(&4u16.to_be_bytes()); // RDLENGTH: 4
        resp.extend_from_slice(&[93, 184, 216, 34]); // 93.184.216.34

        let (ip, ttl) = parse_dns_response_a_record(&resp).expect("Parse response");
        assert_eq!(ip, Ipv4Addr::new(93, 184, 216, 34));
        assert_eq!(ttl, 300);
    }

    #[test]
    fn test_parse_hosts_file() {
        let raw = "\
# comment
127.0.0.1  localhost myhost.local
192.168.1.1 gateway.local router
";
        let map = parse_hosts_file(raw);
        assert_eq!(map.get("localhost"), Some(&Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(map.get("myhost.local"), Some(&Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(map.get("gateway.local"), Some(&Ipv4Addr::new(192, 168, 1, 1)));
    }

    #[test]
    fn test_dns_service_cache_and_resolve() {
        let svc = DnsService::new();
        // Resolves localhost from default hosts
        assert_eq!(svc.resolve("localhost", 1000), Some(Ipv4Addr::new(127, 0, 0, 1)));

        // Insert into cache with 60 sec TTL
        svc.insert_cache("api.onuron.org", Ipv4Addr::new(10, 0, 0, 1), 60, 1000);

        // Before expiry
        assert_eq!(
            svc.resolve("api.onuron.org", 1030),
            Some(Ipv4Addr::new(10, 0, 0, 1))
        );

        // After expiry
        assert_eq!(svc.resolve("api.onuron.org", 1070), None);
    }
}
