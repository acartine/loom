use std::path::Path;
use loom_core::codegen::{self, CodegenTarget};
use loom_core::codegen::toml_emit;

#[derive(Debug, Clone, Copy)]
pub enum EmitFormat {
    Rust,
    Toml,
}

pub fn run(dir: &Path, format: EmitFormat) -> miette::Result<()> {
    let (ir, diag) = loom_core::load_workflow(dir)
        .map_err(|errors| {
            let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            miette::miette!("failed to load workflow:\n{}", msgs.join("\n"))
        })?;

    // Print warnings
    for warning in &diag.warnings {
        eprintln!("{}", warning);
    }

    let output = match format {
        EmitFormat::Rust => codegen::generate(&ir, CodegenTarget::Rust)
            .map_err(|e| miette::miette!("{}", e))?,
        EmitFormat::Toml => toml_emit::emit_toml(&ir),
    };

    print!("{}", output);
    Ok(())
}
