use indexmap::IndexMap;
use std::path::Path;

use super::*;
use crate::error::{Diagnostics, LoomError};
use crate::parse;
use crate::parse::ast::*;
use crate::prompt;

/// Lower an AST workflow into the IR.
/// `workflow_dir` is the directory containing the workflow files (for resolving includes/prompts).
/// `default_profile` is from loom.toml config.
pub fn lower(
    ast: &Workflow,
    workflow_dir: &Path,
) -> Result<(WorkflowIR, Diagnostics), Vec<LoomError>> {
    lower_with_config(ast, workflow_dir, None)
}

/// Lower with optional config metadata.
pub fn lower_with_config(
    ast: &Workflow,
    workflow_dir: &Path,
    default_profile: Option<String>,
) -> Result<(WorkflowIR, Diagnostics), Vec<LoomError>> {
    let mut diag = Diagnostics::new();
    let mut states: IndexMap<String, StateDef> = IndexMap::new();
    let mut steps: IndexMap<String, StepDef> = IndexMap::new();
    let mut phases: IndexMap<String, PhaseDef> = IndexMap::new();
    let mut profiles: IndexMap<String, ProfileDef> = IndexMap::new();
    let mut wildcard_targets: Vec<String> = Vec::new();
    let mut prompts: IndexMap<String, prompt::PromptFile> = IndexMap::new();
    let mut action_prompt_refs: Vec<(String, String)> = Vec::new();
    let mut synthesized_queues: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    // Phase 1: Register all declarations
    for decl in &ast.declarations {
        match decl {
            Declaration::Queue(q) => {
                if states.contains_key(&q.name) {
                    diag.error(LoomError::DuplicateIdentifier {
                        name: q.name.clone(),
                    });
                } else {
                    let display_name = q
                        .display_name
                        .clone()
                        .unwrap_or_else(|| crate::snake_to_title_case(&q.name));
                    states.insert(
                        q.name.clone(),
                        StateDef::Queue {
                            name: q.name.clone(),
                            display_name,
                        },
                    );
                }
            }
            Declaration::Action(a) => {
                if states.contains_key(&a.name) {
                    diag.error(LoomError::DuplicateIdentifier {
                        name: a.name.clone(),
                    });
                } else {
                    let executor = match &a.action_type {
                        ActionType::Produce(e) | ActionType::Gate(_, e) => *e,
                    };
                    let display_name = a
                        .display_name
                        .clone()
                        .unwrap_or_else(|| crate::snake_to_title_case(&a.name));
                    let prompt_name = a.prompt.clone().unwrap_or_else(|| a.name.clone());
                    states.insert(
                        a.name.clone(),
                        StateDef::Action {
                            name: a.name.clone(),
                            display_name,
                            action_type: a.action_type.clone(),
                            prompt_name: prompt_name.clone(),
                            output: a.output.clone(),
                            constraints: a.constraints.clone(),
                            executor,
                        },
                    );
                    action_prompt_refs.push((a.name.clone(), prompt_name));
                }
            }
            Declaration::Terminal(t) => {
                if states.contains_key(&t.name) {
                    diag.error(LoomError::DuplicateIdentifier {
                        name: t.name.clone(),
                    });
                } else {
                    states.insert(
                        t.name.clone(),
                        StateDef::Terminal {
                            name: t.name.clone(),
                            display_name: t
                                .display_name
                                .clone()
                                .unwrap_or_else(|| crate::snake_to_title_case(&t.name)),
                        },
                    );
                }
            }
            Declaration::Escape(e) => {
                if states.contains_key(&e.name) {
                    diag.error(LoomError::DuplicateIdentifier {
                        name: e.name.clone(),
                    });
                } else {
                    states.insert(
                        e.name.clone(),
                        StateDef::Escape {
                            name: e.name.clone(),
                            display_name: e
                                .display_name
                                .clone()
                                .unwrap_or_else(|| crate::snake_to_title_case(&e.name)),
                        },
                    );
                }
            }
            Declaration::Step(s) => {
                let queue_name = match &s.queue {
                    Some(q) => q.clone(),
                    None => format!("ready_for_{}", s.action),
                };

                // Synthesize queue state if using shorthand form
                if s.queue.is_none() {
                    if states.contains_key(&queue_name) && !synthesized_queues.contains(&queue_name)
                    {
                        diag.error(LoomError::DuplicateIdentifier {
                            name: queue_name.clone(),
                        });
                    } else if !states.contains_key(&queue_name) {
                        let queue_display =
                            format!("Ready for {}", crate::snake_to_title_case(&s.action));
                        states.insert(
                            queue_name.clone(),
                            StateDef::Queue {
                                name: queue_name.clone(),
                                display_name: queue_display,
                            },
                        );
                        synthesized_queues.insert(queue_name.clone());
                    }
                }

                if steps.contains_key(&s.name) {
                    diag.error(LoomError::DuplicateIdentifier {
                        name: s.name.clone(),
                    });
                } else {
                    steps.insert(
                        s.name.clone(),
                        StepDef {
                            name: s.name.clone(),
                            queue: queue_name,
                            action: s.action.clone(),
                        },
                    );
                }
            }
            Declaration::Phase(p) => {
                if phases.contains_key(&p.name) {
                    diag.error(LoomError::DuplicateIdentifier {
                        name: p.name.clone(),
                    });
                } else {
                    phases.insert(
                        p.name.clone(),
                        PhaseDef {
                            name: p.name.clone(),
                            produce_step: p.produce_step.clone(),
                            gate_step: p.gate_step.clone(),
                        },
                    );
                }
            }
            Declaration::Profile(p) => {
                register_profile(p, &mut profiles, &mut diag);
            }
            Declaration::Include(inc) => {
                let path = workflow_dir.join(&inc.path);
                match std::fs::read_to_string(&path) {
                    Ok(content) => match parse::parse_profile_file(&content) {
                        Ok(profile_decl) => {
                            register_profile(&profile_decl, &mut profiles, &mut diag);
                        }
                        Err(e) => diag.error(e),
                    },
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
        let prompt_path = workflow_dir
            .join("prompts")
            .join(format!("{}.md", prompt_name));
        match prompt::load_prompt(&prompt_path) {
            Ok(pf) => {
                prompts.insert(prompt_name.clone(), pf);
            }
            Err(_) => {
                diag.error(LoomError::MissingPrompt {
                    name: prompt_name.clone(),
                });
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
        } else if !states.get(&step.queue).is_some_and(|s| s.is_queue()) {
            diag.error(LoomError::StepTypeMismatch {
                message: format!(
                    "step '{}': left side '{}' is not a queue state",
                    step.name, step.queue
                ),
            });
        }

        if !states.contains_key(&step.action) {
            diag.error(LoomError::UnresolvedReference {
                name: step.action.clone(),
                context: format!("step '{}' action reference", step.name),
            });
        } else if !states.get(&step.action).is_some_and(|s| s.is_action()) {
            diag.error(LoomError::StepTypeMismatch {
                message: format!(
                    "step '{}': right side '{}' is not an action state",
                    step.name, step.action
                ),
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
        let profile_actions: Vec<String> = profile
            .phases
            .iter()
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
    // `output` and `output_hint` are implicit params for produce actions —
    // they are injected by the runtime from the action's output declaration.
    let produce_prompts: std::collections::HashSet<&str> = action_prompt_refs
        .iter()
        .filter(|(action_name, _)| states.get(action_name).is_some_and(|s| s.is_produce()))
        .map(|(_, prompt_name)| prompt_name.as_str())
        .collect();

    for (prompt_name, pf) in &prompts {
        let is_produce = produce_prompts.contains(prompt_name.as_str());
        for param in &pf.body_params {
            if is_produce && (param == "output" || param == "output_hint") {
                continue;
            }
            if !pf.params.contains_key(param) {
                diag.error(LoomError::UndeclaredParam {
                    prompt: prompt_name.clone(),
                    param: param.clone(),
                });
            }
        }

        // Semantic validation of prompt parameters
        for (param_name, param_def) in &pf.params {
            // Enum params must have values
            if param_def.param_type == crate::prompt::ParamType::Enum && param_def.values.is_empty()
            {
                diag.error(LoomError::ParamValidation {
                    prompt: prompt_name.clone(),
                    param: param_name.clone(),
                    message: "enum parameter must have 'values' list".to_string(),
                });
            }

            // Default must be in enum values
            if param_def.param_type == crate::prompt::ParamType::Enum {
                if let Some(ref default) = param_def.default {
                    if !param_def.values.contains(default) {
                        diag.error(LoomError::ParamValidation {
                            prompt: prompt_name.clone(),
                            param: param_name.clone(),
                            message: format!(
                                "default '{}' is not in enum values {:?}",
                                default, param_def.values
                            ),
                        });
                    }
                }
            }

            // Bool defaults must be "true" or "false"
            if param_def.param_type == crate::prompt::ParamType::Bool {
                if let Some(ref default) = param_def.default {
                    if default != "true" && default != "false" {
                        diag.error(LoomError::ParamValidation {
                            prompt: prompt_name.clone(),
                            param: param_name.clone(),
                            message: format!(
                                "bool parameter default must be 'true' or 'false', got '{}'",
                                default
                            ),
                        });
                    }
                }
            }

            // Int defaults must parse as integers
            if param_def.param_type == crate::prompt::ParamType::Int {
                if let Some(ref default) = param_def.default {
                    if default.parse::<i64>().is_err() {
                        diag.error(LoomError::ParamValidation {
                            prompt: prompt_name.clone(),
                            param: param_name.clone(),
                            message: format!(
                                "int parameter default must be a valid integer, got '{}'",
                                default
                            ),
                        });
                    }
                }
            }
        }
    }

    // Validate default_profile if set
    if let Some(ref dp) = default_profile {
        if !profiles.contains_key(dp) {
            diag.error(LoomError::InvalidDefaultProfile { name: dp.clone() });
        }
    }

    if diag.has_errors() {
        return Err(diag.errors);
    }

    Ok((
        WorkflowIR {
            name: ast.name.clone(),
            version: ast.version,
            default_profile,
            states,
            steps,
            phases,
            profiles,
            wildcard_targets,
            prompts,
        },
        diag,
    ))
}

fn register_profile(
    p: &crate::parse::ast::ProfileDecl,
    profiles: &mut IndexMap<String, ProfileDef>,
    diag: &mut Diagnostics,
) {
    if profiles.contains_key(&p.name) {
        diag.error(LoomError::DuplicateIdentifier {
            name: p.name.clone(),
        });
        return;
    }

    let mut def = ProfileDef {
        name: p.name.clone(),
        display_name: p.display_name.clone(),
        description: None,
        phases: Vec::new(),
        overrides: IndexMap::new(),
    };

    for field in &p.fields {
        match field {
            ProfileField::Phases(phases) => def.phases = phases.clone(),
            ProfileField::Override(o) => {
                def.overrides.insert(
                    o.action.clone(),
                    ActionOverride {
                        executor: o.executor,
                        output: o.output.clone(),
                    },
                );
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
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/knots_sdlc")
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

    /// Helper: parse and lower an inline workflow string with no prompts dir
    fn lower_inline(src: &str) -> Result<(WorkflowIR, crate::error::Diagnostics), Vec<LoomError>> {
        let ast = crate::parse::parse_workflow(src).unwrap();
        // Use a temp dir with no prompts so prompt loading is skipped for actions
        let tmp = std::env::temp_dir().join("loom_test_empty");
        let _ = std::fs::create_dir_all(&tmp);
        lower(&ast, &tmp)
    }

    #[test]
    fn test_duplicate_identifier() {
        let src = r#"
            workflow test v1 {
                queue q1 "Queue 1"
                queue q1 "Queue 1 again"
            }
        "#;
        let err = lower_inline(src).unwrap_err();
        assert!(err
            .iter()
            .any(|e| matches!(e, LoomError::DuplicateIdentifier { name } if name == "q1")));
    }

    #[test]
    fn test_unresolved_step_queue() {
        let src = r#"
            workflow test v1 {
                action a1 "Action" { produce agent prompt a1 }
                step s1 { nonexistent -> a1 }
            }
        "#;
        let err = lower_inline(src).unwrap_err();
        assert!(err.iter().any(
            |e| matches!(e, LoomError::UnresolvedReference { name, .. } if name == "nonexistent")
        ));
    }

    #[test]
    fn test_step_type_mismatch_queue_not_queue() {
        let src = r#"
            workflow test v1 {
                action a1 "Action" { produce agent prompt a1 }
                action a2 "Action2" { produce agent prompt a2 }
                step s1 { a1 -> a2 }
            }
        "#;
        let err = lower_inline(src).unwrap_err();
        assert!(err
            .iter()
            .any(|e| matches!(e, LoomError::StepTypeMismatch { .. })));
    }

    #[test]
    fn test_invalid_override_action_not_in_phases() {
        let src = r#"
            workflow test v1 {
                queue q1 "Q1"
                queue q2 "Q2"
                action a1 "A1" { produce agent prompt a1 }
                action a2 "A2" { gate review agent prompt a2 }
                action a3 "A3" { produce agent prompt a3 }
                step s1 { q1 -> a1 }
                step s2 { q2 -> a2 }
                phase p1 { produce s1 gate s2 }
                profile test_profile {
                    phases [p1]
                    override a3 { executor human }
                }
            }
        "#;
        let err = lower_inline(src).unwrap_err();
        assert!(err
            .iter()
            .any(|e| matches!(e, LoomError::InvalidOverride { action } if action == "a3")));
    }

    #[test]
    fn test_invalid_default_profile() {
        let src = r#"
            workflow test v1 {
                queue q1 "Q1"
                terminal done
                profile real_profile {
                    phases []
                }
            }
        "#;
        let ast = crate::parse::parse_workflow(src).unwrap();
        let tmp = std::env::temp_dir().join("loom_test_empty");
        let _ = std::fs::create_dir_all(&tmp);
        let err = lower_with_config(&ast, &tmp, Some("nonexistent".to_string())).unwrap_err();
        assert!(err.iter().any(
            |e| matches!(e, LoomError::InvalidDefaultProfile { name } if name == "nonexistent")
        ));
    }

    #[test]
    fn test_enum_param_missing_values() {
        // Create a temp prompt file with an enum param but no values
        let tmp = std::env::temp_dir().join("loom_test_bad_param");
        let prompts_dir = tmp.join("prompts");
        let _ = std::fs::create_dir_all(&prompts_dir);
        std::fs::write(
            prompts_dir.join("work.md"),
            r#"---
accept: []
success:
  done: done
failure: {}
params:
  color:
    type: enum
    description: Pick a color
---
Do the thing.
"#,
        )
        .unwrap();

        let src = r#"
            workflow test v1 {
                queue q1 "Q1"
                action work "Work" { produce agent prompt work }
                terminal done
                step s1 { q1 -> work }
            }
        "#;
        let ast = crate::parse::parse_workflow(src).unwrap();
        let err = lower(&ast, &tmp).unwrap_err();
        assert!(err
            .iter()
            .any(|e| matches!(e, LoomError::ParamValidation { param, .. } if param == "color")));
    }

    #[test]
    fn test_phase_type_mismatch() {
        let src = r#"
            workflow test v1 {
                queue q1 "Q1"
                queue q2 "Q2"
                action a1 "A1" { produce agent prompt a1 }
                action a2 "A2" { produce agent prompt a2 }
                step s1 { q1 -> a1 }
                step s2 { q2 -> a2 }
                phase p1 { produce s1 gate s2 }
            }
        "#;
        let err = lower_inline(src).unwrap_err();
        // s2's action (a2) is produce, but it's used as gate
        assert!(err
            .iter()
            .any(|e| matches!(e, LoomError::PhaseTypeMismatch { .. })));
    }

    #[test]
    fn test_unresolved_step_action() {
        let src = r#"
            workflow test v1 {
                queue q1 "Q1"
                step s1 { q1 -> nonexistent }
            }
        "#;
        let err = lower_inline(src).unwrap_err();
        assert!(err.iter().any(|e| matches!(
            e, LoomError::UnresolvedReference { name, .. } if name == "nonexistent"
        )));
    }

    #[test]
    fn test_step_type_mismatch_action_not_action() {
        let src = r#"
            workflow test v1 {
                queue q1 "Q1"
                queue q2 "Q2"
                step s1 { q1 -> q2 }
            }
        "#;
        let err = lower_inline(src).unwrap_err();
        assert!(err
            .iter()
            .any(|e| matches!(e, LoomError::StepTypeMismatch { .. })));
    }

    #[test]
    fn test_unresolved_phase_produce_step() {
        let src = r#"
            workflow test v1 {
                queue q1 "Q1"
                action a1 "A1" { gate review agent prompt a1 }
                step s1 { q1 -> a1 }
                phase p1 { produce nonexistent gate s1 }
            }
        "#;
        let err = lower_inline(src).unwrap_err();
        assert!(err.iter().any(|e| matches!(
            e, LoomError::UnresolvedReference { name, .. } if name == "nonexistent"
        )));
    }

    #[test]
    fn test_unresolved_phase_gate_step() {
        let src = r#"
            workflow test v1 {
                queue q1 "Q1"
                action a1 "A1" { produce agent prompt a1 }
                step s1 { q1 -> a1 }
                phase p1 { produce s1 gate nonexistent }
            }
        "#;
        let err = lower_inline(src).unwrap_err();
        assert!(err.iter().any(|e| matches!(
            e, LoomError::UnresolvedReference { name, .. } if name == "nonexistent"
        )));
    }

    #[test]
    fn test_profile_undefined_phase() {
        let src = r#"
            workflow test v1 {
                queue q1 "Q1"
                profile p1 {
                    phases [nonexistent_phase]
                }
            }
        "#;
        let err = lower_inline(src).unwrap_err();
        assert!(err
            .iter()
            .any(|e| matches!(e, LoomError::ProfileError { .. })));
    }

    #[test]
    fn test_unresolved_wildcard_target() {
        let src = r#"
            workflow test v1 {
                queue q1 "Q1"
                * -> nonexistent
            }
        "#;
        let err = lower_inline(src).unwrap_err();
        assert!(err.iter().any(|e| matches!(
            e, LoomError::UnresolvedReference { name, .. } if name == "nonexistent"
        )));
    }

    #[test]
    fn test_invalid_outcome_target() {
        let tmp = std::env::temp_dir().join("loom_test_invalid_outcome_target");
        let prompts_dir = tmp.join("prompts");
        let _ = std::fs::create_dir_all(&prompts_dir);
        std::fs::write(
            prompts_dir.join("work.md"),
            r#"---
accept: []
success:
  done: nonexistent_state
failure: {}
params: {}
---
Do work.
"#,
        )
        .unwrap();

        let src = r#"
            workflow test v1 {
                queue q1 "Q1"
                action work "Work" { produce agent prompt work }
                step s1 { q1 -> work }
            }
        "#;
        let ast = crate::parse::parse_workflow(src).unwrap();
        let err = lower(&ast, &tmp).unwrap_err();
        assert!(err
            .iter()
            .any(|e| matches!(e, LoomError::InvalidOutcomeTarget { .. })));
    }

    #[test]
    fn test_undeclared_body_param() {
        let tmp = std::env::temp_dir().join("loom_test_undeclared_body_param");
        let prompts_dir = tmp.join("prompts");
        let _ = std::fs::create_dir_all(&prompts_dir);
        std::fs::write(
            prompts_dir.join("work.md"),
            r#"---
accept: []
success:
  done: done
failure: {}
params: {}
---
Do the {{ unknown }} thing.
"#,
        )
        .unwrap();

        let src = r#"
            workflow test v1 {
                queue q1 "Q1"
                action work "Work" { produce agent prompt work }
                terminal done
                step s1 { q1 -> work }
            }
        "#;
        let ast = crate::parse::parse_workflow(src).unwrap();
        let err = lower(&ast, &tmp).unwrap_err();
        assert!(err.iter().any(|e| matches!(
            e, LoomError::UndeclaredParam { param, .. } if param == "unknown"
        )));
    }

    #[test]
    fn test_bool_param_bad_default() {
        let tmp = std::env::temp_dir().join("loom_test_bool_param_bad_default");
        let prompts_dir = tmp.join("prompts");
        let _ = std::fs::create_dir_all(&prompts_dir);
        std::fs::write(
            prompts_dir.join("work.md"),
            r#"---
accept: []
success:
  done: done
failure: {}
params:
  flag:
    type: bool
    default: "yes"
---
Use {{ flag }}.
"#,
        )
        .unwrap();

        let src = r#"
            workflow test v1 {
                queue q1 "Q1"
                action work "Work" { produce agent prompt work }
                terminal done
                step s1 { q1 -> work }
            }
        "#;
        let ast = crate::parse::parse_workflow(src).unwrap();
        let err = lower(&ast, &tmp).unwrap_err();
        assert!(err.iter().any(|e| matches!(
            e, LoomError::ParamValidation { param, .. } if param == "flag"
        )));
    }

    #[test]
    fn test_int_param_bad_default() {
        let tmp = std::env::temp_dir().join("loom_test_int_param_bad_default");
        let prompts_dir = tmp.join("prompts");
        let _ = std::fs::create_dir_all(&prompts_dir);
        std::fs::write(
            prompts_dir.join("work.md"),
            r#"---
accept: []
success:
  done: done
failure: {}
params:
  count:
    type: int
    default: "abc"
---
Count {{ count }}.
"#,
        )
        .unwrap();

        let src = r#"
            workflow test v1 {
                queue q1 "Q1"
                action work "Work" { produce agent prompt work }
                terminal done
                step s1 { q1 -> work }
            }
        "#;
        let ast = crate::parse::parse_workflow(src).unwrap();
        let err = lower(&ast, &tmp).unwrap_err();
        assert!(err.iter().any(|e| matches!(
            e, LoomError::ParamValidation { param, .. } if param == "count"
        )));
    }

    #[test]
    fn test_enum_param_default_not_in_values() {
        let tmp = std::env::temp_dir().join("loom_test_enum_param_default_not_in_values");
        let prompts_dir = tmp.join("prompts");
        let _ = std::fs::create_dir_all(&prompts_dir);
        std::fs::write(
            prompts_dir.join("work.md"),
            r#"---
accept: []
success:
  done: done
failure: {}
params:
  color:
    type: enum
    values: [red, blue]
    default: "green"
---
Color {{ color }}.
"#,
        )
        .unwrap();

        let src = r#"
            workflow test v1 {
                queue q1 "Q1"
                action work "Work" { produce agent prompt work }
                terminal done
                step s1 { q1 -> work }
            }
        "#;
        let ast = crate::parse::parse_workflow(src).unwrap();
        let err = lower(&ast, &tmp).unwrap_err();
        assert!(err.iter().any(|e| matches!(
            e, LoomError::ParamValidation { param, .. } if param == "color"
        )));
    }

    #[test]
    fn test_duplicate_profile() {
        let src = r#"
            workflow test v1 {
                queue q1 "Q1"
                profile p1 { phases [] }
                profile p1 { phases [] }
            }
        "#;
        let err = lower_inline(src).unwrap_err();
        assert!(err.iter().any(|e| matches!(
            e, LoomError::DuplicateIdentifier { name } if name == "p1"
        )));
    }
}
