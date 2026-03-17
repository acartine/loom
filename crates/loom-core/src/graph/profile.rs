use indexmap::IndexMap;
use std::collections::HashSet;

use crate::ir::{PhaseDef, ProfileDef, StateDef, StepDef, WorkflowIR};
use crate::prompt::PromptFile;

/// Extract the subgraph for a specific profile.
/// Returns a filtered WorkflowIR containing only the states, steps,
/// phases, and prompts relevant to this profile.
pub fn extract_profile_subgraph(ir: &WorkflowIR, profile_name: &str) -> Option<WorkflowIR> {
    let profile = ir.profiles.get(profile_name)?;

    let mut states: IndexMap<String, StateDef> = IndexMap::new();
    let mut steps: IndexMap<String, StepDef> = IndexMap::new();
    let mut phases: IndexMap<String, PhaseDef> = IndexMap::new();
    let mut prompts: IndexMap<String, PromptFile> = IndexMap::new();
    let mut profiles: IndexMap<String, ProfileDef> = IndexMap::new();

    // Collect phases
    for phase_name in &profile.phases {
        if let Some(phase) = ir.phases.get(phase_name) {
            phases.insert(phase_name.clone(), phase.clone());

            // Collect steps from phases
            for step_name in [&phase.produce_step, &phase.gate_step] {
                if let Some(step) = ir.steps.get(step_name) {
                    steps.insert(step_name.clone(), step.clone());

                    // Collect states from steps
                    if let Some(state) = ir.states.get(&step.queue) {
                        states.insert(step.queue.clone(), state.clone());
                    }
                    if let Some(state) = ir.states.get(&step.action) {
                        let mut state = state.clone();
                        // Apply executor overrides
                        if let Some(&executor) = profile.overrides.get(state.name()) {
                            if let StateDef::Action {
                                executor: ref mut e,
                                ..
                            } = state
                            {
                                *e = executor;
                            }
                        }
                        // Collect prompt
                        if let StateDef::Action { prompt_name, .. } = &state {
                            if let Some(prompt) = ir.prompts.get(prompt_name) {
                                prompts.insert(prompt_name.clone(), prompt.clone());
                            }
                        }
                        states.insert(step.action.clone(), state);
                    }
                }
            }
        }
    }

    // Add terminal and escape states
    for (name, state) in &ir.states {
        if state.is_terminal() || state.is_escape() {
            states.insert(name.clone(), state.clone());
        }
    }

    // Also collect states referenced by outcomes that aren't already included
    let mut outcome_targets: HashSet<String> = HashSet::new();
    for prompt in prompts.values() {
        for target in prompt.success.values().chain(prompt.failure.values()) {
            outcome_targets.insert(target.clone());
        }
    }
    for target in &outcome_targets {
        if !states.contains_key(target) {
            if let Some(state) = ir.states.get(target) {
                states.insert(target.clone(), state.clone());
            }
        }
    }

    profiles.insert(profile_name.to_string(), profile.clone());

    Some(WorkflowIR {
        name: ir.name.clone(),
        version: ir.version,
        default_profile: ir.default_profile.clone(),
        states,
        steps,
        phases,
        profiles,
        wildcard_targets: ir.wildcard_targets.clone(),
        prompts,
    })
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
    fn test_extract_autopilot() {
        let ir = load_ir();
        let sub = extract_profile_subgraph(&ir, "autopilot").unwrap();
        // All 3 phases -> 6 steps -> 12 queue/action states + 2 terminals + 1 escape = 15
        assert_eq!(sub.states.len(), 15);
        assert_eq!(sub.steps.len(), 6);
        assert_eq!(sub.phases.len(), 3);
    }

    #[test]
    fn test_extract_no_planning() {
        let ir = load_ir();
        let sub = extract_profile_subgraph(&ir, "autopilot_no_planning").unwrap();
        // 2 phases -> 4 steps -> 8 queue/action states + 2 terminals + 1 escape = 11
        // Plus any states referenced by outcomes (ready_for_planning from failure routes)
        assert!(sub.states.len() >= 11);
        assert_eq!(sub.steps.len(), 4);
        assert_eq!(sub.phases.len(), 2);
    }

    #[test]
    fn test_semiauto_overrides() {
        let ir = load_ir();
        let sub = extract_profile_subgraph(&ir, "semiauto").unwrap();
        // plan_review and implementation_review should have human executor
        if let Some(StateDef::Action { executor, .. }) = sub.states.get("plan_review") {
            assert_eq!(*executor, crate::parse::ast::Executor::Human);
        } else {
            panic!("expected plan_review action");
        }
    }
}
