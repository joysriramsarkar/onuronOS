// alap/src/router.rs — Navigation & Router Stack for Alap
use crate::component::Component;
use std::collections::HashMap;

/// A defined screen destination in an Alap application.
#[derive(Clone)]
pub struct Screen {
    pub name: String,
    pub title: String,
    pub builder: std::rc::Rc<dyn Fn() -> Component>,
}

pub struct Router {
    registered: HashMap<String, Screen>,
    stack: Vec<String>,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    /// Maximum navigation stack depth to prevent unbounded memory growth and cyclic loops.
    pub const MAX_STACK_DEPTH: usize = 32;

    pub fn new() -> Self {
        Self {
            registered: HashMap::new(),
            stack: Vec::new(),
        }
    }

    /// Register a screen destination.
    pub fn register<F: Fn() -> Component + 'static>(&mut self, name: &str, title: &str, builder: F) {
        self.registered.insert(
            name.to_string(),
            Screen {
                name: name.to_string(),
                title: title.to_string(),
                builder: std::rc::Rc::new(builder),
            },
        );
    }

    /// Push a screen onto the navigation stack.
    pub fn push(&mut self, name: &str) -> Result<(), String> {
        if !self.registered.contains_key(name) {
            return Err(format!("Screen '{name}' is not registered"));
        }
        if self.stack.len() >= Self::MAX_STACK_DEPTH {
            return Err(format!(
                "Navigation stack depth exceeded maximum limit ({})",
                Self::MAX_STACK_DEPTH
            ));
        }
        self.stack.push(name.to_string());
        Ok(())
    }

    /// Pop the top screen from the navigation stack.
    pub fn pop(&mut self) -> Option<String> {
        if self.stack.len() > 1 {
            self.stack.pop()
        } else {
            None
        }
    }

    /// Replace the top screen on the navigation stack.
    pub fn replace(&mut self, name: &str) -> Result<(), String> {
        if self.registered.contains_key(name) {
            if !self.stack.is_empty() {
                self.stack.pop();
            }
            self.stack.push(name.to_string());
            Ok(())
        } else {
            Err(format!("Screen '{name}' is not registered"))
        }
    }

    /// Clear all screens above the root screen.
    pub fn clear_to_root(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.truncate(1);
            true
        } else {
            false
        }
    }

    /// Check if back navigation is possible.
    pub fn can_pop(&self) -> bool {
        self.stack.len() > 1
    }

    /// Current active screen name.
    pub fn current_screen_name(&self) -> Option<&str> {
        self.stack.last().map(|s| s.as_str())
    }

    /// Title of the currently active screen.
    pub fn current_title(&self) -> Option<&str> {
        let name = self.current_screen_name()?;
        self.registered.get(name).map(|s| s.title.as_str())
    }

    /// Render the current screen's component tree.
    pub fn render_current(&self) -> Option<Component> {
        let name = self.current_screen_name()?;
        let screen = self.registered.get(name)?;
        Some((screen.builder)())
    }

    /// Depth of navigation stack.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_overflow_protection() {
        let mut router = Router::new();
        router.register("test", "Test", || Component::spacer(0));

        for _ in 0..Router::MAX_STACK_DEPTH {
            assert!(router.push("test").is_ok());
        }
        assert_eq!(router.depth(), Router::MAX_STACK_DEPTH);

        // Pushing one more must fail
        let err = router.push("test").unwrap_err();
        assert!(err.contains("Navigation stack depth exceeded"));
    }

    #[test]
    fn test_clear_to_root_and_current_title() {
        let mut router = Router::new();
        router.register("home", "Home Page", || Component::text("Home"));
        router.register("detail", "Detail View", || Component::text("Detail"));

        assert_eq!(router.current_title(), None);

        router.push("home").unwrap();
        assert_eq!(router.current_title(), Some("Home Page"));

        router.push("detail").unwrap();
        assert_eq!(router.current_title(), Some("Detail View"));
        assert_eq!(router.depth(), 2);

        assert!(router.clear_to_root());
        assert_eq!(router.depth(), 1);
        assert_eq!(router.current_screen_name(), Some("home"));
        assert_eq!(router.current_title(), Some("Home Page"));

        // Already at root, cannot clear further
        assert!(!router.clear_to_root());
    }

    #[test]
    fn test_replace_screen() {
        let mut router = Router::new();
        router.register("a", "Screen A", || Component::text("A"));
        router.register("b", "Screen B", || Component::text("B"));

        router.push("a").unwrap();
        assert_eq!(router.current_screen_name(), Some("a"));

        router.replace("b").unwrap();
        assert_eq!(router.current_screen_name(), Some("b"));
        assert_eq!(router.depth(), 1);
    }
}
