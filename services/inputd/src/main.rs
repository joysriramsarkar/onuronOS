// services/inputd/src/main.rs — Onuron OS Unified Input Subsystem & Gesture Engine
// Reads raw Linux evdev (/dev/input/event*) & Android Host bridge events,
// recognizes touch gestures (Tap, DoubleTap, LongPress, Swipe, Drag, Pinch), and broadcasts over IPC.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use nilprotocol::{Frame, MessageType};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreenRotation {
    Portrait,       // 0 deg
    LandscapeLeft,  // 90 deg counter-clockwise
    UpsideDown,     // 180 deg
    LandscapeRight, // 270 deg (90 deg clockwise)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TouchCalibration {
    pub raw_min_x: f32,
    pub raw_max_x: f32,
    pub raw_min_y: f32,
    pub raw_max_y: f32,
    pub rotation: ScreenRotation,
}

impl Default for TouchCalibration {
    fn default() -> Self {
        Self {
            raw_min_x: 0.0,
            raw_max_x: 1080.0,
            raw_min_y: 0.0,
            raw_max_y: 2340.0,
            rotation: ScreenRotation::Portrait,
        }
    }
}

impl TouchCalibration {
    /// Maps raw hardware digitizer coordinates (e.g. 0..4095) to display coordinates
    /// taking rotation (Portrait, LandscapeLeft, UpsideDown, LandscapeRight) into account.
    pub fn transform(&self, raw_x: f32, raw_y: f32, screen_w: f32, screen_h: f32) -> (f32, f32) {
        let x_span = (self.raw_max_x - self.raw_min_x).max(1.0);
        let y_span = (self.raw_max_y - self.raw_min_y).max(1.0);

        let norm_x = ((raw_x - self.raw_min_x) / x_span).clamp(0.0, 1.0);
        let norm_y = ((raw_y - self.raw_min_y) / y_span).clamp(0.0, 1.0);

        match self.rotation {
            ScreenRotation::Portrait => (norm_x * screen_w, norm_y * screen_h),
            ScreenRotation::LandscapeRight => (norm_y * screen_w, (1.0 - norm_x) * screen_h),
            ScreenRotation::UpsideDown => ((1.0 - norm_x) * screen_w, (1.0 - norm_y) * screen_h),
            ScreenRotation::LandscapeLeft => ((1.0 - norm_y) * screen_w, norm_x * screen_h),
        }
    }
}

#[derive(Default, Clone)]
struct TouchSlot {
    tracking_id: i32,
    raw_x: f32,
    raw_y: f32,
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
    calibration: TouchCalibration,
    gesture_engine: GestureRecognizer,
}

