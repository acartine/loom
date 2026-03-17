use miette::IntoDiagnostic;
use std::fs;
use std::path::Path;

pub fn run(name: &str) -> miette::Result<()> {
    let dir = Path::new(name);
    if dir.exists() {
        return Err(miette::miette!("directory '{}' already exists", name));
    }

    let workflow_name = workflow_name_from_path(dir)?;

    fs::create_dir_all(dir.join("prompts")).into_diagnostic()?;
    fs::create_dir_all(dir.join("profiles")).into_diagnostic()?;

    // loom.toml
    fs::write(
        dir.join("loom.toml"),
        format!(
            r#"[workflow]
name = "{workflow_name}"
version = 1
entry = "workflow.loom"
default_profile = "default"
"#
        ),
    )
    .into_diagnostic()?;

    // workflow.loom — minimal hello-world with a single produce+gate step
    fs::write(
        dir.join("workflow.loom"),
        format!(
            r#"workflow {workflow_name} v1 {{

    queue ready_for_work "Ready for Work"
    queue ready_for_review "Ready for Review"

    action work "Work" {{
        produce agent
        prompt work
    }}

    action review "Review" {{
        gate review human
        prompt review
    }}

    terminal done "Done"

    step do_work {{
        ready_for_work -> work
    }}

    step review_work {{
        ready_for_review -> review
    }}

    phase main {{
        produce do_work
        gate review_work
    }}

    profile default "Default" {{
        description "Default profile"
        phases [main]
        output local
    }}
}}
"#
        ),
    )
    .into_diagnostic()?;

    // prompts/work.md
    fs::write(
        dir.join("prompts/work.md"),
        r#"---
accept:
  - Work is complete
  - Handoff notes are ready for review

success:
  completed: ready_for_review

failure:
  blocked: ready_for_work

params: {}
---

# Work

Do the work and prepare it for review.
"#,
    )
    .into_diagnostic()?;

    // prompts/review.md
    fs::write(
        dir.join("prompts/review.md"),
        r#"---
accept:
  - The work meets the acceptance criteria
  - The result is ready to ship

success:
  approved: done

failure:
  changes_requested: ready_for_work

params: {}
---

# Review

Review the work and either approve it or send it back for changes.
"#,
    )
    .into_diagnostic()?;

    eprintln!("Created workflow '{}' in ./{}/", name, name);
    eprintln!("  workflow.loom   - workflow definition");
    eprintln!("  loom.toml       - package metadata");
    eprintln!("  prompts/work.md - produce prompt");
    eprintln!("  prompts/review.md - gate prompt");
    eprintln!();
    eprintln!("Next steps:");
    eprintln!("  cd {} && loom validate", name);

    Ok(())
}

fn workflow_name_from_path(path: &Path) -> miette::Result<String> {
    let raw_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| miette::miette!("workflow name must end with a valid directory name"))?;

    let normalized = raw_name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();

    let starts_with_valid_char = normalized
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_');

    if normalized.is_empty() || !starts_with_valid_char {
        return Err(miette::miette!(
            "workflow directory name '{}' cannot be converted to a valid Loom identifier",
            raw_name
        ));
    }

    Ok(normalized)
}
