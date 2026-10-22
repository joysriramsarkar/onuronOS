// apps/animdemo/src/main.rs — 120Hz Spring Animation & Fling Physics Engine
use nilui::{App, Element, Ev, FlingDecay, SpringConfig, SpringState};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimState {
    pub position: f32,
    pub velocity: f32,
    pub target: f32,
    pub ticks: u32,
}

impl Default for AnimState {
    fn default() -> Self {
        Self {
            position: 0.0,
            velocity: 0.0,
            target: 100.0,
            ticks: 0,
        }
    }
}

pub fn update(state: &mut AnimState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.target = if state.target > 50.0 { 0.0 } else { 100.0 };
        }
        Ev::DragEnd(vel) => {
            let mut fling = FlingDecay::new(vel as f32);
            state.target += fling.step(1.0 / 60.0);
        }
        Ev::Tick(dt) => {
            let cfg = SpringConfig {
                stiffness: 280.0,
                damping: 24.0,
                mass: 1.0,
            };
            let mut sp = SpringState {
                value: state.position,
                velocity: state.velocity,
                target: state.target,
            };
            sp.step(&cfg, dt.max(1.0 / 120.0));
            state.position = sp.value;
            state.velocity = sp.velocity;
            state.ticks += 1;
        }
        _ => {}
    }
}

pub fn view(state: &AnimState) -> Element {
    Element::Column {
        children: vec![
            Element::Text {
                content: format!(
                    "120Hz Spring Physics • Pos: {:.2} → Target: {:.1} (Tick #{})",
                    state.position, state.target, state.ticks
                ),
            },
            Element::Button {
                label: "Toggle Spring Target".into(),
                on_click_id: 1,
            },
        ],
    }
}

fn main() {
    let app = App {
        state: AnimState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS 120Hz Spring Physics");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spring_step_moves_toward_target() {
        let mut st = AnimState::default();
        assert_eq!(st.position, 0.0);
        for _ in 0..30 {
            update(&mut st, Ev::Tick(1.0 / 60.0));
        }
        assert!(st.position > 50.0);
    }
}
