use std::path::Path;

pub fn run(old_dir: &Path, new_dir: &Path, emit_map: bool) -> miette::Result<()> {
    let (old_ir, _) = loom_core::load_workflow(old_dir)
        .map_err(|errs| miette::miette!("failed to load old workflow: {:?}", errs))?;
    let (new_ir, _) = loom_core::load_workflow(new_dir)
        .map_err(|errs| miette::miette!("failed to load new workflow: {:?}", errs))?;

    let result = loom_core::compat::check_compat(&old_ir, &new_ir);
    let output = loom_core::compat::format_compat(&result);
    print!("{}", output);

    if emit_map {
        println!();
        let map = loom_core::compat::emit_state_map(&old_ir, &new_ir);
        print!("{}", map);
    }

    if !result.is_compatible {
        std::process::exit(1);
    }

    Ok(())
}
