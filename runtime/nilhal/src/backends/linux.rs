// runtime/nilhal/src/backends/linux.rs — Native Linux Hardware Backend (sysfs, evdev, DRM)
use crate::traits::*;
use std::fs;
use std::net::IpAddr;
use std::path::Path;

pub struct LinuxDisplay {
    width: u32,
    height: u32,
    refresh_rate: u32,
}

impl Default for LinuxDisplay {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxDisplay {
    pub fn new() -> Self {
        Self {
            width: 1080,
            height: 2400,
            refresh_rate: 60,
        }
    }
}

impl DisplayHal for LinuxDisplay {
    fn get_dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    fn get_refresh_rate(&self) -> u32 {
        self.refresh_rate
    }
    fn present_frame(&mut self, _buffer: &[u32]) -> Result<(), HalError> {
        // STUB/SIMULATED previously: returned Ok without touching any
        // framebuffer. Real presentation lives in nilui-gpu's DRM/KMS path.
        Err(HalError::BackendUnavailable(
            "LinuxDisplay::present_frame not implemented: no framebuffer/DRM present performed (use nilui-gpu DrmDevice/KmsPresenter)".into(),
        ))
    }
    fn set_brightness(&mut self, percent: u8) -> Result<(), HalError> {
        let backlight_dir = Path::new("/sys/class/backlight");
        if backlight_dir.exists() {
            if let Ok(entries) = fs::read_dir(backlight_dir) {
                for entry in entries.flatten() {
                    let max_path = entry.path().join("max_brightness");
                    let cur_path = entry.path().join("brightness");
                    if let Ok(max_str) = fs::read_to_string(&max_path) {
                        if let Ok(max_val) = max_str.trim().parse::<u32>() {
                            let target = (max_val * (percent as u32)) / 100;
                            return fs::write(&cur_path, target.to_string()).map_err(|e| {
                                HalError::IoError(format!("writing {}: {e}", cur_path.display()))
                            });
                        }
                    }
                }
            }
        }
        Err(HalError::DeviceNotFound(
            "no backlight device found in /sys/class/backlight".into(),
        ))
    }
    fn get_brightness(&self) -> u8 {
        let backlight_dir = Path::new("/sys/class/backlight");
        if backlight_dir.exists() {
            if let Ok(entries) = fs::read_dir(backlight_dir) {
                for entry in entries.flatten() {
                    let max_path = entry.path().join("max_brightness");
                    let cur_path = entry.path().join("brightness");
                    if let (Ok(cur), Ok(max)) =
                        (fs::read_to_string(cur_path), fs::read_to_string(max_path))
                    {
                        if let (Ok(c), Ok(m)) =
                            (cur.trim().parse::<u32>(), max.trim().parse::<u32>())
                        {
                            if let Some(pct) = (c * 100).checked_div(m) {
                                return pct as u8;
                            }
                        }
                    }
                }
            }
        }
        80
    }
}

pub struct LinuxInput {
    events: Vec<HalInputEvent>,
}

impl Default for LinuxInput {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxInput {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }
}

impl InputHal for LinuxInput {
    fn poll_events(&mut self) -> Vec<HalInputEvent> {
        std::mem::take(&mut self.events)
    }
    fn send_event(&mut self, event: HalInputEvent) -> Result<(), HalError> {
        self.events.push(event);
        Ok(())
    }
}

pub struct LinuxNetwork;

fn parse_dns_servers(contents: &str) -> Vec<String> {
    let mut servers = Vec::new();
    for line in contents.lines() {
        let mut fields = line.split('#').next().unwrap_or("").split_whitespace();
        if fields.next() != Some("nameserver") {
            continue;
        }
        if let Some(value) = fields.next() {
            if value.parse::<IpAddr>().is_ok() && !servers.iter().any(|s| s == value) {
                servers.push(value.to_string());
            }
        }
    }
    servers
}

#[cfg(test)]
mod network_tests {
    use super::*;

