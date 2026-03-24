use crate::ir::{ProfileDef, StateDef, WorkflowIR};
use crate::parse::ast::{ActionOutput, ActionType, Constraint, Executor, GateKind};
use crate::prompt::{ParamDef, ParamType, PromptFile};
use indexmap::IndexMap;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct KnotsBundle {
    format: &'static str,
    format_version: u32,
    workflow: WorkflowMetadata,
    states: Vec<StateBundle>,
    steps: Vec<StepBundle>,
    phases: Vec<PhaseBundle>,
    profiles: Vec<ProfileBundle>,
    prompts: Vec<PromptBundle>,
    transitions: TransitionBundle,
}

#[derive(Debug, Serialize)]
struct WorkflowMetadata {
    name: String,
    version: u32,
    default_profile: Option<String>,
}

#[derive(Debug, Serialize)]
struct StateBundle {
    id: String,
    display_name: String,
    kind: &'static str,
    action_kind: Option<&'static str>,
    gate_kind: Option<&'static str>,
    executor: Option<&'static str>,
    constraints: Vec<&'static str>,
    prompt: Option<String>,
    output: Option<String>,
    output_hint: Option<String>,
}

#[derive(Debug, Serialize)]
struct StepBundle {
    id: String,
    queue: String,
    action: String,
}

#[derive(Debug, Serialize)]
struct PhaseBundle {
    id: String,
    produce_step: String,
    gate_step: Option<String>,
}

#[derive(Debug, Serialize)]
struct ProfileBundle {
    id: String,
    display_name: Option<String>,
    description: Option<String>,
    phases: Vec<String>,
    outputs: IndexMap<String, OutputBundle>,
    executors: IndexMap<String, &'static str>,
}

#[derive(Debug, Serialize)]
struct OutputBundle {
    artifact_type: String,
    access_hint: Option<String>,
}

#[derive(Debug, Serialize)]
struct PromptBundle {
    name: String,
    accept: Vec<String>,
    params: Vec<PromptParamBundle>,
    outcomes: Vec<PromptOutcomeBundle>,
    body: String,
    body_params: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PromptParamBundle {
    name: String,
    param_type: &'static str,
    values: Vec<String>,
    required: bool,
    default: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Serialize)]
struct PromptOutcomeBundle {
    name: String,
    target: String,
    is_success: bool,
}

#[derive(Debug, Serialize)]
struct TransitionBundle {
    wildcard_targets: Vec<String>,
    queue_to_action: Vec<StepBundle>,
    outcome_tables: Vec<ActionOutcomeTable>,
}

#[derive(Debug, Serialize)]
struct ActionOutcomeTable {
    action: String,
    prompt: String,
    success: IndexMap<String, String>,
    failure: IndexMap<String, String>,
}

pub fn emit_knots_bundle(ir: &WorkflowIR) -> String {
    let bundle = KnotsBundle {
        format: "knots-bundle",
        format_version: 2,
        workflow: WorkflowMetadata {
            name: ir.name.clone(),
            version: ir.version,
            default_profile: ir.default_profile.clone(),
        },
        states: ir.states.values().map(state_bundle).collect(),
        steps: ir.steps.values().map(step_bundle).collect(),
        phases: ir.phases.values().map(phase_bundle).collect(),
        profiles: ir
            .profiles
            .values()
            .map(|profile| profile_bundle(ir, profile))
            .collect(),
        prompts: ir
            .prompts
            .iter()
            .map(|(name, prompt)| prompt_bundle(name, prompt))
            .collect(),
        transitions: TransitionBundle {
            wildcard_targets: ir.wildcard_targets.clone(),
            queue_to_action: ir.steps.values().map(step_bundle).collect(),
            outcome_tables: ir
                .states
                .values()
                .filter_map(|state| action_outcome_table(state, &ir.prompts))
                .collect(),
        },
    };

    serde_json::to_string_pretty(&bundle).expect("knots bundle should serialize")
}

