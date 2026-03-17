use crate::diff::{diff_workflows, ChangeKind};
use crate::ir::WorkflowIR;
use std::fmt;

/// Severity classification for compatibility changes per spec section 8.1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Safe,
    Breaking,
    MigrationRequired,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Safe => write!(f, "safe"),
            Severity::Breaking => write!(f, "breaking"),
            Severity::MigrationRequired => write!(f, "migration"),
        }
    }
}

/// A single compatibility-classified change.
#[derive(Debug, Clone)]
pub struct CompatChange {
    pub severity: Severity,
    pub category: String,
    pub name: String,
    pub detail: String,
}

/// Result of a compatibility check.
#[derive(Debug)]
pub struct CompatResult {
    pub changes: Vec<CompatChange>,
    pub is_compatible: bool,
}

/// Check backward compatibility between two workflow versions.
///
/// Classification per spec section 8.1:
/// - Safe: Adding new states, outcomes, profiles, phases
/// - Breaking: Removing states, outcomes, or profiles. Renaming state identifiers.
/// - Migration required: Changing outcome targets for existing outcomes
pub fn check_compat(old: &WorkflowIR, new: &WorkflowIR) -> CompatResult {
    let diff_changes = diff_workflows(old, new);
    let mut changes = Vec::new();

    for change in &diff_changes {
        let severity = classify_change(change);
        let detail = change.detail.clone().unwrap_or_default();
        changes.push(CompatChange {
            severity,
            category: change.category.clone(),
            name: change.name.clone(),
            detail,
        });
    }

    let is_compatible = !changes
        .iter()
        .any(|c| matches!(c.severity, Severity::Breaking | Severity::MigrationRequired));

    CompatResult {
        changes,
        is_compatible,
    }
}

fn classify_change(change: &crate::diff::Change) -> Severity {
    match change.kind {
        ChangeKind::Added => Severity::Safe,
        ChangeKind::Removed => Severity::Breaking,
        ChangeKind::Changed => classify_changed(change),
    }
}

fn classify_changed(change: &crate::diff::Change) -> Severity {
    match change.category.as_str() {
        "outcome" => Severity::MigrationRequired,
        "state" => {
            // Display name changes are safe; kind changes are breaking
            if let Some(detail) = &change.detail {
                if detail.starts_with("display_name") {
                    Severity::Safe
                } else {
                    Severity::Breaking
                }
            } else {
                Severity::Breaking
            }
        }
        // Profile or phase structural changes require migration
        _ => Severity::MigrationRequired,
    }
}

/// Format compatibility check results for display.
pub fn format_compat(result: &CompatResult) -> String {
    if result.changes.is_empty() {
        return "No changes detected. Fully compatible.\n".to_string();
    }

    let mut out = String::new();
    let severities = [
        Severity::Safe,
        Severity::Breaking,
        Severity::MigrationRequired,
    ];

    for severity in &severities {
        let matching: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.severity == *severity)
            .collect();
        if matching.is_empty() {
            continue;
        }
        out.push_str(&format!("{}: {} change(s)\n", severity, matching.len()));
        for c in &matching {
            let prefix = match c.severity {
                Severity::Safe => "+",
                Severity::Breaking => "-",
                Severity::MigrationRequired => "~",
            };
            if c.detail.is_empty() {
                out.push_str(&format!("  {} {}: {}\n", prefix, c.category, c.name));
            } else {
                out.push_str(&format!(
                    "  {} {}: {} ({})\n",
                    prefix, c.category, c.name, c.detail
                ));
            }
        }
        out.push('\n');
    }

    let breaking_count = result
        .changes
        .iter()
        .filter(|c| c.severity == Severity::Breaking)
        .count();
    let migration_count = result
        .changes
        .iter()
        .filter(|c| c.severity == Severity::MigrationRequired)
        .count();

    if result.is_compatible {
        out.push_str("Result: backward compatible\n");
    } else {
        out.push_str(&format!(
            "Result: NOT backward compatible ({} breaking, {} migration)\n",
            breaking_count, migration_count
        ));
    }

    out
}

