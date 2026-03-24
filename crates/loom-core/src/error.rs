use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum LoomError {
    #[error("Parse error: {message}")]
    Parse { message: String },

    #[error("Duplicate identifier: '{name}' already declared")]
    DuplicateIdentifier { name: String },

    #[error("Unresolved reference: '{name}' not found")]
    UnresolvedReference { name: String, context: String },

    #[error("Step type mismatch: {message}")]
    StepTypeMismatch { message: String },

    #[error("Phase type mismatch: {message}")]
    PhaseTypeMismatch { message: String },

    #[error("Missing prompt file: prompts/{name}.md")]
    MissingPrompt { name: String },

    #[error("Orphaned prompt file: prompts/{name}.md has no matching action")]
    OrphanedPrompt { name: String },

    #[error("Invalid outcome target: outcome '{outcome}' in prompt '{prompt}' targets unknown state '{target}'")]
    InvalidOutcomeTarget {
        prompt: String,
        outcome: String,
        target: String,
    },

    #[error("Parameter consistency: prompt '{prompt}' uses '{{{{ {param} }}}}' but it is not declared in params")]
    UndeclaredParam { prompt: String, param: String },

    #[error("Parameter validation: {message}")]
    ParamValidation {
        prompt: String,
        param: String,
        message: String,
    },

    #[error("No step defined for action '{name}' — add a step to connect it to the workflow (e.g., `step {hint} -> {name}`)")]
    DeadState { name: String, hint: String },

    #[error("Terminal unreachable: '{name}' cannot reach any terminal state")]
    TerminalUnreachable { name: String },

    #[error("Profile error: {message}")]
    ProfileError { message: String },

    #[error("Profile validation: profile '{profile}': {message}")]
    ProfileValidation { profile: String, message: String },

    #[error("Override validity: override references action '{action}' not in profile's phases")]
    InvalidOverride { action: String },

    #[error("Produce action '{action}' has no success outcomes; produce actions must declare at least one success outcome to advance the workflow")]
    ProduceNoSuccess { action: String },

    #[error("Escape reachability: escape state '{name}' has no path back to the workflow")]
    EscapeUnreachable { name: String },

    #[error("Config: default_profile '{name}' is not defined in the workflow")]
    InvalidDefaultProfile { name: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parse error: {0}")]
    Yaml(String),

    #[error("TOML parse error: {0}")]
    Toml(String),

    #[error("Config error: {message}")]
    Config { message: String },
}

#[derive(Debug)]
pub enum LoomWarning {
    UnusedStep { name: String },
    UnusedPhase { name: String },
    SingleOutcomeAction { name: String },
}

impl std::fmt::Display for LoomWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoomWarning::UnusedStep { name } => {
                write!(
                    f,
                    "warning: step '{name}' is declared but not used in any phase"
                )
            }
            LoomWarning::UnusedPhase { name } => {
                write!(
                    f,
                    "warning: phase '{name}' is declared but not used in any profile"
                )
            }
            LoomWarning::SingleOutcomeAction { name } => {
                write!(f, "warning: action '{name}' has only one success outcome and zero failure outcomes")
            }
        }
    }
}

pub type LoomResult<T> = Result<T, LoomError>;

/// Accumulates multiple errors and warnings
#[derive(Debug, Default)]
pub struct Diagnostics {
    pub errors: Vec<LoomError>,
    pub warnings: Vec<LoomWarning>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn error(&mut self, err: LoomError) {
        self.errors.push(err);
    }

    pub fn warn(&mut self, warning: LoomWarning) {
        self.warnings.push(warning);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn merge(&mut self, other: Diagnostics) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}
