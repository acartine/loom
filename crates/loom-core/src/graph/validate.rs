use petgraph::algo::has_path_connecting;
use petgraph::Direction;
use std::collections::HashSet;

use super::{build_graph, WorkflowGraph};
use crate::error::{Diagnostics, LoomError, LoomWarning};
use crate::ir::{StateDef, WorkflowIR};

/// Run all validation checks on the workflow
pub fn validate(ir: &WorkflowIR) -> Diagnostics {
    let graph = build_graph(ir);
    let mut diag = Diagnostics::new();

    check_produce_has_success(ir, &mut diag);
    check_dead_states(ir, &graph, &mut diag);
    check_terminal_reachability(ir, &graph, &mut diag);
    check_orphaned_prompts(ir, &mut diag);
    check_warnings(ir, &mut diag);

    diag
}

/// Check that every produce action has at least one success outcome.
/// Without this, there is no codegen path to advance through the phase.
fn check_produce_has_success(ir: &WorkflowIR, diag: &mut Diagnostics) {
    for (name, state) in &ir.states {
        if let StateDef::Action { prompt_name, .. } = state {
            if state.is_produce() {
                if let Some(prompt) = ir.prompts.get(prompt_name) {
                    if prompt.success.is_empty() {
                        diag.error(LoomError::ProduceNoSuccess {
                            action: name.clone(),
                        });
                    }
                }
            }
        }
    }
}

/// Check that every non-terminal, non-escape state has at least one inbound edge.
/// Entry queue states (first queue of first phase) are exempt.
fn check_dead_states(ir: &WorkflowIR, graph: &WorkflowGraph, diag: &mut Diagnostics) {
    // Collect entry states: first queue of each phase that appears first in any profile
    let mut entry_states: HashSet<String> = HashSet::new();

    // The first queue state reached by the workflow is the entry point
    // For each profile, the first phase's produce step's queue is the entry
    for profile in ir.profiles.values() {
        if let Some(first_phase_name) = profile.phases.first() {
            if let Some(phase) = ir.phases.get(first_phase_name) {
                if let Some(step) = ir.steps.get(&phase.produce_step) {
                    entry_states.insert(step.queue.clone());
                }
            }
        }
    }

    for (name, _state) in &ir.states {
        if ir
            .states
            .get(name)
            .is_some_and(|s| s.is_terminal() || s.is_escape() || s.is_queue())
        {
            continue;
        }
        if entry_states.contains(name) {
            continue;
        }

        if let Some(&idx) = graph.node_indices.get(name) {
            let inbound = graph.graph.edges_directed(idx, Direction::Incoming).count();
            if inbound == 0 {
                // Generate a short step name hint from the action name
                let hint = name
                    .split('_')
                    .map(|w| w.chars().next().unwrap_or_default().to_string())
                    .collect::<Vec<_>>()
                    .join("");
                diag.error(LoomError::DeadState {
                    name: name.clone(),
                    hint,
                });
            }
        }
    }
}

/// Check that every non-terminal state can reach at least one terminal state
fn check_terminal_reachability(ir: &WorkflowIR, graph: &WorkflowGraph, diag: &mut Diagnostics) {
    let terminal_indices: Vec<_> = ir
        .states
        .iter()
        .filter(|(_, s)| s.is_terminal())
        .filter_map(|(name, _)| graph.node_indices.get(name).copied())
        .collect();

    if terminal_indices.is_empty() {
        return;
    }

    for (name, state) in &ir.states {
        if state.is_terminal() || state.is_escape() {
            continue;
        }

        if let Some(&idx) = graph.node_indices.get(name) {
            let can_reach_terminal = terminal_indices
                .iter()
                .any(|&t_idx| has_path_connecting(&graph.graph, idx, t_idx, None));

            if !can_reach_terminal {
                diag.error(LoomError::TerminalUnreachable { name: name.clone() });
            }
        }
    }
}

/// Check for prompt files that exist on disk but aren't referenced by any action
fn check_orphaned_prompts(ir: &WorkflowIR, diag: &mut Diagnostics) {
    // This check requires scanning the prompts directory, which we skip here
    // since we only load referenced prompts during lowering.
    // The check is done at the CLI level where we have access to the filesystem.
    let _ = (ir, diag);
}

