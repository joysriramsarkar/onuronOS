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
    pub stopwatch_elapsed_secs: f32,
}

impl Default for ClockState {
    fn default() -> Self {
        Self {
            use_24h: true,
            tz_offset_mins: 360,
            stopwatch_running: false,
            stopwatch_elapsed_secs: 0.0,
        }
    }
}

pub fn update(state: &mut ClockState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.use_24h = !state.use_24h;
        }
        Ev::Click(2) => {
            state.stopwatch_running = !state.stopwatch_running;
            if state.stopwatch_running {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs_f32())
                    .unwrap_or(0.0);
                state.stopwatch_elapsed_secs = now;
            }
        }
        Ev::Tick(_) => {
            if state.stopwatch_running {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs_f32())
                    .unwrap_or(0.0);
                state.stopwatch_elapsed_secs = now;
            }
        }
        _ => {}
    }
}

fn format_time(seconds: f32, use_24h: bool) -> String {
    let total_secs = seconds as u64;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if use_24h {
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        let h12 = if hours > 12 { hours - 12 } else { hours };
        let meridiem = if hours < 12 { "AM" } else { "PM" };
        format!("{:02}:{:02}:{:02} {}", h12, mins, secs, meridiem)
    }
}

pub fn view(state: &ClockState) -> Element {
    let ds = DesignSystem::new();
    let palette = Palette::dark();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f32())
        .unwrap_or(0.0);

    Element::Column {
        children: vec![
            Element::Text {
                content: format_time(now + state.tz_offset_mins as f32 * 3600.0, state.use_24h),
            },
            Element::Text {
                content: format!("Stopwatch: {}s", state.stopwatch_elapsed_secs as u64),
            },
            Element::Row {
                children: vec![
                    Element::Button {
                        label: "24h".into(),
                        on_click_id: 1,
                    },
                    Element::Button {
                        label: "Stopwatch".into(),
                        on_click_id: 2,
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
    app.run("OnuronOS Clock Widget");
}