/// Emit a TOML state mapping between old and new workflow versions (spec 8.2).
pub fn emit_state_map(old: &WorkflowIR, new: &WorkflowIR) -> String {
    let mut out = String::from("[state_map]\n");

    // States that exist in both old and new
    for name in old.states.keys() {
        if new.states.contains_key(name) {
            out.push_str(&format!("{} = \"{}\"\n", name, name));
        } else {
            out.push_str(&format!("# {} = REMOVED\n", name));
        }
    }

    // New states not in old
    for name in new.states.keys() {
        if !old.states.contains_key(name) {
            out.push_str(&format!("{} = \"{}\"  # NEW\n", name, name));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_v1() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/knots_sdlc")
    }

    fn fixture_v2() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/knots_sdlc_v2")
    }

    #[test]
    fn test_compat_has_safe_changes() {
        let (old_ir, _) = crate::load_workflow(&fixture_v1()).unwrap();
        let (new_ir, _) = crate::load_workflow(&fixture_v2()).unwrap();
        let result = check_compat(&old_ir, &new_ir);

        let safe: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.severity == Severity::Safe)
            .collect();
        assert!(!safe.is_empty(), "expected safe changes");
        assert!(
            safe.iter()
                .any(|c| c.category == "state" && c.name == "ready_for_triage"),
            "expected safe addition of ready_for_triage"
        );
        assert!(
            safe.iter()
                .any(|c| c.category == "profile" && c.name == "triage"),
            "expected safe addition of triage profile"
        );
    }

    #[test]
    fn test_compat_has_breaking_changes() {
        let (old_ir, _) = crate::load_workflow(&fixture_v1()).unwrap();
        let (new_ir, _) = crate::load_workflow(&fixture_v2()).unwrap();
        let result = check_compat(&old_ir, &new_ir);

        let breaking: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.severity == Severity::Breaking)
            .collect();
        assert!(!breaking.is_empty(), "expected breaking changes");
        assert!(
            breaking.iter().any(|c| c.name == "deferred"),
            "expected breaking removal of deferred state"
        );
    }

    #[test]
    fn test_compat_has_migration_changes() {
        let (old_ir, _) = crate::load_workflow(&fixture_v1()).unwrap();
        let (new_ir, _) = crate::load_workflow(&fixture_v2()).unwrap();
        let result = check_compat(&old_ir, &new_ir);

        let migration: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.severity == Severity::MigrationRequired)
            .collect();
        assert!(!migration.is_empty(), "expected migration-required changes");
        assert!(
            migration.iter().any(|c| c.name == "planning.plan_complete"),
            "expected migration for planning.plan_complete outcome target change"
        );
    }

    #[test]
    fn test_compat_is_not_compatible() {
        let (old_ir, _) = crate::load_workflow(&fixture_v1()).unwrap();
        let (new_ir, _) = crate::load_workflow(&fixture_v2()).unwrap();
        let result = check_compat(&old_ir, &new_ir);
        assert!(!result.is_compatible, "expected NOT compatible");
    }

    #[test]
    fn test_compat_same_workflow_is_compatible() {
        let (ir, _) = crate::load_workflow(&fixture_v1()).unwrap();
        let result = check_compat(&ir, &ir);
        assert!(result.is_compatible, "same workflow should be compatible");
        assert!(result.changes.is_empty());
    }

    #[test]
    fn test_state_map_contains_mappings() {
        let (old_ir, _) = crate::load_workflow(&fixture_v1()).unwrap();
        let (new_ir, _) = crate::load_workflow(&fixture_v2()).unwrap();
        let map = emit_state_map(&old_ir, &new_ir);

        assert!(map.contains("[state_map]"));
        assert!(map.contains("ready_for_planning = \"ready_for_planning\""));
        assert!(map.contains("# deferred = REMOVED"));
        assert!(map.contains("ready_for_triage = \"ready_for_triage\""));
    }

    #[test]
    fn test_format_compat_output() {
        let (old_ir, _) = crate::load_workflow(&fixture_v1()).unwrap();
        let (new_ir, _) = crate::load_workflow(&fixture_v2()).unwrap();
        let result = check_compat(&old_ir, &new_ir);
        let output = format_compat(&result);

        assert!(output.contains("safe:"));
        assert!(output.contains("breaking:"));
        assert!(output.contains("NOT backward compatible"));
    }
}
