# Loom Language Specification

**Version**: 0.1.0
**Date**: 2026-03-16

Loom is a workflow definition language and compiler. It produces typed,
validated workflow definitions that workflow engines (such as Knots) consume.
Loom is not a workflow engine. It does not execute workflows. It defines them.

## Design Principles

1. **Workflows are first-class, not configuration.** A workflow is a typed
   program, not a bag of settings with overrides.
2. **The prompt is the routing table.** Outcomes are not labels. They are
   edges. The prompt defines where a knot goes next.
3. **Prompts are documents, not strings.** Prompt bodies live in markdown
   files. The workflow definition references them by name.
4. **Profiles are subgraphs, not patches.** A profile selects phases and
   assigns ownership. It does not override a "default" workflow.
5. **The compiler catches your mistakes.** Dead states, unreachable
   terminals, dangling prompt references, and undefined edge targets are
   build-time errors, not runtime surprises.

---

## 1. File Structure

A workflow is a directory, not a single file.

```
<workflow-name>/
  workflow.loom           # root definition
  loom.toml               # package metadata
  prompts/
    <action-name>.md      # one per action state
  profiles/
    <profile-name>.loom   # one per profile
```

### 1.1 `loom.toml`

Package metadata. Not part of the language grammar.

```toml
[workflow]
name = "knots_sdlc"
version = 1
entry = "workflow.loom"
default_profile = "autopilot"
```

### 1.2 `workflow.loom`

The root file declares all shared states, action definitions, composites,
and terminal states. Profiles are either inline or in separate files under
`profiles/`.

### 1.3 `prompts/<name>.md`

Markdown files with YAML frontmatter. Each file corresponds to one action
state. The frontmatter declares structured outcome routing. The body is
the prompt template.

### 1.4 `profiles/<name>.loom`

Each profile selects phases from the root workflow and assigns ownership
to each action.

---

## 2. Grammar

The grammar is defined in PEG notation (pest-compatible). Loom files use
the `.loom` extension.

### 2.1 Lexical Rules

```pest
WHITESPACE  = _{ " " | "\t" | "\r" | "\n" }
COMMENT     = _{ "//" ~ (!"\n" ~ ANY)* }

ident       = @{ (ASCII_ALPHA | "_") ~ (ASCII_ALPHANUMERIC | "_")* }
integer     = @{ ASCII_DIGIT+ }
string      = @{ "\"" ~ (!"\"" ~ ANY)* ~ "\"" }
string_list = { "[" ~ (string ~ ","?)* ~ "]" }
ident_list  = { "[" ~ (ident ~ ","?)* ~ "]" }
```

- Commas are optional in lists (newline-separated is fine).
- No semicolons. Declarations are delimited by keywords.
- Line comments start with `//`.

### 2.2 File Structure

```pest
file = { SOI ~ workflow ~ EOI }

workflow = {
    "workflow" ~ ident ~ version ~ "{" ~ declaration* ~ "}"
}

version = { "v" ~ integer }

declaration = {
    queue_decl
  | action_decl
  | step_decl
  | phase_decl
  | terminal_decl
  | escape_decl
  | profile_decl
  | include_decl
}
```

### 2.3 States

#### Queue States

```pest
queue_decl = {
    "queue" ~ ident ~ string?
}
```

A queue state is a waiting point. Something is ready to be picked up.

Queues are typically created implicitly by steps (see Steps below). Explicit
queue declarations are only needed for standalone queues not tied to a step.

```
queue ready_for_triage "Ready for Triage"
queue ready_for_triage                       // display name derived: "Ready For Triage"
```

- First argument: identifier (used in transitions, outcome targets).
- Second argument (optional): human-readable display name. If omitted,
  derived from the identifier by converting snake_case to Title Case.

#### Action States

```pest
action_decl = {
    "action" ~ ident ~ string? ~ "{" ~ action_body ~ "}"
}

action_body = {
    action_type ~ prompt_ref? ~ constraint*
}

action_type = { produce_type | gate_type }

produce_type = { "produce" ~ executor }
gate_type    = { "gate" ~ gate_kind ~ executor }

gate_kind = { "approve" | "auth" | "review" }
executor  = { "agent" | "human" }

prompt_ref = { "prompt" ~ ident }

constraint = { "constraint" ~ constraint_kind }
constraint_kind = { "read_only" | "no_git_write" | "metadata_only" }
```

