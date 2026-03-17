use crate::ir::WorkflowIR;
use super::{build_graph, EdgeKind};

/// Output format for graph rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderFormat {
    Mermaid,
    Dot,
    Ascii,
}

/// Render the workflow graph in the specified format
pub fn render(ir: &WorkflowIR, format: RenderFormat) -> String {
    match format {
        RenderFormat::Mermaid => render_mermaid(ir),
        RenderFormat::Dot => render_dot(ir),
        RenderFormat::Ascii => render_ascii(ir),
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

fn render_ascii(ir: &WorkflowIR) -> String {
    let graph = build_graph(ir);
    let mut out = String::new();

    out.push_str(&format!(
        "=== Workflow: {} v{} ===\n\nStates:\n",
        ir.name, ir.version
    ));

    // Group states by type: queues, actions, terminals, escapes
    let groups: &[(&str, fn(&crate::ir::StateDef) -> bool)] = &[
        ("Q", crate::ir::StateDef::is_queue),
        ("A", crate::ir::StateDef::is_action),
        ("T", crate::ir::StateDef::is_terminal),
        ("E", crate::ir::StateDef::is_escape),
    ];

    for &(tag, predicate) in groups {
        for (name, state) in &ir.states {
            if !predicate(state) {
                continue;
            }
            let suffix = format_action_suffix(state);
            out.push_str(&format!(
                "  [{}] {:<30} \"{}\"{}\n",
                tag,
                name,
                state.display_name(),
                suffix
            ));
        }
    }

    out.push_str("\nTransitions:\n");

    // Non-wildcard edges first
    for edge in graph.graph.edge_indices() {
        let weight = &graph.graph[edge];
        if matches!(weight, EdgeKind::Wildcard) {
            continue;
        }
        let (src, dst) = graph.graph.edge_endpoints(edge).unwrap();
        let label = format_edge_label(weight);
        out.push_str(&format!(
            "  {} --[{}]--> {}\n",
            graph.graph[src], label, graph.graph[dst]
        ));
    }

    // Wildcard edges (deduplicated by target)
    for target in &ir.wildcard_targets {
        out.push_str(&format!("  * --[wildcard]--> {}\n", target));
    }

    out
}

fn format_action_suffix(state: &crate::ir::StateDef) -> String {
    match state {
        crate::ir::StateDef::Action {
            action_type,
            executor,
            ..
        } => {
            let kind = match action_type {
                crate::parse::ast::ActionType::Produce(_) => "produce",
                crate::parse::ast::ActionType::Gate(..) => "gate",
            };
            let exec = match executor {
                crate::parse::ast::Executor::Agent => "agent",
                crate::parse::ast::Executor::Human => "human",
            };
            format!(" ({}, {})", kind, exec)
        }
        _ => String::new(),
    }
}

fn format_edge_label(weight: &EdgeKind) -> String {
    match weight {
        EdgeKind::Claim { step } => format!("claim: {}", step),
        EdgeKind::Outcome { outcome, is_success } => {
            let kind = if *is_success { "ok" } else { "fail" };
            format!("{} ({})", outcome, kind)
        }
        EdgeKind::PhaseLink { phase } => format!("phase: {}", phase),
        EdgeKind::Wildcard => "wildcard".to_string(),
    }
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

    #[test]
    fn test_render_ascii() {
        let input = std::fs::read_to_string(fixture_dir().join("workflow.loom")).unwrap();
        let ast = parse::parse_workflow(&input).unwrap();
        let (ir, _) = lower(&ast, &fixture_dir()).unwrap();
        let output = render(&ir, RenderFormat::Ascii);

        // Header
        assert!(output.contains("=== Workflow: knots_sdlc v1 ==="));

        // States section with type tags
        assert!(output.contains("[Q] ready_for_planning"));
        assert!(output.contains("[A] planning"));
        assert!(output.contains("[T] shipped"));
        assert!(output.contains("[E] deferred"));

        // Action states show type and executor
        assert!(output.contains("(produce, agent)"));

        // Transitions section
        assert!(output.contains("Transitions:"));
        assert!(output.contains("--[claim:"));
        assert!(output.contains("--[wildcard]-->"));
    }
}