    #[test]
    fn dns_parser_accepts_only_unique_ip_nameservers() {
        assert_eq!(parse_dns_servers("nameserver 1.1.1.1\nnameserver 2001:4860::1 # v6\nnameserver bogus\nnameserverx 8.8.8.8\nnameserver 1.1.1.1\n"),
            vec!["1.1.1.1", "2001:4860::1"]);
    }
}

impl NetworkHal for LinuxNetwork {
    fn get_state(&self) -> HalNetworkState {
        let mut is_connected = false;
        let mut active_iface = None;
        let mut conn_type = ConnectionType::None;

        if let Ok(entries) = fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name == "lo" {
                    continue;
                }

                let oper_file = entry.path().join("operstate");
                if let Ok(oper) = fs::read_to_string(oper_file) {
                    if oper.trim() == "up" {
                        is_connected = true;
                        active_iface = Some(name.clone());
                        if name.starts_with("wl") || name.starts_with("wlan") {
                            conn_type = ConnectionType::Wifi;
                        } else if name.starts_with("rmnet") || name.starts_with("wwan") {
                            conn_type = ConnectionType::Cellular;
                        } else if name.starts_with("eth")
                            || name.starts_with("en")
                            || name.starts_with("virt")
                        {
                            conn_type = ConnectionType::Ethernet;
                        } else {
                            conn_type = ConnectionType::None;
                        }
                        break;
                    }
                }
            }
        }

        let dns_servers = fs::read_to_string("/etc/resolv.conf")
            .map(|contents| parse_dns_servers(&contents))
            .unwrap_or_default();
        HalNetworkState {
            is_connected,
            active_interface: active_iface,
            connection_type: conn_type,
            ip_address: None,
            dns_servers,
            wifi_ssid: None,
            cellular_carrier: None,
        }
    }
    fn scan_wifi(&mut self) -> Result<Vec<WifiApInfo>, HalError> {
        Err(HalError::UnsupportedOperation(
            "LinuxNetwork::scan_wifi not implemented: no nl80211/wpa_supplicant backend".into(),
        ))
    }
    fn connect_wifi(&mut self, _ssid: &str, _psk: &str) -> Result<(), HalError> {
        let has_wifi = if let Ok(entries) = fs::read_dir("/sys/class/net") {
            entries.flatten().any(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.starts_with("wl") || name.starts_with("wlan")
            })
        } else {
            false
        };
        if !has_wifi {
            return Err(HalError::BackendUnavailable(
                "No wireless network interface found in /sys/class/net".into(),
            ));
        }
        Err(HalError::UnsupportedOperation(
            "LinuxNetwork::connect_wifi not implemented: interface present but no association/DHCP backend performed the connection".into(),
        ))
    }
    fn set_cellular_enabled(&mut self, _enabled: bool) -> Result<(), HalError> {
        Err(HalError::UnsupportedOperation(
            "LinuxNetwork::set_cellular_enabled not implemented: no modem backend".into(),
        ))
    }
}

pub struct LinuxPower;

impl PowerHal for LinuxPower {
    fn get_battery_info(&self) -> HalBatteryInfo {
        let psupply = Path::new("/sys/class/power_supply");
        let mut cap = 100u8;
        let mut charging = false;
        let mut status = "Full".to_string();
        let mut temp = 25.0f32;
        let mut volt = 4000u32;
        let mut found_battery = false;

        if psupply.exists() {
            if let Ok(entries) = fs::read_dir(psupply) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Ok(c) = fs::read_to_string(path.join("capacity")) {
                        if let Ok(parsed) = c.trim().parse::<u8>() {
                            cap = parsed;
                            found_battery = true;
                        }
                    }
                    if let Ok(s) = fs::read_to_string(path.join("status")) {
                        status = s.trim().to_string();
                        charging = status.eq_ignore_ascii_case("charging");
                    }
                    if let Ok(t) = fs::read_to_string(path.join("temp")) {
                        if let Ok(raw) = t.trim().parse::<f32>() {
                            temp = raw / 10.0;
                        }
                    }
                    if let Ok(v) = fs::read_to_string(path.join("voltage_now")) {
                        if let Ok(raw) = v.trim().parse::<u32>() {
                            volt = raw / 1000;
                        }
                    }
                }
            }
        }

        if !found_battery {
            // Placeholder values below are NOT a reading. Flag them exactly
            // like BatteryInfoPayload::simulated() so no caller can present
            // 100%/"Full" as live telemetry.
            status = "Unknown [SIMULATED]".into();
        }
        HalBatteryInfo {
            capacity: cap,
            is_charging: charging,
            status,
            voltage_mv: volt,
            temp_c: temp,
            health: if found_battery {
                "Good".into()
            } else {
                "Unknown [SIMULATED]".into()
            },
            is_simulated: !found_battery,
        }
    }
    fn set_screen_timeout(&mut self, _seconds: u32) -> Result<(), HalError> {
        Err(HalError::UnsupportedOperation(
            "LinuxPower::set_screen_timeout not implemented".into(),
        ))
    }
    fn set_performance_mode(&mut self, _mode: PerformanceMode) -> Result<(), HalError> {
        Err(HalError::UnsupportedOperation(
            "LinuxPower::set_performance_mode not implemented: no cpufreq backend".into(),
        ))
    }
    fn acquire_wakelock(&mut self, tag: &str) -> Result<(), HalError> {
        fs::write("/sys/power/wake_lock", tag)
            .map_err(|e| HalError::IoError(format!("writing /sys/power/wake_lock: {e}")))
    }
    fn release_wakelock(&mut self, tag: &str) -> Result<(), HalError> {
        fs::write("/sys/power/wake_unlock", tag)
            .map_err(|e| HalError::IoError(format!("writing /sys/power/wake_unlock: {e}")))
    }
}

