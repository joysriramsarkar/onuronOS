# Onuron OS on Samsung Galaxy S25: The Hosted Runtime Architecture

> **Strategic Principle**: Operating System independence is achieved through unified abstractions. We do not need a PinePhone to build real hardware integrations. The Samsung Galaxy S25 (Snapdragon 8 Elite) serves as our production-grade **Hardware Laboratory and Mobile Runtime**, while QEMU maintains pure bare-metal OS integrity.

---

## 1. The Three Operating Modes of OnuronOS

```
                                 ONURON OS
                                     │
         ┌───────────────────────────┼───────────────────────────┐
         ▼                           ▼                           ▼
   Mode 1: QEMU                Mode 2: Hosted              Mode 3: Native
 (Pure OS Development)        (Galaxy S25 Lab)          (Future Bare Metal)
┌─────────────────────┐    ┌─────────────────────┐    ┌─────────────────────┐
│ Onuron Linux Kernel │    │ Samsung Android 15+ │    │ Custom Bootloader   │
│ Virtual / VirtIO HW │    │ Snapdragon 8 Elite  │    │ Onuron Linux Kernel │
│ Onuron PID 1        │    │ Onuron Host APK     │    │ Real SoC Driver HAL │
│ Pure Linux Sysfs    │    │ Android HAL Bridge  │    │ Direct Hardware I/O │
└──────────┬──────────┘    └──────────┬──────────┘    └──────────┬──────────┘
           │                          │                          │
           └──────────────────────────┼──────────────────────────┘
                                      ▼
                           Unified NilHAL Layer
      (Display, Input, Network, Power, Telephony, Camera, Audio, Sensors)
                                      │
                                      ▼
                        Onuron Userspace & Apps
                        (NilLang + Alap + NilUI)
```

| Dimension | Mode 1: QEMU | Mode 2: Android Hosted (S25) | Mode 3: Native ARM64 |
| :--- | :--- | :--- | :--- |
| **Host Environment** | Virtual Machine / VirtIO | Samsung Android 15/16 | Bare Metal Hardware |
| **Target Hardware** | Emulated QEMU aarch64 / x86_64 | Snapdragon 8 Elite (SM8750) | Custom ARM64 Phone (PinePhone / Open SoC) |
| **Display Path** | Minifb / Virtual Framebuffer | 120 Hz Dynamic AMOLED 2X Surface | Direct DRM/KMS |
| **Input Pipeline** | Emulated Evdev / Mouse | MotionEvent Bridge → `inputd` gestures | Kernel Evdev Multi-Touch B |
| **Connectivity** | VirtIO Network (`eth0`) | ConnectivityManager (Wi-Fi 7, 5G) | Linux `wpa_supplicant` / `ModemManager` |
| **Telephony** | Simulated SIM & Dialing | Android Telecom & SMS Intents | Native AT Commands / MBIM / QMI |
| **Audio / Video** | Dummy PCM / Test JPEG | AAudio + Camera2 API Bridge | ALSA / V4L2 Native Drivers |
| **Bricking Risk** | None (Virtual) | **None (Runs as APK sandbox / Service)** | High (Bootloader flashing required) |

---

## 2. NilHAL: The Unified Trait Abstraction

All OnuronOS services (`shell`, `inputd`, `powerd`, `netd`, `camerad`, `audiod`) communicate with hardware exclusively through **NilHAL** traits located in `runtime/nilhal/src/traits.rs`:

