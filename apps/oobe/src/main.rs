// apps/oobe/src/main.rs — NilOS First Boot Wizard
use nilui::{App, Element, Ev};

#[derive(Default)]
struct State;

fn main() {
    let app = App {
        state: State,
        update: |_s, _e| {},
        view: |_s| Element::Text { content: "NilOS First Boot Wizard".into() },
        on_snapshot: None,
        on_restore: None,
    };
    app.run("NilOS First Boot Wizard");
}
