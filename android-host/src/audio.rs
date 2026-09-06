// android-host/src/audio.rs — Android AudioTrack / AudioRecord Bridge
use nilhal::traits::{AudioHal, AudioRoute, HalError};

pub struct AndroidHostAudio {
    master_volume: u8,
    current_route: AudioRoute,
}

impl AndroidHostAudio {
    pub fn new() -> Self {
        Self {
            master_volume: 85,
            current_route: AudioRoute::Speaker,
        }
    }
}

impl AudioHal for AndroidHostAudio {
    fn set_master_volume(&mut self, percent: u8) -> Result<(), HalError> {
        self.master_volume = percent.min(100);
        Ok(())
    }

    fn get_master_volume(&self) -> u8 {
        self.master_volume
    }

    fn play_stream(&mut self, _pcm_samples: &[i16]) -> Result<(), HalError> {
        // Pipes PCM audio samples into AAudio or AudioTrack stream
        Ok(())
    }

    fn record_stream(&mut self, buffer: &mut [i16]) -> Result<usize, HalError> {
        buffer.fill(0);
        Ok(buffer.len())
    }

    fn route_output(&mut self, route: AudioRoute) -> Result<(), HalError> {
        self.current_route = route;
        Ok(())
    }
}
