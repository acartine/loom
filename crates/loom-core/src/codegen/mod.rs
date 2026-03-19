pub mod go;
pub mod knots_bundle;
pub mod python;
pub mod rust;
pub mod toml_emit;

use crate::error::LoomResult;
use crate::ir::WorkflowIR;

/// Target language for code generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenTarget {
    Rust,
    Go,
    Python,
}

/// Generate code for the given target language
pub fn generate(ir: &WorkflowIR, target: CodegenTarget) -> LoomResult<String> {
    match target {
        CodegenTarget::Rust => Ok(rust::generate(ir)),
        CodegenTarget::Go => Ok(go::generate(ir)),
        CodegenTarget::Python => Ok(python::generate(ir)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/knots_sdlc")
    }

    #[test]
    fn test_generate_rust_target() {
        let (ir, _diag) =
            crate::load_workflow(&fixture_dir()).expect("load_workflow should succeed");
        let output = generate(&ir, CodegenTarget::Rust).expect("generate should succeed");
        assert!(
            output.contains("pub enum State"),
            "expected 'pub enum State' in output"
        );
    }
}
