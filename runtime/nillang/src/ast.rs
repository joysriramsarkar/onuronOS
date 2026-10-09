// runtime/nillang/src/ast.rs — AST definitions for NilLang
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub imports: Vec<ImportStmt>,
    pub structs: Vec<StructDef>,
    pub apps: Vec<AppDef>,
    pub components: Vec<ComponentDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportStmt {
    pub symbols: Vec<String>,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructField {
    pub name: String,
    pub field_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateVar {
    pub name: String,
    pub var_type: String,
    pub initial_value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UIProperty {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UIElement {
    pub tag: String,
    pub text_content: Option<String>,
    pub properties: Vec<UIProperty>,
    pub children: Vec<UIElement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppDef {
    pub name: String,
    pub state_vars: Vec<StateVar>,
    pub root_ui: Option<UIElement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentDef {
    pub name: String,
    pub props: Vec<StructField>,
    pub root_ui: Option<UIElement>,
}
