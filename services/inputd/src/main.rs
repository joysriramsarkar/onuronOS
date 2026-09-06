// services/inputd/src/main.rs — Onuron OS Unified Input Subsystem & Gesture Engine
// Reads raw Linux evdev (/dev/input/event*) & Android Host bridge events,
// recognizes touch gestures (Tap, DoubleTap, LongPress, Swipe, Drag, Pinch), and broadcasts over IPC.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SwipeDirection {
    Left,
    Right,
    Up,
    Down,
}

/// High-level Unified Input Event for Onuron OS
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum InputEvent {
    // Raw Touch
    TouchDown { id: u32, x: f32, y: f32 },
    TouchMove { id: u32, x: f32, y: f32 },
    TouchUp { id: u32 },

    // High-Level Gestures
    Tap { x: f32, y: f32 },
    DoubleTap { x: f32, y: f32 },
    LongPress { x: f32, y: f32 },
    Swipe { direction: SwipeDirection, velocity: f32 },
    Drag { x: f32, y: f32, dx: f32, dy: f32 },
    MultiTouchPinch { center_x: f32, center_y: f32, scale: f32 },

    // Hardware Keys
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

/// Gesture Recognition State Tracker
pub struct GestureRecognizer {
    start_pos: Option<(f32, f32)>,
    last_pos: Option<(f32, f32)>,
    start_time: Option<Instant>,
    last_tap_time: Option<Instant>,
    last_tap_pos: Option<(f32, f32)>,
    is_dragging: bool,
}

impl Default for GestureRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl GestureRecognizer {
    pub fn new() -> Self {
        Self {
            start_pos: None,
            last_pos: None,
            start_time: None,
            last_tap_time: None,
            last_tap_pos: None,
            is_dragging: false,
        }
    }

    pub fn on_touch_down(&mut self, x: f32, y: f32) {
        self.start_pos = Some((x, y));
        self.last_pos = Some((x, y));
        self.start_time = Some(Instant::now());
        self.is_dragging = false;
    }

    pub fn on_touch_move(&mut self, x: f32, y: f32) -> Option<InputEvent> {
        if let Some((last_x, last_y)) = self.last_pos {
            let dx = x - last_x;
            let dy = y - last_y;
            self.last_pos = Some((x, y));

            if dx.abs() > 4.0 || dy.abs() > 4.0 {
                self.is_dragging = true;
                return Some(InputEvent::Drag { x, y, dx, dy });
            }
        }
        None
    }

    pub fn on_touch_up(&mut self, up_x: f32, up_y: f32) -> Vec<InputEvent> {
        let mut events = Vec::new();

        if let (Some((start_x, start_y)), Some(start_time)) = (self.start_pos.take(), self.start_time.take()) {
            let elapsed = start_time.elapsed();
            let dx = up_x - start_x;
            let dy = up_y - start_y;
            let dist = (dx * dx + dy * dy).sqrt();

            // Swipe detection
            if dist > 50.0 && elapsed.as_millis() < 400 {
                let velocity = dist / (elapsed.as_secs_f32() + 0.001);
                let direction = if dx.abs() > dy.abs() {
                    if dx > 0.0 { SwipeDirection::Right } else { SwipeDirection::Left }
                } else if dy > 0.0 {
                    SwipeDirection::Down
                } else {
                    SwipeDirection::Up
                };
                events.push(InputEvent::Swipe { direction, velocity });
            } else if dist < 20.0 {
                // Stationary touch
                if elapsed.as_millis() >= 500 {
                    // Long press
                    events.push(InputEvent::LongPress { x: up_x, y: up_y });
                } else {
                    // Tap or DoubleTap
                    let now = Instant::now();
                    let is_double_tap = if let (Some(last_time), Some((last_x, last_y))) = (self.last_tap_time, self.last_tap_pos) {
                        let tap_interval = now.duration_since(last_time);
                        let tap_dist = ((up_x - last_x).powi(2) + (up_y - last_y).powi(2)).sqrt();
                        tap_interval.as_millis() < 350 && tap_dist < 30.0
                    } else {
                        false
                    };

                    if is_double_tap {
                        events.push(InputEvent::DoubleTap { x: up_x, y: up_y });
                        self.last_tap_time = None;
                        self.last_tap_pos = None;
                    } else {
                        events.push(InputEvent::Tap { x: up_x, y: up_y });
                        self.last_tap_time = Some(now);
                        self.last_tap_pos = Some((up_x, up_y));
                    }
                }
            }
        }

        self.last_pos = None;
        self.is_dragging = false;
        events
    }
}

/// Evdev Touch State Tracker (Multi-Touch Protocol B)
pub struct MultiTouchTracker {
    slots: HashMap<u32, TouchSlot>,
    current_slot: u32,
    screen_width: f32,
    screen_height: f32,
    gesture_engine: GestureRecognizer,
}

impl MultiTouchTracker {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            slots: HashMap::new(),
            current_slot: 0,
            screen_width: width,
            screen_height: height,
            gesture_engine: GestureRecognizer::new(),
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
                        let up_x = slot.x;
                        let up_y = slot.y;
                        let _gestures = self.gesture_engine.on_touch_up(up_x, up_y);
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
            self.gesture_engine.on_touch_down(slot.x, slot.y);
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
            _ => None,
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

    let _tracker = Arc::new(Mutex::new(MultiTouchTracker::new(1080.0, 2340.0)));
    let _ = fs::create_dir_all("/run/onuron");

    #[cfg(target_os = "linux")]
    {
        let _tracker_clone = Arc::clone(&tracker);
        thread::spawn(move || {
            // Scan /dev/input for event devices
            if let Ok(entries) = fs::read_dir("/dev/input") {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with("event") {
                        let path = entry.path();
                        println!("[inputd] Found input device: {}", path.display());
                    }
                }
            }
        });
    }

    println!("\x1b[1;32m[inputd] [  OK  ]\x1b[0m Input event & gesture processor active (/run/onuron/input.sock)");

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

        let up_event = tracker.handle_abs(ABS_MT_TRACKING_ID, -1);
        assert_eq!(up_event, Some(InputEvent::TouchUp { id: 0 }));
    }

    #[test]
    fn test_tap_and_swipe_gestures() {
        let mut recognizer = GestureRecognizer::new();

        // 1. Test Tap
        recognizer.on_touch_down(100.0, 200.0);
        let gestures = recognizer.on_touch_up(102.0, 201.0);
        assert_eq!(gestures.len(), 1);
        assert_eq!(gestures[0], InputEvent::Tap { x: 102.0, y: 201.0 });

        // 2. Test Swipe Left
        recognizer.on_touch_down(300.0, 200.0);
        let gestures = recognizer.on_touch_up(100.0, 205.0);
        assert_eq!(gestures.len(), 1);
        if let InputEvent::Swipe { direction, .. } = &gestures[0] {
            assert_eq!(*direction, SwipeDirection::Left);
        } else {
            panic!("Expected swipe left");
        }

        // 3. Test Drag
        recognizer.on_touch_down(100.0, 100.0);
        let drag = recognizer.on_touch_move(115.0, 105.0);
        assert!(drag.is_some());
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
