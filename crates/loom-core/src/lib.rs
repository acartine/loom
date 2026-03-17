pub mod parse;
pub mod prompt;
pub mod config;
pub mod ir;
pub mod graph;
pub mod codegen;
pub mod error;

use std::path::Path;
use error::{Diagnostics, LoomError};
use ir::WorkflowIR;

/// Load and compile a workflow from a directory path.
/// This is the main entry point for the library.
pub fn load_workflow(workflow_dir: &Path) -> Result<(WorkflowIR, Diagnostics), Vec<LoomError>> {
    // Load config
    let config_path = workflow_dir.join("loom.toml");
    let config = config::load_config(&config_path)
        .map_err(|e| vec![e])?;

    // Parse workflow
    let entry_path = workflow_dir.join(&config.workflow.entry);
    let source = std::fs::read_to_string(&entry_path)
        .map_err(|e| vec![LoomError::Io(e)])?;
    let ast = parse::parse_workflow(&source)
        .map_err(|e| vec![e])?;

    // Lower to IR with config metadata
    ir::lower::lower_with_config(
        &ast,
        workflow_dir,
        config.workflow.default_profile,
    )
}

/// Load, lower, and validate a workflow. Returns the IR and all diagnostics.
pub fn validate_workflow(workflow_dir: &Path) -> Result<(WorkflowIR, Diagnostics), Vec<LoomError>> {
    let (ir, mut diag) = load_workflow(workflow_dir)?;

    // Check for orphaned prompts
    let prompts_dir = workflow_dir.join("prompts");
    if prompts_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&prompts_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "md") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let is_referenced = ir.states.values().any(|s| {
                            if let ir::StateDef::Action { prompt_name, .. } = s {
                                prompt_name == stem
                            } else {
                                false
                            }
                        });
                        if !is_referenced {
                            diag.error(LoomError::OrphanedPrompt { name: stem.to_string() });
                        }
                    }
                }
            }
        }
    }

    // Run full-workflow graph validation
    let graph_diag = graph::validate::validate(&ir);
    diag.merge(graph_diag);

    // Run per-profile subgraph validation
    for profile_name in ir.profiles.keys() {
        if let Some(sub_ir) = graph::profile::extract_profile_subgraph(&ir, profile_name) {
            let profile_diag = graph::validate::validate(&sub_ir);
            for err in profile_diag.errors {
                diag.error(LoomError::ProfileValidation {
                    profile: profile_name.clone(),
                    message: err.to_string(),
                });
            }
            // Don't duplicate warnings from profile subgraphs
        }
    }

    Ok((ir, diag))
}
