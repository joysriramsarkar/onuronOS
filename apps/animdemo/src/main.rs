// apps/animdemo/src/main.rs — 120Hz Spring Animation & Fling Physics Engine
// Using the official OnuronOS Design System (dark/light themes)
use nilui::{App, Element, Ev, FlingDecay, SpringConfig, SpringState};
use nilui::design::{DesignSystem, Palette};
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
        Ev::Click(id) if id == 1 => {
            state.target = if state.target >= 100.0 { 0.0 } else { 100.0 };
            state.velocity = 0.0;
            state.ticks = 0;
        }
        Ev::Tick(_) => {
            state.ticks += 1;
            if state.target != state.position {
                let dt = 1.0 / 120.0;
                let displacement = state.position - state.target;
                let spring_force = -220.0 * displacement;
                let damping_force = -15.0 * state.velocity;
                let acceleration = (spring_force + damping_force) / 1.0;
                state.velocity += acceleration * dt;
                state.position += state.velocity * dt;
                if (displacement).abs() < 0.001 && state.velocity.abs() < 0.001 {
                    state.position = state.target;
                    state.velocity = 0.0;
                }
            }
        }
        _ => {}
    }
}

pub fn view(state: &AnimState) -> Element {
    let ds = DesignSystem::new();
    let palette = Palette::dark();

    Element::Column {
        children: vec![
            Element::Text {
                content: format!("Animated Spring: {:.2}", state.position),
            },
            Element::Button {
                label: if state.target >= 100.0 { "◀ Animate Left" } else { "Animate Right ▶" }.into(),
                on_click_id: 1,
            },
            Element::Text {
                content: format!("Ticks: {}", state.ticks),
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
    app.run("OnuronOS 120Hz Spring Animation Engine");
}
