// runtime/nilhal/src/lib.rs — Unified Rust Hardware Abstraction Layer (NilHAL)
// Supports Mode 1 (QEMU/Simulator), Mode 2 (Android Hosted S25), and Mode 3 (Linux / Native ARM64).

pub mod traits;
pub mod backends;

pub use traits::*;
pub use backends::{BackendType, qemu, linux, android, fake};

use std::path::Path;
use std::ffi::CStr;
use libloading::{Library, Symbol};

// ─── Runtime Environment Auto-Detection ───────────────────────────────────────

/// Automatically detect the underlying hardware execution environment
pub fn detect_backend() -> BackendType {
    // 1. Explicit override via environment variable
    if let Ok(val) = std::env::var("ONURON_BACKEND") {
        match val.to_lowercase().as_str() {
            "fake" | "mock" | "test" => return BackendType::Fake,
            "android" | "s25" | "mode2" => return BackendType::Android,
            "linux" | "drm" => return BackendType::Linux,
            "qemu" | "sim" | "desktop" => return BackendType::Qemu,
            "native" | "arm64" => return BackendType::NativeArm64,
            _ => {}
        }
    }

    // 2. Detect Android Host Environment (Samsung Galaxy S25 APK / Termux / Container)
    if std::env::var("ANDROID_ROOT").is_ok()
        || std::env::var("ONURON_ANDROID_HOST").is_ok()
        || Path::new("/system/lib64/libandroid_runtime.so").exists()
        || Path::new("/data/data/org.onuron.mobile").exists()
        || Path::new("/run/onuron/android_bridge.sock").exists()
    {
        return BackendType::Android;
    }

    // 3. Detect Native Linux device
    #[cfg(target_os = "linux")]
    {
        if Path::new("/sys/class/power_supply").exists()
            || Path::new("/dev/input").exists()
            || Path::new("/dev/dri/card0").exists()
        {
            return BackendType::Linux;
        }
    }

    // 4. Default to QEMU / Simulated Environment (Safe desktop fallback)
    BackendType::Qemu
}

// ─── Unified NilHal Hardware Instance ────────────────────────────────────────

pub struct NilHal {
    pub backend_type: BackendType,
    pub display: Box<dyn DisplayHal>,
    pub input: Box<dyn InputHal>,
    pub network: Box<dyn NetworkHal>,
    pub power: Box<dyn PowerHal>,
    pub telephony: Box<dyn TelephonyHal>,
    pub camera: Box<dyn CameraHal>,
    pub audio: Box<dyn AudioHal>,
    pub bluetooth: Box<dyn BluetoothHal>,
    pub sensors: Box<dyn SensorHal>,
}

impl NilHal {
    /// Initialize NilHAL with auto-detected backend
    pub fn auto() -> Self {
        Self::new(detect_backend())
    }

    /// Initialize NilHAL with an explicit backend type
    pub fn new(backend: BackendType) -> Self {
        match backend {
            BackendType::Android => {
                let bridge = android::AndroidBridgeClient::new();
                Self {
                    backend_type: backend,
                    display: Box::new(android::AndroidDisplay::new(bridge.clone())),
                    input: Box::new(android::AndroidInput::new(bridge.clone())),
                    network: Box::new(android::AndroidNetwork::new(bridge.clone())),
                    power: Box::new(android::AndroidPower::new(bridge.clone())),
                    telephony: Box::new(android::AndroidTelephony::new(bridge.clone())),
                    camera: Box::new(android::AndroidCamera::new(bridge.clone())),
                    audio: Box::new(android::AndroidAudio::new(bridge.clone())),
                    bluetooth: Box::new(android::AndroidBluetooth::new(bridge.clone())),
                    sensors: Box::new(android::AndroidSensors::new(bridge)),
                }
            }
            BackendType::Linux => {
                Self {
                    backend_type: backend,
                    display: Box::new(linux::LinuxDisplay::new()),
                    input: Box::new(linux::LinuxInput::new()),
                    network: Box::new(linux::LinuxNetwork),
                    power: Box::new(linux::LinuxPower),
                    telephony: Box::new(linux::LinuxTelephony),
                    camera: Box::new(linux::LinuxCamera),
                    audio: Box::new(linux::LinuxAudio),
                    bluetooth: Box::new(linux::LinuxBluetooth),
                    sensors: Box::new(linux::LinuxSensors),
                }
            }
            BackendType::Fake => {
                Self {
                    backend_type: backend,
                    display: Box::new(fake::FakeDisplay::default()),
                    input: Box::new(fake::FakeInput::new()),
                    network: Box::new(fake::FakeNetwork::new()),
                    power: Box::new(fake::FakePower::new()),
                    telephony: Box::new(fake::FakeTelephony::default()),
                    camera: Box::new(fake::FakeCamera::default()),
                    audio: Box::new(fake::FakeAudio::default()),
                    bluetooth: Box::new(fake::FakeBluetooth::default()),
                    sensors: Box::new(fake::FakeSensors),
                }
            }
            BackendType::Qemu | BackendType::NativeArm64 => {
                Self {
                    backend_type: backend,
                    display: Box::new(qemu::QemuDisplay::new()),
                    input: Box::new(qemu::QemuInput::new()),
                    network: Box::new(qemu::QemuNetwork),
                    power: Box::new(qemu::QemuPower::new()),
                    telephony: Box::new(qemu::QemuTelephony::new()),
                    camera: Box::new(qemu::QemuCamera::new()),
                    audio: Box::new(qemu::QemuAudio::new()),
                    bluetooth: Box::new(qemu::QemuBluetooth),
                    sensors: Box::new(qemu::QemuSensors),
                }
            }
        }
    }
}

