// runtime/nillang/src/lib.rs — Onuron OS Native NilLang Platform
pub mod ast;
pub mod bytecode;
pub mod parser;
pub mod vm;

pub use ast::*;
pub use bytecode::CompiledPackage;
pub use parser::Parser;
pub use vm::NilVM;

/// Compiles `.nil` source code into portable binary bytecode.
pub fn compile_source(source: &str, app_name: &str) -> Result<Vec<u8>, String> {
    let mut parser = Parser::new(source);
    let program = parser.parse()?;
    let package = CompiledPackage::new(app_name.to_string(), program);
    package.serialize()
}

/// Loads and prepares a compiled bytecode package for execution.
pub fn load_package(bytes: &[u8]) -> Result<NilVM, String> {
    let package = CompiledPackage::deserialize(bytes)?;
    Ok(NilVM::new(package))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_NIL: &str = r#"
import { Text, Column, Button } from "nil/ui"

struct Bookmark {
    id: string
    url: string
}

app NotesApp {
    @State title: string = "My Quick Notes"
    @State count: string = "5"

    build() {
        Column {
            Text("Notes Application")
            Button("Add Note")
        }
    }
}
"#;

    #[test]
    fn test_parse_and_compile_roundtrip() {
        let bytecode = compile_source(SAMPLE_NIL, "notes").expect("compile successful");
        assert!(bytecode.len() > 10);
        let vm = load_package(&bytecode).expect("load successful");
        assert_eq!(vm.package.app_name, "notes");
        assert_eq!(vm.state.get("title").map(|s| s.as_str()), Some("My Quick Notes"));
        let scene = vm.render_scene().expect("scene rendered");
        assert!(scene.contains("<Column>"));
        assert!(scene.contains("Notes Application"));
    }

    #[test]
    fn test_corrupt_bytecode_rejected() {
        let corrupt = b"BAD_DATA_FOR_BYTECODE";
        assert!(load_package(corrupt).is_err());
    }
}