pub struct LinuxTelephony;

impl TelephonyHal for LinuxTelephony {
    fn dial(&mut self, number: &str) -> Result<String, HalError> {
        let has_modem = Path::new("/dev/cdc-wdm0").exists()
            || Path::new("/dev/ttyUSB0").exists()
            || Path::new("/dev/smd11").exists()
            || Path::new("/dev/modem").exists();
        if !has_modem {
            return Err(HalError::BackendUnavailable(
                "No physical or virtual cellular modem device found (/dev/cdc-wdm0, /dev/ttyUSB0, /dev/smd11)".into(),
            ));
        }
        Ok(format!("linux_call_{}", number))
    }
    fn hangup(&mut self, _call_id: &str) -> Result<(), HalError> {
        Err(HalError::UnsupportedOperation(
            "LinuxTelephony::hangup not implemented: no modem call-control backend".into(),
        ))
    }
    fn answer(&mut self, _call_id: &str) -> Result<(), HalError> {
        Err(HalError::UnsupportedOperation(
            "LinuxTelephony::answer not implemented: no modem call-control backend".into(),
        ))
    }
    fn send_sms(&mut self, _recipient: &str, _message: &str) -> Result<(), HalError> {
        let has_modem = Path::new("/dev/cdc-wdm0").exists()
            || Path::new("/dev/ttyUSB0").exists()
            || Path::new("/dev/smd11").exists()
            || Path::new("/dev/modem").exists();
        if !has_modem {
            return Err(HalError::BackendUnavailable(
                "No cellular modem device available for SMS transmission".into(),
            ));
        }
        Ok(())
    }
    fn get_call_state(&self) -> CallState {
        CallState::Idle
    }
    fn get_sim_status(&self) -> SimStatus {
        let has_modem = Path::new("/dev/cdc-wdm0").exists()
            || Path::new("/dev/ttyUSB0").exists()
            || Path::new("/dev/smd11").exists()
            || Path::new("/dev/modem").exists();
        SimStatus {
            slot: 1,
            is_ready: has_modem,
            carrier: if has_modem {
                "Linux Modem (oFono/ModemManager)".into()
            } else {
                "No Modem Detected [SIMULATED]".into()
            },
            phone_number: None,
        }
    }
}

pub struct LinuxCamera;