// ─── Legacy C-ABI dlopen Compatibility Layer ─────────────────────────────────
pub const NIL_HAL_API_VERSION: u32 = 3;

#[repr(C)]
pub struct NilHalModule {
    pub api_version: u32,
    pub hal_type: u32,
    pub name: *const i8,
    pub author: *const i8,
    pub init: Option<unsafe extern "C" fn() -> i32>,
    pub deinit: Option<unsafe extern "C" fn() -> i32>,
    pub reserved: [*mut std::ffi::c_void; 8],
}

pub struct HalDevice {
    _lib: Library,
    module: *const NilHalModule,
}

impl HalDevice {
    pub fn load(name: &str) -> Result<Self, String> {
        let paths = [
            format!("/vendor/lib/nilhal/libnilhal_{}.so", name),
            format!("/usr/lib/nilhal/libnilhal_{}.so", name),
            format!("./libnilhal_{}.so", name),
        ];

        for p in &paths {
            if Path::new(p).exists() {
                unsafe {
                    let lib = Library::new(p).map_err(|e| e.to_string())?;
                    let sym: Symbol<*const NilHalModule> = lib.get(b"NIL_HAL_MODULE_INFO\0")
                        .map_err(|e| e.to_string())?;
                    let module = *sym;
                    if (*module).api_version != NIL_HAL_API_VERSION {
                        return Err(format!("HAL API version mismatch: expected {}, got {}", NIL_HAL_API_VERSION, (*module).api_version));
                    }
                    if let Some(init) = (*module).init {
                        if init() != 0 {
                            return Err("HAL init failed".into());
                        }
                    }
                    return Ok(HalDevice { _lib: lib, module });
                }
            }
        }

        // Fallback to libhybris if on Android-based platform
        if Path::new("/system/lib64/libandroid_runtime.so").exists() {
            println!("[nilhal] Attempting libhybris bridge fallback for {}", name);
        }

        Err(format!("HAL driver for '{}' not found in search paths", name))
    }

    pub fn get_name(&self) -> String {
        unsafe {
            if (*self.module).name.is_null() {
                "unknown".to_string()
            } else {
                CStr::from_ptr((*self.module).name).to_string_lossy().into_owned()
            }
        }
    }
}

impl Drop for HalDevice {
    fn drop(&mut self) {
        unsafe {
            if let Some(deinit) = (*self.module).deinit {
                deinit();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_detection_and_qemu_hal() {
        let hal = NilHal::new(BackendType::Qemu);
        assert_eq!(hal.display.get_dimensions(), (1080, 2340));
        assert_eq!(hal.power.get_battery_info().capacity, 92);
        assert!(hal.network.get_state().is_connected);
    }

    #[test]
    fn test_android_backend_s25_specs() {
        let hal = NilHal::new(BackendType::Android);
        // S25 features 120Hz display
        assert_eq!(hal.display.get_refresh_rate(), 120);
        let sim = hal.telephony.get_sim_status();
        assert!(sim.is_ready);
        let sensors = hal.sensors.get_accelerometer();
        assert_eq!(sensors.1, 9.81);
    }

    #[test]
    fn test_fake_backend_in_memory_simulation() {
        let mut hal = NilHal::new(BackendType::Fake);
        assert_eq!(hal.display.get_dimensions(), (1080, 2400));
        assert_eq!(hal.display.get_brightness(), 100);
        assert!(hal.display.present_frame(&[0xFFFFFFFF, 0x00000000]).is_ok());

        // Test input injection and polling
        let event = HalInputEvent::KeyDown {
            code: 28,
            name: "Enter".to_string(),
        };
        assert!(hal.input.send_event(event).is_ok());
        let polled = hal.input.poll_events();
        assert_eq!(polled.len(), 1);

        // Test power and battery
        assert_eq!(hal.power.get_battery_info().capacity, 85);
        assert!(hal.power.acquire_wakelock("test-screen").is_ok());
        assert!(hal.power.release_wakelock("test-screen").is_ok());

        // Test network
        let net = hal.network.get_state();
        assert!(net.is_connected);
        assert_eq!(net.connection_type, ConnectionType::Wifi);

        // Test telephony
        let call_id = hal.telephony.dial("+1234567890").unwrap();
        assert!(!call_id.is_empty());
        match hal.telephony.get_call_state() {
            CallState::Active { number, .. } => assert_eq!(number, "+1234567890"),
            _ => panic!("Expected active call"),
        }
        assert!(hal.telephony.hangup(&call_id).is_ok());
        assert_eq!(hal.telephony.get_call_state(), CallState::Idle);

        // Test audio
        assert!(hal.audio.set_master_volume(75).is_ok());
        assert_eq!(hal.audio.get_master_volume(), 75);
        assert!(hal.audio.play_stream(&[100, 200, -100]).is_ok());
    }
}
