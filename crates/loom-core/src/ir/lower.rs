use std::path::Path;
use indexmap::IndexMap;

use crate::error::{Diagnostics, LoomError};
use crate::parse::ast::*;
use crate::parse;
use crate::prompt;
use super::*;

/// Lower an AST workflow into the IR.
/// `workflow_dir` is the directory containing the workflow files (for resolving includes/prompts).
pub fn lower(
    ast: &Workflow,
    workflow_dir: &Path,
) -> Result<(WorkflowIR, Diagnostics), Vec<LoomError>> {
    let mut diag = Diagnostics::new();
    let mut states: IndexMap<String, StateDef> = IndexMap::new();
    let mut steps: IndexMap<String, StepDef> = IndexMap::new();
    let mut phases: IndexMap<String, PhaseDef> = IndexMap::new();
    let mut profiles: IndexMap<String, ProfileDef> = IndexMap::new();
    let mut wildcard_targets: Vec<String> = Vec::new();
    let mut prompts: IndexMap<String, prompt::PromptFile> = IndexMap::new();
    let mut action_prompt_refs: Vec<(String, String)> = Vec::new();

    // Phase 1: Register all declarations
    for decl in &ast.declarations {
        match decl {
            Declaration::Queue(q) => {
                if states.contains_key(&q.name) {
                    diag.error(LoomError::DuplicateIdentifier { name: q.name.clone() });
                } else {
                    states.insert(q.name.clone(), StateDef::Queue {
                        name: q.name.clone(),
                        display_name: q.display_name.clone(),
                    });
                }
            }
            Declaration::Action(a) => {
                if states.contains_key(&a.name) {
                    diag.error(LoomError::DuplicateIdentifier { name: a.name.clone() });
                } else {
                    let executor = match &a.action_type {
                        ActionType::Produce(e) | ActionType::Gate(_, e) => *e,
                    };
                    states.insert(a.name.clone(), StateDef::Action {
                        name: a.name.clone(),
                        display_name: a.display_name.clone(),
                        action_type: a.action_type.clone(),
                        prompt_name: a.prompt.clone(),
                        constraints: a.constraints.clone(),
                        executor,
                    });
                    action_prompt_refs.push((a.name.clone(), a.prompt.clone()));
                }
            }
            Declaration::Terminal(t) => {
                if states.contains_key(&t.name) {
                    diag.error(LoomError::DuplicateIdentifier { name: t.name.clone() });
                } else {
                    states.insert(t.name.clone(), StateDef::Terminal {
                        name: t.name.clone(),
                        display_name: t.display_name.clone().unwrap_or_else(|| t.name.clone()),
                    });
                }
            }
            Declaration::Escape(e) => {
                if states.contains_key(&e.name) {
                    diag.error(LoomError::DuplicateIdentifier { name: e.name.clone() });
                } else {
                    states.insert(e.name.clone(), StateDef::Escape {
                        name: e.name.clone(),
                        display_name: e.display_name.clone().unwrap_or_else(|| e.name.clone()),
                    });
                }
            }
            Declaration::Step(s) => {
                if steps.contains_key(&s.name) {
                    diag.error(LoomError::DuplicateIdentifier { name: s.name.clone() });
                } else {
                    steps.insert(s.name.clone(), StepDef {
                        name: s.name.clone(),
                        queue: s.queue.clone(),
                        action: s.action.clone(),
                    });
                }
            }
            Declaration::Phase(p) => {
                if phases.contains_key(&p.name) {
                    diag.error(LoomError::DuplicateIdentifier { name: p.name.clone() });
                } else {
                    phases.insert(p.name.clone(), PhaseDef {
                        name: p.name.clone(),
                        produce_step: p.produce_step.clone(),
                        gate_step: p.gate_step.clone(),
                    });
                }
            }
            Declaration::Profile(p) => {
                register_profile(p, &mut profiles, &mut diag);
            }
            Declaration::Include(inc) => {
                let path = workflow_dir.join(&inc.path);
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        match parse::parse_profile_file(&content) {
                            Ok(profile_decl) => {
                                register_profile(&profile_decl, &mut profiles, &mut diag);
                            }
                            Err(e) => diag.error(e),
                        }
                    }
                    Err(e) => diag.error(LoomError::Io(e)),
                }
            }
            Declaration::WildcardTransition(w) => {
                wildcard_targets.push(w.target.clone());
            }
        }
    }

    // Phase 2: Load prompts
    for (_action_name, prompt_name) in &action_prompt_refs {
        if prompts.contains_key(prompt_name) {
            continue;
        }
        let prompt_path = workflow_dir.join("prompts").join(format!("{}.md", prompt_name));
        match prompt::load_prompt(&prompt_path) {
            Ok(pf) => {
                prompts.insert(prompt_name.clone(), pf);
            }
            Err(_) => {
                diag.error(LoomError::MissingPrompt { name: prompt_name.clone() });
            }
        }
    }

    // Phase 3: Resolve references
    // Check step references
    for step in steps.values() {
        if !states.contains_key(&step.queue) {
            diag.error(LoomError::UnresolvedReference {
                name: step.queue.clone(),
                context: format!("step '{}' queue reference", step.name),
            });
        } else if !states.get(&step.queue).map_or(false, |s| s.is_queue()) {
            diag.error(LoomError::StepTypeMismatch {
                message: format!("step '{}': left side '{}' is not a queue state", step.name, step.queue),
            });
        }

        if !states.contains_key(&step.action) {
            diag.error(LoomError::UnresolvedReference {
                name: step.action.clone(),
                context: format!("step '{}' action reference", step.name),
            });
        } else if !states.get(&step.action).map_or(false, |s| s.is_action()) {
            diag.error(LoomError::StepTypeMismatch {
                message: format!("step '{}': right side '{}' is not an action state", step.name, step.action),
            });
        }
    }

    // Check phase references
    for phase in phases.values() {
        if !steps.contains_key(&phase.produce_step) {
            diag.error(LoomError::UnresolvedReference {
                name: phase.produce_step.clone(),
                context: format!("phase '{}' produce step reference", phase.name),
            });
        } else {
            let step = &steps[&phase.produce_step];
            if let Some(action_state) = states.get(&step.action) {
                if action_state.is_gate() {
                    diag.error(LoomError::PhaseTypeMismatch {
                        message: format!(
                            "phase '{}': produce step '{}' has a gate action",
                            phase.name, phase.produce_step
                        ),
                    });
                }
            }
        }

        if !steps.contains_key(&phase.gate_step) {
            diag.error(LoomError::UnresolvedReference {
                name: phase.gate_step.clone(),
                context: format!("phase '{}' gate step reference", phase.name),
            });
        } else {
            let step = &steps[&phase.gate_step];
            if let Some(action_state) = states.get(&step.action) {
                if action_state.is_produce() {
                    diag.error(LoomError::PhaseTypeMismatch {
                        message: format!(
                            "phase '{}': gate step '{}' has a produce action",
                            phase.name, phase.gate_step
                        ),
                    });
                }
            }
        }
    }

    // Check profile references
    for profile in profiles.values() {
        for phase_name in &profile.phases {
            if !phases.contains_key(phase_name) {
                diag.error(LoomError::ProfileError {
                    message: format!(
                        "profile '{}' references undefined phase '{}'",
                        profile.name, phase_name
                    ),
                });
            }
        }

        // Check override references
        let profile_actions: Vec<String> = profile.phases.iter()
            .filter_map(|pn| phases.get(pn))
            .flat_map(|phase| {
                let mut actions = Vec::new();
                if let Some(step) = steps.get(&phase.produce_step) {
                    actions.push(step.action.clone());
                }
                if let Some(step) = steps.get(&phase.gate_step) {
                    actions.push(step.action.clone());
                }
                actions
            })
            .collect();

        for (action_name, _) in &profile.overrides {
            if !states.contains_key(action_name) {
                diag.error(LoomError::UnresolvedReference {
                    name: action_name.clone(),
                    context: format!("profile '{}' override", profile.name),
                });
            } else if !profile_actions.contains(action_name) {
                diag.error(LoomError::InvalidOverride {
                    action: action_name.clone(),
                });
            }
        }
    }

    // Check wildcard targets
    for target in &wildcard_targets {
        if !states.contains_key(target) {
            diag.error(LoomError::UnresolvedReference {
                name: target.clone(),
                context: "wildcard transition target".to_string(),
            });
        }
    }

    // Check outcome targets
    for (prompt_name, pf) in &prompts {
        for (outcome, target) in pf.success.iter().chain(pf.failure.iter()) {
            if !states.contains_key(target) {
                diag.error(LoomError::InvalidOutcomeTarget {
                    prompt: prompt_name.clone(),
                    outcome: outcome.clone(),
                    target: target.clone(),
                });
            }
        }
    }

    // Check parameter consistency
    for (prompt_name, pf) in &prompts {
        for param in &pf.body_params {
            if !pf.params.contains_key(param) {
                diag.error(LoomError::UndeclaredParam {
                    prompt: prompt_name.clone(),
                    param: param.clone(),
                });
            }
        }
    }

    if diag.has_errors() {
        return Err(diag.errors);
    }

    Ok((WorkflowIR {
        name: ast.name.clone(),
        version: ast.version,
        states,
        steps,
        phases,
        profiles,
        wildcard_targets,
        prompts,
    }, diag))
}

