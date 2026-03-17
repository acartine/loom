use std::fs;
use std::path::Path;
use miette::IntoDiagnostic;

pub fn run(name: &str) -> miette::Result<()> {
    let dir = Path::new(name);
    if dir.exists() {
        return Err(miette::miette!("directory '{}' already exists", name));
    }

    fs::create_dir_all(dir.join("prompts")).into_diagnostic()?;
    fs::create_dir_all(dir.join("profiles")).into_diagnostic()?;

    // loom.toml
    fs::write(
        dir.join("loom.toml"),
        format!(
            r#"[workflow]
name = "{name}"
version = 1
entry = "workflow.loom"
default_profile = "default"
"#
        ),
    ).into_diagnostic()?;

    // workflow.loom — minimal hello-world with a single produce+gate step
    fs::write(
        dir.join("workflow.loom"),
        format!(
            r#"workflow {name} v1 {{

    queue ready "Ready"

    action work "Work" {{
        produce agent
        prompt work
    }}

    terminal done "Done"

    step do_work {{
        ready -> work
    }}

    profile default "Default" {{
        description "Default profile"
        phases []
        output local
    }}
}}
"#
        ),
    ).into_diagnostic()?;

    // prompts/work.md
    fs::write(
        dir.join("prompts/work.md"),
        r#"---
accept:
  - Work is complete

success:
  completed: done

failure:
  blocked: ready

params: {}
---

# Work

Do the work.
"#,
    ).into_diagnostic()?;

    eprintln!("Created workflow '{}' in ./{}/", name, name);
    eprintln!("  workflow.loom   - workflow definition");
    eprintln!("  loom.toml       - package metadata");
    eprintln!("  prompts/work.md - example prompt");
    eprintln!();
    eprintln!("Next steps:");
    eprintln!("  cd {} && loom validate", name);

    Ok(())
}
