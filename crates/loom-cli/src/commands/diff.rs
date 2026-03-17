use std::path::Path;

pub fn run(old_dir: &Path, new_dir: &Path) -> miette::Result<()> {
    let (old_ir, _) = loom_core::load_workflow(old_dir)
        .map_err(|errs| miette::miette!("failed to load old workflow: {:?}", errs))?;
    let (new_ir, _) = loom_core::load_workflow(new_dir)
        .map_err(|errs| miette::miette!("failed to load new workflow: {:?}", errs))?;

    let changes = loom_core::diff::diff_workflows(&old_ir, &new_ir);
    let output = loom_core::diff::format_diff(&changes);
    print!("{}", output);

    Ok(())
}