```rust
// Unified Rust Hardware Traits
pub trait DisplayHal: Send + Sync {
    fn get_dimensions(&self) -> (u32, u32);
    fn get_refresh_rate(&self) -> u32; // 120Hz on S25
    fn present_frame(&mut self, buffer: &[u32]) -> Result<(), HalError>;
    fn set_brightness(&mut self, percent: u8) -> Result<(), HalError>;
    fn get_brightness(&self) -> u8;
}

pub trait InputHal: Send + Sync {
    fn poll_events(&mut self) -> Vec<HalInputEvent>;
    fn send_event(&mut self, event: HalInputEvent) -> Result<(), HalError>;
}

pub trait NetworkHal: Send + Sync {
    fn get_state(&self) -> HalNetworkState;
    fn scan_wifi(&mut self) -> Result<Vec<WifiApInfo>, HalError>;
    fn connect_wifi(&mut self, ssid: &str, psk: &str) -> Result<(), HalError>;
    fn set_cellular_enabled(&mut self, enabled: bool) -> Result<(), HalError>;
}

pub trait PowerHal: Send + Sync {
    fn get_battery_info(&self) -> HalBatteryInfo;
    fn set_screen_timeout(&mut self, seconds: u32) -> Result<(), HalError>;
    fn set_performance_mode(&mut self, mode: PerformanceMode) -> Result<(), HalError>;
    fn acquire_wakelock(&mut self, tag: &str) -> Result<(), HalError>;
    fn release_wakelock(&mut self, tag: &str) -> Result<(), HalError>;
}

pub trait TelephonyHal: Send + Sync {
    fn dial(&mut self, number: &str) -> Result<String, HalError>;
    fn hangup(&mut self, call_id: &str) -> Result<(), HalError>;
    fn answer(&mut self, call_id: &str) -> Result<(), HalError>;
    fn send_sms(&mut self, recipient: &str, message: &str) -> Result<(), HalError>;
    fn get_call_state(&self) -> CallState;
    fn get_sim_status(&self) -> SimStatus;
}
```

### Auto-Detection
NilHAL automatically detects whether it is running on Samsung Galaxy S25 Android Host, Native Linux, or QEMU:
```rust
let hal = nilhal::NilHal::auto();
println!("Active backend: {}", hal.backend_type);
```

---

## 3. The `android-host/` Subsystem Layout

Located at `onuronOS/android-host/`:

```
android-host/
├── Cargo.toml          # Rust crate manifest
├── src/
│   ├── lib.rs          # Subsystem entry point
│   ├── bridge.rs       # Bidirectional JSON wire protocol over Unix domain / TCP sockets
│   ├── display.rs      # S25 120Hz AMOLED Framebuffer / Surface transport
│   ├── input.rs        # MotionEvent & KeyEvent translator into inputd events
│   ├── network.rs      # ConnectivityManager (Wi-Fi 7 / 5G) telemetry
│   ├── telephony.rs    # TelecomManager call dispatch & SMS bridge
│   ├── camera.rs       # Camera2 frame capture & torch bridge
│   ├── audio.rs        # AAudio / AudioTrack PCM stream forwarding
│   ├── sensors.rs      # SensorManager (accel, gyro, light, proximity, GPS)
│   └── storage.rs      # Scoped Storage & SAF translation
└── app/                # Android APK Harness
    ├── AndroidManifest.xml
    ├── build.gradle.kts
    └── src/main/java/org/onuron/mobile/
        ├── MainActivity.java        # Fullscreen SurfaceView & touch sink
        ├── OnuronBridgeService.java # Foreground hardware daemon
        └── NativeBridge.java        # JNI C/Rust bridge wrapper
```

---

## 4. Real Gesture Engine in `inputd`

`services/inputd` processes raw touch slots and classifies gestures into real user interactions:
- **Tap**: `TouchDown` + `TouchUp` within 300ms and < 20px delta.
- **DoubleTap**: Two consecutive taps within 350ms and < 30px proximity.
- **LongPress**: Sustained stationary contact > 500ms.
- **Swipe**: Directional gesture (`Left`, `Right`, `Up`, `Down`) with velocity calculation.
- **Drag**: Continuous tracking with `(dx, dy)` deltas.
- **MultiTouchPinch**: 2 active slots computing scale magnification.

---

## 5. Summary: Architectural Superiority

By separating the **Host Bridge** from the **Userspace Runtime**, we turn our lack of a PinePhone into an architectural asset:
1. **Developer Experience**: We can develop and test complex apps on a real Snapdragon 8 Elite with 120Hz display, high-resolution cameras, GPS, and 5G.
2. **Identical Code**: The exact same `Hello.nil`, `Phone.nil`, `Settings.nil`, and `Camera.nil` apps run on desktop QEMU and the S25 without a single conditional compile tag.
3. **Future Proofing**: When open-hardware native ARM64 phones become available, we simply add a native driver to `nilhal` without modifying any upper-layer components.
