use std::path::Path;

use crate::templates;

pub fn run(name: &str, template_id: &str) -> miette::Result<()> {
    let dir = Path::new(name);
    if dir.exists() {
        return Err(miette::miette!("directory '{}' already exists", name));
    }

    let workflow_name = workflow_name_from_path(dir)?;
    let template = templates::init(dir, template_id, &workflow_name)?;

    eprintln!(
        "Created workflow '{}' from template '{}' in ./{}/",
        name, template.id, name
    );
    eprintln!("  {}", template.description);
    for file in template.files {
        eprintln!("  {}", file.relative_path);
    }
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