/// Generate warnings
fn check_warnings(ir: &WorkflowIR, diag: &mut Diagnostics) {
    // Unused steps: steps not in any phase
    let phase_steps: HashSet<String> = ir
        .phases
        .values()
        .flat_map(|p| p.step_names().into_iter().cloned().collect::<Vec<_>>())
        .collect();

    for name in ir.steps.keys() {
        if !phase_steps.contains(name) {
            diag.warn(LoomWarning::UnusedStep { name: name.clone() });
        }
    }

    // Single-outcome actions
    for (prompt_name, prompt) in &ir.prompts {
        if prompt.success.len() == 1 && prompt.failure.is_empty() {
            // Find the action name for this prompt
            if let Some(action_name) = ir.states.values().find_map(|s| {
                if let StateDef::Action {
                    name,
                    prompt_name: pn,
                    ..
                } = s
                {
                    if pn == prompt_name {
                        Some(name.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }) {
                diag.warn(LoomWarning::SingleOutcomeAction { name: action_name });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::lower::lower;
    use crate::parse;
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/knots_sdlc")
    }

    use crate::ir::{PhaseDef, ProfileDef, StepDef};
    use crate::parse::ast::{ActionType, Executor};
    use crate::prompt::PromptFile;
    use indexmap::IndexMap;

    /// Regression test: a produce action with no success outcomes must be
    /// rejected at validation time. Without this check the graph and codegen
    /// disagree — the graph could show an implicit phase link while apply()
    /// has no success variant to reach the gate queue.
    #[test]
    fn test_produce_no_success_is_error() {
        let mut states = IndexMap::new();
        states.insert(
            "q1".into(),
            StateDef::Queue {
                name: "q1".into(),
                display_name: "Q1".into(),
            },
        );
        states.insert(
            "a1".into(),
            StateDef::Action {
                name: "a1".into(),
                display_name: "A1".into(),
                action_type: ActionType::Produce(Executor::Agent),
                prompt_name: "a1_prompt".into(),
                output: None,
                constraints: vec![],
                executor: Executor::Agent,
            },
        );
        states.insert(
            "q2".into(),
            StateDef::Queue {
                name: "q2".into(),
                display_name: "Q2".into(),
            },
        );
        states.insert(
            "a2".into(),
            StateDef::Action {
                name: "a2".into(),
                display_name: "A2".into(),
                action_type: ActionType::Gate(
                    crate::parse::ast::GateKind::Approve,
                    Executor::Human,
                ),
                prompt_name: "a2_prompt".into(),
                output: None,
                constraints: vec![],
                executor: Executor::Human,
            },
        );
        states.insert(
            "done".into(),
            StateDef::Terminal {
                name: "done".into(),
                display_name: "Done".into(),
            },
        );

        let mut steps = IndexMap::new();
        steps.insert(
            "s1".into(),
            StepDef {
                name: "s1".into(),
                queue: "q1".into(),
                action: "a1".into(),
            },
        );
        steps.insert(
            "s2".into(),
            StepDef {
                name: "s2".into(),
                queue: "q2".into(),
                action: "a2".into(),
            },
        );

        let mut phases = IndexMap::new();
        phases.insert(
            "p1".into(),
            PhaseDef {
                name: "p1".into(),
                produce_step: "s1".into(),
                gate_step: Some("s2".into()),
            },
        );

        let mut profiles = IndexMap::new();
        profiles.insert(
            "default".into(),
            ProfileDef {
                name: "default".into(),
                display_name: None,
                description: None,
                phases: vec!["p1".into()],
                overrides: IndexMap::new(),
            },
        );

        let mut prompts = IndexMap::new();
        // Produce prompt with NO success outcomes — this is the bug trigger
        prompts.insert(
            "a1_prompt".into(),
            PromptFile {
                accept: vec![],
                success: IndexMap::new(),
                failure: IndexMap::from([("fail".into(), "q1".into())]),
                params: IndexMap::new(),
                body: String::new(),
                body_params: vec![],
            },
        );
        prompts.insert(
            "a2_prompt".into(),
            PromptFile {
                accept: vec![],
                success: IndexMap::from([("approved".into(), "done".into())]),
                failure: IndexMap::from([("rejected".into(), "q1".into())]),
                params: IndexMap::new(),
                body: String::new(),
                body_params: vec![],
            },
        );

        let ir = WorkflowIR {
            name: "test".into(),
            version: 1,
            default_profile: Some("default".into()),
            states,
            steps,
            phases,
            profiles,
            wildcard_targets: vec![],
            prompts,
        };

        let diag = validate(&ir);
        let has_produce_no_success = diag
            .errors
            .iter()
            .any(|e| matches!(e, LoomError::ProduceNoSuccess { action } if action == "a1"));
        assert!(
            has_produce_no_success,
            "expected ProduceNoSuccess error for 'a1', got errors: {:?}",
            diag.errors
        );
    }

    #[test]
    fn test_validate_knots_sdlc() {
        let input = std::fs::read_to_string(fixture_dir().join("workflow.loom")).unwrap();
        let ast = parse::parse_workflow(&input).unwrap();
        let (ir, _) = lower(&ast, &fixture_dir()).unwrap();
        let diag = validate(&ir);

        // Print any errors for debugging
        for err in &diag.errors {
            eprintln!("ERROR: {}", err);
        }
        for warn in &diag.warnings {
            eprintln!("WARN: {}", warn);
        }

        assert!(
            !diag.has_errors(),
            "knots_sdlc should validate clean, got {} errors",
            diag.errors.len()
        );
    }
}