impl CameraHal for LinuxCamera {
    fn open(&mut self, camera_id: u32) -> Result<(), HalError> {
        let dev = format!("/dev/video{}", camera_id);
        if !Path::new(&dev).exists() {
            return Err(HalError::DeviceNotFound(format!(
                "Linux V4L2 camera device {} not found",
                dev
            )));
        }
        Ok(())
    }
    fn capture_frame(&mut self) -> Result<Vec<u8>, HalError> {
        if !Path::new("/dev/video0").exists() {
            return Err(HalError::BackendUnavailable(
                "No Linux V4L2 camera capture device (/dev/video0) available".into(),
            ));
        }
        // Previously returned a 4-byte fake JPEG header. No frame is read.
        Err(HalError::UnsupportedOperation(
            "LinuxCamera::capture_frame not implemented: /dev/video0 present but no V4L2 frame read".into(),
        ))
    }
    fn start_preview(&mut self) -> Result<(), HalError> {
        if !Path::new("/dev/video0").exists() {
            return Err(HalError::BackendUnavailable(
                "Linux V4L2 camera preview device not available".into(),
            ));
        }
        Err(HalError::UnsupportedOperation(
            "LinuxCamera::start_preview not implemented: no V4L2 streaming started".into(),
        ))
    }
    fn stop_preview(&mut self) -> Result<(), HalError> {
        Ok(())
    }
    fn set_torch(&mut self, on: bool) -> Result<(), HalError> {
        let flash_dir = Path::new("/sys/class/leds");
        if flash_dir.exists() {
            if let Ok(entries) = fs::read_dir(flash_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.contains("torch") || name.contains("flash") {
                        let _ =
                            fs::write(entry.path().join("brightness"), if on { "1" } else { "0" });
                        return Ok(());
                    }
                }
            }
        }
        if on {
            return Err(HalError::UnsupportedOperation(
                "No flash/torch LED node found in /sys/class/leds".into(),
            ));
        }
        Ok(())
    }
}

pub struct LinuxAudio;

impl AudioHal for LinuxAudio {
    fn set_master_volume(&mut self, _percent: u8) -> Result<(), HalError> {
        Err(HalError::UnsupportedOperation(
            "LinuxAudio::set_master_volume not implemented: no ALSA mixer backend".into(),
        ))
    }
    fn get_master_volume(&self) -> u8 {
        // Infallible trait getter with no error/flag channel; 80 is a
        // placeholder, documented as simulated in docs/maturity.toml.
        80
    }
    fn play_stream(&mut self, _pcm_samples: &[i16]) -> Result<(), HalError> {
        if !Path::new("/proc/asound").exists() && !Path::new("/dev/snd").exists() {
            return Err(HalError::BackendUnavailable(
                "ALSA sound card (/proc/asound or /dev/snd) not available".into(),
            ));
        }
        Err(HalError::UnsupportedOperation(
            "LinuxAudio::play_stream not implemented: no PCM written to ALSA".into(),
        ))
    }
    fn record_stream(&mut self, _buffer: &mut [i16]) -> Result<usize, HalError> {
        if !Path::new("/proc/asound").exists() && !Path::new("/dev/snd").exists() {
            return Err(HalError::BackendUnavailable(
                "ALSA recording capture device not available".into(),
            ));
        }
        // Previously filled the buffer with silence and reported success.
        Err(HalError::UnsupportedOperation(
            "LinuxAudio::record_stream not implemented: no PCM read from ALSA (silence is not a recording)".into(),
        ))
    }
    fn route_output(&mut self, _route: AudioRoute) -> Result<(), HalError> {
        Err(HalError::UnsupportedOperation(
            "LinuxAudio::route_output not implemented: no ALSA routing backend".into(),
        ))
    }
}

pub struct LinuxBluetooth;

impl BluetoothHal for LinuxBluetooth {
    fn set_powered(&mut self, _powered: bool) -> Result<(), HalError> {
        if !Path::new("/sys/class/bluetooth").exists() {
            return Err(HalError::BackendUnavailable(
                "no Bluetooth adapter found in /sys/class/bluetooth".into(),
            ));
        }
        Err(HalError::UnsupportedOperation(
            "LinuxBluetooth::set_powered not implemented: no rfkill/BlueZ backend".into(),
        ))
    }
    fn start_discovery(&mut self) -> Result<(), HalError> {
        if !Path::new("/sys/class/bluetooth").exists() {
            return Err(HalError::BackendUnavailable(
                "no Bluetooth adapter found in /sys/class/bluetooth".into(),
            ));
        }
        Err(HalError::UnsupportedOperation(
            "LinuxBluetooth::start_discovery not implemented: no BlueZ backend".into(),
        ))
    }
    fn get_paired_devices(&self) -> Vec<BluetoothDeviceEntry> {
        Vec::new()
    }
    fn pair(&mut self, _address: &str) -> Result<(), HalError> {
        if !Path::new("/sys/class/bluetooth").exists() {
            return Err(HalError::BackendUnavailable(
                "no Bluetooth adapter found in /sys/class/bluetooth".into(),
            ));
        }
        Err(HalError::UnsupportedOperation(
            "LinuxBluetooth::pair not implemented: no BlueZ backend".into(),
        ))
    }
}