fn state_bundle(state: &StateDef) -> StateBundle {
    match state {
        StateDef::Queue { name, display_name } => StateBundle {
            id: name.clone(),
            display_name: display_name.clone(),
            kind: "queue",
            action_kind: None,
            gate_kind: None,
            executor: None,
            constraints: Vec::new(),
            prompt: None,
            output: None,
            output_hint: None,
        },
        StateDef::Action {
            name,
            display_name,
            action_type,
            prompt_name,
            output,
            constraints,
            executor,
        } => StateBundle {
            id: name.clone(),
            display_name: display_name.clone(),
            kind: "action",
            action_kind: Some(action_kind_name(action_type)),
            gate_kind: gate_kind_for_action(action_type),
            executor: Some(executor_name(*executor)),
            constraints: constraints
                .iter()
                .map(|constraint| constraint_name(*constraint))
                .collect(),
            prompt: Some(prompt_name.clone()),
            output: output.as_ref().map(|o| o.artifact_type.clone()),
            output_hint: output.as_ref().and_then(|o| o.access_hint.clone()),
        },
        StateDef::Terminal { name, display_name } => StateBundle {
            id: name.clone(),
            display_name: display_name.clone(),
            kind: "terminal",
            action_kind: None,
            gate_kind: None,
            executor: None,
            constraints: Vec::new(),
            prompt: None,
            output: None,
            output_hint: None,
        },
        StateDef::Escape { name, display_name } => StateBundle {
            id: name.clone(),
            display_name: display_name.clone(),
            kind: "escape",
            action_kind: None,
            gate_kind: None,
            executor: None,
            constraints: Vec::new(),
            prompt: None,
            output: None,
            output_hint: None,
        },
    }
}

fn step_bundle(step: &crate::ir::StepDef) -> StepBundle {
    StepBundle {
        id: step.name.clone(),
        queue: step.queue.clone(),
        action: step.action.clone(),
    }
}

fn phase_bundle(phase: &crate::ir::PhaseDef) -> PhaseBundle {
    PhaseBundle {
        id: phase.name.clone(),
        produce_step: phase.produce_step.clone(),
        gate_step: phase.gate_step.clone(),
    }
}

fn profile_bundle(ir: &WorkflowIR, profile: &ProfileDef) -> ProfileBundle {
    ProfileBundle {
        id: profile.name.clone(),
        display_name: profile.display_name.clone(),
        description: profile.description.clone(),
        phases: profile.phases.clone(),
        outputs: materialize_profile_outputs(ir, profile),
        executors: materialize_profile_executors(ir, profile),
    }
}

fn prompt_bundle(name: &str, prompt: &PromptFile) -> PromptBundle {
    let mut outcomes = Vec::with_capacity(prompt.success.len() + prompt.failure.len());
    for (outcome, target) in &prompt.success {
        outcomes.push(PromptOutcomeBundle {
            name: outcome.clone(),
            target: target.clone(),
            is_success: true,
        });
    }
    for (outcome, target) in &prompt.failure {
        outcomes.push(PromptOutcomeBundle {
            name: outcome.clone(),
            target: target.clone(),
            is_success: false,
        });
    }

    PromptBundle {
        name: name.to_string(),
        accept: prompt.accept.clone(),
        params: prompt
            .params
            .iter()
            .map(|(name, param)| prompt_param_bundle(name, param))
            .collect(),
        outcomes,
        body: prompt.body.clone(),
        body_params: prompt.body_params.clone(),
    }
}

fn prompt_param_bundle(name: &str, param: &ParamDef) -> PromptParamBundle {
    PromptParamBundle {
        name: name.to_string(),
        param_type: param_type_name(&param.param_type),
        values: param.values.clone(),
        required: param.required,
        default: param.default.clone(),
        description: param.description.clone(),
    }
}

fn action_outcome_table(
    state: &StateDef,
    prompts: &IndexMap<String, PromptFile>,
) -> Option<ActionOutcomeTable> {
    let StateDef::Action {
        name, prompt_name, ..
    } = state
    else {
        return None;
    };

    let prompt = prompts.get(prompt_name)?;

    Some(ActionOutcomeTable {
        action: name.clone(),
        prompt: prompt_name.clone(),
        success: prompt.success.clone(),
        failure: prompt.failure.clone(),
    })
}

fn materialize_profile_executors(
    ir: &WorkflowIR,
    profile: &ProfileDef,
) -> IndexMap<String, &'static str> {
    let mut executors = IndexMap::new();

    for phase_name in &profile.phases {
        if let Some(phase) = ir.phases.get(phase_name) {
            for step_name in phase.step_names() {
                if let Some(step) = ir.steps.get(step_name) {
                    if let Some(state) = ir.states.get(&step.action) {
                        let executor = profile
                            .overrides
                            .get(&step.action)
                            .and_then(|o| o.executor)
                            .or_else(|| state.executor())
                            .unwrap_or(Executor::Agent);
                        executors.insert(step.action.clone(), executor_name(executor));
                    }
                }
            }
        }
    }

    executors
}

