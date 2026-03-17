use crate::ir::WorkflowIR;
use std::fmt;

/// The kind of change detected between two workflow versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Removed,
    Changed,
}

/// A single structural change between two workflow versions.
#[derive(Debug, Clone)]
pub struct Change {
    pub kind: ChangeKind,
    pub category: String,
    pub name: String,
    pub detail: Option<String>,
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.kind {
            ChangeKind::Added => "+",
            ChangeKind::Removed => "-",
            ChangeKind::Changed => "~",
        };
        match &self.detail {
            Some(detail) => write!(
                f,
                "  {} {}: {} ({})",
                prefix, self.category, self.name, detail
            ),
            None => write!(f, "  {} {}: {}", prefix, self.category, self.name),
        }
    }
}

/// Compare two workflow IRs and return all structural changes.
pub fn diff_workflows(old: &WorkflowIR, new: &WorkflowIR) -> Vec<Change> {
    let mut changes = Vec::new();
    diff_states(old, new, &mut changes);
    diff_steps(old, new, &mut changes);
    diff_phases(old, new, &mut changes);
    diff_profiles(old, new, &mut changes);
    diff_outcomes(old, new, &mut changes);
    changes
}

/// Format a list of changes into human-readable output.
pub fn format_diff(changes: &[Change]) -> String {
    if changes.is_empty() {
        return "No changes detected.\n".to_string();
    }

    let mut out = String::new();
    let categories: &[&str] = &["state", "step", "phase", "profile", "outcome"];

    for &cat in categories {
        let cat_changes: Vec<_> = changes.iter().filter(|c| c.category == cat).collect();
        if cat_changes.is_empty() {
            continue;
        }
        let header = capitalize_category(cat);
        out.push_str(&format!("{}:\n", header));
        for c in cat_changes {
            out.push_str(&format!("{}\n", c));
        }
        out.push('\n');
    }

    out
}

