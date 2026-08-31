// apps/camdemo/src/main.rs — Camera Preview Demo
use nilui::{App, Element, Ev};

#[derive(Default)]
struct State;

fn main() {
    let app = App {
        state: State,
        update: |_s, _e| {},
        view: |_s| Element::Text { content: "Camera Preview Demo".into() },
        on_snapshot: None,
        on_restore: None,
    };
    app.run("Camera Preview Demo");
}
