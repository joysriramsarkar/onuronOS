// runtime/nilui/src/state.rs — NILS Handoff State Snapshot Serialization
use std::io::Read;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crc32fast::Hasher;

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
        if slice.len() < 28 {
            return Err("Payload too short".into());
        }
        let magic = slice.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
        if magic != NILS_MAGIC {
            return Err("Invalid NILS magic header".into());
        }
        let _version = slice.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
        let app_len = slice.read_u16::<BigEndian>().map_err(|e| e.to_string())? as usize;
        let mut app_id_bytes = vec![0u8; app_len];
        slice.read_exact(&mut app_id_bytes).map_err(|e| e.to_string())?;
        let app_id = String::from_utf8(app_id_bytes).map_err(|e| e.to_string())?;
        
        let schema_hash = slice.read_u64::<BigEndian>().map_err(|e| e.to_string())?;
        let timestamp_ms = slice.read_u64::<BigEndian>().map_err(|e| e.to_string())?;
        let data_len = slice.read_u32::<BigEndian>().map_err(|e| e.to_string())? as usize;
        
        let mut data = vec![0u8; data_len];
        slice.read_exact(&mut data).map_err(|e| e.to_string())?;
        
        Ok(Self {
            app_id,
            schema_hash,
            timestamp_ms,
            data,
        })
    }
}
