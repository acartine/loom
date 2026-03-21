use std::path::Path;

use crate::templates;

pub fn run(name: &str, template_id: Option<&str>) -> miette::Result<()> {
    let dir = Path::new(name);
    if dir.exists() {
        return Err(miette::miette!("directory '{}' already exists", name));
    }

    let workflow_name = workflow_name_from_path(dir)?;
    let template_id = resolve_template_id(name, template_id);
    let template = templates::init(dir, template_id, &workflow_name)?;
    let display_dir = display_dir(dir);

    eprintln!(
        "Created workflow '{}' from template '{}' in {display_dir}",
        name, template.id
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

fn resolve_template_id<'a>(name: &'a str, template_id: Option<&'a str>) -> &'a str {
    if let Some(template_id) = template_id {
        return template_id;
    }

    if templates::get(name).is_some() {
        return name;
    }

    templates::default_template_id()
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

fn display_dir(path: &Path) -> String {
    if path.is_absolute() {
        format!("{}/", path.display())
    } else {
        format!("./{}/", path.display())
    }
}

#[cfg(test)]
mod tests {
    use super::{display_dir, resolve_template_id};
    use std::path::Path;

    #[test]
    fn defaults_to_minimal_for_unknown_name() {
        assert_eq!(resolve_template_id("payments_flow", None), "minimal");
    }

    #[test]
    fn treats_builtin_template_name_as_shorthand() {
        assert_eq!(resolve_template_id("knots_sdlc", None), "knots_sdlc");
    }

    #[test]
    fn explicit_template_wins_over_shorthand() {
        assert_eq!(
            resolve_template_id("knots_sdlc", Some("minimal")),
            "minimal"
        );
    }

    #[test]
    fn formats_relative_init_path_for_display() {
        assert_eq!(display_dir(Path::new("payments_flow")), "./payments_flow/");
    }

    #[test]
    fn formats_absolute_init_path_for_display() {
        assert_eq!(
            display_dir(Path::new("/tmp/payments_flow")),
            "/tmp/payments_flow/"
        );
    }
}
