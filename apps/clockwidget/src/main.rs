// apps/clockwidget/src/main.rs — OnuronOS Live Clock & Stopwatch Widget
// Using the official OnuronOS Design System (dark/light themes)
use nilui::design::{DesignSystem, Palette};
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClockState {
    pub use_24h: bool,
    pub tz_offset_mins: i64,
    pub stopwatch_running: bool,
    pub stopwatch_start: Option<f32>,
    pub stopwatch_accumulated: f32,
    pub stopwatch_elapsed_secs: f32,
}

impl Default for ClockState {
    fn default() -> Self {
        Self {
            use_24h: true,
            tz_offset_mins: 360, // UTC+6 Bangladesh Standard Time
            stopwatch_running: false,
            stopwatch_start: None,
            stopwatch_accumulated: 0.0,
            stopwatch_elapsed_secs: 0.0,
        }
    }
}

pub fn get_current_epoch_secs() -> f32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f32())
        .unwrap_or(0.0)
}

pub fn update(state: &mut ClockState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.use_24h = !state.use_24h;
        }
        Ev::Click(2) => {
            state.stopwatch_running = !state.stopwatch_running;
            let now = get_current_epoch_secs();
            if state.stopwatch_running {
                state.stopwatch_start = Some(now);
            } else {
                state.stopwatch_accumulated = state.stopwatch_elapsed_secs;
                state.stopwatch_start = None;
            }
        }
        Ev::Click(3) => {
            // Reset stopwatch
            state.stopwatch_running = false;
            state.stopwatch_start = None;
            state.stopwatch_accumulated = 0.0;
            state.stopwatch_elapsed_secs = 0.0;
        }
        Ev::Tick(_) => {
            if state.stopwatch_running {
                let now = get_current_epoch_secs();
                if let Some(start) = state.stopwatch_start {
                    state.stopwatch_elapsed_secs = state.stopwatch_accumulated + (now - start);
                }
            }
        }
        _ => {}
    }
}

pub fn format_time(seconds: f32, use_24h: bool) -> String {
    let total_secs = seconds.max(0.0) as u64;
    let hours = (total_secs / 3600) % 24;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if use_24h {
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        let h12 = if hours == 0 {
            12
        } else if hours > 12 {
            hours - 12
        } else {
            hours
        };
        let meridiem = if hours < 12 { "AM" } else { "PM" };
        format!("{:02}:{:02}:{:02} {}", h12, mins, secs, meridiem)
    }
}

pub fn view(state: &ClockState) -> Element {
    let _ds = DesignSystem::new();
    let _palette = Palette::dark();

    let now = get_current_epoch_secs();
    let local_secs = now + (state.tz_offset_mins as f32 * 60.0);

    Element::Column {
        children: vec![
            Element::Text {
                content: format!("🕒 {}", format_time(local_secs, state.use_24h)),
            },
            Element::Text {
                content: format!("⏱️ Stopwatch: {:.1}s", state.stopwatch_elapsed_secs),
            },
            Element::Row {
                children: vec![
                    Element::Button {
                        label: if state.use_24h { "12h" } else { "24h" }.into(),
                        on_click_id: 1,
                    },
                    Element::Button {
                        label: if state.stopwatch_running { "Pause" } else { "Start" }.into(),
                        on_click_id: 2,
                    },
                    Element::Button {
                        label: "Reset".into(),
                        on_click_id: 3,
                    },
                ],
            },
        ],
    }
}

fn main() {
    let app = App {
        state: ClockState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS Live Clock & Stopwatch");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time_24h_and_12h() {
        // 14:30:15 UTC -> 52215 seconds from start of day
        let secs = 14.0 * 3600.0 + 30.0 * 60.0 + 15.0;
        assert_eq!(format_time(secs, true), "14:30:15");
        assert_eq!(format_time(secs, false), "02:30:15 PM");

        // Midnight
        assert_eq!(format_time(0.0, true), "00:00:00");
        assert_eq!(format_time(0.0, false), "12:00:00 AM");
    }

    #[test]
    fn test_stopwatch_toggle_and_reset() {
        let mut state = ClockState::default();
        assert!(!state.stopwatch_running);

        update(&mut state, Ev::Click(2)); // Start
        assert!(state.stopwatch_running);
        assert!(state.stopwatch_start.is_some());

        update(&mut state, Ev::Click(2)); // Pause
        assert!(!state.stopwatch_running);
        assert!(state.stopwatch_start.is_none());

        update(&mut state, Ev::Click(3)); // Reset
        assert_eq!(state.stopwatch_elapsed_secs, 0.0);
    }
}
