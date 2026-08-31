// apps/animdemo/src/main.rs — 120Hz Animation Physics Demo
use nilui::{App, Element, Ev};

#[derive(Default)]
struct State;

fn main() {
    let app = App {
        state: State,
        update: |_s, _e| {},
        view: |_s| Element::Text { content: "120Hz Animation Physics Demo".into() },
        on_snapshot: None,
        on_restore: None,
    };
    app.run("120Hz Animation Physics Demo");
}