An action state is where work happens.

```
action planning {
    produce agent
}
```

- `produce` actions create output (code, plans, artifacts).
- `gate` actions make pass/fail decisions (reviews, approvals, auth checks).
- `executor` declares who owns this action: `agent` or `human`.
  Profiles can override this per-profile.
- Display name (optional string after identifier): derived from identifier
  if omitted. Use an explicit override when the derived name doesn't fit:
  `action planning "Plan Creation" { ... }`.
- Prompt reference (optional `prompt <ident>`): defaults to the action
  identifier. `action planning { ... }` resolves to `prompts/planning.md`.
  Use an explicit override when the prompt name differs from the action name.
- `constraint` declares behavioral boundaries for the action session.

#### Terminal States

```pest
terminal_decl = {
    "terminal" ~ ident ~ string?
}
```

A terminal state is a final resting point. No outbound transitions.

```
terminal shipped "Shipped"
terminal abandoned "Abandoned"
```

#### Escape States

```pest
escape_decl = {
    "escape" ~ ident ~ string?
}
```

An escape state is reachable from any non-terminal state via a wildcard
transition. Unlike terminals, escapes may or may not be final (e.g.,
`deferred` is an escape that can be re-entered into the workflow).

```
escape deferred "Deferred"
```

The compiler generates a wildcard transition from every non-terminal state
to each escape state. Escape states that are also terminal should be
declared as `terminal` instead and use an explicit `* -> <terminal>`
transition (see Section 2.5).

### 2.4 Composites

#### Steps

```pest
step_decl = {
    "step" ~ ident ~ "{" ~ ident ~ "->" ~ ident ~ "}"
  | "step" ~ ident ~ "->" ~ ident
}
```

A step pairs a queue state with its action state. The arrow declares an
implicit transition: entering the queue leads to claiming the action.

**Shorthand form** (preferred): the compiler derives the queue automatically.

```
step plan -> planning
```

This creates an implicit queue state `ready_for_planning` with display name
`"Ready for Planning"` and connects it to the `planning` action.

**Explicit form** (override): reference a specific queue state.

```
step plan {
    ready_for_planning -> planning
}
```

- Shorthand form: the right side must be an `action` state. The queue
  `ready_for_{action}` is synthesized automatically.
- Explicit form: left side must be a `queue` state, right side must be
  an `action` state.
- The implicit transition (queue -> action on claim) is generated by the
  compiler. You do not declare it separately.

#### Phases

```pest
phase_decl = {
    "phase" ~ ident ~ "{" ~ phase_body ~ "}"
}

phase_body = {
    "produce" ~ ident ~ "gate" ~ ident
}
```

A phase groups a produce step with its gate step.

```
phase planning_phase {
    produce plan
    gate plan_rev
}
```

- `produce` references a step whose action type is `produce`.
- `gate` references a step whose action type is `gate`.
- The compiler generates an implicit transition from the produce action's
  success outcomes to the gate step's queue.

### 2.5 Prompt Files

Prompt files live in `prompts/` and use markdown with YAML frontmatter.

#### Frontmatter Schema

```yaml
---
# Acceptance criteria: what must be true for the action to succeed.
accept:
  - <string>

# Success outcomes: named outcomes that route to target states.
# Each key is an outcome identifier. Each value is a target state ident.
success:
  <outcome_ident>: <target_state_ident>

# Failure outcomes: named outcomes that route to target states.
failure:
  <outcome_ident>: <target_state_ident>

# Parameters: typed inputs available in the prompt body via {{ name }}.
params:
  <param_name>:
    type: string | int | bool | enum
    values: [<string>, ...]     # required for enum type
    required: true | false      # default: true
    default: <value>            # optional
    description: <string>
---
```

#### Outcome Routing

Every outbound edge from an action state is declared in the prompt
frontmatter, not in the `.loom` file. The prompt author decides what
outcomes exist and where they route.

```yaml
success:
  approved: ready_for_implementation
  approved_fast_track: ready_for_shipment

failure:
  scope_unclear: ready_for_planning
  missing_criteria: ready_for_planning
  blocked_on_dependency: deferred
```

