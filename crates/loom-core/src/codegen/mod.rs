pub mod rust;
pub mod toml_emit;

use crate::ir::WorkflowIR;
use crate::error::LoomResult;

/// Target language for code generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenTarget {
    Rust,
}

/// Generate code for the given target language
pub fn generate(ir: &WorkflowIR, target: CodegenTarget) -> LoomResult<String> {
    match target {
        CodegenTarget::Rust => Ok(rust::generate(ir)),
    }
}
