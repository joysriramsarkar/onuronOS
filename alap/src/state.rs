// alap/src/state.rs — Reactive State & Binding Engine for Alap
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A unique identifier for a reactive state cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct StateId(pub u64);

/// Central store for reactive state variables within an Alap application.
#[derive(Default, Clone)]
pub struct StateStore {
    inner: Arc<Mutex<StateStoreInner>>,
}

#[derive(Default)]
struct StateStoreInner {
    next_id: u64,
    values: HashMap<StateId, serde_json::Value>,
    dirty: Vec<StateId>,
    invalidation_count: u64,
}

impl StateStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new managed state variable with an initial value.
    pub fn create_state<T: serde::Serialize>(&self, initial: T) -> State<T> {
        let mut inner = self.inner.lock().unwrap();
        inner.next_id += 1;
        let id = StateId(inner.next_id);
        let val = serde_json::to_value(initial).unwrap_or(serde_json::Value::Null);
        inner.values.insert(id, val);
        State {
            id,
            store: self.clone(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Read raw state value.
    pub fn get_raw(&self, id: StateId) -> Option<serde_json::Value> {
        let inner = self.inner.lock().unwrap();
        inner.values.get(&id).cloned()
    }

    /// Update raw state value and mark dirty.
    pub fn set_raw(&self, id: StateId, val: serde_json::Value) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(existing) = inner.values.get_mut(&id) {
            if *existing != val {
                *existing = val;
                inner.dirty.push(id);
                inner.invalidation_count += 1;
            }
        }
    }

    /// Take all dirty state changes.
    pub fn flush_dirty(&self) -> Vec<StateId> {
        let mut inner = self.inner.lock().unwrap();
        std::mem::take(&mut inner.dirty)
    }

    /// Check if any state is dirty.
    pub fn is_dirty(&self) -> bool {
        let inner = self.inner.lock().unwrap();
        !inner.dirty.is_empty()
    }

    pub fn invalidation_count(&self) -> u64 {
        let inner = self.inner.lock().unwrap();
        inner.invalidation_count
    }
}

/// A strongly typed reference to a reactive state variable.
pub struct State<T> {
    pub id: StateId,
    store: StateStore,
    _marker: std::marker::PhantomData<T>,
}

impl<T: serde::Serialize + serde::de::DeserializeOwned + Clone> State<T> {
    pub fn get(&self) -> Option<T> {
        let raw = self.store.get_raw(self.id)?;
        serde_json::from_value(raw).ok()
    }

    pub fn set(&self, value: T) {
        if let Ok(raw) = serde_json::to_value(value) {
            self.store.set_raw(self.id, raw);
        }
    }

    pub fn update<F: FnOnce(&mut T)>(&self, f: F) {
        if let Some(mut current) = self.get() {
            f(&mut current);
            self.set(current);
        }
    }
}

impl<T> Clone for State<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            store: self.store.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}
