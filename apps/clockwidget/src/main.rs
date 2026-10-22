// apps/clockwidget/src/main.rs — OnuronOS Live Clock & Stopwatch Widget
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClockState {
    pub use_24h: bool,
    pub tz_offset_mins: i64,
    pub stopwatch_running: bool,
    pub stopwatch_elapsed_secs: f32,
}

impl Default for ClockState {
    fn default() -> Self {
        Self {
            use_24h: true,
            tz_offset_mins: 360, // UTC+6 Dhaka
            stopwatch_running: false,
            stopwatch_elapsed_secs: 0.0,
        }
    }
}

pub fn format_epoch_clock(epoch_secs: u64, tz_offset_mins: i64, use_24h: bool) -> String {
    let local = (epoch_secs as i64 + tz_offset_mins * 60).max(0) as u64;
    let h24 = ((local / 3600) % 24) as u8;
    let m = ((local / 60) % 60) as u8;
    let s = (local % 60) as u8;
    if use_24h {
        format!("{:02}:{:02}:{:02}", h24, m, s)
    } else {
        let am_pm = if h24 >= 12 { "PM" } else { "AM" };
        let h12 = match h24 % 12 {
            0 => 12,
            v => v,
        };
        format!("{:02}:{:02}:{:02} {}", h12, m, s, am_pm)
    }
}

pub fn update(state: &mut ClockState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.use_24h = !state.use_24h;
        }
        Ev::Click(2) => {
            state.stopwatch_running = !state.stopwatch_running;
        }
        Ev::Click(3) => {
            state.stopwatch_running = false;
            state.stopwatch_elapsed_secs = 0.0;
        }
        Ev::Tick(dt) => {
            if state.stopwatch_running && dt > 0.0 {
                state.stopwatch_elapsed_secs += dt;
            }
        }
        _ => {}
    }
}

pub fn view(state: &ClockState) -> Element {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let clock_str = format_epoch_clock(now, state.tz_offset_mins, state.use_24h);
    Element::Column {
        children: vec![
            Element::Text {
                content: format!("🕒 {}", clock_str),
            },
            Element::Text {
                content: format!("Stopwatch: {:.2}s", state.stopwatch_elapsed_secs),
            },
            Element::Row {
                children: vec![
                    Element::Button {
                        label: if state.use_24h {
                            "12h Mode".into()
                        } else {
                            "24h Mode".into()
                        },
                        on_click_id: 1,
                    },
                    Element::Button {
                        label: if state.stopwatch_running {
                            "Pause".into()
                        } else {
                            "Start".into()
                        },
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
        on_snapshot: Some(|s| serde_json::to_vec(s).unwrap_or_default()),
        on_restore: None,
    };
    app.run("OnuronOS Clock & Stopwatch Widget");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_formatting_and_stopwatch() {
        assert_eq!(format_epoch_clock(0, 0, true), "00:00:00");
        assert_eq!(format_epoch_clock(0, 0, false), "12:00:00 AM");
        assert_eq!(
            format_epoch_clock(13 * 3600 + 5 * 60, 0, false),
            "01:05:00 PM"
        );

        let mut st = ClockState::default();
        update(&mut st, Ev::Click(2));
        assert!(st.stopwatch_running);
        update(&mut st, Ev::Tick(0.5));
        assert!((st.stopwatch_elapsed_secs - 0.5).abs() < 1e-5);
        update(&mut st, Ev::Click(3));
        assert!(!st.stopwatch_running);
        assert_eq!(st.stopwatch_elapsed_secs, 0.0);
    }
}
