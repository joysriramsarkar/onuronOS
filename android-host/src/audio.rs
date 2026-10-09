// android-host/src/audio.rs — Android AudioTrack / AudioRecord Bridge
// Bridges PCM audio streams between Onuron userspace and Android AAudio/AudioTrack.

use crate::bridge::GuestToHostCommand;
use crate::jni_bridge;
use nilhal::traits::{AudioHal, AudioRoute, HalError};

pub struct AndroidHostAudio {
    master_volume: u8,
    is_muted: bool,
    current_route: AudioRoute,
    played_sample_count: u64,
    last_played: Vec<i16>,
    pending_record: Vec<i16>,
}

impl AndroidHostAudio {
    pub fn new() -> Self {
        Self {
            master_volume: 85,
            is_muted: false,
            current_route: AudioRoute::Speaker,
            played_sample_count: 0,
            last_played: Vec::new(),
            pending_record: Vec::new(),
        }
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.is_muted = muted;
    }

    pub fn is_muted(&self) -> bool {
        self.is_muted
    }

    pub fn get_played_sample_count(&self) -> u64 {
        self.played_sample_count
    }

    pub fn get_current_route(&self) -> AudioRoute {
        self.current_route.clone()
    }

    pub fn get_last_played_samples(&self) -> &[i16] {
        &self.last_played
    }

    pub fn inject_record_samples(&mut self, samples: &[i16]) {
        self.pending_record.extend_from_slice(samples);
    }
}

impl Default for AndroidHostAudio {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioHal for AndroidHostAudio {
    fn set_master_volume(&mut self, percent: u8) -> Result<(), HalError> {
        self.master_volume = percent.min(100);

        // Notify Android host to adjust AudioManager stream volume
        jni_bridge::enqueue_guest_command(GuestToHostCommand::SetVolume {
            stream_type: "STREAM_MUSIC".into(),
            percent: self.master_volume,
        });

        Ok(())
    }

    fn get_master_volume(&self) -> u8 {
        self.master_volume
    }

    fn play_stream(&mut self, pcm_samples: &[i16]) -> Result<(), HalError> {
        if pcm_samples.is_empty() {
            return Ok(());
        }

        let volume_scale = if self.is_muted {
            0.0
        } else {
            self.master_volume as f32 / 100.0
        };

        // Scale raw PCM samples with current master volume attenuation
        let scaled: Vec<i16> = pcm_samples
            .iter()
            .map(|&sample| ((sample as f32) * volume_scale).clamp(i16::MIN as f32, i16::MAX as f32) as i16)
            .collect();

        self.last_played = scaled.clone();

        // Enqueue into JNI audio playback ring buffer for Android AudioTrack consumption
        jni_bridge::push_audio_playback(&scaled);
        self.played_sample_count = self.played_sample_count.saturating_add(scaled.len() as u64);

        Ok(())
    }

    fn record_stream(&mut self, buffer: &mut [i16]) -> Result<usize, HalError> {
        if buffer.is_empty() {
            return Ok(0);
        }

        if !self.pending_record.is_empty() {
            let count = self.pending_record.len().min(buffer.len());
            buffer[..count].copy_from_slice(&self.pending_record[..count]);
            self.pending_record.drain(..count);
            return Ok(count);
        }

        // Pull captured PCM microphone samples fed by Android AudioRecord over JNI
        let samples_read = jni_bridge::pull_audio_record(buffer);
        Ok(samples_read)
    }

    fn route_output(&mut self, route: AudioRoute) -> Result<(), HalError> {
        self.current_route = route;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_playback_and_volume_scaling() {
        let mut audio = AndroidHostAudio::new();
        assert_eq!(audio.get_master_volume(), 85);
        assert_eq!(audio.get_current_route(), AudioRoute::Speaker);

        // Set volume to 50%
        assert!(audio.set_master_volume(50).is_ok());
        assert_eq!(audio.get_master_volume(), 50);

        let input_samples = vec![1000i16, -2000i16, 4000i16];
        assert!(audio.play_stream(&input_samples).is_ok());
        assert_eq!(audio.get_played_sample_count(), 3);
        assert_eq!(audio.get_last_played_samples(), &[500, -1000, 2000]);
    }

    #[test]
    fn test_audio_mute_and_record() {
        let mut audio = AndroidHostAudio::new();
        audio.set_muted(true);

        let samples = vec![1000i16, 2000i16];
        assert!(audio.play_stream(&samples).is_ok());
        assert_eq!(audio.get_last_played_samples(), &[0, 0]); // Muted to zero

        // Inject simulated microphone samples
        audio.inject_record_samples(&[123i16, 456, 789]);
        let mut mic_buf = [0i16; 3];
        let n = audio.record_stream(&mut mic_buf).expect("Record should succeed");
        assert_eq!(n, 3);
        assert_eq!(mic_buf, [123, 456, 789]);
    }

    #[test]
    fn test_route_switching() {
        let mut audio = AndroidHostAudio::new();
        assert!(audio.route_output(AudioRoute::Bluetooth).is_ok());
        assert_eq!(audio.get_current_route(), AudioRoute::Bluetooth);
    }
}
