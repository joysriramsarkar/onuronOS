// protocol/src/lib.rs — OnuronOS Canonical Versioned Framed IPC Protocol
//
// Every Onuron system service communicates over Unix domain sockets using this
// length-prefixed, versioned, framed binary protocol.
//
// Frame Layout (Wire Format):
// ┌────────────────┬───────────────────┬────────────────┬─────────────────┬───────────┬───────────────┐
// │ Magic (4B)     │ Payload Len (4B)  │ Version (2B)   │ Msg Type (2B)   │ Req ID (8B)│ Flags (4B)    │
// ├────────────────┼───────────────────┼────────────────┼─────────────────┼───────────┼───────────────┤
// │ "ONUR" [ASCII] │ Big-Endian u32    │ Big-Endian u16 │ Big-Endian u16  │ Big-End u64│ Big-End u32   │
// └────────────────┴───────────────────┴────────────────┴─────────────────┴───────────┴───────────────┘
// Followed by `payload_len` bytes of serialized payload (JSON / Bincode / Protobuf).

use std::io::{self, Read, Write};
use serde::{Deserialize, Serialize};

/// 4-byte protocol magic preamble: "ONUR"
pub const PROTOCOL_MAGIC: [u8; 4] = *b"ONUR";

/// Current protocol wire version
pub const PROTOCOL_VERSION_1: u16 = 1;

/// Maximum payload size allowed per frame (1 MiB). Unbounded frames are rejected.
pub const MAX_PAYLOAD_SIZE: u32 = 1024 * 1024;

/// Standard frame header size in bytes: 4 (magic) + 4 (len) + 2 (ver) + 2 (type) + 8 (req) + 4 (flags) = 24 bytes
pub const HEADER_SIZE: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frame {
    pub version: u16,
    pub message_type: u16,
    pub request_id: u64,
    pub flags: u32,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub enum IpcError {
    InvalidMagic([u8; 4]),
    UnsupportedVersion(u16),
    PayloadTooLarge { length: u32, max: u32 },
    Io(io::Error),
    Serialization(String),
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpcError::InvalidMagic(m) => write!(f, "Invalid protocol magic: {:?}", m),
            IpcError::UnsupportedVersion(v) => write!(f, "Unsupported protocol version: {}", v),
            IpcError::PayloadTooLarge { length, max } => {
                write!(f, "Frame payload too large ({} bytes > max {})", length, max)
            }
            IpcError::Io(e) => write!(f, "IPC I/O error: {}", e),
            IpcError::Serialization(s) => write!(f, "Serialization error: {}", s),
        }
    }
}

impl std::error::Error for IpcError {}

impl From<io::Error> for IpcError {
    fn from(err: io::Error) -> Self {
        IpcError::Io(err)
    }
}

// ─── Standard Message Types Catalog ──────────────────────────────────────────

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    // System Core (0x0001 - 0x00FF)
    Ping = 0x0001,
    Pong = 0x0002,
    ErrorResponse = 0x0003,

    // Power Service (0x0100 - 0x01FF)
    PowerGetBattery = 0x0101,
    PowerBatteryInfo = 0x0102,
    PowerAcquireWakelock = 0x0103,
    PowerReleaseWakelock = 0x0104,

    // Input Service (0x0200 - 0x02FF)
    InputPollEvents = 0x0201,
    InputEventsBatch = 0x0202,
    InputInjectEvent = 0x0203,

    // Network Service (0x0300 - 0x03FF)
    NetGetState = 0x0301,
    NetStateInfo = 0x0302,
    NetScanWifi = 0x0303,

    // Telephony Service (0x0400 - 0x04FF)
    TelephonyDial = 0x0401,
    TelephonyHangup = 0x0402,
    TelephonySendSms = 0x0403,

    // Update Service (0x0500 - 0x05FF)
    UpdateGetStatus = 0x0501,
    UpdateStatusInfo = 0x0502,
    UpdateApplyPayload = 0x0503,
    UpdateRollback = 0x0504,

    // Security / Keystore Service (0x0600 - 0x06FF)
    KeyGetStatus = 0x0601,
    KeyStatusInfo = 0x0602,
    KeyUnlock = 0x0603,

    // Unknown/Custom
    Custom(u16),
}

