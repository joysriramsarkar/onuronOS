// apps/busdemo/src/main.rs — SoftBus Cross-Device Demo
use nilui::{App, Element, Ev};

#[derive(Default)]
struct State;

fn main() {
    let app = App {
        state: State,
        update: |_s, _e| {},
        view: |_s| Element::Text { content: "SoftBus Cross-Device Demo".into() },
        on_snapshot: None,
        on_restore: None,
    };
    app.run("SoftBus Cross-Device Demo");
}