- Outcome identifiers become enum variants in generated code.
- Target state identifiers must resolve to states declared in the
  workflow. The compiler validates this.
- The compiler classifies outcomes as success or failure based on which
  block they appear in.

#### Prompt Body

The body is markdown with optional template parameters.

```markdown
Break the knot into actionable implementation steps.

Consider the {{ complexity }} of the change and identify all
affected components within the {{ domain }} area.

## Deliverables

1. A sequenced list of implementation steps
2. Estimated effort per step
```

- `{{ param }}` references are validated against the `params` block.
- The prompt body is opaque to the compiler beyond parameter validation.
  It is passed through to the consuming engine as-is.

### 2.6 Profiles

```pest
profile_decl = {
    "profile" ~ ident ~ string? ~ "{" ~ profile_body ~ "}"
}

profile_body = {
    profile_field*
}

profile_field = {
    profile_phases
  | profile_output
  | profile_override
  | profile_description
}

profile_phases  = { "phases" ~ ident_list }
profile_output  = { "output" ~ output_kind }
profile_override = {
    "override" ~ ident ~ "{" ~ override_body ~ "}"
}
profile_description = { "description" ~ string }

output_kind = { "local" | "remote" | "remote_main" | "pr" }

override_body = {
    override_field*
}

override_field = {
    executor_override
}

executor_override = { "executor" ~ executor }
```

Profiles select which phases are active and can override action ownership.

```
profile semiauto "Human-gated reviews" {
    description "Human reviews plan and implementation; agents do the rest"
    phases [planning_phase, implementation_phase, shipment_phase]
    output remote_main

    override plan_review {
        executor human
    }

    override implementation_review {
        executor human
    }
}
```

- `phases` lists which phases this profile includes. Omitted phases are
  skipped entirely -- their states are unreachable under this profile.
- `output` declares the expected artifact type for produce actions.
- `override` blocks change the executor for specific actions within
  this profile. The action identifier must reference an action declared
  in the workflow.

#### Profile in Separate File

When a profile is in `profiles/<name>.loom`, the file contains just the
profile body without the outer workflow wrapper:

```
// profiles/semiauto.loom

profile semiauto "Human-gated reviews" {
    description "Human reviews plan and implementation; agents do the rest"
    phases [planning_phase, implementation_phase, shipment_phase]
    output remote_main

    override plan_review {
        executor human
    }

    override implementation_review {
        executor human
    }
}
```

### 2.7 Includes

```pest
include_decl = { "include" ~ string }
```

Includes reference profile files relative to the workflow root:

```
include "profiles/semiauto.loom"
```

### 2.8 Wildcard Transitions

Explicit wildcard transitions can target terminal or escape states:

```
* -> abandoned
* -> deferred
```

These generate a transition from every non-terminal state to the target.
Declared in the workflow body, not inside profiles.

```pest
wildcard_transition = {
    "*" ~ "->" ~ ident
}
```

(Add to `declaration` alternatives.)

---

## 3. Compiler Semantics

### 3.1 Name Resolution

All identifiers are resolved within the workflow scope:

- Action prompt references default to the action identifier and resolve
  to `prompts/<ident>.md`. An explicit `prompt <ident>` overrides this.
- Action display names default to the identifier converted from
  snake_case to Title Case. An explicit string overrides this.
- Step shorthand (`step X -> Y`) synthesizes a queue state
  `ready_for_{action}` with a derived display name.
- Step queue/action references resolve to declared or synthesized
  queue/action states.
- Phase produce/gate references resolve to declared steps.
- Profile phase references resolve to declared phases.
- Outcome target states resolve to declared states (queue, action,
  terminal, or escape).
- Override action references resolve to declared action states.

Unresolved references are compile-time errors.

### 3.2 Implicit Transitions

The compiler generates transitions that are structurally implied:

1. **Step transitions**: `queue -> action` (on claim).
2. **Phase transitions**: produce action success outcomes ->
   gate step queue (when both are in the same phase).
3. **Wildcard transitions**: `* -> escape` and `* -> terminal`
   (when declared).

Explicit outcome routing in prompts takes precedence over implicit
phase transitions. If a produce action's success outcome explicitly
targets a state, the compiler uses that target, not the phase's gate
queue.