fn capitalize_category(cat: &str) -> String {
    let mut chars = cat.chars();
    match chars.next() {
        Some(c) => format!("{}{}s", c.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

fn state_kind_label(state: &crate::ir::StateDef) -> &'static str {
    match state {
        crate::ir::StateDef::Queue { .. } => "queue",
        crate::ir::StateDef::Action { .. } => "action",
        crate::ir::StateDef::Terminal { .. } => "terminal",
        crate::ir::StateDef::Escape { .. } => "escape",
    }
}

fn diff_states(old: &WorkflowIR, new: &WorkflowIR, changes: &mut Vec<Change>) {
    for (name, new_state) in &new.states {
        match old.states.get(name) {
            None => changes.push(Change {
                kind: ChangeKind::Added,
                category: "state".into(),
                name: name.clone(),
                detail: Some(format!(
                    "{} \"{}\"",
                    state_kind_label(new_state),
                    new_state.display_name()
                )),
            }),
            Some(old_state) => {
                diff_single_state(name, old_state, new_state, changes);
            }
        }
    }
    for name in old.states.keys() {
        if !new.states.contains_key(name) {
            let old_state = &old.states[name];
            changes.push(Change {
                kind: ChangeKind::Removed,
                category: "state".into(),
                name: name.clone(),
                detail: Some(format!(
                    "{} \"{}\"",
                    state_kind_label(old_state),
                    old_state.display_name()
                )),
            });
        }
    }
}

fn diff_single_state(
    name: &str,
    old: &crate::ir::StateDef,
    new: &crate::ir::StateDef,
    changes: &mut Vec<Change>,
) {
    let old_kind = state_kind_label(old);
    let new_kind = state_kind_label(new);
    if old_kind != new_kind {
        changes.push(Change {
            kind: ChangeKind::Changed,
            category: "state".into(),
            name: name.to_string(),
            detail: Some(format!("kind {} -> {}", old_kind, new_kind)),
        });
    }
    if old.display_name() != new.display_name() {
        changes.push(Change {
            kind: ChangeKind::Changed,
            category: "state".into(),
            name: name.to_string(),
            detail: Some(format!(
                "display_name \"{}\" -> \"{}\"",
                old.display_name(),
                new.display_name()
            )),
        });
    }
}

fn diff_steps(old: &WorkflowIR, new: &WorkflowIR, changes: &mut Vec<Change>) {
    for (name, new_step) in &new.steps {
        match old.steps.get(name) {
            None => changes.push(Change {
                kind: ChangeKind::Added,
                category: "step".into(),
                name: name.clone(),
                detail: None,
            }),
            Some(old_step) => {
                if old_step.queue != new_step.queue || old_step.action != new_step.action {
                    changes.push(Change {
                        kind: ChangeKind::Changed,
                        category: "step".into(),
                        name: name.clone(),
                        detail: Some(format!(
                            "{}->{} -> {}->{}",
                            old_step.queue, old_step.action, new_step.queue, new_step.action
                        )),
                    });
                }
            }
        }
    }
    for name in old.steps.keys() {
        if !new.steps.contains_key(name) {
            changes.push(Change {
                kind: ChangeKind::Removed,
                category: "step".into(),
                name: name.clone(),
                detail: None,
            });
        }
    }
}

fn diff_phases(old: &WorkflowIR, new: &WorkflowIR, changes: &mut Vec<Change>) {
    for (name, new_phase) in &new.phases {
        match old.phases.get(name) {
            None => changes.push(Change {
                kind: ChangeKind::Added,
                category: "phase".into(),
                name: name.clone(),
                detail: None,
            }),
            Some(old_phase) => {
                if old_phase.produce_step != new_phase.produce_step
                    || old_phase.gate_step != new_phase.gate_step
                {
                    changes.push(Change {
                        kind: ChangeKind::Changed,
                        category: "phase".into(),
                        name: name.clone(),
                        detail: Some(format!(
                            "produce {}->{}, gate {}->{}",
                            old_phase.produce_step,
                            new_phase.produce_step,
                            old_phase.gate_step,
                            new_phase.gate_step
                        )),
                    });
                }
            }
        }
    }
    for name in old.phases.keys() {
        if !new.phases.contains_key(name) {
            changes.push(Change {
                kind: ChangeKind::Removed,
                category: "phase".into(),
                name: name.clone(),
                detail: None,
            });
        }
    }
}

fn diff_profiles(old: &WorkflowIR, new: &WorkflowIR, changes: &mut Vec<Change>) {
    for (name, new_profile) in &new.profiles {
        match old.profiles.get(name) {
            None => changes.push(Change {
                kind: ChangeKind::Added,
                category: "profile".into(),
                name: name.clone(),
                detail: None,
            }),
            Some(old_profile) => {
                diff_single_profile(name, old_profile, new_profile, changes);
            }
        }
    }
    for name in old.profiles.keys() {
        if !new.profiles.contains_key(name) {
            changes.push(Change {
                kind: ChangeKind::Removed,
                category: "profile".into(),
                name: name.clone(),
                detail: None,
            });
        }
    }
}

fn diff_single_profile(
    name: &str,
    old: &crate::ir::ProfileDef,
    new: &crate::ir::ProfileDef,
    changes: &mut Vec<Change>,
) {
    if old.phases != new.phases {
        changes.push(Change {
            kind: ChangeKind::Changed,
            category: "profile".into(),
            name: name.to_string(),
            detail: Some(format!("phases {:?} -> {:?}", old.phases, new.phases)),
        });
    }
    if old.output != new.output {
        changes.push(Change {
            kind: ChangeKind::Changed,
            category: "profile".into(),
            name: name.to_string(),
            detail: Some("output changed".to_string()),
        });
    }
    if old.overrides != new.overrides {
        changes.push(Change {
            kind: ChangeKind::Changed,
            category: "profile".into(),
            name: name.to_string(),
            detail: Some("overrides changed".to_string()),
        });
    }
}

fn diff_outcomes(old: &WorkflowIR, new: &WorkflowIR, changes: &mut Vec<Change>) {
    // Compare outcomes per prompt (action state)
    for (prompt_name, new_prompt) in &new.prompts {
        match old.prompts.get(prompt_name) {
            None => {
                // All outcomes are new — covered by state additions
            }
            Some(old_prompt) => {
                diff_outcome_map(
                    prompt_name,
                    "success",
                    &old_prompt.success,
                    &new_prompt.success,
                    changes,
                );
                diff_outcome_map(
                    prompt_name,
                    "failure",
                    &old_prompt.failure,
                    &new_prompt.failure,
                    changes,
                );
            }
        }
    }
    // Check for removed prompts (outcomes removed)
    for prompt_name in old.prompts.keys() {
        if !new.prompts.contains_key(prompt_name) {
            // Prompt removed entirely — covered by state removals
        }
    }
}

fn diff_outcome_map(
    prompt_name: &str,
    outcome_kind: &str,
    old_map: &indexmap::IndexMap<String, String>,
    new_map: &indexmap::IndexMap<String, String>,
    changes: &mut Vec<Change>,
) {
    for (outcome_name, new_target) in new_map {
        match old_map.get(outcome_name) {
            None => changes.push(Change {
                kind: ChangeKind::Added,
                category: "outcome".into(),
                name: format!("{}.{}", prompt_name, outcome_name),
                detail: Some(format!("-> {} [{}]", new_target, outcome_kind)),
            }),
            Some(old_target) => {
                if old_target != new_target {
                    changes.push(Change {
                        kind: ChangeKind::Changed,
                        category: "outcome".into(),
                        name: format!("{}.{}", prompt_name, outcome_name),
                        detail: Some(format!("{} -> {}", old_target, new_target)),
                    });
                }
            }
        }
    }
    for outcome_name in old_map.keys() {
        if !new_map.contains_key(outcome_name) {
            changes.push(Change {
                kind: ChangeKind::Removed,
                category: "outcome".into(),
                name: format!("{}.{}", prompt_name, outcome_name),
                detail: Some(format!("[{}]", outcome_kind)),
            });
        }
    }
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
    fn test_diff_detects_added_state() {
        let (old_ir, _) = crate::load_workflow(&fixture_v1()).unwrap();
        let (new_ir, _) = crate::load_workflow(&fixture_v2()).unwrap();
        let changes = diff_workflows(&old_ir, &new_ir);

        let added_states: Vec<_> = changes
            .iter()
            .filter(|c| c.category == "state" && c.kind == ChangeKind::Added)
            .collect();
        assert!(
            added_states.iter().any(|c| c.name == "ready_for_triage"),
            "expected added state ready_for_triage, got: {:?}",
            added_states
        );
    }

    #[test]
    fn test_diff_detects_removed_state() {
        let (old_ir, _) = crate::load_workflow(&fixture_v1()).unwrap();
        let (new_ir, _) = crate::load_workflow(&fixture_v2()).unwrap();
        let changes = diff_workflows(&old_ir, &new_ir);

        let removed_states: Vec<_> = changes
            .iter()
            .filter(|c| c.category == "state" && c.kind == ChangeKind::Removed)
            .collect();
        assert!(
            removed_states.iter().any(|c| c.name == "deferred"),
            "expected removed state deferred, got: {:?}",
            removed_states
        );
    }

    #[test]
    fn test_diff_detects_display_name_change() {
        let (old_ir, _) = crate::load_workflow(&fixture_v1()).unwrap();
        let (new_ir, _) = crate::load_workflow(&fixture_v2()).unwrap();
        let changes = diff_workflows(&old_ir, &new_ir);

        let changed = changes.iter().find(|c| {
            c.category == "state"
                && c.name == "planning"
                && c.kind == ChangeKind::Changed
                && c.detail
                    .as_deref()
                    .is_some_and(|d| d.contains("display_name"))
        });
        assert!(
            changed.is_some(),
            "expected display_name change for planning"
        );
    }

    #[test]
    fn test_diff_detects_outcome_target_change() {
        let (old_ir, _) = crate::load_workflow(&fixture_v1()).unwrap();
        let (new_ir, _) = crate::load_workflow(&fixture_v2()).unwrap();
        let changes = diff_workflows(&old_ir, &new_ir);

        let outcome_change = changes.iter().find(|c| {
            c.category == "outcome"
                && c.name == "planning.plan_complete"
                && c.kind == ChangeKind::Changed
        });
        assert!(
            outcome_change.is_some(),
            "expected outcome target change for planning.plan_complete"
        );
    }

    #[test]
    fn test_diff_detects_added_profile() {
        let (old_ir, _) = crate::load_workflow(&fixture_v1()).unwrap();
        let (new_ir, _) = crate::load_workflow(&fixture_v2()).unwrap();
        let changes = diff_workflows(&old_ir, &new_ir);

        let added_profiles: Vec<_> = changes
            .iter()
            .filter(|c| c.category == "profile" && c.kind == ChangeKind::Added)
            .collect();
        assert!(
            added_profiles.iter().any(|c| c.name == "triage"),
            "expected added profile triage, got: {:?}",
            added_profiles
        );
    }

    #[test]
    fn test_diff_no_changes_same_workflow() {
        let (ir, _) = crate::load_workflow(&fixture_v1()).unwrap();
        let changes = diff_workflows(&ir, &ir);
        assert!(
            changes.is_empty(),
            "expected no changes, got: {:?}",
            changes.len()
        );
    }

    #[test]
    fn test_format_diff_output() {
        let (old_ir, _) = crate::load_workflow(&fixture_v1()).unwrap();
        let (new_ir, _) = crate::load_workflow(&fixture_v2()).unwrap();
        let changes = diff_workflows(&old_ir, &new_ir);
        let output = format_diff(&changes);

        assert!(
            output.contains("States:"),
            "output should have States section"
        );
        assert!(
            output.contains("+ state: ready_for_triage"),
            "output should show added state"
        );
        assert!(
            output.contains("- state: deferred"),
            "output should show removed state"
        );
    }
}