fn action_kind_name(action_type: &ActionType) -> &'static str {
    match action_type {
        ActionType::Produce(_) => "produce",
        ActionType::Gate(_, _) => "gate",
    }
}

fn gate_kind_for_action(action_type: &ActionType) -> Option<&'static str> {
    match action_type {
        ActionType::Produce(_) => None,
        ActionType::Gate(kind, _) => Some(gate_kind_name(*kind)),
    }
}

fn gate_kind_name(kind: GateKind) -> &'static str {
    match kind {
        GateKind::Approve => "approve",
        GateKind::Auth => "auth",
        GateKind::Review => "review",
    }
}

fn executor_name(executor: Executor) -> &'static str {
    match executor {
        Executor::Agent => "agent",
        Executor::Human => "human",
    }
}

fn constraint_name(constraint: Constraint) -> &'static str {
    match constraint {
        Constraint::ReadOnly => "read_only",
        Constraint::NoGitWrite => "no_git_write",
        Constraint::MetadataOnly => "metadata_only",
    }
}

fn materialize_profile_outputs(
    ir: &WorkflowIR,
    profile: &ProfileDef,
) -> IndexMap<String, OutputBundle> {
    let mut outputs = IndexMap::new();

    for phase_name in &profile.phases {
        if let Some(phase) = ir.phases.get(phase_name) {
            for step_name in phase.step_names() {
                if let Some(step) = ir.steps.get(step_name) {
                    if let Some(state) = ir.states.get(&step.action) {
                        let output = profile
                            .overrides
                            .get(&step.action)
                            .and_then(|o| o.output.as_ref())
                            .or_else(|| state.output());
                        if let Some(o) = output {
                            outputs.insert(step.action.clone(), output_bundle(o));
                        }
                    }
                }
            }
        }
    }

    outputs
}

fn output_bundle(output: &ActionOutput) -> OutputBundle {
    OutputBundle {
        artifact_type: output.artifact_type.clone(),
        access_hint: output.access_hint.clone(),
    }
}

fn param_type_name(param_type: &ParamType) -> &'static str {
    match param_type {
        ParamType::String => "string",
        ParamType::Int => "int",
        ParamType::Bool => "bool",
        ParamType::Enum => "enum",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/knots_sdlc")
    }

    #[test]
    fn test_emit_knots_bundle() {
        let (ir, _diag) =
            crate::load_workflow(&fixture_dir()).expect("fixture workflow should load");

        let output = emit_knots_bundle(&ir);
        let json: Value = serde_json::from_str(&output).expect("bundle should be valid json");

        assert_eq!(json["format"], "knots-bundle");
        assert_eq!(json["format_version"], 2);
        assert_eq!(json["workflow"]["name"], "knots_sdlc");
        assert_eq!(json["workflow"]["default_profile"], "autopilot");

        let states = json["states"]
            .as_array()
            .expect("states should be an array");
        assert!(
            states.iter().any(|state| state["id"] == "planning"),
            "planning state should be present in knots bundle"
        );

        let steps = json["steps"].as_array().expect("steps should be an array");
        assert!(
            steps.iter().any(|step| step["id"] == "planning"),
            "planning step should be present in knots bundle"
        );

        let prompts = json["prompts"]
            .as_array()
            .expect("prompts should be an array");
        let planning_prompt = prompts
            .iter()
            .find(|prompt| prompt["name"] == "planning")
            .expect("planning prompt should exist");
        let params = planning_prompt["params"]
            .as_array()
            .expect("planning params should be an array");
        let complexity = params
            .iter()
            .find(|param| param["name"] == "complexity")
            .expect("planning prompt should declare complexity");
        assert_eq!(complexity["param_type"], "enum");

        let profiles = json["profiles"]
            .as_array()
            .expect("profiles should be an array");
        let semiauto = profiles
            .iter()
            .find(|profile| profile["id"] == "semiauto")
            .expect("semiauto profile should exist");
        assert_eq!(semiauto["executors"]["plan_review"], "human");

        let transitions = json["transitions"]
            .as_object()
            .expect("transitions should be an object");
        assert!(
            transitions["queue_to_action"]
                .as_array()
                .expect("queue_to_action should be an array")
                .iter()
                .any(|step| step["id"] == "planning"),
            "queue_to_action should include the planning step"
        );
    }
}
