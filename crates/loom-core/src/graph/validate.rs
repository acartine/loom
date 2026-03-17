use std::collections::HashSet;
use petgraph::algo::has_path_connecting;
use petgraph::Direction;

use crate::error::{Diagnostics, LoomError, LoomWarning};
use crate::ir::{StateDef, WorkflowIR};
use super::{WorkflowGraph, build_graph};

/// Run all validation checks on the workflow
pub fn validate(ir: &WorkflowIR) -> Diagnostics {
    let graph = build_graph(ir);
    let mut diag = Diagnostics::new();

    check_produce_has_success(ir, &mut diag);
    check_dead_states(ir, &graph, &mut diag);
    check_terminal_reachability(ir, &graph, &mut diag);
    check_orphaned_prompts(ir, &mut diag);
    check_escape_reachability(ir, &graph, &mut diag);
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
        if ir.states.get(name).map_or(false, |s| s.is_terminal() || s.is_escape()) {
            continue;
        }
        if entry_states.contains(name) {
            continue;
        }

        if let Some(&idx) = graph.node_indices.get(name) {
            let inbound = graph.graph.edges_directed(idx, Direction::Incoming).count();
            if inbound == 0 {
                diag.error(LoomError::DeadState { name: name.clone() });
            }
        }
    }
}

/// Check that every non-terminal state can reach at least one terminal state
fn check_terminal_reachability(ir: &WorkflowIR, graph: &WorkflowGraph, diag: &mut Diagnostics) {
    let terminal_indices: Vec<_> = ir.states.iter()
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
            let can_reach_terminal = terminal_indices.iter()
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

/// Check that escape states have at least one outbound edge back to the workflow
fn check_escape_reachability(ir: &WorkflowIR, graph: &WorkflowGraph, diag: &mut Diagnostics) {
    for (name, state) in &ir.states {
        if !state.is_escape() {
            continue;
        }
        if let Some(&idx) = graph.node_indices.get(name) {
            let outbound = graph.graph.edges_directed(idx, Direction::Outgoing).count();
            if outbound == 0 {
                // Escape states with no explicit re-entry transitions.
                // Per spec open question #4, re-entry is currently implicit,
                // so this is a warning rather than an error.
                diag.warn(LoomWarning::EscapeNoReentry { name: name.clone() });
            }
        }
    }
}

/// Generate warnings
fn check_warnings(ir: &WorkflowIR, diag: &mut Diagnostics) {
    // Unused states: states not in any step
    let step_states: HashSet<String> = ir.steps.values()
        .flat_map(|s| vec![s.queue.clone(), s.action.clone()])
        .collect();

    for (name, state) in &ir.states {
        if state.is_terminal() || state.is_escape() {
            continue;
        }
        if !step_states.contains(name) {
            diag.warn(LoomWarning::UnusedState { name: name.clone() });
        }
    }

    // Unused steps: steps not in any phase
    let phase_steps: HashSet<String> = ir.phases.values()
        .flat_map(|p| vec![p.produce_step.clone(), p.gate_step.clone()])
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
                if let StateDef::Action { name, prompt_name: pn, .. } = s {
                    if pn == prompt_name { Some(name.clone()) } else { None }
                } else {
                    None
                }
            }) {
                diag.warn(LoomWarning::SingleOutcomeAction { name: action_name });
            }
        }
    }

    // Symmetric failure routing
    for (prompt_name, prompt) in &ir.prompts {
        if prompt.failure.len() > 1 {
            let targets: HashSet<&String> = prompt.failure.values().collect();
            if targets.len() == 1 {
                let target = targets.into_iter().next().unwrap();
                if let Some(action_name) = ir.states.values().find_map(|s| {
                    if let StateDef::Action { name, prompt_name: pn, .. } = s {
                        if pn == prompt_name { Some(name.clone()) } else { None }
                    } else {
                        None
                    }
                }) {
                    diag.warn(LoomWarning::SymmetricFailureRouting {
                        name: action_name,
                        target: target.clone(),
                    });
                }
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
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/knots_sdlc")
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

        assert!(!diag.has_errors(), "knots_sdlc should validate clean, got {} errors", diag.errors.len());
    }
}
