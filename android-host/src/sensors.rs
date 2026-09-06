// android-host/src/sensors.rs — Android SensorManager & Location Bridge
use nilhal::traits::SensorHal;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SensorState {
    pub accel: (f32, f32, f32),
    pub gyro: (f32, f32, f32),
    pub light: f32,
    pub proximity: bool,
    pub gps: Option<(f64, f64, f32)>,
}

pub struct AndroidHostSensors {
    state: Arc<Mutex<SensorState>>,
}

impl AndroidHostSensors {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(SensorState {
                accel: (0.0, 9.81, 0.0),
                gyro: (0.0, 0.0, 0.0),
                light: 450.0,
                proximity: false,
                gps: Some((22.5726, 88.3639, 12.0)),
            })),
        }
    }

    pub fn update_accel(&self, x: f32, y: f32, z: f32) {
        if let Ok(mut lock) = self.state.lock() {
            lock.accel = (x, y, z);
        }
    }

    pub fn update_gps(&self, lat: f64, lon: f64, alt: f32) {
        if let Ok(mut lock) = self.state.lock() {
            lock.gps = Some((lat, lon, alt));
        }
    }
}

impl SensorHal for AndroidHostSensors {
    fn get_accelerometer(&self) -> (f32, f32, f32) {
        self.state.lock().map(|s| s.accel).unwrap_or((0.0, 9.81, 0.0))
    }

    fn get_gyroscope(&self) -> (f32, f32, f32) {
        self.state.lock().map(|s| s.gyro).unwrap_or((0.0, 0.0, 0.0))
    }

    fn get_ambient_light(&self) -> f32 {
        self.state.lock().map(|s| s.light).unwrap_or(450.0)
    }

    fn get_proximity(&self) -> bool {
        self.state.lock().map(|s| s.proximity).unwrap_or(false)
    }

    fn get_gps_coordinates(&self) -> Option<(f64, f64, f32)> {
        self.state.lock().ok().and_then(|s| s.gps)
    }
}