### 3.3 Graph Validation

The compiler performs the following checks on each profile's resolved
subgraph:

| Check                      | Error if                                          |
|----------------------------|---------------------------------------------------|
| Terminal reachability       | Any non-terminal state cannot reach a terminal    |
| Dead states                | A state has no inbound transitions                |
| Orphaned prompts           | A prompt file exists with no matching action      |
| Missing prompts            | An action references a prompt with no file        |
| Outcome target validity    | An outcome targets a state not in the workflow    |
| Parameter consistency      | A `{{ param }}` in the body has no `params` entry |
| Step type mismatch         | Step left side is not queue or right is not action|
| Phase type mismatch        | Phase produce step has gate action or vice versa  |
| Profile completeness       | Profile phases reference undefined phases         |
| Override validity          | Override references action not in profile's phases|
| Duplicate identifiers      | Two declarations share the same identifier        |
| Escape reachability        | Escape state has no path back to the workflow     |

Warnings (non-fatal):

| Warning                    | Condition                                         |
|----------------------------|---------------------------------------------------|
| Unused state               | State declared but not in any step                |
| Unused step                | Step declared but not in any phase                |
| Single-outcome action      | Action has only one success and zero failure      |

### 3.4 Profile Subgraph Extraction

For each profile, the compiler:

1. Collects all phases listed in the profile.
2. Collects all steps from those phases.
3. Collects all states from those steps.
4. Collects all outcomes from the prompts of those action states.
5. Adds terminal and escape states.
6. Validates the resulting subgraph independently.

This means a workflow can have phases that are invalid in isolation but
valid when composed. Each profile must be independently valid.

---

## 4. Code Generation

`loom build` compiles the workflow directory into target-language code.
The generated code is self-contained: no loom runtime dependency.

### 4.1 Generated Artifacts

For each target language, loom generates:

1. **State enum** -- all states as typed variants.
2. **Outcome enums** -- one per action, with success/failure variants.
3. **Transition function** -- `apply(state, outcome) -> Result<State>`.
4. **Profile struct** -- phase list, ownership map, output kind.
5. **Prompt metadata** -- acceptance criteria, parameters, outcome lists.
   Prompt body text is embedded as a const string or loaded at runtime
   (configurable).

### 4.2 Rust Example

```rust
// Generated by loom v0.1.0 from knots_sdlc v1

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    ReadyForPlanning,
    Planning,
    ReadyForPlanReview,
    PlanReview,
    ReadyForImplementation,
    Implementation,
    ReadyForImplementationReview,
    ImplementationReview,
    ReadyForShipment,
    Shipment,
    ReadyForShipmentReview,
    ShipmentReview,
    Shipped,
    Abandoned,
    Deferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanningOutcome {
    PlanComplete,
    InsufficientContext,
    OutOfScope,
}

impl PlanningOutcome {
    pub fn target(&self) -> State {
        match self {
            Self::PlanComplete => State::ReadyForPlanReview,
            Self::InsufficientContext => State::ReadyForPlanning,
            Self::OutOfScope => State::ReadyForPlanning,
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::PlanComplete)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Executor {
    Agent,
    Human,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputKind {
    Local,
    Remote,
    RemoteMain,
    Pr,
}

pub struct Profile {
    pub id: &'static str,
    pub description: &'static str,
    pub phases: &'static [&'static str],
    pub output: OutputKind,
    pub executors: &'static [(State, Executor)],
}
```

### 4.3 Go Example

```go
// Generated by loom v0.1.0 from knots_sdlc v1

type State int

const (
    ReadyForPlanning State = iota
    Planning
    ReadyForPlanReview
    PlanReview
    // ...
    Shipped
    Abandoned
    Deferred
)

type PlanningOutcome int

const (
    PlanComplete PlanningOutcome = iota
    InsufficientContext
    OutOfScope
)

func (o PlanningOutcome) Target() State {
    switch o {
    case PlanComplete:
        return ReadyForPlanReview
    case InsufficientContext:
        return ReadyForPlanning
    case OutOfScope:
        return ReadyForPlanning
    default:
        panic("unreachable")
    }
}

func (o PlanningOutcome) IsSuccess() bool {
    return o == PlanComplete
}
```

