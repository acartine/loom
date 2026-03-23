/// Raw AST types matching the grammar productions.
/// These are produced by the parser and consumed by the IR lowering pass.

#[derive(Debug, Clone)]
pub struct Workflow {
    pub name: String,
    pub version: u32,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone)]
pub enum Declaration {
    Queue(QueueDecl),
    Action(ActionDecl),
    Terminal(TerminalDecl),
    Escape(EscapeDecl),
    Step(StepDecl),
    Phase(PhaseDecl),
    Profile(ProfileDecl),
    Include(IncludeDecl),
    WildcardTransition(WildcardTransitionDecl),
}

#[derive(Debug, Clone)]
pub struct QueueDecl {
    pub name: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ActionDecl {
    pub name: String,
    pub display_name: Option<String>,
    pub action_type: ActionType,
    pub prompt: Option<String>,
    pub output: Option<ActionOutput>,
    pub constraints: Vec<Constraint>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionType {
    Produce(Executor),
    Gate(GateKind, Executor),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateKind {
    Approve,
    Auth,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Executor {
    Agent,
    Human,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionOutput {
    pub artifact_type: String,
    pub access_hint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Constraint {
    ReadOnly,
    NoGitWrite,
    MetadataOnly,
}

#[derive(Debug, Clone)]
pub struct TerminalDecl {
    pub name: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EscapeDecl {
    pub name: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StepDecl {
    pub name: String,
    pub queue: Option<String>,
    pub action: String,
}

#[derive(Debug, Clone)]
pub struct PhaseDecl {
    pub name: String,
    pub produce_step: String,
    pub gate_step: String,
}

#[derive(Debug, Clone)]
pub struct ProfileDecl {
    pub name: String,
    pub display_name: Option<String>,
    pub fields: Vec<ProfileField>,
}

#[derive(Debug, Clone)]
pub enum ProfileField {
    Phases(Vec<String>),
    Override(OverrideDecl),
    Description(String),
}

#[derive(Debug, Clone)]
pub struct OverrideDecl {
    pub action: String,
    pub executor: Option<Executor>,
    pub output: Option<ActionOutput>,
}

#[derive(Debug, Clone)]
pub struct IncludeDecl {
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct WildcardTransitionDecl {
    pub target: String,
}
