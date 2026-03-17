use std::path::Path;
use loom_core::codegen::{self, CodegenTarget};
use loom_core::codegen::toml_emit;

#[derive(Debug, Clone, Copy)]
pub enum EmitFormat {
    Rust,
    Toml,
}

pub fn run(dir: &Path, format: EmitFormat) -> miette::Result<()> {
    // Build runs full validation — loom is a validating compiler
    let (ir, diag) = loom_core::validate_workflow(dir)
        .map_err(|errors| {
            let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            miette::miette!("failed to load workflow:\n{}", msgs.join("\n"))
        })?;

    // Print warnings
    for warning in &diag.warnings {
        eprintln!("{}", warning);
    }

    // Refuse to emit artifacts if validation failed
    if diag.has_errors() {
        for err in &diag.errors {
            eprintln!("error: {}", err);
        }
        return Err(miette::miette!(
            "refusing to build: {} validation error(s)",
            diag.errors.len()
        ));
    }

    let output = match format {
        EmitFormat::Rust => codegen::generate(&ir, CodegenTarget::Rust)
            .map_err(|e| miette::miette!("{}", e))?,
        EmitFormat::Toml => toml_emit::emit_toml(&ir),
    };

    print!("{}", output);
    Ok(())
}
