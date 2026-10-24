// apps/hello/src/main.rs — First Native NilOS App (Stateful Counter with Handoff)
// Using the official OnuronOS Design System (dark/light themes)
use nilui::design::{DesignSystem, Palette};
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
pub struct State {
    pub count: u32,
    pub message: String,
}

pub fn update(state: &mut State, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.count += 1;
            state.message = format!("NilOS Button Tapped {} times!", state.count);
        }
        _ => {}
    }
}

pub fn view(state: &State) -> Element {
    let _ds = DesignSystem::new();
    let _palette = Palette::dark();

    Element::Column {
        children: vec![
            Element::Text {
                content: "Welcome to OnuronOS — Clean, Fast & Open".into(),
            },
            Element::Text {
                content: state.message.clone(),
            },
            Element::Button {
                label: "Tap Me (120Hz Spring Physics)".into(),
                on_click_id: 1,
            },
        ],
    }
}

fn main() {
    let initial_state = State {
        count: 0,
        message: "Hello World from OnuronOS!".into(),
    };

    let app = App {
        state: initial_state,
        update,
        view,
        on_snapshot: Some(|s| serde_json::to_vec(s).unwrap_or_default()),
        on_restore: Some(|s, data| {
            if let Ok(restored) = serde_json::from_slice::<State>(data) {
                *s = restored;
            }
        }),
    };
    app.run("OnuronOS Hello App");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_counter_increment() {
        let mut state = State {
            count: 0,
            message: "Initial".into(),
        };
        update(&mut state, Ev::Click(1));
        assert_eq!(state.count, 1);
        assert!(state.message.contains("1 times"));
    }
}
