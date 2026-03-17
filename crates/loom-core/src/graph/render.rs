use crate::ir::WorkflowIR;
use super::{build_graph, EdgeKind};

/// Output format for graph rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderFormat {
    Mermaid,
    Dot,
}

/// Render the workflow graph in the specified format
pub fn render(ir: &WorkflowIR, format: RenderFormat) -> String {
    match format {
        RenderFormat::Mermaid => render_mermaid(ir),
        RenderFormat::Dot => render_dot(ir),
    }
}

fn render_mermaid(ir: &WorkflowIR) -> String {
    let graph = build_graph(ir);
    let mut out = String::new();
    out.push_str("stateDiagram-v2\n");

    // Node declarations with display names
    for (name, state) in &ir.states {
        let display = state.display_name();
        if state.is_terminal() {
            out.push_str(&format!("    {} : {}\n", name, display));
            out.push_str(&format!("    {} --> [*]\n", name));
        } else if state.is_escape() {
            out.push_str(&format!("    {} : {} (escape)\n", name, display));
        } else {
            out.push_str(&format!("    {} : {}\n", name, display));
        }
    }

    out.push('\n');

    // Entry point
    for profile in ir.profiles.values() {
        if let Some(first_phase) = profile.phases.first() {
            if let Some(phase) = ir.phases.get(first_phase) {
                if let Some(step) = ir.steps.get(&phase.produce_step) {
                    out.push_str(&format!("    [*] --> {}\n", step.queue));
                    break;
                }
            }
        }
    }

    // Edges
    for edge in graph.graph.edge_indices() {
        let (src, dst) = graph.graph.edge_endpoints(edge).unwrap();
        let src_name = &graph.graph[src];
        let dst_name = &graph.graph[dst];
        let weight = &graph.graph[edge];

        match weight {
            EdgeKind::Claim { step } => {
                out.push_str(&format!("    {} --> {} : claim ({})\n", src_name, dst_name, step));
            }
            EdgeKind::Outcome { outcome, is_success } => {
                let kind = if *is_success { "ok" } else { "fail" };
                out.push_str(&format!("    {} --> {} : {} [{}]\n", src_name, dst_name, outcome, kind));
            }
            EdgeKind::PhaseLink { phase } => {
                out.push_str(&format!("    {} --> {} : phase ({})\n", src_name, dst_name, phase));
            }
            EdgeKind::Wildcard => {
                // Skip wildcard edges in mermaid to avoid clutter
                // They can be shown with a note instead
            }
        }
    }

    out
}

fn render_dot(ir: &WorkflowIR) -> String {
    let graph = build_graph(ir);
    let mut out = String::new();
    out.push_str("digraph workflow {\n");
    out.push_str("    rankdir=TB;\n");
    out.push_str("    node [shape=box, style=rounded];\n\n");

    // Node styling
    for (name, state) in &ir.states {
        let display = state.display_name();
        let attrs = if state.is_terminal() {
            "shape=doublecircle, style=filled, fillcolor=\"#d4edda\""
        } else if state.is_escape() {
            "shape=diamond, style=filled, fillcolor=\"#fff3cd\""
        } else if state.is_queue() {
            "shape=box, style=\"rounded,filled\", fillcolor=\"#cce5ff\""
        } else if state.is_action() {
            "shape=box, style=\"rounded,filled\", fillcolor=\"#e2e3e5\""
        } else {
            "shape=box"
        };
        out.push_str(&format!("    {} [label=\"{}\", {}];\n", name, display, attrs));
    }

    out.push('\n');

    // Edges
    for edge in graph.graph.edge_indices() {
        let (src, dst) = graph.graph.edge_endpoints(edge).unwrap();
        let src_name = &graph.graph[src];
        let dst_name = &graph.graph[dst];
        let weight = &graph.graph[edge];

        let (label, style) = match weight {
            EdgeKind::Claim { step } => (format!("claim ({})", step), ""),
            EdgeKind::Outcome { outcome, is_success } => {
                let color = if *is_success { ", color=green" } else { ", color=red" };
                (outcome.clone(), color)
            }
            EdgeKind::PhaseLink { phase } => (format!("phase ({})", phase), ", style=dashed"),
            EdgeKind::Wildcard => ("*".to_string(), ", style=dotted, color=gray"),
        };

        out.push_str(&format!(
            "    {} -> {} [label=\"{}\"{}];\n",
            src_name, dst_name, label, style
        ));
    }

    out.push_str("}\n");
    out
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
    fn test_render_mermaid() {
        let input = std::fs::read_to_string(fixture_dir().join("workflow.loom")).unwrap();
        let ast = parse::parse_workflow(&input).unwrap();
        let (ir, _) = lower(&ast, &fixture_dir()).unwrap();
        let output = render(&ir, RenderFormat::Mermaid);
        assert!(output.contains("stateDiagram-v2"));
        assert!(output.contains("ready_for_planning"));
    }

    #[test]
    fn test_render_dot() {
        let input = std::fs::read_to_string(fixture_dir().join("workflow.loom")).unwrap();
        let ast = parse::parse_workflow(&input).unwrap();
        let (ir, _) = lower(&ast, &fixture_dir()).unwrap();
        let output = render(&ir, RenderFormat::Dot);
        assert!(output.contains("digraph workflow"));
        assert!(output.contains("ready_for_planning"));
    }
}
