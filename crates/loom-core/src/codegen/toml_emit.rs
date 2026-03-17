use crate::ir::{StateDef, WorkflowIR};
use crate::parse::ast::{ActionType, Executor, GateKind, OutputKind};

/// Emit the workflow IR as a TOML interchange format
pub fn emit_toml(ir: &WorkflowIR) -> String {
    let mut out = String::new();

    out.push_str(&format!("[workflow]\nname = \"{}\"\nversion = {}\n\n", ir.name, ir.version));

    // States
    for (name, state) in &ir.states {
        out.push_str(&format!("[states.{}]\n", name));
        out.push_str(&format!("display_name = \"{}\"\n", state.display_name()));
        let kind = match state {
            StateDef::Queue { .. } => "queue",
            StateDef::Action { .. } => "action",
            StateDef::Terminal { .. } => "terminal",
            StateDef::Escape { .. } => "escape",
        };
        out.push_str(&format!("kind = \"{}\"\n", kind));

        if let StateDef::Action { action_type, executor, prompt_name, constraints, .. } = state {
            let (at, gk) = match action_type {
                ActionType::Produce(_) => ("produce", None),
                ActionType::Gate(kind, _) => ("gate", Some(kind)),
            };
            out.push_str(&format!("action_type = \"{}\"\n", at));
            if let Some(gk) = gk {
                let gk_str = match gk {
                    GateKind::Approve => "approve",
                    GateKind::Auth => "auth",
                    GateKind::Review => "review",
                };
                out.push_str(&format!("gate_kind = \"{}\"\n", gk_str));
            }
            let exec = match executor {
                Executor::Agent => "agent",
                Executor::Human => "human",
            };
            out.push_str(&format!("executor = \"{}\"\n", exec));
            out.push_str(&format!("prompt = \"{}\"\n", prompt_name));
            if !constraints.is_empty() {
                out.push_str("constraints = [");
                for c in constraints {
                    let cs = match c {
                        crate::parse::ast::Constraint::ReadOnly => "read_only",
                        crate::parse::ast::Constraint::NoGitWrite => "no_git_write",
                        crate::parse::ast::Constraint::MetadataOnly => "metadata_only",
                    };
                    out.push_str(&format!("\"{}\", ", cs));
                }
                out.push_str("]\n");
            }
        }
        out.push('\n');
    }

    // Steps
    for step in ir.steps.values() {
        out.push_str(&format!("[steps.{}]\n", step.name));
        out.push_str(&format!("queue = \"{}\"\n", step.queue));
        out.push_str(&format!("action = \"{}\"\n\n", step.action));
    }

    // Phases
    for phase in ir.phases.values() {
        out.push_str(&format!("[phases.{}]\n", phase.name));
        out.push_str(&format!("produce = \"{}\"\n", phase.produce_step));
        out.push_str(&format!("gate = \"{}\"\n\n", phase.gate_step));
    }

    // Profiles
    for profile in ir.profiles.values() {
        out.push_str(&format!("[profiles.{}]\n", profile.name));
        if let Some(desc) = &profile.description {
            out.push_str(&format!("description = \"{}\"\n", desc));
        }
        out.push_str("phases = [");
        for p in &profile.phases {
            out.push_str(&format!("\"{}\", ", p));
        }
        out.push_str("]\n");
        if let Some(output) = &profile.output {
            let o = match output {
                OutputKind::Local => "local",
                OutputKind::Remote => "remote",
                OutputKind::RemoteMain => "remote_main",
                OutputKind::Pr => "pr",
            };
            out.push_str(&format!("output = \"{}\"\n", o));
        }
        if !profile.overrides.is_empty() {
            out.push_str("[profiles.");
            out.push_str(&profile.name);
            out.push_str(".overrides]\n");
            for (action, executor) in &profile.overrides {
                let exec = match executor {
                    Executor::Agent => "agent",
                    Executor::Human => "human",
                };
                out.push_str(&format!("{} = \"{}\"\n", action, exec));
            }
        }
        out.push('\n');
    }

    // Outcomes
    for (prompt_name, prompt) in &ir.prompts {
        out.push_str(&format!("[prompts.{}]\n", prompt_name));
        out.push_str("accept = [");
        for a in &prompt.accept {
            let escaped = a.replace('"', "\\\"");
            out.push_str(&format!("\"{}\", ", escaped));
        }
        out.push_str("]\n");

        if !prompt.success.is_empty() {
            out.push_str(&format!("[prompts.{}.success]\n", prompt_name));
            for (outcome, target) in &prompt.success {
                out.push_str(&format!("{} = \"{}\"\n", outcome, target));
            }
        }
        if !prompt.failure.is_empty() {
            out.push_str(&format!("[prompts.{}.failure]\n", prompt_name));
            for (outcome, target) in &prompt.failure {
                out.push_str(&format!("{} = \"{}\"\n", outcome, target));
            }
        }
        out.push('\n');
    }

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
    fn test_emit_toml() {
        let input = std::fs::read_to_string(fixture_dir().join("workflow.loom")).unwrap();
        let ast = parse::parse_workflow(&input).unwrap();
        let (ir, _) = lower(&ast, &fixture_dir()).unwrap();
        let toml_out = emit_toml(&ir);

        assert!(toml_out.contains("[workflow]"));
        assert!(toml_out.contains("name = \"knots_sdlc\""));
        assert!(toml_out.contains("[states.ready_for_planning]"));
        assert!(toml_out.contains("[profiles.autopilot]"));
    }
}
