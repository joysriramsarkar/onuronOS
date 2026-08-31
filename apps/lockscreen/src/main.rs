// apps/lockscreen/src/main.rs — NilOS Swipe-to-Unlock Lockscreen
use nilui::{App, Element, Ev};

#[derive(Default)]
struct State;

fn main() {
    let app = App {
        state: State,
        update: |_s, _e| {},
        view: |_s| Element::Text { content: "NilOS Swipe-to-Unlock Lockscreen".into() },
        on_snapshot: None,
        on_restore: None,
    };
    app.run("NilOS Swipe-to-Unlock Lockscreen");
}
