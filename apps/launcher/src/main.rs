// apps/launcher/src/main.rs — NilOS Home Grid Launcher
use nilui::{App, Element, Ev};

#[derive(Default)]
struct State;

fn main() {
    let app = App {
        state: State,
        update: |_s, _e| {},
        view: |_s| Element::Text { content: "NilOS Home Grid Launcher".into() },
        on_snapshot: None,
        on_restore: None,
    };
    app.run("NilOS Home Grid Launcher");
}
