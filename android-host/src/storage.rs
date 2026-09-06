// android-host/src/storage.rs — Android SAF & Scoped Storage Bridge
// Maps Android app-private storage (/data/user/0/org.onuron.mobile/files) and shared storage
// into standard Onuron OS filesystem paths (/data, /home/user, /data/media).

use std::path::{Path, PathBuf};

pub struct AndroidHostStorage {
    internal_files_dir: PathBuf,
    #[allow(dead_code)]
    external_files_dir: Option<PathBuf>,
}

impl AndroidHostStorage {
    pub fn new() -> Self {
        let internal = std::env::var("ONURON_STORAGE_INTERNAL")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/data/data/org.onuron.mobile/files"));

        let external = std::env::var("ONURON_STORAGE_EXTERNAL")
            .ok()
            .map(PathBuf::from);

        Self {
            internal_files_dir: internal,
            external_files_dir: external,
        }
    }

    pub fn get_onuron_root(&self) -> &Path {
        &self.internal_files_dir
    }

    pub fn resolve_path(&self, onuron_virtual_path: &str) -> PathBuf {
        let stripped = onuron_virtual_path.trim_start_matches('/');
        self.internal_files_dir.join(stripped)
    }
}
