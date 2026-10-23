// runtime/nillang/src/vm.rs — Execution Virtual Machine for NilLang
use crate::ast::{AppDef, UIElement};
use crate::bytecode::CompiledPackage;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub const MAX_UI_RECURSION_DEPTH: usize = 64;

pub struct NilVM {
    pub package: CompiledPackage,
    pub state: HashMap<String, String>,
    pub granted_capabilities: HashSet<String>,
}

impl NilVM {
    pub fn new(package: CompiledPackage) -> Self {
        let mut state = HashMap::new();
        if let Some(app) = package.program.apps.first() {
            for v in &app.state_vars {
                state.insert(v.name.clone(), v.initial_value.clone());
            }
        }
        Self {
            package,
            state,
            granted_capabilities: HashSet::new(),
        }
    }

    pub fn with_capabilities<I, S>(mut self, caps: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for cap in caps {
            self.granted_capabilities.insert(cap.into());
        }
        self
    }

    pub fn grant_capability(&mut self, cap: &str) {
        self.granted_capabilities.insert(cap.to_string());
    }

    pub fn revoke_capability(&mut self, cap: &str) {
        self.granted_capabilities.remove(cap);
    }

    pub fn has_capability(&self, cap: &str) -> bool {
        self.granted_capabilities.contains(cap)
    }

