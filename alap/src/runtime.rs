// alap/src/runtime.rs — Application Runtime Engine for Alap
use crate::component::Component;
use crate::lifecycle::AppLifecycle;
use crate::router::Router;
use crate::state::StateStore;
use std::collections::HashMap;

pub type EventHandler = Box<dyn FnMut() + Send + 'static>;

/// Represents an active Alap application execution instance.
pub struct AlapApp {
    pub name: String,
    pub state_store: StateStore,
    pub lifecycle: AppLifecycle,
    pub router: Router,
    handlers: HashMap<String, EventHandler>,
}

impl AlapApp {
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            state_store: StateStore::new(),
            lifecycle: AppLifecycle::new(),
            router: Router::new(),
            handlers: HashMap::new(),
        }
    }

    /// Register a named event handler for UI buttons/switches.
    pub fn register_handler<S: Into<String>, F: FnMut() + Send + 'static>(&mut self, id: S, handler: F) {
        self.handlers.insert(id.into(), Box::new(handler));
    }

    /// Dispatch a UI event by ID (e.g., button press). Returns true if a handler executed.
    pub fn dispatch_event(&mut self, id: &str) -> bool {
        if let Some(handler) = self.handlers.get_mut(id) {
            handler();
            true
        } else {
            false
        }
    }

    /// Render the current screen from the router.
    pub fn render(&self) -> Option<Component> {
        self.router.render_current()
    }
}
