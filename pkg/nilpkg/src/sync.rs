// pkg/nilpkg/src/sync.rs — Chunk-based delta sync client (casync style)
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};

pub struct ChunkSyncClient {
    pub store_url: String,
    pub local_cache_dir: String,
}

impl ChunkSyncClient {
    pub fn new(store_url: &str, cache_dir: &str) -> Self {
        fs::create_dir_all(cache_dir).ok();
        Self {
            store_url: store_url.to_string(),
            local_cache_dir: cache_dir.to_string(),
        }
    }

    pub fn download_delta(&self, required_chunks: &[String]) -> Result<Vec<u8>, String> {
        println!("[nilpkg:sync] Syncing {} content-addressed chunks...", required_chunks.len());
        let mut full_payload = Vec::new();
        for hash in required_chunks {
            let chunk_path = format!("{}/{}", self.local_cache_dir, hash);
            if let Ok(mut f) = File::open(&chunk_path) {
                let mut buf = Vec::new();
                f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                full_payload.extend_from_slice(&buf);
            } else {
                println!("[nilpkg:sync] Fetching missing chunk {} from {}", hash, self.store_url);
                // In production: HTTP / Softbus get chunk
                full_payload.extend_from_slice(b"chunk_data");
            }
        }
        Ok(full_payload)
    }
}
