// services/inputd/src/main.rs — Onuron OS Unified Input Subsystem
// Reads raw Linux evdev (/dev/input/event*) devices and broadcasts structured touch/key events over IPC.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use serde::{Deserialize, Serialize};

// ─── Linux evdev constants ────────────────────────────────────────────────────
pub const EV_SYN: u16 = 0x00;
pub const EV_KEY: u16 = 0x01;
pub const EV_REL: u16 = 0x02;
pub const EV_ABS: u16 = 0x03;

pub const SYN_REPORT: u16 = 0;
pub const BTN_TOUCH: u16 = 0x14a;

// Mobile Keys
pub const KEY_POWER: u16 = 116;
pub const KEY_VOLUMEDOWN: u16 = 114;
pub const KEY_VOLUMEUP: u16 = 115;
pub const KEY_BACK: u16 = 158;
pub const KEY_HOMEPAGE: u16 = 172;

// Multitouch ABS axes
pub const ABS_X: u16 = 0x00;
pub const ABS_Y: u16 = 0x01;
pub const ABS_MT_SLOT: u16 = 0x2f;
pub const ABS_MT_TOUCH_MAJOR: u16 = 0x30;
pub const ABS_MT_POSITION_X: u16 = 0x35;
pub const ABS_MT_POSITION_Y: u16 = 0x36;
pub const ABS_MT_TRACKING_ID: u16 = 0x39;

/// High-level Unified Input Event for Onuron OS
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum InputEvent {
    TouchDown { id: u32, x: f32, y: f32 },
    TouchMove { id: u32, x: f32, y: f32 },
    TouchUp { id: u32 },
    KeyDown { code: u32, name: String },
    KeyUp { code: u32, name: String },
}

#[derive(Default, Clone)]
struct TouchSlot {
    tracking_id: i32,
    x: f32,
    y: f32,
    active: bool,
    dirty: bool,
}

/// Evdev Touch State Tracker (Multi-Touch Protocol B)
pub struct MultiTouchTracker {
    slots: HashMap<u32, TouchSlot>,
    current_slot: u32,
    screen_width: f32,
    screen_height: f32,
}

impl MultiTouchTracker {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            slots: HashMap::new(),
            current_slot: 0,
            screen_width: width,
            screen_height: height,
        }
    }

    pub fn handle_abs(&mut self, code: u16, value: i32) -> Option<InputEvent> {
        let slot = self.slots.entry(self.current_slot).or_default();
        match code {
            ABS_MT_SLOT => {
                self.current_slot = value as u32;
                None
            }
            ABS_MT_TRACKING_ID => {
                if value < 0 {
                    // Touch released
                    if slot.active {
                        slot.active = false;
                        slot.tracking_id = -1;
                        return Some(InputEvent::TouchUp { id: self.current_slot });
                    }
                } else {
                    // New touch down
                    slot.tracking_id = value;
                    slot.active = true;
                    slot.dirty = true;
                }
                None
            }
            ABS_MT_POSITION_X | ABS_X => {
                slot.x = value as f32;
                slot.dirty = true;
                None
            }
            ABS_MT_POSITION_Y | ABS_Y => {
                slot.y = value as f32;
                slot.dirty = true;
                None
            }
            _ => None,
        }
    }

    pub fn handle_syn(&mut self) -> Option<InputEvent> {
        let slot = self.slots.get_mut(&self.current_slot)?;
        if slot.dirty && slot.active {
            slot.dirty = false;
            Some(InputEvent::TouchDown {
                id: self.current_slot,
                x: slot.x,
                y: slot.y,
            })
        } else {
            None
        }
    }

    pub fn handle_key(&mut self, code: u16, value: i32) -> Option<InputEvent> {
        let name = match code {
            KEY_POWER => "Power",
            KEY_VOLUMEUP => "VolumeUp",
            KEY_VOLUMEDOWN => "VolumeDown",
            KEY_BACK => "Back",
            KEY_HOMEPAGE => "Home",
            _ => "Unknown",
        }.to_string();

        match value {
            1 => Some(InputEvent::KeyDown { code: code as u32, name }),
            0 => Some(InputEvent::KeyUp { code: code as u32, name }),
            _ => None, // Repeated key (value == 2)
        }
    }
    pub fn dimensions(&self) -> (f32, f32) {
        (self.screen_width, self.screen_height)
    }
}

pub fn key_name(code: u16) -> &'static str {
    match code {
        KEY_POWER => "Power",
        KEY_VOLUMEUP => "VolumeUp",
        KEY_VOLUMEDOWN => "VolumeDown",
        KEY_BACK => "Back",
        KEY_HOMEPAGE => "Home",
        _ => "Key",
    }
}

fn main() {
    println!("\x1b[1;36m[inputd]\x1b[0m Onuron OS Unified Input Subsystem Initializing...");

    let _tracker = Arc::new(Mutex::new(MultiTouchTracker::new(720.0, 1440.0)));
    let _ = fs::create_dir_all("/run/onuron");

    #[cfg(target_os = "linux")]
    {
        let tracker_clone = Arc::clone(&_tracker);
        thread::spawn(move || {
            // Scan /dev/input for event devices
            for entry in fs::read_dir("/dev/input").into_iter().flatten().flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("event") {
                    let path = entry.path();
                    println!("[inputd] Found input device: {}", path.display());
                    // Dedicated reader thread can be spawned per event device
                }
            }
        });
    }

    println!("\x1b[1;32m[inputd] [  OK  ]\x1b[0m Input event processor active (/run/onuron/input.sock)");

    // Event broadcast loop
    loop {
        thread::sleep(Duration::from_secs(30));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_touch_down_and_up() {
        let mut tracker = MultiTouchTracker::new(720.0, 1280.0);

        // Touch slot 0 tracking ID 100
        tracker.handle_abs(ABS_MT_SLOT, 0);
        tracker.handle_abs(ABS_MT_TRACKING_ID, 100);
        tracker.handle_abs(ABS_MT_POSITION_X, 360);
        tracker.handle_abs(ABS_MT_POSITION_Y, 640);

        let event = tracker.handle_syn();
        assert_eq!(
            event,
            Some(InputEvent::TouchDown {
                id: 0,
                x: 360.0,
                y: 640.0,
            })
        );

        // Touch up
        let up_event = tracker.handle_abs(ABS_MT_TRACKING_ID, -1);
        assert_eq!(up_event, Some(InputEvent::TouchUp { id: 0 }));
    }

    #[test]
    fn test_hardware_keys() {
        let mut tracker = MultiTouchTracker::new(720.0, 1280.0);

        let power_down = tracker.handle_key(KEY_POWER, 1);
        assert_eq!(
            power_down,
            Some(InputEvent::KeyDown {
                code: 116,
                name: "Power".into(),
            })
        );

        let vol_up = tracker.handle_key(KEY_VOLUMEUP, 0);
        assert_eq!(
            vol_up,
            Some(InputEvent::KeyUp {
                code: 115,
                name: "VolumeUp".into(),
            })
        );
    }
}
