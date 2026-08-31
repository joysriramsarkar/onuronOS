// runtime/nilui/src/testharness.rs — Headless UI Test Harness
use crate::{Element, Ev};

pub struct HeadlessTester<S> {
    pub state: S,
    pub update_fn: fn(&mut S, Ev),
    pub view_fn: fn(&S) -> Element,
}

impl<S> HeadlessTester<S> {
    pub fn new(initial: S, update: fn(&mut S, Ev), view: fn(&S) -> Element) -> Self {
        Self {
            state: initial,
            update_fn: update,
            view_fn: view,
        }
    }

    pub fn send_event(&mut self, ev: Ev) {
        (self.update_fn)(&mut self.state, ev);
    }

    pub fn render_tree(&self) -> Element {
        (self.view_fn)(&self.state)
    }
}
