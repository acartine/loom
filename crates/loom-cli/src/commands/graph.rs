use loom_core::graph::profile;
use loom_core::graph::render::{self, RenderFormat};
use std::path::Path;

pub fn run(dir: &Path, profile_name: Option<&str>, format: RenderFormat) -> miette::Result<()> {
    let (ir, _diag) = loom_core::load_workflow(dir).map_err(|errors| {
        let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        miette::miette!("failed to load workflow:\n{}", msgs.join("\n"))
    })?;

    let output = if let Some(pname) = profile_name {
        let sub_ir = profile::extract_profile_subgraph(&ir, pname)
            .ok_or_else(|| miette::miette!("profile '{}' not found", pname))?;
        render::render(&sub_ir, format)
    } else {
        render::render(&ir, format)
    };

    print!("{}", output);
    Ok(())
}
