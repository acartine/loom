pub mod validate;
pub mod profile;
pub mod render;

use indexmap::IndexMap;
use petgraph::graph::{DiGraph, NodeIndex};

use crate::ir::WorkflowIR;

/// Edge types in the workflow graph
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeKind {
    /// Step transition: queue -> action (on claim)
    Claim { step: String },
    /// Outcome routing: action -> target state
    Outcome { outcome: String, is_success: bool },
    /// Phase link: produce action -> gate queue (implicit)
    PhaseLink { phase: String },
    /// Wildcard: * -> terminal/escape
    Wildcard,
}

/// A built workflow graph
#[derive(Debug)]
pub struct WorkflowGraph {
    pub graph: DiGraph<String, EdgeKind>,
    pub node_indices: IndexMap<String, NodeIndex>,
}

impl WorkflowGraph {
    pub fn node_index(&self, name: &str) -> Option<NodeIndex> {
        self.node_indices.get(name).copied()
    }
}

/// Build a petgraph from the IR
pub fn build_graph(ir: &WorkflowIR) -> WorkflowGraph {
    let mut graph = DiGraph::new();
    let mut node_indices: IndexMap<String, NodeIndex> = IndexMap::new();

    // Add all states as nodes
    for name in ir.states.keys() {
        let idx = graph.add_node(name.clone());
        node_indices.insert(name.clone(), idx);
    }

    // Add step transitions (queue -> action)
    for step in ir.steps.values() {
        if let (Some(&q_idx), Some(&a_idx)) = (node_indices.get(&step.queue), node_indices.get(&step.action)) {
            graph.add_edge(q_idx, a_idx, EdgeKind::Claim { step: step.name.clone() });
        }
    }

    // Add outcome edges from prompts
    for (prompt_name, prompt) in &ir.prompts {
        // Find the action that uses this prompt
        let action_name = ir.states.values()
            .find(|s| {
                if let crate::ir::StateDef::Action { prompt_name: pn, .. } = s {
                    pn == prompt_name
                } else {
                    false
                }
            })
            .map(|s| s.name().to_string());

        if let Some(action_name) = action_name {
            if let Some(&action_idx) = node_indices.get(&action_name) {
                for (outcome, target) in &prompt.success {
                    if let Some(&target_idx) = node_indices.get(target) {
                        graph.add_edge(action_idx, target_idx, EdgeKind::Outcome {
                            outcome: outcome.clone(),
                            is_success: true,
                        });
                    }
                }
                for (outcome, target) in &prompt.failure {
                    if let Some(&target_idx) = node_indices.get(target) {
                        graph.add_edge(action_idx, target_idx, EdgeKind::Outcome {
                            outcome: outcome.clone(),
                            is_success: false,
                        });
                    }
                }
            }
        }
    }

    // Add phase links: produce action success -> gate queue (implicit)
    // Per spec 3.2: if a produce action's success outcomes don't already target
    // the gate queue, the compiler generates an implicit phase link edge.
    for phase in ir.phases.values() {
        if let (Some(produce_step), Some(gate_step)) = (
            ir.steps.get(&phase.produce_step),
            ir.steps.get(&phase.gate_step),
        ) {
            let produce_action = &produce_step.action;
            let gate_queue = &gate_step.queue;

            if let Some(state) = ir.states.get(produce_action) {
                if let crate::ir::StateDef::Action { prompt_name, .. } = state {
                    if let Some(prompt) = ir.prompts.get(prompt_name) {
                        // Check if any success outcome already routes to the gate queue
                        let already_routed = prompt.success.values()
                            .any(|target| target == gate_queue);
                        if !already_routed {
                            // Add implicit phase link from produce action to gate queue
                            if let (Some(&action_idx), Some(&queue_idx)) = (
                                node_indices.get(produce_action),
                                node_indices.get(gate_queue),
                            ) {
                                graph.add_edge(action_idx, queue_idx, EdgeKind::PhaseLink {
                                    phase: phase.name.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Add wildcard transitions
    for target in &ir.wildcard_targets {
        if let Some(&target_idx) = node_indices.get(target) {
            for (name, state) in &ir.states {
                if state.is_terminal() || state.is_escape() {
                    continue;
                }
                if name == target {
                    continue;
                }
                if let Some(&source_idx) = node_indices.get(name) {
                    graph.add_edge(source_idx, target_idx, EdgeKind::Wildcard);
                }
            }
        }
    }

    WorkflowGraph { graph, node_indices }
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
    fn test_build_graph_knots_sdlc() {
        let input = std::fs::read_to_string(fixture_dir().join("workflow.loom")).unwrap();
        let ast = parse::parse_workflow(&input).unwrap();
        let (ir, _) = lower(&ast, &fixture_dir()).unwrap();
        let graph = build_graph(&ir);

        assert_eq!(graph.graph.node_count(), 15);
        // Should have edges: 6 step claims + outcomes from 6 prompts + wildcards
        assert!(graph.graph.edge_count() > 0);
    }
}
