// services/mediad/src/main.rs — Media Indexer & Scanner Daemon (mediad)
// Scans device storage (/data/media, /data/dcim, /sdcard), classifies audio, video,
// and images by MIME type, maintains indexed catalog, and serves media queries to apps.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const MEDIA_SOCK_PATH: &str = "/run/nilos/media.sock";
pub const MEDIA_INDEX_PATH: &str = "/data/media/index.json";
pub const DEFAULT_MEDIA_SCAN_DIRS: &[&str] = &[
    "/data/media",
    "/data/dcim",
    "/sdcard/Music",
    "/sdcard/Pictures",
    "/sdcard/Movies",
    "/sdcard/Download",
];

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Audio,
    Video,
    Image,
    Other,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MediaItem {
    pub id: u64,
    pub path: String,
    pub file_name: String,
    pub media_type: MediaType,
    pub size_bytes: u64,
    pub mime_type: String,
    pub modified_ts: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MediaCatalogStats {
    pub total_items: usize,
    pub total_audio: usize,
    pub total_video: usize,
    pub total_image: usize,
    pub total_bytes: u64,
}

/// Detects MediaType and MIME string from file extension
pub fn classify_media_file(filename: &str) -> Option<(MediaType, &'static str)> {
    let lower = filename.to_lowercase();
    let ext = lower.rsplit('.').next()?;

    match ext {
        // Audio
        "mp3" => Some((MediaType::Audio, "audio/mpeg")),
        "wav" => Some((MediaType::Audio, "audio/wav")),
        "flac" => Some((MediaType::Audio, "audio/flac")),
        "aac" => Some((MediaType::Audio, "audio/aac")),
        "ogg" => Some((MediaType::Audio, "audio/ogg")),
        "m4a" => Some((MediaType::Audio, "audio/mp4")),

        // Video
        "mp4" => Some((MediaType::Video, "video/mp4")),
        "mkv" => Some((MediaType::Video, "video/x-matroska")),
        "webm" => Some((MediaType::Video, "video/webm")),
        "avi" => Some((MediaType::Video, "video/x-msvideo")),

        // Image
        "jpg" | "jpeg" => Some((MediaType::Image, "image/jpeg")),
        "png" => Some((MediaType::Image, "image/png")),
        "webp" => Some((MediaType::Image, "image/webp")),
        "gif" => Some((MediaType::Image, "image/gif")),

        _ => None,
    }
}

/// Recursively scans a directory for recognized media files
pub fn scan_directory_for_media(dir: &Path, out_items: &mut Vec<MediaItem>, next_id: &mut u64) {
    if !dir.exists() || !dir.is_dir() {
        return;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_directory_for_media(&path, out_items, next_id);
            } else if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if let Some((mtype, mime)) = classify_media_file(file_name) {
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        let modified = entry
                            .metadata()
                            .ok()
                            .and_then(|m| m.modified().ok())
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0);

                        out_items.push(MediaItem {
                            id: *next_id,
                            path: path.to_string_lossy().to_string(),
                            file_name: file_name.to_string(),
                            media_type: mtype,
                            size_bytes: size,
                            mime_type: mime.to_string(),
                            modified_ts: modified,
                        });
                        *next_id += 1;
                    }
                }
            }
        }
    }
}

pub struct MediaServiceState {
    pub items: Vec<MediaItem>,
    pub next_id: u64,
}

pub struct MediaService {
    pub state: Arc<Mutex<MediaServiceState>>,
    pub index_path: PathBuf,
}

impl MediaService {
    pub fn new(index_path: PathBuf) -> Self {
        let mut svc = Self {
            state: Arc::new(Mutex::new(MediaServiceState {
                items: Vec::new(),
                next_id: 1,
            })),
            index_path,
        };
        svc.load_index();
        svc
    }

