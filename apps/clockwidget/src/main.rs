// apps/clockwidget/src/main.rs — Desktop Clock Widget
use nilui::{App, Element, Ev};

#[derive(Default)]
struct State;

fn main() {
    let app = App {
        state: State,
        update: |_s, _e| {},
        view: |_s| Element::Text { content: "Desktop Clock Widget".into() },
        on_snapshot: None,
        on_restore: None,
    };
    app.run("Desktop Clock Widget");
}
