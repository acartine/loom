pub mod lower;

use crate::parse::ast::{ActionOutput, ActionType, Constraint, Executor, GateKind};
use crate::prompt::PromptFile;
use indexmap::IndexMap;

/// The intermediate representation of a complete workflow.
/// All names are resolved, all includes are loaded, all prompts are parsed.
#[derive(Debug, Clone)]
pub struct WorkflowIR {
    pub name: String,
    pub version: u32,
    pub default_profile: Option<String>,
    pub states: IndexMap<String, StateDef>,
    pub steps: IndexMap<String, StepDef>,
    pub phases: IndexMap<String, PhaseDef>,
    pub profiles: IndexMap<String, ProfileDef>,
    pub wildcard_targets: Vec<String>,
    pub prompts: IndexMap<String, PromptFile>,
}

#[derive(Debug, Clone)]
pub enum StateDef {
    Queue {
        name: String,
        display_name: String,
    },
    Action {
        name: String,
        display_name: String,
        action_type: ActionType,
        prompt_name: String,
        output: Option<ActionOutput>,
        constraints: Vec<Constraint>,
        executor: Executor,
    },
    Terminal {
        name: String,
        display_name: String,
    },
    Escape {
        name: String,
        display_name: String,
    },
}

impl StateDef {
    pub fn name(&self) -> &str {
        match self {
            StateDef::Queue { name, .. } => name,
            StateDef::Action { name, .. } => name,
            StateDef::Terminal { name, .. } => name,
            StateDef::Escape { name, .. } => name,
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            StateDef::Queue { display_name, .. } => display_name,
            StateDef::Action { display_name, .. } => display_name,
            StateDef::Terminal { display_name, .. } => display_name,
            StateDef::Escape { display_name, .. } => display_name,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, StateDef::Terminal { .. })
    }

    pub fn is_escape(&self) -> bool {
        matches!(self, StateDef::Escape { .. })
    }

    pub fn is_action(&self) -> bool {
        matches!(self, StateDef::Action { .. })
    }

    pub fn is_queue(&self) -> bool {
        matches!(self, StateDef::Queue { .. })
    }

    pub fn executor(&self) -> Option<Executor> {
        match self {
            StateDef::Action { executor, .. } => Some(*executor),
            _ => None,
        }
    }

    pub fn output(&self) -> Option<&ActionOutput> {
        match self {
            StateDef::Action { output, .. } => output.as_ref(),
            _ => None,
        }
    }

    pub fn action_type(&self) -> Option<&ActionType> {
        match self {
            StateDef::Action { action_type, .. } => Some(action_type),
            _ => None,
        }
    }

    pub fn gate_kind(&self) -> Option<GateKind> {
        match self {
            StateDef::Action {
                action_type: ActionType::Gate(kind, _),
                ..
            } => Some(*kind),
            _ => None,
        }
    }

    pub fn is_produce(&self) -> bool {
        matches!(
            self,
            StateDef::Action {
                action_type: ActionType::Produce(_),
                ..
            }
        )
    }

    pub fn is_gate(&self) -> bool {
        matches!(
            self,
            StateDef::Action {
                action_type: ActionType::Gate(..),
                ..
            }
        )
    }
}

#[derive(Debug, Clone)]
pub struct StepDef {
    pub name: String,
    pub queue: String,
    pub action: String,
}

#[derive(Debug, Clone)]
pub struct PhaseDef {
    pub name: String,
    pub produce_step: String,
    pub gate_step: Option<String>,
}

impl PhaseDef {
    /// Returns all step names in this phase (produce + optional gate).
    pub fn step_names(&self) -> Vec<&String> {
        let mut names = vec![&self.produce_step];
        if let Some(gs) = &self.gate_step {
            names.push(gs);
        }
        names
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionOverride {
    pub executor: Option<Executor>,
    pub output: Option<ActionOutput>,
}

#[derive(Debug, Clone)]
pub struct ProfileDef {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub phases: Vec<String>,
    pub overrides: IndexMap<String, ActionOverride>,
}
