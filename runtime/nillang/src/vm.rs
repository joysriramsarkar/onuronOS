// runtime/nillang/src/vm.rs — Execution Virtual Machine for NilLang
use std::collections::HashMap;
use crate::ast::{AppDef, UIElement};
use crate::bytecode::CompiledPackage;

pub const MAX_UI_RECURSION_DEPTH: usize = 64;

pub struct NilVM {
    pub package: CompiledPackage,
    pub state: HashMap<String, String>,
}

impl NilVM {
    pub fn new(package: CompiledPackage) -> Self {
        let mut state = HashMap::new();
        if let Some(app) = package.program.apps.first() {
            for v in &app.state_vars {
                state.insert(v.name.clone(), v.initial_value.clone());
            }
        }
        Self { package, state }
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

    pub fn render_scene(&self) -> Result<String, String> {
        let app = self.primary_app().ok_or("No app definition found in package")?;
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
        let app = self.primary_app().ok_or("No app definition found in package")?;
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
            elem.properties
                .iter()
                .find(|p| p.name == name)
                .map(|p| {
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
                let id = resolve_prop("id").unwrap_or_else(|| "btn".into());
                let label = elem
                    .text_content
                    .clone()
                    .or_else(|| resolve_prop("label"))
                    .or_else(|| resolve_prop("text"))
                    .unwrap_or_default();
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

        let comp = vm.to_alap_component().expect("Alap conversion should succeed");
        assert_eq!(comp.node_count(), 4); // Column + Text + Text + Button

        if let alap::Component::Column { children, spacing, .. } = comp {
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
