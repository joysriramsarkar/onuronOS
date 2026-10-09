// runtime/nillang/src/vm.rs — Execution Virtual Machine for NilLang
use std::collections::HashMap;
use crate::ast::{AppDef, UIElement};
use crate::bytecode::CompiledPackage;

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

    pub fn render_scene(&self) -> Result<String, String> {
        let app = self.primary_app().ok_or("No app definition found in package")?;
        let root = app.root_ui.as_ref().ok_or("No root UI element defined")?;
        let mut out = String::new();
        self.render_element(root, 0, &mut out);
        Ok(out)
    }

    fn render_element(&self, elem: &UIElement, indent: usize, out: &mut String) {
        let pad = "  ".repeat(indent);
        out.push_str(&format!("{pad}<{}", elem.tag));
        if let Some(text) = &elem.text_content {
            out.push_str(&format!(" text=\"{text}\""));
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
                self.render_element(child, indent + 1, out);
            }
            out.push_str(&format!("{pad}</{}>\n", elem.tag));
        }
    }
}
