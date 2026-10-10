// services/audiod/src/main.rs — Onuron OS Audio Policy & Mixing Server (audiod)
// Manages stream priorities, dynamic ducking, ALSA PCM routing, volume curves,
// and canonical framed IPC over `/run/onuron/audio.sock` and `/run/nilos/audio.sock`.

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
#[cfg(unix)]
use std::thread;

use nilprotocol::{
    AudioSetMutePayload, AudioSetRoutePayload, AudioSetVolumePayload, AudioStatusPayload, Frame,
    MessageType,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AudioStreamType {
    Media = 10,
    Notification = 30,
    Alarm = 50,
    VoiceCall = 80,
    EmergencyCall = 100,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioRoute {
    Speaker,
    Earpiece,
    Headset,
    Bluetooth,
}

impl AudioRoute {
    pub fn as_str(&self) -> &'static str {
        match self {
            AudioRoute::Speaker => "Speaker (ALSA Default)",
            AudioRoute::Earpiece => "Earpiece (Mobile Receiver)",
            AudioRoute::Headset => "Headset (3.5mm / Type-C)",
            AudioRoute::Bluetooth => "Bluetooth (A2DP / HFP)",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "earpiece" => AudioRoute::Earpiece,
            "headset" => AudioRoute::Headset,
            "bluetooth" => AudioRoute::Bluetooth,
            _ => AudioRoute::Speaker,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlsaPcmDevice {
    pub card: u32,
    pub device: u32,
    pub name: String,
    pub is_playback: bool,
    pub is_capture: bool,
}

pub fn parse_asound_pcm(content: &str) -> Vec<AlsaPcmDevice> {
    let mut devices = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(':').collect();
        if parts.is_empty() {
            continue;
        }
        let id_part = parts[0].trim();
        let card_dev: Vec<&str> = id_part.split('-').collect();
        if card_dev.len() != 2 {
            continue;
        }
        let card = card_dev[0].parse::<u32>().unwrap_or(0);
        let device = card_dev[1].parse::<u32>().unwrap_or(0);
        let name = if parts.len() > 1 {
            parts[1].trim().to_string()
        } else {
            "Unknown PCM".to_string()
        };

        let is_playback = line.contains("playback");
        let is_capture = line.contains("capture");

        devices.push(AlsaPcmDevice {
            card,
            device,
            name,
            is_playback,
            is_capture,
        });
    }
    devices
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioServer {
    pub volume: u8,
    pub is_muted: bool,
    pub active_route: AudioRoute,
    pub active_stream: Option<AudioStreamType>,
    pub ducked: bool,
    pub hardware_detected: bool,
    pub alsa_devices: Vec<AlsaPcmDevice>,
}

impl Default for AudioServer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioServer {
    pub fn new() -> Self {
        let (found, default_sink, alsa_devices) = Self::detect_audio_hardware();
        Self {
            volume: 75,
            is_muted: false,
            active_route: default_sink,
            active_stream: None,
            ducked: false,
            hardware_detected: found,
            alsa_devices,
        }
    }

    /// Detect Linux ALSA sound cards via `/proc/asound/cards`, `/proc/asound/pcm`, and `/dev/snd/`.
    pub fn detect_audio_hardware() -> (bool, AudioRoute, Vec<AlsaPcmDevice>) {
        let mut devices = Vec::new();
        let asound_pcm = Path::new("/proc/asound/pcm");
        if asound_pcm.is_file() {
            if let Ok(content) = fs::read_to_string(asound_pcm) {
                devices = parse_asound_pcm(&content);
            }
        }

        let asound_cards = Path::new("/proc/asound/cards");
        let has_cards = if asound_cards.is_file() {
            fs::read_to_string(asound_cards)
                .map(|c| !c.trim().is_empty())
                .unwrap_or(false)
        } else {
            false
        };

        let found = has_cards || !devices.is_empty();
        (found, AudioRoute::Speaker, devices)
    }

    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume.min(100);
    }

    pub fn set_mute(&mut self, is_muted: bool) {
        self.is_muted = is_muted;
    }

    pub fn set_route(&mut self, route: AudioRoute) {
        self.active_route = route;
    }

    /// Calculate effective output volume taking mute and focus ducking into account.
    pub fn effective_volume(&self) -> u8 {
        if self.is_muted {
            0
        } else if self.ducked {
            (self.volume as u32 * 25 / 100) as u8
        } else {
            self.volume
        }
    }

    /// Policy arbitrator: acquire focus for a new audio stream.
    /// If incoming stream is higher priority, it can duck or preempt existing streams.
    pub fn request_stream_focus(&mut self, stream: AudioStreamType) {
        match self.active_stream {
            Some(current) if current < stream => {
                // Higher priority stream preempts lower priority
                if stream >= AudioStreamType::Notification && current == AudioStreamType::Media {
                    self.ducked = true;
                }
                self.active_stream = Some(stream);
            }
            None => {
                self.active_stream = Some(stream);
                self.ducked = false;
            }
            _ => {}
        }
    }

    /// Release focus when stream playback completes.
    pub fn release_stream_focus(&mut self, stream: AudioStreamType) {
        if self.active_stream == Some(stream) {
            self.active_stream = None;
            self.ducked = false;
        }
    }

    pub fn get_status_payload(&self) -> AudioStatusPayload {
        let sink_name = if self.hardware_detected {
            self.active_route.as_str().to_string()
        } else {
            format!("{} [SIMULATED]", self.active_route.as_str())
        };
        AudioStatusPayload {
            volume: self.effective_volume(),
            is_muted: self.is_muted,
            sink_name,
        }
    }
}

pub fn handle_ipc_request(server: &mut AudioServer, frame: &Frame) -> Frame {
    let msg_type = MessageType::from(frame.message_type);
    match msg_type {
        MessageType::Ping => {
            Frame::new(MessageType::Pong, frame.request_id, b"pong".to_vec())
        }
        MessageType::ServiceStatusRequest => {
            let payload = nilprotocol::ServiceStatusPayload {
                service_name: "audiod".to_string(),
                is_ready: true,
                is_simulated: !server.hardware_detected,
                backend_name: if server.hardware_detected { "alsa".to_string() } else { "simulated".to_string() },
                uptime_secs: 0,
                request_count: 1,
                last_error: None,
            };
            Frame::with_json(MessageType::ServiceStatusResponse, frame.request_id, &payload)
                .unwrap_or_else(|_| Frame::new(MessageType::ErrorResponse, frame.request_id, b"encode error".to_vec()))
        }
        MessageType::AudioGetStatus => {
            let payload = server.get_status_payload();
            Frame::with_json(MessageType::AudioStatusInfo, frame.request_id, &payload)
                .unwrap_or_else(|_| Frame::new(MessageType::ErrorResponse, frame.request_id, b"encode error".to_vec()))
        }
        MessageType::AudioSetVolume => {
            if let Ok(req) = frame.parse_json::<AudioSetVolumePayload>() {
                server.set_volume(req.volume);
            }
            let payload = server.get_status_payload();
            Frame::with_json(MessageType::AudioStatusInfo, frame.request_id, &payload)
                .unwrap_or_else(|_| Frame::new(MessageType::ErrorResponse, frame.request_id, b"encode error".to_vec()))
        }
        MessageType::AudioSetMute => {
            if let Ok(req) = frame.parse_json::<AudioSetMutePayload>() {
                server.set_mute(req.is_muted);
            }
            let payload = server.get_status_payload();
            Frame::with_json(MessageType::AudioStatusInfo, frame.request_id, &payload)
                .unwrap_or_else(|_| Frame::new(MessageType::ErrorResponse, frame.request_id, b"encode error".to_vec()))
        }
        MessageType::AudioSetRoute => {
            if let Ok(req) = frame.parse_json::<AudioSetRoutePayload>() {
                server.set_route(AudioRoute::from_str(&req.route));
            }
            let payload = server.get_status_payload();
            Frame::with_json(MessageType::AudioStatusInfo, frame.request_id, &payload)
                .unwrap_or_else(|_| Frame::new(MessageType::ErrorResponse, frame.request_id, b"encode error".to_vec()))
        }
        _ => Frame::new(MessageType::ErrorResponse, frame.request_id, b"unsupported msg".to_vec()),
    }
}

fn main() {
    println!("\x1b[1;36m[audiod]\x1b[0m Onuron Audio Policy Server & Mixer starting...");

    let server = Arc::new(Mutex::new(AudioServer::new()));

    #[cfg(unix)]
    {
        use std::os::unix::net::UnixListener;

        let socket_paths = ["/run/onuron/audio.sock", "/run/nilos/audio.sock"];
        for path_str in socket_paths {
            let path = Path::new(path_str);
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::remove_file(path);

            if let Ok(listener) = UnixListener::bind(path) {
                println!("\x1b[1;32m[audiod] [  OK  ]\x1b[0m Listening on {}", path.display());
                let server_clone = Arc::clone(&server);
                thread::spawn(move || {
                    for mut sock in listener.incoming().flatten() {
                        let s_clone = Arc::clone(&server_clone);
                        thread::spawn(move || {
                            while let Ok(frame) = nilprotocol::read_frame(&mut sock) {
                                let resp = {
                                    let mut s = s_clone.lock().unwrap();
                                    handle_ipc_request(&mut s, &frame)
                                };
                                if nilprotocol::write_frame(&mut sock, &resp).is_err() {
                                    break;
                                }
                            }
                        });
                    }
                });
            }
        }
    }

    #[cfg(not(unix))]
    {
        println!("[audiod] Host simulation active: default volume {}%", server.lock().unwrap().volume);
    }

    let _ = nilsd::notify_ready("audiod", Some("/run/onuron/audio.sock"));
    println!("[audiod] Service ready.");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_status_roundtrip() {
        let mut server = AudioServer::new();
        let frame = Frame::new(MessageType::AudioGetStatus, 1, vec![]);
        let resp = handle_ipc_request(&mut server, &frame);

        assert_eq!(resp.message_type, u16::from(MessageType::AudioStatusInfo));
        let info = resp.parse_json::<AudioStatusPayload>().unwrap();
        assert_eq!(info.volume, 75);
        assert!(!info.is_muted);
    }

    #[test]
    fn test_audio_set_volume_and_mute() {
        let mut server = AudioServer::new();

        let vol_req = Frame::with_json(
            MessageType::AudioSetVolume,
            2,
            &AudioSetVolumePayload { volume: 88 },
        )
        .unwrap();
        let resp = handle_ipc_request(&mut server, &vol_req);
        let info = resp.parse_json::<AudioStatusPayload>().unwrap();
        assert_eq!(info.volume, 88);
        assert_eq!(server.volume, 88);

        let mute_req = Frame::with_json(
            MessageType::AudioSetMute,
            3,
            &AudioSetMutePayload { is_muted: true },
        )
        .unwrap();
        let resp = handle_ipc_request(&mut server, &mute_req);
        let info = resp.parse_json::<AudioStatusPayload>().unwrap();
        assert!(info.is_muted);
        assert!(server.is_muted);
        assert_eq!(server.effective_volume(), 0);
    }

    #[test]
    fn test_audio_route_switching() {
        let mut server = AudioServer::new();
        assert_eq!(server.active_route, AudioRoute::Speaker);

        let route_req = Frame::with_json(
            MessageType::AudioSetRoute,
            4,
            &AudioSetRoutePayload { route: "Bluetooth".to_string() },
        )
        .unwrap();
        let resp = handle_ipc_request(&mut server, &route_req);
        let info = resp.parse_json::<AudioStatusPayload>().unwrap();
        assert_eq!(server.active_route, AudioRoute::Bluetooth);
        assert!(info.sink_name.contains("Bluetooth"));
    }

    #[test]
    fn test_audio_focus_and_ducking() {
        let mut server = AudioServer::new();
        server.set_volume(80);

        // Start media playback
        server.request_stream_focus(AudioStreamType::Media);
        assert_eq!(server.effective_volume(), 80);
        assert!(!server.ducked);

        // Incoming notification ducks media
        server.request_stream_focus(AudioStreamType::Notification);
        assert!(server.ducked);
        assert_eq!(server.effective_volume(), 20); // 80 * 25% = 20

        // Release notification restores full volume
        server.release_stream_focus(AudioStreamType::Notification);
        assert_eq!(server.effective_volume(), 80);
    }

    #[test]
    fn test_parse_asound_pcm() {
        let sample_proc = r#"
00-00: ALC892 Analog : ALC892 Analog : playback 1 : capture 1
00-01: ALC892 Digital : ALC892 Digital : playback 1
00-03: HDMI 0 : HDMI 0 : playback 1
"#;
        let devices = parse_asound_pcm(sample_proc);
        assert_eq!(devices.len(), 3);
        assert_eq!(devices[0].card, 0);
        assert_eq!(devices[0].device, 0);
        assert_eq!(devices[0].name, "ALC892 Analog");
        assert!(devices[0].is_playback);
        assert!(devices[0].is_capture);

        assert_eq!(devices[1].device, 1);
        assert!(devices[1].is_playback);
        assert!(!devices[1].is_capture);

        // Test mobile SDM845 WCD9340 ALSA PCM format
        let mobile_proc = r#"
00-00: MultiMedia1 (*) : : playback 1 : capture 1
00-01: MultiMedia2 (*) : : playback 1 : capture 1
00-07: VoiceMMode1 (*) : : playback 1 : capture 1
"#;
        let mobile_devs = parse_asound_pcm(mobile_proc);
        assert_eq!(mobile_devs.len(), 3);
        assert_eq!(mobile_devs[2].device, 7);
        assert_eq!(mobile_devs[2].name, "VoiceMMode1 (*)");
    }

    #[test]
    fn test_audiod_service_status_request() {
        let mut server = AudioServer::new();
        let frame = Frame::new(MessageType::ServiceStatusRequest, 99, vec![]);
        let resp = handle_ipc_request(&mut server, &frame);
        assert_eq!(resp.message_type, u16::from(MessageType::ServiceStatusResponse));
        let status = resp.parse_json::<nilprotocol::ServiceStatusPayload>().unwrap();
        assert_eq!(status.service_name, "audiod");
        assert!(status.is_ready);
    }
}