fn register_profile(
    p: &crate::parse::ast::ProfileDecl,
    profiles: &mut IndexMap<String, ProfileDef>,
    diag: &mut Diagnostics,
) {
    if profiles.contains_key(&p.name) {
        diag.error(LoomError::DuplicateIdentifier { name: p.name.clone() });
        return;
    }

    let mut def = ProfileDef {
        name: p.name.clone(),
        display_name: p.display_name.clone(),
        description: None,
        phases: Vec::new(),
        output: None,
        overrides: IndexMap::new(),
    };

    for field in &p.fields {
        match field {
            ProfileField::Phases(phases) => def.phases = phases.clone(),
            ProfileField::Output(kind) => def.output = Some(*kind),
            ProfileField::Override(o) => {
                def.overrides.insert(o.action.clone(), o.executor);
            }
            ProfileField::Description(d) => def.description = Some(d.clone()),
        }
    }

    profiles.insert(p.name.clone(), def);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/knots_sdlc")
    }

    #[test]
    fn test_lower_knots_sdlc() {
        let input = std::fs::read_to_string(fixture_dir().join("workflow.loom")).unwrap();
        let ast = crate::parse::parse_workflow(&input).unwrap();
        let (ir, diag) = lower(&ast, &fixture_dir()).unwrap();

        assert_eq!(ir.name, "knots_sdlc");
        assert_eq!(ir.version, 1);
        assert_eq!(ir.states.len(), 15); // 6 queues + 6 actions + 2 terminals + 1 escape
        assert_eq!(ir.steps.len(), 6);
        assert_eq!(ir.phases.len(), 3);
        assert_eq!(ir.profiles.len(), 6);
        assert_eq!(ir.wildcard_targets.len(), 2);
        assert_eq!(ir.prompts.len(), 6);
        assert!(!diag.has_errors());
    }
}