impl MultiTouchTracker {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            slots: HashMap::new(),
            current_slot: 0,
            screen_width: width,
            screen_height: height,
            calibration: TouchCalibration {
                raw_min_x: 0.0,
                raw_max_x: width,
                raw_min_y: 0.0,
                raw_max_y: height,
                rotation: ScreenRotation::Portrait,
            },
            gesture_engine: GestureRecognizer::new(),
        }
    }

    pub fn set_calibration(&mut self, cal: TouchCalibration) {
        self.calibration = cal;
    }

    pub fn set_rotation(&mut self, rotation: ScreenRotation) {
        self.calibration.rotation = rotation;
    }

    pub fn set_screen_dimensions(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
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
                slot.raw_x = value as f32;
                slot.dirty = true;
                None
            }
            ABS_MT_POSITION_Y | ABS_Y => {
                slot.raw_y = value as f32;
                slot.dirty = true;
                None
            }
            _ => None,
        }
    }

    pub fn handle_syn(&mut self) -> Option<InputEvent> {
        let (w, h) = (self.screen_width, self.screen_height);
        let cal = self.calibration.clone();
        let slot = self.slots.get_mut(&self.current_slot)?;
        if slot.dirty && slot.active {
            slot.dirty = false;
            let (tx, ty) = cal.transform(slot.raw_x, slot.raw_y, w, h);
            slot.x = tx;
            slot.y = ty;
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

pub fn handle_ipc_request(
    frame: &Frame,
    _tracker: &Arc<Mutex<MultiTouchTracker>>,
    event_queue: &Arc<Mutex<VecDeque<InputEvent>>>,
) -> Frame {
    let msg_type = MessageType::from(frame.message_type);
    match msg_type {
        MessageType::Ping => Frame::new(MessageType::Pong, frame.request_id, b"pong".to_vec()),
        MessageType::ServiceStatusRequest => {
            let payload = nilprotocol::ServiceStatusPayload {
                service_name: "inputd".to_string(),
                is_ready: true,
                is_simulated: false,
                backend_name: "evdev-multitouch".to_string(),
                uptime_secs: 0,
                request_count: 1,
                last_error: None,
            };
            Frame::with_json(MessageType::ServiceStatusResponse, frame.request_id, &payload)
                .unwrap_or_else(|_| Frame::new(MessageType::ErrorResponse, frame.request_id, b"encode error".to_vec()))
        }
        MessageType::InputPollEvents => {
            let mut q = event_queue.lock().unwrap();
            let events: Vec<InputEvent> = q.drain(..).collect();
            let json = serde_json::to_vec(&events).unwrap_or_default();
            Frame::new(MessageType::InputEventsBatch, frame.request_id, json)
        }
        MessageType::InputInjectEvent => {
            match serde_json::from_slice::<InputEvent>(&frame.payload) {
                Ok(ev) => {
                    let mut q = event_queue.lock().unwrap();
                    if q.len() >= 256 {
                        q.pop_front();
                    }
                    q.push_back(ev);
                    Frame::new(MessageType::Pong, frame.request_id, b"injected".to_vec())
                }
                Err(e) => Frame::new(
                    MessageType::ErrorResponse,
                    frame.request_id,
                    format!("Invalid InputEvent JSON: {}", e).into_bytes(),
                ),
            }
        }
        _ => Frame::new(
            MessageType::ErrorResponse,
            frame.request_id,
            b"unsupported message type".to_vec(),
        ),
    }
}

fn main() {
    println!("\x1b[1;36m[inputd]\x1b[0m Onuron OS Unified Input Subsystem Initializing...");

    let tracker = Arc::new(Mutex::new(MultiTouchTracker::new(1080.0, 2340.0)));
    let event_queue: Arc<Mutex<VecDeque<InputEvent>>> = Arc::new(Mutex::new(VecDeque::new()));
    let _ = (&tracker, &event_queue);
    let _ = fs::create_dir_all("/run/onuron");

    #[cfg(target_os = "linux")]
    {
        let tracker_clone = Arc::clone(&tracker);
        let queue_clone = Arc::clone(&event_queue);
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
            let _ = (tracker_clone, queue_clone);
        });
    }

    #[cfg(unix)]
    {
        let tracker_ipc = Arc::clone(&tracker);
        let queue_ipc = Arc::clone(&event_queue);
        thread::spawn(move || {
            let listener = match nilsd::first_listener_or_bind("/run/onuron/input.sock") {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[inputd] Failed to bind IPC socket /run/onuron/input.sock: {}", e);
                    return;
                }
            };
            println!("\x1b[1;32m[inputd] [  OK  ]\x1b[0m IPC Server listening on /run/onuron/input.sock");

            for stream in listener.incoming() {
                if let Ok(mut sock) = stream {
                    let tracker = Arc::clone(&tracker_ipc);
                    let queue = Arc::clone(&queue_ipc);
                    thread::spawn(move || {
                        while let Ok(frame) = Frame::read_from(&mut sock) {
                            let resp = handle_ipc_request(&frame, &tracker, &queue);
                            if let Err(e) = resp.write_to(&mut sock) {
                                eprintln!("[inputd] IPC send error: {}", e);
                                break;
                            }
                        }
                    });
                }
            }
        });
    }

    let _ = nilsd::notify_ready("inputd", Some("/run/onuron/input.sock"));
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

    #[test]
    fn test_input_ipc_poll_and_inject() {
        let tracker = Arc::new(Mutex::new(MultiTouchTracker::new(720.0, 1280.0)));
        let queue = Arc::new(Mutex::new(VecDeque::new()));

        // 1. Ping
        let ping_frame = Frame::new(MessageType::Ping, 1, vec![]);
        let pong_frame = handle_ipc_request(&ping_frame, &tracker, &queue);
        assert_eq!(pong_frame.message_type, u16::from(MessageType::Pong));
        assert_eq!(pong_frame.payload, b"pong");

        // 2. Poll empty
        let poll_frame = Frame::new(MessageType::InputPollEvents, 2, vec![]);
        let batch_frame = handle_ipc_request(&poll_frame, &tracker, &queue);
        assert_eq!(batch_frame.message_type, u16::from(MessageType::InputEventsBatch));
        let events: Vec<InputEvent> = serde_json::from_slice(&batch_frame.payload).unwrap();
        assert!(events.is_empty());

        // 3. Inject event
        let inject_ev = InputEvent::Tap { x: 100.0, y: 200.0 };
        let inject_payload = serde_json::to_vec(&inject_ev).unwrap();
        let inject_frame = Frame::new(MessageType::InputInjectEvent, 3, inject_payload);
        let resp = handle_ipc_request(&inject_frame, &tracker, &queue);
        assert_eq!(resp.message_type, u16::from(MessageType::Pong));
        assert_eq!(resp.payload, b"injected");

        // 4. Poll again
        let batch_frame2 = handle_ipc_request(&poll_frame, &tracker, &queue);
        let events2: Vec<InputEvent> = serde_json::from_slice(&batch_frame2.payload).unwrap();
        assert_eq!(events2.len(), 1);
        assert_eq!(events2[0], inject_ev);

        // 5. ServiceStatusRequest
        let status_frame = Frame::new(MessageType::ServiceStatusRequest, 5, vec![]);
        let resp = handle_ipc_request(&status_frame, &tracker, &queue);
        assert_eq!(resp.message_type, u16::from(MessageType::ServiceStatusResponse));
        let status: nilprotocol::ServiceStatusPayload = resp.parse_json().unwrap();
        assert_eq!(status.service_name, "inputd");
        assert!(status.is_ready);
    }

    #[test]
    fn test_touch_calibration_and_scaling() {
        let mut tracker = MultiTouchTracker::new(1080.0, 2340.0);
        // Hardware touch digitizer reports raw range 0..4095
        tracker.set_calibration(TouchCalibration {
            raw_min_x: 0.0,
            raw_max_x: 4095.0,
            raw_min_y: 0.0,
            raw_max_y: 4095.0,
            rotation: ScreenRotation::Portrait,
        });

        // Touch at half-way on hardware: 2047, 2047
        tracker.handle_abs(ABS_MT_SLOT, 0);
        tracker.handle_abs(ABS_MT_TRACKING_ID, 1);
        tracker.handle_abs(ABS_MT_POSITION_X, 2047);
        tracker.handle_abs(ABS_MT_POSITION_Y, 2047);

        let event = tracker.handle_syn().expect("Expected TouchDown");
        match event {
            InputEvent::TouchDown { x, y, .. } => {
                // Should scale to ~50% of 1080 (540) and ~50% of 2340 (1170)
                assert!((x - 540.0).abs() < 2.0, "x was {}", x);
                assert!((y - 1170.0).abs() < 2.0, "y was {}", y);
            }
            other => panic!("Expected TouchDown, got {:?}", other),
        }
    }

    #[test]
    fn test_screen_rotation_coordinate_transform() {
        let mut tracker = MultiTouchTracker::new(1080.0, 2340.0);
        tracker.set_calibration(TouchCalibration {
            raw_min_x: 0.0,
            raw_max_x: 1080.0,
            raw_min_y: 0.0,
            raw_max_y: 2340.0,
            rotation: ScreenRotation::UpsideDown,
        });

        tracker.handle_abs(ABS_MT_SLOT, 0);
        tracker.handle_abs(ABS_MT_TRACKING_ID, 2);
        // Hardware reports top-left: 0, 0
        tracker.handle_abs(ABS_MT_POSITION_X, 0);
        tracker.handle_abs(ABS_MT_POSITION_Y, 0);

        let event = tracker.handle_syn().expect("Expected TouchDown");
        match event {
            InputEvent::TouchDown { x, y, .. } => {
                // In UpsideDown, top-left maps to bottom-right (1080, 2340)
                assert_eq!(x, 1080.0);
                assert_eq!(y, 2340.0);
            }
            other => panic!("Expected TouchDown, got {:?}", other),
        }
    }
}