### 4.4 Intermediate Representation

`loom build --emit toml` produces a flat TOML file for tools that want
the data without generated code. This is the interchange format.

---

## 5. CLI Reference

```
loom init <name>               Create a new workflow directory
loom validate                  Validate the workflow directory
loom build [--lang <target>]   Compile to target language (rust, go, python)
loom build --emit toml         Emit TOML interchange format
loom graph [--profile <name>]  Print the state graph (mermaid, dot, ascii)
loom sim <profile>             Interactive transition simulator
loom diff <v1-dir> <v2-dir>    Diff two workflow versions
loom check-compat <old> <new>  Check backward compatibility
```

---

## 6. Formal Type Hierarchy

From smallest to largest:

```
Prompt
  - instruction (markdown body)
  - acceptance criteria (string list)
  - success outcomes (map: ident -> state)
  - failure outcomes (map: ident -> state)
  - parameters (map: ident -> typed definition)

State
  - QueueState: name, display_name
  - ActionState: name, display_name, action_type, executor, prompt, constraints
  - TerminalState: name, display_name
  - EscapeState: name, display_name

Action Type
  - Produce: executor
  - Gate: gate_kind (approve | auth | review), executor

Step = QueueState + ActionState
  - implicit transition: queue -> action

Phase = ProduceStep + GateStep
  - implicit transition: produce.action -> gate.queue

Profile = Phase[] + Override[] + OutputKind
  - subgraph of the workflow

Workflow = State[] + Phase[] + Profile[] + EscapeState[] + TerminalState[]
  - the complete definition
```

---

## 7. Reference: Knots SDLC Workflow

The following sections contain the complete Loom definition of the Knots
default software development lifecycle workflow with all six profiles.

### 7.1 Package Metadata

**`knots_sdlc/loom.toml`**

```toml
[workflow]
name = "knots_sdlc"
version = 1
entry = "workflow.loom"
default_profile = "autopilot"
```

### 7.2 Root Workflow

**`knots_sdlc/workflow.loom`**

```
workflow knots_sdlc v1 {

    // ── Action States ─────────────────────────────────────────────

    action planning {
        produce agent
    }

    action plan_review {
        gate review agent
        constraint read_only
        constraint no_git_write
        constraint metadata_only
    }

    action implementation {
        produce agent
    }

    action implementation_review {
        gate review agent
        constraint read_only
        constraint no_git_write
        constraint metadata_only
    }

    action shipment {
        produce agent
    }

    action shipment_review {
        gate review agent
        constraint read_only
        constraint no_git_write
        constraint metadata_only
    }

    // ── Terminal & Escape States ──────────────────────────────────

    terminal shipped
    terminal abandoned
    escape   deferred

    // ── Wildcard Transitions ─────────────────────────────────────

    * -> abandoned
    * -> deferred

    // ── Steps ─────────────────────────────────────────────────────

    step plan -> planning
    step plan_rev -> plan_review
    step impl -> implementation
    step impl_rev -> implementation_review
    step ship -> shipment
    step ship_rev -> shipment_review

    // ── Phases ────────────────────────────────────────────────────

    phase planning_phase {
        produce plan
        gate plan_rev
    }

    phase implementation_phase {
        produce impl
        gate impl_rev
    }

    phase shipment_phase {
        produce ship
        gate ship_rev
    }

    // ── Profiles ──────────────────────────────────────────────────

    include "profiles/autopilot.loom"
    include "profiles/autopilot_with_pr.loom"
    include "profiles/semiauto.loom"
    include "profiles/autopilot_no_planning.loom"
    include "profiles/autopilot_with_pr_no_planning.loom"
    include "profiles/semiauto_no_planning.loom"
}
```

### 7.3 Profiles

**`knots_sdlc/profiles/autopilot.loom`**

```
profile autopilot "Autopilot" {
    description "Agent-owned full flow with remote main output"
    phases [planning_phase, implementation_phase, shipment_phase]
    output remote_main
}
```

**`knots_sdlc/profiles/autopilot_with_pr.loom`**

```
profile autopilot_with_pr "Autopilot with PR" {
    description "Agent-owned full flow with PR output"
    phases [planning_phase, implementation_phase, shipment_phase]
    output pr
}
```

