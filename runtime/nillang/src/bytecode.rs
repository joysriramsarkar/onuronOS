// runtime/nillang/src/bytecode.rs — Binary Bytecode Format for NilLang
use serde::{Deserialize, Serialize};
use crate::ast::Program;

pub const BYTECODE_MAGIC: &[u8; 4] = b"NILB";
pub const BYTECODE_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledPackage {
    pub magic: [u8; 4],
    pub version: u16,
    pub app_name: String,
    pub program: Program,
}

impl CompiledPackage {
    pub fn new(app_name: String, program: Program) -> Self {
        Self {
            magic: *BYTECODE_MAGIC,
            version: BYTECODE_VERSION,
            app_name,
            program,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|e| format!("Bytecode serialization error: {e}"))
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, String> {
        let pkg: Self = serde_json::from_slice(bytes).map_err(|e| format!("Bytecode parse error: {e}"))?;
        if &pkg.magic != BYTECODE_MAGIC {
            return Err("Invalid bytecode magic header".to_string());
        }
        if pkg.version != BYTECODE_VERSION {
            return Err(format!("Unsupported bytecode version: {}", pkg.version));
        }
        Ok(pkg)
    }
}