pub struct LinuxSensors;

impl SensorHal for LinuxSensors {
    fn get_accelerometer(&self) -> (f32, f32, f32) {
        (0.0, 0.0, 0.0)
    }
    fn get_gyroscope(&self) -> (f32, f32, f32) {
        (0.0, 0.0, 0.0)
    }
    fn get_ambient_light(&self) -> f32 {
        0.0
    }
    fn get_proximity(&self) -> bool {
        false
    }
    fn get_gps_coordinates(&self) -> Option<(f64, f64, f32)> {
        None
    }
}

#[cfg(test)]
mod linux_backend_tests {
    use super::*;

    #[test]
    fn test_linux_telephony_honest_error_when_modem_absent() {
        let mut tel = LinuxTelephony;
        let sim = tel.get_sim_status();
        assert!(!sim.is_ready);
        assert!(sim.carrier.contains("[SIMULATED]"));
        assert!(tel.dial("+1234567890").is_err());
        assert!(tel.send_sms("+1234567890", "test").is_err());
    }

    #[test]
    fn test_linux_camera_honest_error_when_v4l2_absent() {
        let mut cam = LinuxCamera;
        assert!(cam.open(0).is_err());
        assert!(cam.capture_frame().is_err());
        assert!(cam.start_preview().is_err());
    }

    #[test]
    fn test_linux_audio_honest_error_when_alsa_absent() {
        let mut audio = LinuxAudio;
        assert_eq!(audio.get_master_volume(), 80);
        #[cfg(not(target_os = "linux"))]
        {
            assert!(audio.play_stream(&[1, 2, 3]).is_err());
            let mut buf = [0i16; 64];
            assert!(audio.record_stream(&mut buf).is_err());
        }
    }

    #[test]
    fn test_linux_hal_no_fabricated_success() {
        let mut display = LinuxDisplay::new();
        assert!(display.present_frame(&[0xFFFF_FFFF]).is_err());
        let mut net = LinuxNetwork;
        assert!(net.scan_wifi().is_err());
        assert!(net.connect_wifi("ssid", "psk").is_err());
        assert!(net.set_cellular_enabled(true).is_err());
        let mut power = LinuxPower;
        assert!(power.set_screen_timeout(30).is_err());
        assert!(power
            .set_performance_mode(PerformanceMode::Balanced)
            .is_err());
        let mut bt = LinuxBluetooth;
        assert!(bt.set_powered(true).is_err());
        assert!(bt.start_discovery().is_err());
        assert!(bt.pair("AA:BB:CC:DD:EE:FF").is_err());
        let mut tel = LinuxTelephony;
        assert!(tel.hangup("x").is_err());
        assert!(tel.answer("x").is_err());
    }

    #[test]
    fn test_linux_battery_placeholder_is_flagged_simulated() {
        let info = LinuxPower.get_battery_info();
        if info.is_simulated {
            assert!(info.status.contains("[SIMULATED]"), "got: {}", info.status);
            assert!(info.health.contains("[SIMULATED]"), "got: {}", info.health);
        } else {
            assert!(!info.status.contains("[SIMULATED]"));
        }
    }

    #[test]
    fn test_linux_sensors_honest_zero_fallbacks() {
        let sensors = LinuxSensors;
        assert_eq!(sensors.get_accelerometer(), (0.0, 0.0, 0.0));
        assert_eq!(sensors.get_gyroscope(), (0.0, 0.0, 0.0));
        assert_eq!(sensors.get_ambient_light(), 0.0);
        assert!(!sensors.get_proximity());
        assert_eq!(sensors.get_gps_coordinates(), None);
    }
}
