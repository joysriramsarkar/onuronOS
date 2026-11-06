// runtime/nilui/src/state.rs — NILS Handoff State Snapshot Serialization
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crc32fast::Hasher;
use std::io::Read;

pub const NILS_MAGIC: u32 = 0x4E494C53; // 'NILS'
pub const NILS_VERSION: u32 = 1;

pub struct SnapshotPayload {
    pub app_id: String,
    pub schema_hash: u64,
    pub timestamp_ms: u64,
    pub data: Vec<u8>,
}

impl SnapshotPayload {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_u32::<BigEndian>(NILS_MAGIC).unwrap();
        buf.write_u32::<BigEndian>(NILS_VERSION).unwrap();

        let app_bytes = self.app_id.as_bytes();
        buf.write_u16::<BigEndian>(app_bytes.len() as u16).unwrap();
        buf.extend_from_slice(app_bytes);

        buf.write_u64::<BigEndian>(self.schema_hash).unwrap();
        buf.write_u64::<BigEndian>(self.timestamp_ms).unwrap();

        buf.write_u32::<BigEndian>(self.data.len() as u32).unwrap();
        buf.extend_from_slice(&self.data);

        let mut hasher = Hasher::new();
        hasher.update(&buf);
        let checksum = hasher.finalize();
        buf.write_u32::<BigEndian>(checksum).unwrap();

        buf
    }

    pub fn deserialize(mut slice: &[u8]) -> Result<Self, String> {
        const MAX_APP_LEN: usize = 256;
        const MAX_DATA_LEN: usize = 64 * 1024 * 1024; // 64 MiB cap
        if slice.len() < 32 {
            return Err("Payload too short".into());
        }
        // Verify checksum first: the trailing u32 must match CRC32 over the
        // preceding bytes.
        let body_len = slice.len() - 4;
        let body = &slice[..body_len];
        let stored_crc = u32::from_be_bytes(slice[body_len..].try_into().unwrap());
        let mut hasher = Hasher::new();
        hasher.update(body);
        if hasher.finalize() != stored_crc {
            return Err("NILS checksum mismatch (forged or corrupted snapshot)".into());
        }

        let mut cur = body;
        let magic = cur.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
        if magic != NILS_MAGIC {
            return Err("Invalid NILS magic header".into());
        }
        let version = cur.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
        if version != NILS_VERSION {
            return Err(format!("Unsupported NILS version {version}"));
        }
        let app_len = cur.read_u16::<BigEndian>().map_err(|e| e.to_string())? as usize;
        if app_len == 0 || app_len > MAX_APP_LEN {
            return Err("Invalid app_id length".into());
        }
        let mut app_id_bytes = vec![0u8; app_len];
        cur.read_exact(&mut app_id_bytes)
            .map_err(|e| e.to_string())?;
        let app_id = String::from_utf8(app_id_bytes).map_err(|e| e.to_string())?;

        let schema_hash = cur.read_u64::<BigEndian>().map_err(|e| e.to_string())?;
        let timestamp_ms = cur.read_u64::<BigEndian>().map_err(|e| e.to_string())?;
        let data_len = cur.read_u32::<BigEndian>().map_err(|e| e.to_string())? as usize;
        if data_len > MAX_DATA_LEN {
            return Err(format!("Snapshot data too large: {data_len} bytes"));
        }
        if cur.len() < data_len {
            return Err("Truncated snapshot data".into());
        }
        let mut data = vec![0u8; data_len];
        cur.read_exact(&mut data).map_err(|e| e.to_string())?;

        Ok(Self {
            app_id,
            schema_hash,
            timestamp_ms,
            data,
        })
    }
}
