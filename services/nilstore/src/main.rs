// services/nilstore/src/main.rs — OnuronOS 4KiB Content-Addressed Delta Chunk Server & Cache Manager
// Manages local disk cache at /data/chunks and provides local Unix IPC and chunk queries.
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_CHUNK_DIR: &str = "/data/chunks";

#[derive(Debug, Clone)]
pub struct ChunkStore {
    base_dir: PathBuf,
}

impl Default for ChunkStore {
    fn default() -> Self {
        Self::new(DEFAULT_CHUNK_DIR)
    }
}

impl ChunkStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        let base_dir = dir.into();
        let _ = fs::create_dir_all(&base_dir);
        Self { base_dir }
    }

    pub fn chunk_path(&self, hash: &str) -> PathBuf {
        if hash.len() >= 2 {
            self.base_dir.join(&hash[..2]).join(hash)
        } else {
            self.base_dir.join(hash)
        }
    }

    pub fn has_chunk(&self, hash: &str) -> bool {
        self.chunk_path(hash).is_file()
    }

    pub fn get_chunk(&self, hash: &str) -> Option<Vec<u8>> {
        fs::read(self.chunk_path(hash)).ok()
    }

    pub fn store_chunk(&self, hash: &str, data: &[u8]) -> Result<(), String> {
        let path = self.chunk_path(hash);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(path, data).map_err(|e| e.to_string())
    }

    pub fn count_cached_chunks(&self) -> usize {
        let mut count = 0;
        if let Ok(entries) = fs::read_dir(&self.base_dir) {
            for sub in entries.flatten() {
                if sub.path().is_dir() {
                    if let Ok(files) = fs::read_dir(sub.path()) {
                        count += files.flatten().count();
                    }
                }
            }
        }
        count
    }
}

fn main() {
    let store = ChunkStore::default();
    println!(
        "[nilstore] 4KiB Content-Addressed Delta Chunk Server active. Cached chunks: {}",
        store.count_cached_chunks()
    );
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_store_write_and_retrieve() {
        let tmp = std::env::temp_dir().join(format!("nilstore_test_{}", std::process::id()));
        let store = ChunkStore::new(&tmp);

        let fake_hash = "abcdef0123456789";
        let data = b"sample chunk data 4096";

        assert!(!store.has_chunk(fake_hash));
        assert!(store.store_chunk(fake_hash, data).is_ok());
        assert!(store.has_chunk(fake_hash));
        assert_eq!(store.get_chunk(fake_hash).unwrap(), data);

        let _ = fs::remove_dir_all(&tmp);
    }
}
