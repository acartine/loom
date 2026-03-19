use miette::IntoDiagnostic;
use std::fs;
use std::path::Path;

pub struct TemplateFile {
    pub relative_path: &'static str,
    pub contents: &'static str,
}

pub struct TemplateDefinition {
    pub id: &'static str,
    pub description: &'static str,
    pub files: &'static [TemplateFile],
}

const MINIMAL_FILES: &[TemplateFile] = &[
    TemplateFile {
        relative_path: "loom.toml",
        contents: r#"[workflow]
name = "__WORKFLOW_NAME__"
version = 1
entry = "workflow.loom"
default_profile = "default"
"#,
    },
    TemplateFile {
        relative_path: "workflow.loom",
        contents: r#"workflow __WORKFLOW_NAME__ v1 {

    queue ready_for_work "Ready for Work"
    queue ready_for_review "Ready for Review"

    action work "Work" {
        produce agent
        prompt work
    }

    action review "Review" {
        gate review human
        prompt review
    }

    terminal done "Done"

    step do_work {
        ready_for_work -> work
    }

    step review_work {
        ready_for_review -> review
    }

    phase main {
        produce do_work
        gate review_work
    }

    profile default "Default" {
        description "Default profile"
        phases [main]
        output local
    }
}
"#,
    },
    TemplateFile {
        relative_path: "prompts/work.md",
        contents: r#"---
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
    },
    TemplateFile {
        relative_path: "prompts/review.md",
        contents: r#"---
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
    },
];

const KNOTS_SDLC_FILES: &[TemplateFile] = &[
    TemplateFile {
        relative_path: "loom.toml",
        contents: include_str!("../../../templates/knots_sdlc/loom.toml"),
    },
    TemplateFile {
        relative_path: "workflow.loom",
        contents: include_str!("../../../templates/knots_sdlc/workflow.loom"),
    },
    TemplateFile {
        relative_path: "profiles/autopilot.loom",
        contents: include_str!("../../../templates/knots_sdlc/profiles/autopilot.loom"),
    },
    TemplateFile {
        relative_path: "profiles/autopilot_no_planning.loom",
        contents: include_str!("../../../templates/knots_sdlc/profiles/autopilot_no_planning.loom"),
    },
    TemplateFile {
        relative_path: "profiles/autopilot_with_pr.loom",
        contents: include_str!("../../../templates/knots_sdlc/profiles/autopilot_with_pr.loom"),
    },
    TemplateFile {
        relative_path: "profiles/autopilot_with_pr_no_planning.loom",
        contents: include_str!(
            "../../../templates/knots_sdlc/profiles/autopilot_with_pr_no_planning.loom"
        ),
    },
    TemplateFile {
        relative_path: "profiles/semiauto.loom",
        contents: include_str!("../../../templates/knots_sdlc/profiles/semiauto.loom"),
    },
    TemplateFile {
        relative_path: "profiles/semiauto_no_planning.loom",
        contents: include_str!("../../../templates/knots_sdlc/profiles/semiauto_no_planning.loom"),
    },
    TemplateFile {
        relative_path: "prompts/implementation.md",
        contents: include_str!("../../../templates/knots_sdlc/prompts/implementation.md"),
    },
    TemplateFile {
        relative_path: "prompts/implementation_review.md",
        contents: include_str!("../../../templates/knots_sdlc/prompts/implementation_review.md"),
    },
    TemplateFile {
        relative_path: "prompts/plan_review.md",
        contents: include_str!("../../../templates/knots_sdlc/prompts/plan_review.md"),
    },
    TemplateFile {
        relative_path: "prompts/planning.md",
        contents: include_str!("../../../templates/knots_sdlc/prompts/planning.md"),
    },
    TemplateFile {
        relative_path: "prompts/shipment.md",
        contents: include_str!("../../../templates/knots_sdlc/prompts/shipment.md"),
    },
    TemplateFile {
        relative_path: "prompts/shipment_review.md",
        contents: include_str!("../../../templates/knots_sdlc/prompts/shipment_review.md"),
    },
];

const TEMPLATES: &[TemplateDefinition] = &[
    TemplateDefinition {
        id: "minimal",
        description: "One produce step, one review step, one phase, one default profile",
        files: MINIMAL_FILES,
    },
    TemplateDefinition {
        id: "knots_sdlc",
        description: "Planning, implementation, review, shipment, and multiple execution profiles",
        files: KNOTS_SDLC_FILES,
    },
];

pub fn list() -> &'static [TemplateDefinition] {
    TEMPLATES
}

pub fn get(id: &str) -> Option<&'static TemplateDefinition> {
    TEMPLATES.iter().find(|template| template.id == id)
}

pub fn default_template_id() -> &'static str {
    "minimal"
}

pub fn init(
    dir: &Path,
    template_id: &str,
    workflow_name: &str,
) -> miette::Result<&'static TemplateDefinition> {
    let template = get(template_id).ok_or_else(|| {
        let available = available_template_ids();
        miette::miette!(
            "unknown template '{}'; available templates: {}",
            template_id,
            available
        )
    })?;

    write_template(dir, template, workflow_name)?;
    Ok(template)
}

pub fn write_template(
    dir: &Path,
    template: &TemplateDefinition,
    workflow_name: &str,
) -> miette::Result<()> {
    fs::create_dir_all(dir).into_diagnostic()?;

    for file in template.files {
        let target_path = dir.join(file.relative_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).into_diagnostic()?;
        }
        let contents = render_contents(
            template.id,
            file.relative_path,
            file.contents,
            workflow_name,
        );
        fs::write(target_path, contents).into_diagnostic()?;
    }

    Ok(())
}

fn available_template_ids() -> String {
    list()
        .iter()
        .map(|template| template.id)
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_contents(
    template_id: &str,
    relative_path: &str,
    contents: &str,
    workflow_name: &str,
) -> String {
    match template_id {
        "minimal" => contents.replace("__WORKFLOW_NAME__", workflow_name),
        "knots_sdlc" if matches!(relative_path, "loom.toml" | "workflow.loom") => {
            contents.replace("knots_sdlc", workflow_name)
        }
        _ => contents.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        path.push(format!("{}_{}_{}", prefix, std::process::id(), nanos));
        path
    }

    #[test]
    fn test_registry_contains_builtin_templates() {
        let templates = list();
        assert_eq!(templates.len(), 2);
        assert_eq!(templates[0].id, "minimal");
        assert_eq!(templates[1].id, "knots_sdlc");
        assert_eq!(default_template_id(), "minimal");
    }

    #[test]
    fn test_init_writes_minimal_template() {
        let dir = unique_temp_dir("loom_templates_minimal");
        let workflow_name = "payments_flow";

        let template = init(&dir, "minimal", workflow_name).expect("init should succeed");

        assert_eq!(template.id, "minimal");
        assert!(dir.join("loom.toml").exists());
        assert!(dir.join("workflow.loom").exists());
        assert!(dir.join("prompts/work.md").exists());
        assert!(dir.join("prompts/review.md").exists());

        let loom_toml = fs::read_to_string(dir.join("loom.toml")).expect("loom.toml should exist");
        let workflow =
            fs::read_to_string(dir.join("workflow.loom")).expect("workflow should exist");
        assert!(loom_toml.contains("name = \"payments_flow\""));
        assert!(workflow.contains("workflow payments_flow v1"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_knots_sdlc_template_rewrites_workflow_name() {
        let rendered = render_contents(
            "knots_sdlc",
            "workflow.loom",
            "workflow knots_sdlc v1 {}",
            "payments_sdlc",
        );
        assert_eq!(rendered, "workflow payments_sdlc v1 {}");
    }
}