impl From<u16> for MessageType {
    fn from(code: u16) -> Self {
        match code {
            0x0001 => MessageType::Ping,
            0x0002 => MessageType::Pong,
            0x0003 => MessageType::ErrorResponse,
            0x0101 => MessageType::PowerGetBattery,
            0x0102 => MessageType::PowerBatteryInfo,
            0x0103 => MessageType::PowerAcquireWakelock,
            0x0104 => MessageType::PowerReleaseWakelock,
            0x0201 => MessageType::InputPollEvents,
            0x0202 => MessageType::InputEventsBatch,
            0x0203 => MessageType::InputInjectEvent,
            0x0301 => MessageType::NetGetState,
            0x0302 => MessageType::NetStateInfo,
            0x0303 => MessageType::NetScanWifi,
            0x0401 => MessageType::TelephonyDial,
            0x0402 => MessageType::TelephonyHangup,
            0x0403 => MessageType::TelephonySendSms,
            0x0501 => MessageType::UpdateGetStatus,
            0x0502 => MessageType::UpdateStatusInfo,
            0x0503 => MessageType::UpdateApplyPayload,
            0x0504 => MessageType::UpdateRollback,
            0x0601 => MessageType::KeyGetStatus,
            0x0602 => MessageType::KeyStatusInfo,
            0x0603 => MessageType::KeyUnlock,
            other => MessageType::Custom(other),
        }
    }
}

impl From<MessageType> for u16 {
    fn from(msg: MessageType) -> Self {
        match msg {
            MessageType::Ping => 0x0001,
            MessageType::Pong => 0x0002,
            MessageType::ErrorResponse => 0x0003,
            MessageType::PowerGetBattery => 0x0101,
            MessageType::PowerBatteryInfo => 0x0102,
            MessageType::PowerAcquireWakelock => 0x0103,
            MessageType::PowerReleaseWakelock => 0x0104,
            MessageType::InputPollEvents => 0x0201,
            MessageType::InputEventsBatch => 0x0202,
            MessageType::InputInjectEvent => 0x0203,
            MessageType::NetGetState => 0x0301,
            MessageType::NetStateInfo => 0x0302,
            MessageType::NetScanWifi => 0x0303,
            MessageType::TelephonyDial => 0x0401,
            MessageType::TelephonyHangup => 0x0402,
            MessageType::TelephonySendSms => 0x0403,
            MessageType::UpdateGetStatus => 0x0501,
            MessageType::UpdateStatusInfo => 0x0502,
            MessageType::UpdateApplyPayload => 0x0503,
            MessageType::UpdateRollback => 0x0504,
            MessageType::KeyGetStatus => 0x0601,
            MessageType::KeyStatusInfo => 0x0602,
            MessageType::KeyUnlock => 0x0603,
            MessageType::Custom(c) => c,
        }
    }
}

// ─── Frame Reader & Writer ───────────────────────────────────────────────────

impl Frame {
    pub fn new(message_type: impl Into<u16>, request_id: u64, payload: Vec<u8>) -> Self {
        Self {
            version: PROTOCOL_VERSION_1,
            message_type: message_type.into(),
            request_id,
            flags: 0,
            payload,
        }
    }

    pub fn with_json<T: Serialize>(
        message_type: impl Into<u16>,
        request_id: u64,
        value: &T,
    ) -> Result<Self, IpcError> {
        let payload = serde_json::to_vec(value)
            .map_err(|e| IpcError::Serialization(e.to_string()))?;
        Ok(Self::new(message_type, request_id, payload))
    }

    pub fn parse_json<T: for<'a> Deserialize<'a>>(&self) -> Result<T, IpcError> {
        serde_json::from_slice(&self.payload)
            .map_err(|e| IpcError::Serialization(e.to_string()))
    }
}

/// Read a single framed message from a synchronous reader.
pub fn read_frame<R: Read>(reader: &mut R) -> Result<Frame, IpcError> {
    let mut header = [0u8; HEADER_SIZE];
    reader.read_exact(&mut header)?;

    // 1. Verify Magic
    let magic: [u8; 4] = header[0..4].try_into().unwrap();
    if magic != PROTOCOL_MAGIC {
        return Err(IpcError::InvalidMagic(magic));
    }

    // 2. Read Payload Length
    let payload_len = u32::from_be_bytes(header[4..8].try_into().unwrap());
    if payload_len > MAX_PAYLOAD_SIZE {
        return Err(IpcError::PayloadTooLarge {
            length: payload_len,
            max: MAX_PAYLOAD_SIZE,
        });
    }

    // 3. Read Header Fields
    let version = u16::from_be_bytes(header[8..10].try_into().unwrap());
    if version != PROTOCOL_VERSION_1 {
        return Err(IpcError::UnsupportedVersion(version));
    }

    let message_type = u16::from_be_bytes(header[10..12].try_into().unwrap());
    let request_id = u64::from_be_bytes(header[12..20].try_into().unwrap());
    let flags = u32::from_be_bytes(header[20..24].try_into().unwrap());

    // 4. Read Payload Bytes
    let mut payload = vec![0u8; payload_len as usize];
    reader.read_exact(&mut payload)?;

    Ok(Frame {
        version,
        message_type,
        request_id,
        flags,
        payload,
    })
}

