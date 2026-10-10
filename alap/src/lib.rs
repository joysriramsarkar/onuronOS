// alap/src/lib.rs — Alap (আলাপ) Declarative Native Mobile Framework for OnuronOS
pub mod component;
pub mod state;
pub mod lifecycle;
pub mod router;
pub mod services;
pub mod runtime;

pub use component::{Alignment, Component, ScrollDirection};

pub mod prelude {
    pub use crate::component::{Alignment, Component, ScrollDirection};
    pub use crate::state::{State, StateId, StateStore};
    pub use crate::lifecycle::{AppLifecycle, LifecycleState};
    pub use crate::router::{Router, Screen};
    pub use crate::runtime::AlapApp;
    pub use crate::services::{ServiceError, ServiceResult, ServiceStatus};
}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_reactive_state_and_invalidation() {
        let store = StateStore::new();
        let count_state = store.create_state(0i32);

        assert_eq!(count_state.get(), Some(0));
        assert_eq!(store.invalidation_count(), 0);

        count_state.set(42);
        assert_eq!(count_state.get(), Some(42));
        assert_eq!(store.invalidation_count(), 1);
        assert!(store.is_dirty());

        let dirty = store.flush_dirty();
        assert_eq!(dirty, vec![count_state.id]);
        assert!(!store.is_dirty());

        count_state.update(|c| *c += 1);
        assert_eq!(count_state.get(), Some(43));
        assert_eq!(store.invalidation_count(), 2);
    }

    #[test]
    fn test_component_graph_structure() {
        let col = Component::column(vec![
            Component::text("Welcome to OnuronOS"),
            Component::spacer(16),
            Component::row(vec![
                Component::button("btn_ok", "Confirm"),
                Component::button("btn_cancel", "Cancel"),
            ]),
        ]);

        // Column (1) + Text (1) + Spacer (1) + Row (1) + Button (1) + Button (1) = 6 nodes
        assert_eq!(col.node_count(), 6);

        // Serialize to JSON and ensure round-trip parity
        let json = serde_json::to_string(&col).expect("Serialization failed");
        let deserialized: Component = serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(col, deserialized);
    }

    #[test]
    fn test_app_lifecycle_transitions() {
        let mut lifecycle = AppLifecycle::new();
        assert_eq!(lifecycle.state(), LifecycleState::Uninitialized);

        let inited = Arc::new(AtomicBool::new(false));
        let inited_clone = Arc::clone(&inited);
        lifecycle.on_init(move || {
            inited_clone.store(true, Ordering::SeqCst);
        });

        lifecycle.start();
        assert_eq!(lifecycle.state(), LifecycleState::Running);
        assert!(inited.load(Ordering::SeqCst));

        // Start again is idempotent
        lifecycle.start();
        assert_eq!(lifecycle.state(), LifecycleState::Running);

        lifecycle.pause();
        assert_eq!(lifecycle.state(), LifecycleState::Paused);

        lifecycle.resume();
        assert_eq!(lifecycle.state(), LifecycleState::Running);

        lifecycle.dispose();
        assert_eq!(lifecycle.state(), LifecycleState::Disposed);
    }

    #[test]
    fn test_router_navigation_stack() {
        let mut router = Router::new();
        router.register("home", "Home Screen", || Component::text("Home Content"));
        router.register("settings", "Settings Screen", || Component::text("Settings Content"));

        assert_eq!(router.depth(), 0);
        assert!(!router.can_pop());

        router.push("home").unwrap();
        assert_eq!(router.depth(), 1);
        assert_eq!(router.current_screen_name(), Some("home"));
        assert!(!router.can_pop());

        router.push("settings").unwrap();
        assert_eq!(router.depth(), 2);
        assert_eq!(router.current_screen_name(), Some("settings"));
        assert!(router.can_pop());

        let popped = router.pop();
        assert_eq!(popped, Some("settings".to_string()));
        assert_eq!(router.current_screen_name(), Some("home"));
        assert_eq!(router.depth(), 1);

        // Cannot pop the root screen
        assert_eq!(router.pop(), None);
    }

    #[test]
    fn test_runtime_app_event_dispatch() {
        let mut app = AlapApp::new("DemoApp");
        let clicked = Arc::new(AtomicBool::new(false));
        let clicked_clone = Arc::clone(&clicked);

        app.register_handler("click_me", move || {
            clicked_clone.store(true, Ordering::SeqCst);
        });

        assert!(!app.dispatch_event("nonexistent"));
        assert!(!clicked.load(Ordering::SeqCst));

        assert!(app.dispatch_event("click_me"));
        assert!(clicked.load(Ordering::SeqCst));
    }
}
