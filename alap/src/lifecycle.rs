// alap/src/lifecycle.rs — Application Lifecycle Engine for Alap
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleState {
    Uninitialized,
    Running,
    Paused,
    Disposed,
}

pub struct AppLifecycle {
    state: LifecycleState,
    init_hooks: Vec<Box<dyn FnMut() + Send + 'static>>,
    pause_hooks: Vec<Box<dyn FnMut() + Send + 'static>>,
    resume_hooks: Vec<Box<dyn FnMut() + Send + 'static>>,
    dispose_hooks: Vec<Box<dyn FnMut() + Send + 'static>>,
}

impl Default for AppLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

impl AppLifecycle {
    pub fn new() -> Self {
        Self {
            state: LifecycleState::Uninitialized,
            init_hooks: Vec::new(),
            pause_hooks: Vec::new(),
            resume_hooks: Vec::new(),
            dispose_hooks: Vec::new(),
        }
    }

    pub fn state(&self) -> LifecycleState {
        self.state
    }

    pub fn on_init<F: FnMut() + Send + 'static>(&mut self, f: F) {
        self.init_hooks.push(Box::new(f));
    }

    pub fn on_pause<F: FnMut() + Send + 'static>(&mut self, f: F) {
        self.pause_hooks.push(Box::new(f));
    }

    pub fn on_resume<F: FnMut() + Send + 'static>(&mut self, f: F) {
        self.resume_hooks.push(Box::new(f));
    }

    pub fn on_dispose<F: FnMut() + Send + 'static>(&mut self, f: F) {
        self.dispose_hooks.push(Box::new(f));
    }

    /// Trigger application startup (onInit). Idempotent if already running.
    pub fn start(&mut self) {
        if self.state == LifecycleState::Uninitialized {
            for hook in &mut self.init_hooks {
                hook();
            }
            self.state = LifecycleState::Running;
        }
    }

    /// Pause application (onPause).
    pub fn pause(&mut self) {
        if self.state == LifecycleState::Running {
            for hook in &mut self.pause_hooks {
                hook();
            }
            self.state = LifecycleState::Paused;
        }
    }

    /// Resume application (onResume).
    pub fn resume(&mut self) {
        if self.state == LifecycleState::Paused {
            for hook in &mut self.resume_hooks {
                hook();
            }
            self.state = LifecycleState::Running;
        }
    }

    /// Dispose and clean up application resources (onDispose).
    pub fn dispose(&mut self) {
        if self.state != LifecycleState::Disposed {
            for hook in &mut self.dispose_hooks {
                hook();
            }
            self.state = LifecycleState::Disposed;
        }
    }
}
