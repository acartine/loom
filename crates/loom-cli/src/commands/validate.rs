use std::path::Path;

pub fn run(dir: &Path) -> miette::Result<()> {
    match loom_core::validate_workflow(dir) {
        Ok((ir, diag)) => {
            // Print warnings
            for warning in &diag.warnings {
                eprintln!("{}", warning);
            }

            if diag.has_errors() {
                for err in &diag.errors {
                    eprintln!("error: {}", err);
                }
                Err(miette::miette!(
                    "{} error(s) found",
                    diag.errors.len()
                ))
            } else {
                eprintln!(
                    "ok: {} v{} ({} states, {} steps, {} phases, {} profiles)",
                    ir.name,
                    ir.version,
                    ir.states.len(),
                    ir.steps.len(),
                    ir.phases.len(),
                    ir.profiles.len(),
                );
                if !diag.warnings.is_empty() {
                    eprintln!("{} warning(s)", diag.warnings.len());
                }
                Ok(())
            }
        }
        Err(errors) => {
            for err in &errors {
                eprintln!("error: {}", err);
            }
            Err(miette::miette!("{} error(s) found", errors.len()))
        }
    }
}