**`knots_sdlc/profiles/semiauto.loom`**

```
profile semiauto "Semi-automatic" {
    description "Human-gated plan and implementation reviews"
    phases [planning_phase, implementation_phase, shipment_phase]
    output remote_main

    override plan_review {
        executor human
    }

    override implementation_review {
        executor human
    }
}
```

**`knots_sdlc/profiles/autopilot_no_planning.loom`**

```
profile autopilot_no_planning "Autopilot (no planning)" {
    description "Agent-owned flow starting at implementation"
    phases [implementation_phase, shipment_phase]
    output remote_main
}
```

**`knots_sdlc/profiles/autopilot_with_pr_no_planning.loom`**

```
profile autopilot_with_pr_no_planning "Autopilot with PR (no planning)" {
    description "Agent-owned flow with PR output and no planning"
    phases [implementation_phase, shipment_phase]
    output pr
}
```

**`knots_sdlc/profiles/semiauto_no_planning.loom`**

```
profile semiauto_no_planning "Semi-automatic (no planning)" {
    description "Human-gated implementation review with skipped planning"
    phases [implementation_phase, shipment_phase]
    output remote_main

    override implementation_review {
        executor human
    }
}
```

### 7.4 Prompts

**`knots_sdlc/prompts/planning.md`**

```markdown
---
accept:
  - Actionable implementation steps with clear deliverables
  - Scope estimated with complexity assessment
  - Dependencies and risks identified
  - Test strategy covers requirements
  - All invariants respected in the plan

success:
  plan_complete: ready_for_plan_review

failure:
  insufficient_context: ready_for_planning
  out_of_scope: ready_for_planning

params:
  complexity:
    type: enum
    values: ["small", "medium", "large"]
    required: false
    description: Expected implementation complexity
---

# Planning

Break the knot into actionable implementation steps.

## Invariant Adherence

- If the knot has invariants, read and understand each one before planning.
- Every step in the plan must respect all invariant conditions.
- Scope invariants constrain what the work may touch.
- State invariants constrain what must remain true throughout execution.
- If any planned step would violate an invariant, redesign the approach or
  flag the conflict in the plan note.

## Step Boundary

- This session is authorized only for planning.
- Complete exactly one planning action, then stop.
- Creating child knots is planning output only. Do not claim, start, or
  execute those child knots in this session.
- Do not edit repository code or perform git write operations during
  planning.
- After the note, handoff, and transition commands for this step succeed,
  stop immediately.

## Actions

1. Analyze the knot requirements and constraints
2. Review knot invariants and ensure the plan respects them
3. Research relevant code, dependencies, and prior art
4. Draft an implementation plan with steps, file changes, and test strategy
5. Estimate complexity and identify risks
6. Write the plan as a knot note
7. Create a hierarchy of child knots if needed

## Output

- Detailed implementation plan attached as a knot note
- Hierarchy of knots created
- Handoff capsule summarizing the plan
```

**`knots_sdlc/prompts/plan_review.md`**

```markdown
---
accept:
  - Plan is complete, correct, and feasible
  - Test strategy covers requirements
  - No security, performance, or maintainability concerns
  - All invariants respected

success:
  approved: ready_for_implementation

failure:
  plan_flawed: ready_for_planning
  requirements_changed: ready_for_planning

params: {}
---

# Plan Review

Review the implementation plan for completeness, correctness, and
feasibility.

## Write Constraints

- Review work is read-only for repository code and git state.
- Do not edit code, tests, docs, configs, or other repository files.
- Do not run git write operations.
- Allowed writes are knot metadata updates only.

## Invariant Review

- If the knot has invariants, verify the plan does not violate any of them.
- For each invariant, confirm the planned steps respect the condition.
- Reject the plan if any step would breach a scope or state invariant.

## Step Boundary

- This session is authorized only for plan review.
- Complete exactly one review action, then stop.
- Do not start implementation work after approving the plan.
- After the review decision, handoff, and transition commands succeed, stop
  immediately.

## Actions

1. Review the plan for completeness, correctness, and feasibility
2. Verify the plan respects all knot invariants
3. Verify test strategy covers requirements
4. Check for security, performance, and maintainability concerns
5. Approve or request revisions
```

