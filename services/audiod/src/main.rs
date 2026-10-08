// services/audiod/src/main.rs — Audio Service & Focus Manager Daemon
// Communicates via nilprotocol framed binary IPC over /run/onuron/audio.sock

use std::sync::{Arc, Mutex};
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
use std::os::unix::net::UnixListener;

use nilprotocol::{
    AudioSetMutePayload, AudioSetVolumePayload, AudioStatusPayload, Frame, MessageType,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioState {
    pub volume: u8,
    pub is_muted: bool,
    pub active_sink: String,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            volume: 75,
            is_muted: false,
            active_sink: "Speaker (ALSA/PipeWire Default)".to_string(),
        }
    }
}

pub fn handle_ipc_request(state: &mut AudioState, frame: &Frame) -> Frame {
    let msg_type = MessageType::from(frame.message_type);
    match msg_type {
        MessageType::Ping => {
            Frame::new(MessageType::Pong, frame.request_id, b"pong".to_vec())
        }
        MessageType::AudioGetStatus => {
            let payload = AudioStatusPayload {
                volume: state.volume,
                is_muted: state.is_muted,
                sink_name: state.active_sink.clone(),
            };
            Frame::with_json(MessageType::AudioStatusInfo, frame.request_id, &payload)
                .unwrap_or_else(|_| Frame::new(MessageType::ErrorResponse, frame.request_id, b"encode error".to_vec()))
        }
        MessageType::AudioSetVolume => {
            if let Ok(req) = frame.parse_json::<AudioSetVolumePayload>() {
                state.volume = req.volume.min(100);
            }
            let payload = AudioStatusPayload {
                volume: state.volume,
                is_muted: state.is_muted,
                sink_name: state.active_sink.clone(),
            };
            Frame::with_json(MessageType::AudioStatusInfo, frame.request_id, &payload)
                .unwrap_or_else(|_| Frame::new(MessageType::ErrorResponse, frame.request_id, b"encode error".to_vec()))
        }
        MessageType::AudioSetMute => {
            if let Ok(req) = frame.parse_json::<AudioSetMutePayload>() {
                state.is_muted = req.is_muted;
            }
            let payload = AudioStatusPayload {
                volume: state.volume,
                is_muted: state.is_muted,
                sink_name: state.active_sink.clone(),
            };
            Frame::with_json(MessageType::AudioStatusInfo, frame.request_id, &payload)
                .unwrap_or_else(|_| Frame::new(MessageType::ErrorResponse, frame.request_id, b"encode error".to_vec()))
        }
        _ => Frame::new(MessageType::ErrorResponse, frame.request_id, b"unsupported msg".to_vec()),
    }
}

fn main() {
    println!("\x1b[1;36m[audiod]\x1b[0m Onuron Audio Daemon & Focus Manager starting...");

    let state = Arc::new(Mutex::new(AudioState::default()));

    #[cfg(unix)]
    {
        let socket_path = "/run/onuron/audio.sock";
        let _ = std::fs::create_dir_all("/run/onuron");
        let _ = std::fs::remove_file(socket_path);

        if let Ok(listener) = UnixListener::bind(socket_path) {
            println!("\x1b[1;32m[audiod] [  OK  ]\x1b[0m Listening on {}", socket_path);
            let policy = nilsd::auth::load_default_policy();

            let state_clone = Arc::clone(&state);
            thread::spawn(move || {
                for mut sock in listener.incoming().flatten() {
                    if !nilsd::auth::authorize_stream(&policy, "audiod", &sock) {
                        continue;
                    }
                    while let Ok(frame) = Frame::read_from(&mut sock) {
                        let mut st = state_clone.lock().unwrap();
                        let resp = handle_ipc_request(&mut st, &frame);
                        if resp.write_to(&mut sock).is_err() {
                            break;
                        }
                    }
                }
            });
        }
    }

    #[cfg(not(unix))]
    {
        println!("[audiod] Host simulation active: default volume {}%", state.lock().unwrap().volume);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_status_roundtrip() {
        let mut state = AudioState::default();
        let frame = Frame::new(MessageType::AudioGetStatus, 1, vec![]);
        let resp = handle_ipc_request(&mut state, &frame);

        assert_eq!(resp.message_type, u16::from(MessageType::AudioStatusInfo));
        let info = resp.parse_json::<AudioStatusPayload>().unwrap();
        assert_eq!(info.volume, 75);
        assert!(!info.is_muted);
    }

    #[test]
    fn test_audio_set_volume_and_mute() {
        let mut state = AudioState::default();

        let vol_req = Frame::with_json(
            MessageType::AudioSetVolume,
            2,
            &AudioSetVolumePayload { volume: 88 },
        )
        .unwrap();
        let resp = handle_ipc_request(&mut state, &vol_req);
        let info = resp.parse_json::<AudioStatusPayload>().unwrap();
        assert_eq!(info.volume, 88);
        assert_eq!(state.volume, 88);

        let mute_req = Frame::with_json(
            MessageType::AudioSetMute,
            3,
            &AudioSetMutePayload { is_muted: true },
        )
        .unwrap();
        let resp = handle_ipc_request(&mut state, &mute_req);
        let info = resp.parse_json::<AudioStatusPayload>().unwrap();
        assert!(info.is_muted);
        assert!(state.is_muted);
    }
}