    pub fn load_index(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.index_path) {
            if let Ok(items) = serde_json::from_str::<Vec<MediaItem>>(&content) {
                let mut st = self.state.lock().unwrap();
                let max_id = items.iter().map(|it| it.id).max().unwrap_or(0);
                st.next_id = max_id + 1;
                st.items = items;
            }
        }
    }

    pub fn save_index(&self) -> std::io::Result<()> {
        if let Some(parent) = self.index_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let st = self.state.lock().unwrap();
        let json = serde_json::to_string_pretty(&st.items)?;
        fs::write(&self.index_path, json)
    }

    pub fn rescan_paths(&self, paths: &[&str]) {
        let mut new_items = Vec::new();
        let mut next_id = {
            let st = self.state.lock().unwrap();
            st.next_id
        };

        for &p in paths {
            let path = Path::new(p);
            scan_directory_for_media(path, &mut new_items, &mut next_id);
        }

        let mut st = self.state.lock().unwrap();
        st.next_id = next_id;
        st.items = new_items;
        drop(st);

        let _ = self.save_index();
    }

    pub fn query(&self, search_text: Option<&str>, filter_type: Option<MediaType>) -> Vec<MediaItem> {
        let st = self.state.lock().unwrap();
        st.items
            .iter()
            .filter(|it| {
                if let Some(mtype) = filter_type {
                    if it.media_type != mtype {
                        return false;
                    }
                }
                if let Some(query) = search_text {
                    let q = query.to_lowercase();
                    if !it.file_name.to_lowercase().contains(&q) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    pub fn get_stats(&self) -> MediaCatalogStats {
        let st = self.state.lock().unwrap();
        let total_items = st.items.len();
        let total_audio = st.items.iter().filter(|i| i.media_type == MediaType::Audio).count();
        let total_video = st.items.iter().filter(|i| i.media_type == MediaType::Video).count();
        let total_image = st.items.iter().filter(|i| i.media_type == MediaType::Image).count();
        let total_bytes = st.items.iter().map(|i| i.size_bytes).sum();

        MediaCatalogStats {
            total_items,
            total_audio,
            total_video,
            total_image,
            total_bytes,
        }
    }
}

fn main() {
    println!("[mediad] Media Indexer & Scanner Daemon starting...");
    let index_path = PathBuf::from(MEDIA_INDEX_PATH);
    let service = MediaService::new(index_path);

    // Initial scan
    service.rescan_paths(DEFAULT_MEDIA_SCAN_DIRS);
    let stats = service.get_stats();
    println!(
        "[mediad] Indexed {} media items (Audio: {}, Video: {}, Images: {}, Total size: {} bytes)",
        stats.total_items, stats.total_audio, stats.total_video, stats.total_image, stats.total_bytes
    );

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_media_file_types() {
        assert_eq!(
            classify_media_file("song.mp3"),
            Some((MediaType::Audio, "audio/mpeg"))
        );
        assert_eq!(
            classify_media_file("movie.mp4"),
            Some((MediaType::Video, "video/mp4"))
        );
        assert_eq!(
            classify_media_file("photo.PNG"),
            Some((MediaType::Image, "image/png"))
        );
        assert_eq!(classify_media_file("archive.zip"), None);
    }

    #[test]
    fn test_scan_and_query_flow() {
        let tmp = std::env::temp_dir().join(format!("test_mediad_{}", std::process::id()));
        let _ = fs::create_dir_all(&tmp);

        // Create mock media files
        let music_dir = tmp.join("Music");
        let dcim_dir = tmp.join("DCIM");
        let _ = fs::create_dir_all(&music_dir);
        let _ = fs::create_dir_all(&dcim_dir);

        fs::write(music_dir.join("track01.mp3"), b"ID3 fake audio").unwrap();
        fs::write(music_dir.join("track02.flac"), b"fLaC fake audio").unwrap();
        fs::write(dcim_dir.join("IMG_001.jpg"), b"\xff\xd8\xff fake jpeg").unwrap();
        fs::write(dcim_dir.join("notes.txt"), b"Not media").unwrap();

        let index_file = tmp.join("index.json");
        let service = MediaService::new(index_file);

        let tmp_str = tmp.to_string_lossy().to_string();
        service.rescan_paths(&[&tmp_str]);

        let stats = service.get_stats();
        assert_eq!(stats.total_items, 3);
        assert_eq!(stats.total_audio, 2);
        assert_eq!(stats.total_image, 1);
        assert_eq!(stats.total_video, 0);

        // Query by search string
        let q_results = service.query(Some("track01"), None);
        assert_eq!(q_results.len(), 1);
        assert_eq!(q_results[0].file_name, "track01.mp3");

        // Query by type
        let img_results = service.query(None, Some(MediaType::Image));
        assert_eq!(img_results.len(), 1);
        assert_eq!(img_results[0].file_name, "IMG_001.jpg");

        let _ = fs::remove_dir_all(tmp);
    }
}