**`knots_sdlc/prompts/implementation.md`**

```markdown
---
accept:
  - Working implementation on feature branch
  - All tests passing with coverage threshold met
  - All invariants respected in the implementation
  - Commits tagged on the knot

success:
  implementation_complete: ready_for_implementation_review

failure:
  blocked_by_dependency: deferred
  implementation_infeasible: ready_for_planning
  merge_conflict: ready_for_implementation

params:
  output_kind:
    type: enum
    values: ["local", "remote", "remote_main", "pr"]
    required: true
    description: Artifact output type from the profile
---

# Implementation

Implement the approved plan on a feature branch.

## Invariant Adherence

- If the knot has invariants, strictly adhere to every invariant condition
  throughout implementation.
- Scope invariants limit what code, modules, or systems may be touched.
- State invariants define properties that must remain true at all times.
- If an implementation step would violate an invariant, stop and redesign
  the approach rather than proceeding.

## Step Boundary

- This session is authorized only for implementation.
- Complete exactly one implementation action, then stop.
- Do not merge the feature branch to main, perform shipment work, or
  continue into later workflow stages in this session.
- Opening or updating a review artifact for the implementation branch is
  allowed only if the profile explicitly requires it.
- After the implementation handoff and transition commands succeed, stop
  immediately.

## Actions

1. Create a feature branch from main in a worktree
2. Implement changes following the plan while respecting all invariants
3. Write tests for all new behavior
4. Run any sanity gates defined in the project or the plan
5. Commit and push the feature branch
6. Tag the knot with each commit hash
7. Open or update a PR if the profile output kind requires it

## Output

- Working implementation on feature branch
- All tests passing with coverage threshold met
- Handoff capsule with implementation summary
```

**`knots_sdlc/prompts/implementation_review.md`**

```markdown
---
accept:
  - Code matches knot description and acceptance criteria
  - All invariants respected in the implementation
  - Tests cover required behavior
  - All sanity gates pass
  - No security issues or regressions

success:
  approved: ready_for_shipment

failure:
  changes_requested: ready_for_implementation
  architecture_concern: ready_for_implementation
  critical_issues: ready_for_implementation

params: {}
---

# Implementation Review

Review the implementation against the knot description and acceptance
criteria.

## Write Constraints

- Review work is read-only for repository code and git state.
- Do not edit code, tests, docs, configs, or other repository files.
- Do not run git write operations.
- Allowed writes are knot metadata updates only.
- If code/git writes are needed to complete review, stop and use the
  reject path to move the knot back to ready for implementation.

## Invariant Review

- If the knot has invariants, verify the implementation does not violate
  any of them.
- For each scope invariant, confirm changes are limited to the allowed
  scope.
- For each state invariant, confirm the required property holds in the
  implemented code.
- Reject the implementation if any invariant condition is breached.

## Review Basis

- Base approval strictly on the code under review and the knot description
  plus acceptance criteria.
- Treat acceptance criteria as the source of truth when present; otherwise
  use the description as the requirement baseline.
- Do not use knot notes or prior handoff capsules to decide approval.

## Step Boundary

- This session is authorized only for implementation review.
- Complete exactly one review action, then stop.
- Do not patch code, amend commits, or continue into shipment after a
  review decision.
- After the review decision, handoff, and transition commands succeed,
  stop immediately.

## Actions

1. Review code changes against the knot description and acceptance criteria
2. Verify the implementation respects all knot invariants
3. Verify tests cover the required behavior
4. Verify all sanity gates pass
5. Validate no security issues or regressions introduced
6. Approve or request changes based only on specification and code drift
```

**`knots_sdlc/prompts/shipment.md`**

