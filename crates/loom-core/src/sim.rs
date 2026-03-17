use petgraph::visit::EdgeRef;

use crate::graph::{build_graph, EdgeKind};
use crate::ir::WorkflowIR;

/// A transition that can be taken from the current state.
#[derive(Debug, Clone)]
pub struct SimTransition {
    pub label: String,
    pub from: String,
    pub to: String,
    pub kind: TransitionKind,
}

/// The kind of a simulator transition.
#[derive(Debug, Clone, PartialEq)]
pub enum TransitionKind {
    Claim { step: String },
    Outcome { outcome: String, is_success: bool },
    Wildcard,
}

/// The simulator state, tracking current position and history.
#[derive(Debug, Clone)]
pub struct SimState {
    pub current: String,
    pub history: Vec<SimTransition>,
}

/// Create a new simulator state at the entry queue of the workflow.
///
/// The entry queue is the queue of the first step in the first phase
/// of the given profile (or the first phase in the IR if no profile).
pub fn new(ir: &WorkflowIR, profile: Option<&str>) -> Result<SimState, String> {
    let first_phase_name = if let Some(pname) = profile {
        let prof = ir
            .profiles
            .get(pname)
            .ok_or_else(|| format!("profile '{}' not found", pname))?;
        prof.phases
            .first()
            .ok_or_else(|| format!("profile '{}' has no phases", pname))?
            .clone()
    } else {
        ir.phases
            .keys()
            .next()
            .ok_or_else(|| "workflow has no phases".to_string())?
            .clone()
    };

    let phase = ir
        .phases
        .get(&first_phase_name)
        .ok_or_else(|| format!("phase '{}' not found", first_phase_name))?;

    let step = ir
        .steps
        .get(&phase.produce_step)
        .ok_or_else(|| format!("step '{}' not found", phase.produce_step))?;

    Ok(SimState {
        current: step.queue.clone(),
        history: Vec::new(),
    })
}

/// Return all valid transitions from the current state.
pub fn available_transitions(state: &SimState, ir: &WorkflowIR) -> Vec<SimTransition> {
    let graph = build_graph(ir);
    let node_idx = match graph.node_index(&state.current) {
        Some(idx) => idx,
        None => return Vec::new(),
    };

    let mut transitions = Vec::new();

    for edge in graph.graph.edges(node_idx) {
        let target_name = &graph.graph[edge.target()];
        let kind = edge.weight();

        let (label, tk) = match kind {
            EdgeKind::Claim { step } => (
                format!("claim ({}) -> {}", step, target_name),
                TransitionKind::Claim { step: step.clone() },
            ),
            EdgeKind::Outcome {
                outcome,
                is_success,
            } => {
                let tag = if *is_success { "ok" } else { "fail" };
                (
                    format!("[{}] {} -> {}", tag, outcome, target_name),
                    TransitionKind::Outcome {
                        outcome: outcome.clone(),
                        is_success: *is_success,
                    },
                )
            }
            EdgeKind::PhaseLink { phase } => (
                format!("phase link ({}) -> {}", phase, target_name),
                TransitionKind::Claim {
                    step: phase.clone(),
                },
            ),
            EdgeKind::Wildcard => (format!("[*] -> {}", target_name), TransitionKind::Wildcard),
        };

        transitions.push(SimTransition {
            label,
            from: state.current.clone(),
            to: target_name.clone(),
            kind: tk,
        });
    }

    // Sort: claims first, then outcomes, then wildcards
    transitions.sort_by_key(|t| match &t.kind {
        TransitionKind::Claim { .. } => 0,
        TransitionKind::Outcome {
            is_success: true, ..
        } => 1,
        TransitionKind::Outcome {
            is_success: false, ..
        } => 2,
        TransitionKind::Wildcard => 3,
    });

    transitions
}

/// Advance the simulator to the next state.
pub fn apply(state: &mut SimState, transition: &SimTransition) {
    state.current = transition.to.clone();
    state.history.push(transition.clone());
}

/// Check whether the current state is terminal (no outgoing non-wildcard edges,
/// or the state is a terminal/escape state).
pub fn is_terminal(state: &SimState, ir: &WorkflowIR) -> bool {
    match ir.states.get(&state.current) {
        Some(s) => s.is_terminal() || s.is_escape(),
        None => true,
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

    fn load_ir() -> WorkflowIR {
        let input = std::fs::read_to_string(fixture_dir().join("workflow.loom")).unwrap();
        let ast = parse::parse_workflow(&input).unwrap();
        let (ir, _) = lower(&ast, &fixture_dir()).unwrap();
        ir
    }

    #[test]
    fn test_new_creates_state_at_entry_queue() {
        let ir = load_ir();
        let state = new(&ir, None).unwrap();
        assert_eq!(state.current, "ready_for_planning");
        assert!(state.history.is_empty());
    }

    #[test]
    fn test_new_with_profile() {
        let ir = load_ir();
        let state = new(&ir, Some("autopilot_no_planning")).unwrap();
        // autopilot_no_planning skips planning, starts at implementation
        assert_eq!(state.current, "ready_for_implementation");
    }

    #[test]
    fn test_new_unknown_profile_returns_error() {
        let ir = load_ir();
        let result = new(&ir, Some("nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn test_available_transitions_from_queue_returns_claims() {
        let ir = load_ir();
        let state = new(&ir, None).unwrap();
        let transitions = available_transitions(&state, &ir);

        // From a queue, we should have claim edges + wildcard edges
        let claims: Vec<_> = transitions
            .iter()
            .filter(|t| matches!(&t.kind, TransitionKind::Claim { .. }))
            .collect();
        assert!(
            !claims.is_empty(),
            "expected at least one claim transition from queue"
        );
        assert_eq!(claims[0].to, "planning");
    }

    #[test]
    fn test_available_transitions_from_action_returns_outcomes_and_wildcards() {
        let ir = load_ir();
        let state = SimState {
            current: "planning".to_string(),
            history: Vec::new(),
        };
        let transitions = available_transitions(&state, &ir);

        let outcomes: Vec<_> = transitions
            .iter()
            .filter(|t| matches!(&t.kind, TransitionKind::Outcome { .. }))
            .collect();
        let wildcards: Vec<_> = transitions
            .iter()
            .filter(|t| matches!(&t.kind, TransitionKind::Wildcard))
            .collect();

        assert!(
            !outcomes.is_empty(),
            "expected outcome transitions from action"
        );
        assert!(
            !wildcards.is_empty(),
            "expected wildcard transitions from action"
        );
    }

    #[test]
    fn test_apply_advances_state() {
        let ir = load_ir();
        let mut state = new(&ir, None).unwrap();
        let transitions = available_transitions(&state, &ir);

        let claim = transitions
            .iter()
            .find(|t| matches!(&t.kind, TransitionKind::Claim { .. }))
            .expect("should have a claim transition");

        apply(&mut state, claim);
        assert_eq!(state.current, "planning");
        assert_eq!(state.history.len(), 1);
    }

    #[test]
    fn test_terminal_state_detected() {
        let ir = load_ir();
        let state = SimState {
            current: "shipped".to_string(),
            history: Vec::new(),
        };
        assert!(is_terminal(&state, &ir));

        let queue_state = SimState {
            current: "ready_for_planning".to_string(),
            history: Vec::new(),
        };
        assert!(!is_terminal(&queue_state, &ir));
    }

    #[test]
    fn test_escape_state_is_terminal() {
        let ir = load_ir();
        let state = SimState {
            current: "deferred".to_string(),
            history: Vec::new(),
        };
        assert!(is_terminal(&state, &ir));
    }
}