/// Write a single framed message to a synchronous writer.
pub fn write_frame<W: Write>(writer: &mut W, frame: &Frame) -> Result<(), IpcError> {
    let payload_len = frame.payload.len() as u32;
    if payload_len > MAX_PAYLOAD_SIZE {
        return Err(IpcError::PayloadTooLarge {
            length: payload_len,
            max: MAX_PAYLOAD_SIZE,
        });
    }

    let mut header = [0u8; HEADER_SIZE];
    header[0..4].copy_from_slice(&PROTOCOL_MAGIC);
    header[4..8].copy_from_slice(&payload_len.to_be_bytes());
    header[8..10].copy_from_slice(&frame.version.to_be_bytes());
    header[10..12].copy_from_slice(&frame.message_type.to_be_bytes());
    header[12..20].copy_from_slice(&frame.request_id.to_be_bytes());
    header[20..24].copy_from_slice(&frame.flags.to_be_bytes());

    writer.write_all(&header)?;
    writer.write_all(&frame.payload)?;
    writer.flush()?;

    Ok(())
}

// ─── Peer Identity & Credentials ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerCredentials {
    pub pid: i32,
    pub uid: u32,
    pub gid: u32,
}

#[cfg(unix)]
pub fn get_peer_credentials(stream: &std::os::unix::net::UnixStream) -> io::Result<PeerCredentials> {
    use std::os::unix::io::AsRawFd;

    #[cfg(target_os = "linux")]
    {
        let fd = stream.as_raw_fd();
        let mut ucred = libc::ucred {
            pid: 0,
            uid: 0,
            gid: 0,
        };
        let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;

        let ret = unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                &mut ucred as *mut _ as *mut libc::c_void,
                &mut len,
            )
        };

        if ret == 0 {
            Ok(PeerCredentials {
                pid: ucred.pid as i32,
                uid: ucred.uid,
                gid: ucred.gid,
            })
        } else {
            Err(io::Error::last_os_error())
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok(PeerCredentials { pid: 1000, uid: 1000, gid: 1000 })
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct MockPayload {
        user: String,
        count: u32,
    }

    #[test]
    fn test_frame_roundtrip() {
        let original = Frame::new(MessageType::PowerGetBattery, 42, b"hello payload".to_vec());
        let mut buffer = Vec::new();

        write_frame(&mut buffer, &original).expect("write frame");
        assert_eq!(buffer.len(), HEADER_SIZE + b"hello payload".len());

        let mut cursor = Cursor::new(buffer);
        let decoded = read_frame(&mut cursor).expect("read frame");

        assert_eq!(decoded.version, PROTOCOL_VERSION_1);
        assert_eq!(decoded.message_type, u16::from(MessageType::PowerGetBattery));
        assert_eq!(decoded.request_id, 42);
        assert_eq!(decoded.payload, b"hello payload".to_vec());
    }

    #[test]
    fn test_frame_with_json_payload() {
        let data = MockPayload {
            user: "onuron".to_string(),
            count: 7,
        };

        let frame = Frame::with_json(MessageType::Ping, 101, &data).expect("frame with json");
        let mut buffer = Vec::new();
        write_frame(&mut buffer, &frame).expect("write");

        let mut cursor = Cursor::new(buffer);
        let read = read_frame(&mut cursor).expect("read");
        let parsed: MockPayload = read.parse_json().expect("parse json");

        assert_eq!(parsed, data);
    }

    #[test]
    fn test_invalid_magic_rejected() {
        let mut bad_header = [0u8; HEADER_SIZE];
        bad_header[0..4].copy_from_slice(b"BAD!");
        let mut cursor = Cursor::new(bad_header);

        match read_frame(&mut cursor) {
            Err(IpcError::InvalidMagic(m)) => assert_eq!(&m, b"BAD!"),
            other => panic!("Expected InvalidMagic, got {:?}", other),
        }
    }

    #[test]
    fn test_oversized_payload_rejected() {
        // artificially set huge size in header
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&PROTOCOL_MAGIC);
        buffer.extend_from_slice(&(MAX_PAYLOAD_SIZE + 100).to_be_bytes());
        buffer.extend_from_slice(&1u16.to_be_bytes()); // version
        buffer.extend_from_slice(&1u16.to_be_bytes()); // type
        buffer.extend_from_slice(&1u64.to_be_bytes()); // req_id
        buffer.extend_from_slice(&0u32.to_be_bytes()); // flags

        let mut cursor = Cursor::new(buffer);
        match read_frame(&mut cursor) {
            Err(IpcError::PayloadTooLarge { length, .. }) => {
                assert_eq!(length, MAX_PAYLOAD_SIZE + 100);
            }
            other => panic!("Expected PayloadTooLarge, got {:?}", other),
        }
    }
}