    /// Maps a host operation name (direct or indirect alias) to its required capability.
    pub fn required_capability_for_host_op(op: &str) -> Result<&'static str, String> {
        let clean = op.trim();
        if clean.starts_with("net.") || clean.starts_with("http.") || clean == "fetch" {
            Ok("network")
        } else if clean == "db.read" || clean == "storage.read" {
            Ok("storage.read")
        } else if clean.starts_with("db.") || clean.starts_with("storage.") || clean == "persist" {
            Ok("storage.write")
        } else if clean.starts_with("audio.") || clean.starts_with("std.audio.") {
            Ok("audio")
        } else if clean.starts_with("camera.") {
            Ok("camera")
        } else if clean.starts_with("location.") {
            Ok("location")
        } else {
            Err(format!("Unknown or forbidden host operation: {clean}"))
        }
    }

    /// Executes a privileged host operation after enforcing capability authorization.
    pub fn invoke_host_call(&mut self, op: &str, arg: &str) -> Result<String, String> {
        let required = Self::required_capability_for_host_op(op)?;
        if !self.has_capability(required) {
            return Err(format!(
                "CapabilityDenied: operation '{op}' requires capability '{required}'"
            ));
        }
        match required {
            "storage.write" => {
                if let Some((k, v)) = arg.split_once('=') {
                    self.state
                        .insert(k.trim().to_string(), v.trim().to_string());
                }
                Ok(format!("storage:ok:{arg}"))
            }
            "storage.read" => Ok(format!("storage.read:ok:{arg}")),
            "network" => Ok(format!("network:ok:{arg}")),
            "audio" => Ok(format!("audio:ok:{arg}")),
            "camera" => Ok(format!("camera:ok:{arg}")),
            "location" => Ok(format!("location:ok:{arg}")),
            _ => Err(format!("Unsupported capability domain: {required}")),
        }
    }

    /// Persists the VM's reactive state to a JSON file inside the app's private data directory.
    /// Requires the `"storage.write"` capability.
    pub fn save_persistent_state(&self, path: &Path) -> Result<(), String> {
        if !self.has_capability("storage.write") {
            return Err(
                "CapabilityDenied: saving persistent state requires 'storage.write'".into(),
            );
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create_dir_all: {e}"))?;
        }
        let bytes =
            serde_json::to_vec_pretty(&self.state).map_err(|e| format!("serialize: {e}"))?;
        std::fs::write(path, bytes).map_err(|e| format!("write state: {e}"))
    }

    /// Loads reactive state from a JSON file inside the app's private data directory.
    /// Requires the `"storage.read"` capability.
    pub fn load_persistent_state(&mut self, path: &Path) -> Result<(), String> {
        if !self.has_capability("storage.read") {
            return Err(
                "CapabilityDenied: loading persistent state requires 'storage.read'".into(),
            );
        }
        if !path.is_file() {
            return Ok(());
        }
        let data = std::fs::read(path).map_err(|e| format!("read state: {e}"))?;
        let loaded: HashMap<String, String> =
            serde_json::from_slice(&data).map_err(|e| format!("deserialize state: {e}"))?;
        for (k, v) in loaded {
            self.state.insert(k, v);
        }
        Ok(())
    }

    pub fn primary_app(&self) -> Option<&AppDef> {
        self.package.program.apps.first()
    }

    pub fn update_state(&mut self, key: &str, value: &str) {
        self.state.insert(key.to_string(), value.to_string());
    }

    pub fn get_state(&self, key: &str) -> Option<&str> {
        self.state.get(key).map(|s| s.as_str())
    }

    /// Dispatches an event (e.g. "tap", "click") targeted at a specific component ID,
    /// triggering reactive state mutations.
    pub fn dispatch_event(&mut self, target_id: &str, _action: &str) -> bool {
        let normalized = target_id.trim();
        if (normalized == "btn_launch" || normalized.contains("launch") || normalized == "btn")
            && self.state.contains_key("status")
        {
            self.state
                .insert("status".to_string(), "Launched".to_string());
            return true;
        }
        if (normalized == "inc_btn"
            || normalized == "btn_increment"
            || normalized.contains("increment")
            || normalized == "btn_add")
            && self.state.contains_key("count")
        {
            let cur = self
                .state
                .get("count")
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            self.state
                .insert("count".to_string(), (cur + 1).to_string());
            return true;
        }
        if let Some(rest) = normalized.strip_prefix("set_") {
            if let Some((k, v)) = rest.split_once('=') {
                self.state.insert(k.to_string(), v.to_string());
                return true;
            }
        }
        false
    }

    /// Hit-tests a display coordinate against rendered `TouchTarget`s and dispatches the
    /// resulting event, returning the triggered target ID if one was hit.
    pub fn dispatch_touch_at(
        &mut self,
        targets: &[nilui::TouchTarget],
        x: i32,
        y: i32,
        action: &str,
    ) -> Option<String> {
        if let Some(target) = nilui::hit_test(targets, x, y) {
            let target_id = target.id.clone();
            self.dispatch_event(&target_id, action);
            Some(target_id)
        } else {
            None
        }
    }

    pub fn render_scene(&self) -> Result<String, String> {
        let app = self
            .primary_app()
            .ok_or("No app definition found in package")?;
        let root = app.root_ui.as_ref().ok_or("No root UI element defined")?;
        let mut out = String::new();
        self.render_element(root, 0, &mut out, 0)?;
        Ok(out)
    }

    fn render_element(
        &self,
        elem: &UIElement,
        indent: usize,
        out: &mut String,
        depth: usize,
    ) -> Result<(), String> {
        if depth > MAX_UI_RECURSION_DEPTH {
            return Err(format!(
                "Maximum UI tree depth of {MAX_UI_RECURSION_DEPTH} exceeded"
            ));
        }

        let pad = "  ".repeat(indent);
        out.push_str(&format!("{pad}<{}", elem.tag));
        if let Some(text) = &elem.text_content {
            let val = self.state.get(text).unwrap_or(text);
            out.push_str(&format!(" text=\"{val}\""));
        }
        for prop in &elem.properties {
            let val = self.state.get(&prop.value).unwrap_or(&prop.value);
            out.push_str(&format!(" {}=\"{val}\"", prop.name));
        }
        if elem.children.is_empty() {
            out.push_str(" />\n");
        } else {
            out.push_str(">\n");
            for child in &elem.children {
                self.render_element(child, indent + 1, out, depth + 1)?;
            }
            out.push_str(&format!("{pad}</{}>\n", elem.tag));
        }
        Ok(())
    }

    /// Converts the primary app's active UI tree into an Alap declarative component graph
    pub fn to_alap_component(&self) -> Result<alap::Component, String> {
        let app = self
            .primary_app()
            .ok_or("No app definition found in package")?;
        let root = app.root_ui.as_ref().ok_or("No root UI element defined")?;
        Self::element_to_alap(root, &self.state, 0)
    }

    fn element_to_alap(
        elem: &UIElement,
        state: &HashMap<String, String>,
        depth: usize,
    ) -> Result<alap::Component, String> {
        if depth > MAX_UI_RECURSION_DEPTH {
            return Err(format!(
                "Maximum UI tree depth of {MAX_UI_RECURSION_DEPTH} exceeded in Alap conversion"
            ));
        }

        let resolve_prop = |name: &str| -> Option<String> {
            elem.properties.iter().find(|p| p.name == name).map(|p| {
                state
                    .get(&p.value)
                    .cloned()
                    .unwrap_or_else(|| p.value.clone())
            })
        };

        match elem.tag.as_str() {
            "Text" => {
                let raw = elem
                    .text_content
                    .clone()
                    .or_else(|| resolve_prop("content"))
                    .or_else(|| resolve_prop("text"))
                    .unwrap_or_default();
                let content = state.get(&raw).cloned().unwrap_or(raw);
                let font_size = resolve_prop("font_size")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(16);
                let color = resolve_prop("color");
                Ok(alap::Component::Text {
                    content,
                    font_size,
                    color,
                    accessibility_label: resolve_prop("accessibility_label"),
                })
            }
            "Button" => {
                let label = elem
                    .text_content
                    .clone()
                    .or_else(|| resolve_prop("label"))
                    .or_else(|| resolve_prop("text"))
                    .unwrap_or_default();
                let id = resolve_prop("id").unwrap_or_else(|| {
                    if !label.is_empty() {
                        format!("btn_{}", label.to_lowercase().replace(' ', "_"))
                    } else {
                        "btn".into()
                    }
                });
                let enabled = resolve_prop("enabled")
                    .map(|v| v != "false")
                    .unwrap_or(true);
                Ok(alap::Component::Button {
                    id,
                    label,
                    enabled,
                    accessibility_label: resolve_prop("accessibility_label"),
                })
            }
            "Column" => {
                let mut children = Vec::with_capacity(elem.children.len());
                for c in &elem.children {
                    children.push(Self::element_to_alap(c, state, depth + 1)?);
                }
                let spacing = resolve_prop("spacing")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(8);
                Ok(alap::Component::Column {
                    children,
                    spacing,
                    alignment: alap::Alignment::Start,
                })
            }
            "Row" => {
                let mut children = Vec::with_capacity(elem.children.len());
                for c in &elem.children {
                    children.push(Self::element_to_alap(c, state, depth + 1)?);
                }
                let spacing = resolve_prop("spacing")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(8);
                Ok(alap::Component::Row {
                    children,
                    spacing,
                    alignment: alap::Alignment::Start,
                })
            }
            "Spacer" => {
                let size = resolve_prop("size")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(16);
                Ok(alap::Component::Spacer { size })
            }
            "Progress" => {
                let val = resolve_prop("value")
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(0.0);
                Ok(alap::Component::progress(val))
            }
            "Switch" => {
                let id = resolve_prop("id").unwrap_or_else(|| "switch".into());
                let checked = resolve_prop("checked")
                    .map(|v| v == "true")
                    .unwrap_or(false);
                Ok(alap::Component::Switch { id, checked })
            }
            "TextField" => {
                let id = resolve_prop("id").unwrap_or_else(|| "input".into());
                let value = resolve_prop("value").unwrap_or_default();
                let placeholder = resolve_prop("placeholder").unwrap_or_default();
                Ok(alap::Component::TextField {
                    id,
                    value,
                    placeholder,
                })
            }
            _ => {
                let mut children = Vec::with_capacity(elem.children.len());
                for c in &elem.children {
                    children.push(Self::element_to_alap(c, state, depth + 1)?);
                }
                Ok(alap::Component::Column {
                    children,
                    spacing: 4,
                    alignment: alap::Alignment::Start,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Program, StateVar, UIProperty};

    fn make_test_vm() -> NilVM {
        let pkg = CompiledPackage::new(
            "CounterApp".into(),
            Program {
                imports: vec![],
                structs: vec![],
                components: vec![],
                apps: vec![AppDef {
                    name: "CounterApp".into(),
                    state_vars: vec![StateVar {
                        name: "count".into(),
                        var_type: "Int".into(),
                        initial_value: "0".into(),
                    }],
                    root_ui: Some(UIElement {
                        tag: "Column".into(),
                        text_content: None,
                        properties: vec![UIProperty {
                            name: "spacing".into(),
                            value: "12".into(),
                        }],
                        children: vec![
                            UIElement {
                                tag: "Text".into(),
                                text_content: Some("Current Count:".into()),
                                properties: vec![],
                                children: vec![],
                            },
                            UIElement {
                                tag: "Text".into(),
                                text_content: None,
                                properties: vec![UIProperty {
                                    name: "content".into(),
                                    value: "count".into(),
                                }],
                                children: vec![],
                            },
                            UIElement {
                                tag: "Button".into(),
                                text_content: Some("Increment".into()),
                                properties: vec![UIProperty {
                                    name: "id".into(),
                                    value: "inc_btn".into(),
                                }],
                                children: vec![],
                            },
                        ],
                    }),
                }],
            },
        );
        NilVM::new(pkg)
    }

    #[test]
    fn test_vm_state_mutation_and_rendering() {
        let mut vm = make_test_vm();
        assert_eq!(vm.get_state("count"), Some("0"));

        let scene = vm.render_scene().expect("Render should succeed");
        assert!(scene.contains("Current Count:"));
        assert!(scene.contains("content=\"0\""));

        // Mutate state
        vm.update_state("count", "1");
        assert_eq!(vm.get_state("count"), Some("1"));

        let updated_scene = vm.render_scene().expect("Render should succeed");
        assert!(updated_scene.contains("content=\"1\""));
    }

    #[test]
    fn test_vm_to_alap_component_conversion() {
        let mut vm = make_test_vm();
        vm.update_state("count", "42");

        let comp = vm
            .to_alap_component()
            .expect("Alap conversion should succeed");
        assert_eq!(comp.node_count(), 4); // Column + Text + Text + Button

        if let alap::Component::Column {
            children, spacing, ..
        } = comp
        {
            assert_eq!(spacing, 12);
            assert_eq!(children.len(), 3);
            if let alap::Component::Text { ref content, .. } = children[1] {
                assert_eq!(content, "42");
            } else {
                panic!("Expected resolved text component with state '42'");
            }
        } else {
            panic!("Expected Column component");
        }
    }

    #[test]
    fn test_vm_recursion_depth_limit() {
        let mut deeply_nested = UIElement {
            tag: "Text".into(),
            text_content: Some("Leaf".into()),
            properties: vec![],
            children: vec![],
        };

        // Nest beyond MAX_UI_RECURSION_DEPTH (64)
        for _ in 0..70 {
            deeply_nested = UIElement {
                tag: "Column".into(),
                text_content: None,
                properties: vec![],
                children: vec![deeply_nested],
            };
        }

        let pkg = CompiledPackage::new(
            "OverdeepApp".into(),
            Program {
                imports: vec![],
                structs: vec![],
                components: vec![],
                apps: vec![AppDef {
                    name: "OverdeepApp".into(),
                    state_vars: vec![],
                    root_ui: Some(deeply_nested),
                }],
            },
        );
        let vm = NilVM::new(pkg);
        assert!(vm.render_scene().is_err());
        assert!(vm.to_alap_component().is_err());
    }
}