```markdown
---
accept:
  - Code merged and pushed to main
  - CI green on remote
  - All invariants still hold after merge
  - All commits tagged on the knot

success:
  shipment_complete: ready_for_shipment_review

failure:
  merge_conflicts: ready_for_implementation
  ci_failure: ready_for_implementation

params:
  output_kind:
    type: enum
    values: ["local", "remote", "remote_main", "pr"]
    required: true
    description: Artifact output type from the profile
---

# Shipment

Merge the approved implementation to main and push to remote.

## Invariant Adherence

- If the knot has invariants, verify they still hold after merge and
  before pushing to remote.
- Scope invariants: confirm no out-of-scope changes leaked into the merge.
- State invariants: confirm the required properties hold in the merged
  code on main.

## Step Boundary

- This session is authorized only for shipment.
- Complete exactly one shipment action, then stop.
- Do not perform shipment review or final sign-off in this step.
- After the merge, push, handoff, and transition commands for shipment
  succeed, stop immediately.

## Actions

1. Merge feature branch to main if the profile output kind requires it
2. Tag the knot with any new commit hashes created during merge
3. Push main to remote if the profile output kind requires it
4. Verify CI passes on remote

## Output

- Code merged and pushed to main
- CI green on remote
- Handoff capsule summarizing shipment
```

**`knots_sdlc/prompts/shipment_review.md`**

```markdown
---
accept:
  - Change is live on main branch
  - Every commit tagged on the knot
  - All invariants hold in shipped code
  - CI/CD pipeline completed successfully
  - No regressions in dependent systems

success:
  approved: shipped
  approved_already_merged: shipped

failure:
  needs_revision: ready_for_implementation
  critical_regression: ready_for_implementation
  deployment_issue: ready_for_shipment
  dirty_workspace: ready_for_implementation

params: {}
---

# Shipment Review

Verify the shipped code is live, correct, and regression-free.

## Write Constraints

- Review work is read-only for repository code and git state.
- Do not edit code, tests, docs, configs, or other repository files.
- Do not run git write operations.
- Allowed writes are knot metadata updates only.
- If code/git writes are needed to complete review, stop and use the
  reject path.

## Invariant Review

- If the knot has invariants, verify the shipped code does not violate
  any of them.
- For each scope invariant, confirm only allowed areas were changed.
- For each state invariant, confirm the property still holds on main.
- Reject if any invariant condition is breached.

## Step Boundary

- This session is authorized only for shipment review.
- Complete exactly one review action, then stop.
- Do not fix code, re-run shipment, or continue into other workflow
  stages in this session.
- After the review decision, handoff, and transition commands succeed,
  stop immediately.

## Actions

1. Verify the change is live on main branch
2. Confirm every commit from implementation and shipment is tagged on the
   knot
3. Verify all knot invariants hold in the shipped code
4. Confirm CI/CD pipeline completed successfully
5. Validate no regressions in dependent systems
6. Final sign-off
```

---

## 8. Compatibility and Migration

### 8.1 `loom check-compat`

Checks whether a new workflow version is backward-compatible with an
existing one:

- **Safe**: Adding new states, outcomes, profiles, or phases.
- **Breaking**: Removing states, outcomes, or transitions that existing
  knots may depend on. Renaming state identifiers.
- **Migration required**: Changing outcome targets for existing outcomes.

### 8.2 State Mapping

When migrating knots from a hardcoded workflow to a loom-defined one,
the engine maps existing state strings to state identifiers. Loom can
emit a migration map:

```toml
[state_map]
ready_for_planning = "ready_for_planning"
planning = "planning"
ready_for_plan_review = "ready_for_plan_review"
# ...
```

---

## 9. Open Questions

1. **Should prompts support conditional sections?** For example, showing
   different instructions based on the profile's output kind. Currently
   the `{{ param }}` system handles this via string interpolation, but
   conditional blocks (`{% if output_kind == "pr" %}`) may be needed.

2. **Should phases support more than two steps?** The current model is
   strictly produce + gate. A three-step phase (produce + review + sign-off)
   would require either nested phases or a more flexible composition.

3. **Should loom support workflow inheritance?** A "content pipeline"
   workflow that extends the SDLC workflow with additional states. This
   adds significant complexity and may not be worth it if profiles and
   phase selection cover the use cases.

4. **Should escape states declare their re-entry transitions?** Currently
   `deferred` is reachable from anywhere but re-entry is implicit. An
   explicit `deferred -> ready_for_planning` re-entry declaration would
   make the graph more complete.

5. **Hook points.** Should the language support declaring hook/callback
   points at transitions (e.g., "on entering ready_for_shipment, notify
   Slack")? Or is that purely a runtime concern for the consuming engine?
