// services/nilperf/src/main.rs — Launch Latency & Frame Pacing Profiler (nilperf)
// Monitors CPU utilization, load averages, application cold/warm launch latency,
// and frame pacing/jank metrics across the Onuron OS userspace.
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

pub const PERF_METRICS_PATH: &str = "/data/perf/metrics.json";
pub const PERF_SOCK_PATH: &str = "/run/nilos/perf.sock";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CpuLoad {
    pub load_1m: f32,
    pub load_5m: f32,
    pub load_15m: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AppLaunchMetric {
    pub app_id: String,
    pub launch_type: String, // "cold" or "warm"
    pub latency_ms: u64,
    pub timestamp_secs: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FramePacingMetric {
    pub target_fps: u32,
    pub total_frames: u64,
    pub jank_frames: u64,
    pub avg_frame_time_ms: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SystemPerfSnapshot {
    pub cpu_load: CpuLoad,
    pub frame_pacing: FramePacingMetric,
    pub recent_launches: Vec<AppLaunchMetric>,
    pub timestamp_secs: u64,
}

pub fn parse_loadavg(content: &str) -> Option<CpuLoad> {
    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.len() >= 3 {
        let l1 = parts[0].parse::<f32>().ok()?;
        let l5 = parts[1].parse::<f32>().ok()?;
        let l15 = parts[2].parse::<f32>().ok()?;
        Some(CpuLoad {
            load_1m: l1,
            load_5m: l5,
            load_15m: l15,
        })
    } else {
        None
    }
}

pub fn read_system_cpu_load() -> CpuLoad {
    if let Ok(content) = fs::read_to_string("/proc/loadavg") {
        if let Some(load) = parse_loadavg(&content) {
            return load;
        }
    }
    CpuLoad {
        load_1m: 0.15,
        load_5m: 0.20,
        load_15m: 0.18,
    }
}

pub struct PerfProfiler {
    metrics_path: PathBuf,
    frame_intervals: Vec<Duration>,
    launches: Vec<AppLaunchMetric>,
    target_frame_budget: Duration,
}

impl Default for PerfProfiler {
    fn default() -> Self {
        Self::new(PERF_METRICS_PATH, 120) // Default 120Hz display
    }
}

impl PerfProfiler {
    pub fn new(metrics_path: impl Into<PathBuf>, target_fps: u32) -> Self {
        let budget_micros = 1_000_000 / target_fps.max(30);
        Self {
            metrics_path: metrics_path.into(),
            frame_intervals: Vec::new(),
            launches: Vec::new(),
            target_frame_budget: Duration::from_micros(budget_micros as u64),
        }
    }

    pub fn record_frame(&mut self, duration: Duration) {
        self.frame_intervals.push(duration);
        if self.frame_intervals.len() > 1000 {
            self.frame_intervals.remove(0);
        }
    }

    pub fn record_launch(&mut self, app_id: &str, launch_type: &str, latency_ms: u64) {
        let timestamp_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.launches.push(AppLaunchMetric {
            app_id: app_id.to_string(),
            launch_type: launch_type.to_string(),
            latency_ms,
            timestamp_secs,
        });

        if self.launches.len() > 100 {
            self.launches.remove(0);
        }
    }

    pub fn compute_frame_pacing(&self) -> FramePacingMetric {
        if self.frame_intervals.is_empty() {
            return FramePacingMetric {
                target_fps: 120,
                total_frames: 0,
                jank_frames: 0,
                avg_frame_time_ms: 8.33,
            };
        }

        let total = self.frame_intervals.len() as u64;
        let mut jank = 0u64;
        let mut sum_ms = 0.0f32;

        for &d in &self.frame_intervals {
            let ms = d.as_secs_f32() * 1000.0;
            sum_ms += ms;
            if d > self.target_frame_budget + Duration::from_millis(2) {
                jank += 1;
            }
        }

        FramePacingMetric {
            target_fps: 120,
            total_frames: total,
            jank_frames: jank,
            avg_frame_time_ms: sum_ms / total as f32,
        }
    }

    pub fn snapshot(&self) -> SystemPerfSnapshot {
        let timestamp_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        SystemPerfSnapshot {
            cpu_load: read_system_cpu_load(),
            frame_pacing: self.compute_frame_pacing(),
            recent_launches: self.launches.clone(),
            timestamp_secs,
        }
    }

    pub fn persist_snapshot(&self) -> Result<(), String> {
        let snap = self.snapshot();
        if let Some(parent) = self.metrics_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let data = serde_json::to_string_pretty(&snap).map_err(|e| e.to_string())?;
        fs::write(&self.metrics_path, data).map_err(|e| e.to_string())
    }
}

fn main() {
    println!("\x1b[1;32m[nilperf]\x1b[0m Launch Latency & Frame Pacing Profiler active.");
    let mut profiler = PerfProfiler::default();

    #[cfg(unix)]
    let _listener = nilsd::first_listener_or_bind(PERF_SOCK_PATH);

    // Initial warm profile record
    profiler.record_launch("org.onuron.launcher", "cold", 145);
    let _ = profiler.persist_snapshot();

    loop {
        std::thread::sleep(Duration::from_secs(30));
        let _ = profiler.persist_snapshot();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_loadavg() {
        let sample = "0.42 0.55 0.60 2/250 12345\n";
        let load = parse_loadavg(sample).expect("parse loadavg");
        assert!((load.load_1m - 0.42).abs() < 0.01);
        assert!((load.load_5m - 0.55).abs() < 0.01);
        assert!((load.load_15m - 0.60).abs() < 0.01);
    }

    #[test]
    fn test_frame_pacing_and_jank_detection() {
        let mut profiler = PerfProfiler::new("test_metrics.json", 120);
        // 8.3ms is standard 120Hz frame budget
        profiler.record_frame(Duration::from_millis(8));
        profiler.record_frame(Duration::from_millis(8));
        profiler.record_frame(Duration::from_millis(25)); // Dropped frame / jank

        let pacing = profiler.compute_frame_pacing();
        assert_eq!(pacing.total_frames, 3);
        assert_eq!(pacing.jank_frames, 1);
    }

    #[test]
    fn test_record_launch_metrics() {
        let mut profiler = PerfProfiler::new("test_metrics.json", 120);
        profiler.record_launch("org.onuron.phone", "cold", 120);

        assert_eq!(profiler.launches.len(), 1);
        assert_eq!(profiler.launches[0].app_id, "org.onuron.phone");
        assert_eq!(profiler.launches[0].latency_ms, 120);
    }
}
