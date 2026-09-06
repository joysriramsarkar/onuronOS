// android-host/src/lib.rs — Onuron Mobile Runtime Android Host Subsystem
// Official Android host integration for Samsung Galaxy S25 (Snapdragon 8 Elite) and Android devices.

pub mod bridge;
pub mod display;
pub mod input;
pub mod network;
pub mod telephony;
pub mod camera;
pub mod audio;
pub mod sensors;
pub mod storage;

pub use bridge::*;
pub use display::AndroidHostDisplay;
pub use input::AndroidHostInput;
pub use network::AndroidHostNetwork;
pub use telephony::AndroidHostTelephony;
pub use camera::AndroidHostCamera;
pub use audio::AndroidHostAudio;
pub use sensors::AndroidHostSensors;
pub use storage::AndroidHostStorage;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_input_translation() {
        let mut input = AndroidHostInput::new();
        input.ingest_host_event(HostToGuestEvent::TouchEvent {
            action: "DOWN".into(),
            pointer_id: 0,
            x: 540.0,
            y: 1170.0,
            pressure: 1.0,
        });

        use nilhal::traits::InputHal;
        let events = input.poll_events();
        assert_eq!(events.len(), 1);
        if let nilhal::traits::HalInputEvent::TouchDown { id, x, y } = events[0] {
            assert_eq!(id, 0);
            assert_eq!(x, 540.0);
            assert_eq!(y, 1170.0);
        } else {
            panic!("Expected TouchDown event");
        }
    }

    #[test]
    fn test_host_display_s25() {
        use nilhal::traits::DisplayHal;
        let display = AndroidHostDisplay::new();
        assert_eq!(display.get_dimensions(), (1080, 2340));
        assert_eq!(display.get_refresh_rate(), 120);
    }
}
