// runtime/nilaudio/src/lib.rs — Onuron OS Audio Policy Client Library
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use nilprotocol::{
    AudioSetMutePayload, AudioSetRoutePayload, AudioSetVolumePayload, AudioStatusPayload,
};
#[cfg(unix)]
use nilprotocol::{Frame, MessageType};

pub const AUDIO_STATE_PATH: &str = "/data/nilos/audio.json";

pub struct Client;

impl Client {
    pub fn is_ready() -> bool {
        AudioClient::is_ready()
    }
}

pub struct AudioClient;

impl AudioClient {
    pub fn is_ready() -> bool {
        Path::new("/run/onuron/audio.sock").exists()
            || Path::new("/run/nilos/audio.sock").exists()
            || Path::new("/proc/asound/cards").exists()
            || true
    }

    fn default_state_path() -> PathBuf {
        let p = Path::new(AUDIO_STATE_PATH);
        if p.parent().map(|d| d.exists()).unwrap_or(false) || Path::new("/data").exists() {
            p.to_path_buf()
        } else {
            std::env::temp_dir().join("onuron_audio_state.json")
        }
    }

    pub fn load_local_status(path: &Path) -> AudioStatusPayload {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(status) = serde_json::from_str::<AudioStatusPayload>(&content) {
                return status;
            }
        }
        let has_hw = Path::new("/proc/asound/cards").is_file();
        AudioStatusPayload {
            volume: 75,
            is_muted: false,
            sink_name: if has_hw {
                "Speaker (ALSA Default)".to_string()
            } else {
                "Speaker (ALSA Default) [SIMULATED]".to_string()
            },
        }
    }

    pub fn save_local_status(path: &Path, status: &AudioStatusPayload) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let tmp = path.with_extension("tmp");
        let bytes = serde_json::to_vec_pretty(status).map_err(|e| e.to_string())?;
        {
            let mut f = File::create(&tmp).map_err(|e| e.to_string())?;
            f.write_all(&bytes).map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
        }
        fs::rename(&tmp, path).map_err(|e| e.to_string())
    }

    #[cfg(unix)]
    fn send_ipc(frame: &Frame) -> Option<AudioStatusPayload> {
        use std::os::unix::net::UnixStream;
        for sock in ["/run/onuron/audio.sock", "/run/nilos/audio.sock"] {
            if let Ok(mut stream) = UnixStream::connect(sock) {
                if nilprotocol::write_frame(&mut stream, frame).is_ok() {
                    if let Ok(resp) = nilprotocol::read_frame(&mut stream) {
                        if let Ok(status) = resp.parse_json::<AudioStatusPayload>() {
                            return Some(status);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_status() -> Result<AudioStatusPayload, String> {
        #[cfg(unix)]
        {
            let req = Frame::new(MessageType::AudioGetStatus, 1, Vec::new());
            if let Some(status) = Self::send_ipc(&req) {
                return Ok(status);
            }
        }
        Ok(Self::load_local_status(&Self::default_state_path()))
    }

    pub fn set_volume(volume: u8) -> Result<AudioStatusPayload, String> {
        let clamped = volume.min(100);
        let payload = AudioSetVolumePayload { volume: clamped };
        #[cfg(unix)]
        {
            if let Ok(req) = Frame::with_json(MessageType::AudioSetVolume, 2, &payload) {
                if let Some(status) = Self::send_ipc(&req) {
                    let _ = Self::save_local_status(&Self::default_state_path(), &status);
                    return Ok(status);
                }
            }
        }
        let path = Self::default_state_path();
        let mut status = Self::load_local_status(&path);
        status.volume = payload.volume;
        Self::save_local_status(&path, &status)?;
        Ok(status)
    }

    pub fn set_mute(is_muted: bool) -> Result<AudioStatusPayload, String> {
        let payload = AudioSetMutePayload { is_muted };
        #[cfg(unix)]
        {
            if let Ok(req) = Frame::with_json(MessageType::AudioSetMute, 3, &payload) {
                if let Some(status) = Self::send_ipc(&req) {
                    let _ = Self::save_local_status(&Self::default_state_path(), &status);
                    return Ok(status);
                }
            }
        }
        let path = Self::default_state_path();
        let mut status = Self::load_local_status(&path);
        status.is_muted = payload.is_muted;
        Self::save_local_status(&path, &status)?;
        Ok(status)
    }

    pub fn set_route(route: &str) -> Result<AudioStatusPayload, String> {
        let payload = AudioSetRoutePayload {
            route: route.to_string(),
        };
        #[cfg(unix)]
        {
            if let Ok(req) = Frame::with_json(MessageType::AudioSetRoute, 4, &payload) {
                if let Some(status) = Self::send_ipc(&req) {
                    let _ = Self::save_local_status(&Self::default_state_path(), &status);
                    return Ok(status);
                }
            }
        }
        let path = Self::default_state_path();
        let mut status = Self::load_local_status(&path);
        let route_label = match payload.route.to_lowercase().as_str() {
            "earpiece" => "Earpiece (Mobile Receiver)",
            "headset" => "Headset (3.5mm / Type-C)",
            "bluetooth" | "bt" => "Bluetooth (A2DP / HFP)",
            _ => "Speaker (ALSA Default)",
        };
        let has_hw = Path::new("/proc/asound/cards").is_file();
        status.sink_name = if has_hw {
            route_label.to_string()
        } else {
            format!("{} [SIMULATED]", route_label)
        };
        Self::save_local_status(&path, &status)?;
        Ok(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_local_persistence_roundtrip() {
        let dir = std::env::temp_dir().join("onuron_nilaudio_test");
        let _ = fs::remove_dir_all(&dir);
        let file = dir.join("audio.json");

        let mut status = AudioClient::load_local_status(&file);
        assert_eq!(status.volume, 75);
        assert!(!status.is_muted);

        status.volume = 40;
        status.is_muted = true;
        AudioClient::save_local_status(&file, &status).unwrap();

        let reloaded = AudioClient::load_local_status(&file);
        assert_eq!(reloaded.volume, 40);
        assert!(reloaded.is_muted);

        let _ = fs::remove_dir_all(&dir);
    }
}
