// android-host/src/input.rs — Android MotionEvent & KeyEvent Translator
// Bridges Android MotionEvents and KeyEvents into Onuron OS unified input events.

use crate::bridge::HostToGuestEvent;
use nilhal::traits::{HalInputEvent, InputHal, HalError};

pub struct AndroidHostInput {
    event_queue: Vec<HalInputEvent>,
}

impl AndroidHostInput {
    pub fn new() -> Self {
        Self {
            event_queue: Vec::new(),
        }
    }

    /// Ingest a raw host event received from the Android Host bridge
    pub fn ingest_host_event(&mut self, host_event: HostToGuestEvent) {
        match host_event {
            HostToGuestEvent::TouchEvent { action, pointer_id, x, y, .. } => {
                match action.as_str() {
                    "DOWN" => {
                        self.event_queue.push(HalInputEvent::TouchDown { id: pointer_id, x, y });
                    }
                    "MOVE" => {
                        self.event_queue.push(HalInputEvent::TouchMove { id: pointer_id, x, y });
                    }
                    "UP" | "CANCEL" => {
                        self.event_queue.push(HalInputEvent::TouchUp { id: pointer_id });
                    }
                    _ => {}
                }
            }
            HostToGuestEvent::KeyEvent { action, keycode, .. } => {
                let name = match keycode {
                    4 => "Back".to_string(),
                    3 => "Home".to_string(),
                    24 => "VolumeUp".to_string(),
                    25 => "VolumeDown".to_string(),
                    26 => "Power".to_string(),
                    _ => format!("Key_{}", keycode),
                };

                if action == "DOWN" {
                    self.event_queue.push(HalInputEvent::KeyDown { code: keycode, name });
                } else {
                    self.event_queue.push(HalInputEvent::KeyUp { code: keycode, name });
                }
            }
            _ => {}
        }
    }
}

impl InputHal for AndroidHostInput {
    fn poll_events(&mut self) -> Vec<HalInputEvent> {
        std::mem::take(&mut self.event_queue)
    }

    fn send_event(&mut self, event: HalInputEvent) -> Result<(), HalError> {
        self.event_queue.push(event);
        Ok(())
    }
}
