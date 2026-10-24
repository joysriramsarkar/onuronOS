// tools/nilstore/src/main.rs — Content-Addressed Delta Chunk Indexer & Packaging CLI
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

const CHUNK_SIZE: usize = 4096;

pub fn chunk_hash(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

pub fn index_directory(store_dir: &str) -> Result<usize, String> {
    let p = Path::new(store_dir);
    if !p.is_dir() {
        return Err(format!("Store path is not a directory: {store_dir}"));
    }
    let chunks_base = p.join("chunks");
    fs::create_dir_all(&chunks_base).map_err(|e| e.to_string())?;

    let mut entries = Vec::new();
    let read_dir = fs::read_dir(p).map_err(|e| e.to_string())?;

    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_file() {
            let ext = path.extension().and_then(|x| x.to_str()).unwrap_or("");
            if ext == "nilax" || ext == "nilpkg" || ext == "img" {
                let blob = fs::read(&path).map_err(|e| e.to_string())?;
                let mut chunk_list = Vec::new();
                for c in blob.chunks(CHUNK_SIZE) {
                    let hh = chunk_hash(c);
                    let sub_dir = chunks_base.join(&hh[..2]);
                    fs::create_dir_all(&sub_dir).ok();
                    let _ = fs::write(sub_dir.join(&hh), c);
                    chunk_list.push(hh);
                }
                let mut whole_hasher = Sha256::new();
                whole_hasher.update(&blob);
                let whole_hash: String = whole_hasher
                    .finalize()
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect();

                let stem = path.file_stem().unwrap().to_string_lossy().to_string();
                let chunks_json = chunk_list
                    .iter()
                    .map(|c| format!("\"{}\"", c))
                    .collect::<Vec<_>>()
                    .join(",");

                entries.push(format!(
                    "{{\"name\":\"{}\",\"size\":{},\"sha256\":\"{}\",\"chunks\":[{}]}}",
                    stem,
                    blob.len(),
                    whole_hash,
                    chunks_json
                ));
            }
        }
    }

    let index_file = p.join("index.json");
    let content = format!("[\n{}\n]\n", entries.join(",\n"));
    fs::write(&index_file, content).map_err(|e| e.to_string())?;
    println!(
        "[nilstore-cli] Indexed {} packages into {}",
        entries.len(),
        index_file.display()
    );
    Ok(entries.len())
}

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("index") => {
            if let Some(dir) = args.next() {
                if let Err(e) = index_directory(&dir) {
                    eprintln!("[nilstore-cli] Error: {e}");
                    std::process::exit(1);
                }
            } else {
                eprintln!("Usage: nilstore-cli index <store_directory>");
                std::process::exit(1);
            }
        }
        Some("-h") | Some("--help") | None => {
            println!("Usage:");
            println!("  nilstore-cli index <store_directory>   # Slice packages into 4KiB chunks & generate index.json");
        }
        Some(other) => {
            eprintln!("[nilstore-cli] Unknown command: {other}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_hash_deterministic() {
        let h1 = chunk_hash(b"hello world");
        let h2 = chunk_hash(b"hello world");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }
}
