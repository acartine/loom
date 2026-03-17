# Loom CLI — Phase 2 Implementation Plan

## Context

Phase 1 shipped the core Loom compiler: PEG parser, AST, IR with two-phase name resolution, petgraph-based graph construction and validation, profile subgraph extraction, Rust codegen, TOML interchange emit, mermaid/DOT rendering, and a clap CLI with `init`, `validate`, `build`, and `graph` commands. 24 tests pass. The knots_sdlc reference workflow parses, validates, and generates compilable Rust code.

This plan covers the deferred items from the original plan plus quality-of-life improvements identified during phase 1.

## What's Deferred

From the spec (sections 4.3, 5, 8):
1. **Go codegen** — `loom build --lang go` (spec section 4.3 has a full example)
2. **Python codegen** — `loom build --lang python`
3. **Interactive simulator** — `loom sim <profile>`
4. **Workflow diff** — `loom diff <v1-dir> <v2-dir>`
5. **Compatibility checking** — `loom check-compat <old> <new>` (spec section 8)

## Phases

### Phase 7: Go Codegen

**File:** `crates/loom-core/src/codegen/go.rs`

Generate Go code matching the spec section 4.3 example:
- `type State int` with `iota` constants
- One `type XxxOutcome int` per action with `iota` constants
- `func (o XxxOutcome) Target() State` — switch-based routing
- `func (o XxxOutcome) IsSuccess() bool`
- `type Executor int` with `Agent`/`Human` constants
- `type OutputKind int` with constants
- Profile structs with phase list, output kind, executor map

Wire into `CodegenTarget::Go` and `loom build --lang go`.

**Test:** Generate Go for knots_sdlc, write to a temp file, run `go vet` on it.

### Phase 8: Python Codegen

**File:** `crates/loom-core/src/codegen/python.rs`

Generate Python code:
- `class State(enum.Enum)` with members
- `class XxxOutcome(enum.Enum)` per action with `target()` and `is_success()` methods
- `class Executor(enum.Enum)` with `AGENT`/`HUMAN`
- `class OutputKind(enum.Enum)` with members
- `@dataclass` Profile with phase list, output kind, executor overrides
- Prompt metadata as module-level constants

Wire into `CodegenTarget::Python` and `loom build --lang python`.

**Test:** Generate Python for knots_sdlc, write to a temp file, run `python -m py_compile` on it.

### Phase 9: Interactive Simulator

**Files:**
- `crates/loom-core/src/sim.rs` — simulator engine
- `crates/loom-cli/src/commands/sim.rs` — CLI command

The simulator lets you walk a workflow interactively:

```
$ loom sim autopilot
[autopilot] Starting at: ready_for_planning

Current state: ready_for_planning (Ready for Planning)
  1. claim (plan) -> planning

> 1
Transitioned to: planning (Planning)
  Outcomes:
    1. [ok]   plan_complete -> ready_for_plan_review
    2. [fail] insufficient_context -> ready_for_planning
    3. [fail] out_of_scope -> ready_for_planning
    4. [*]    -> abandoned
    5. [*]    -> deferred

> 1
Transitioned to: ready_for_plan_review (Ready for Plan Review)
...
```

Engine:
- `SimState` holds current state name, history of transitions taken
- `available_transitions(state, ir, profile) -> Vec<SimTransition>` — returns claim edges, outcome edges, and wildcard edges from the graph
- `apply(transition) -> SimState` — advance the state

CLI:
- `loom sim <profile> [--dir <dir>]` — interactive REPL loop
- Print current state, numbered transition options
- Read user input, apply transition, repeat
- Exit on terminal state or `q`/`quit`

**Test:** Unit test `available_transitions` and `apply` against knots_sdlc IR. No interactive test needed.

### Phase 10: Workflow Diff

**Files:**
- `crates/loom-core/src/diff.rs` — diff engine
- `crates/loom-cli/src/commands/diff.rs` — CLI command

`loom diff <v1-dir> <v2-dir>` loads both workflows and reports structural changes:

- **States:** added, removed, renamed (display name changed), type changed
- **Steps:** added, removed, queue/action changed
- **Phases:** added, removed, steps changed
- **Profiles:** added, removed, phases/output/overrides changed
- **Outcomes:** added, removed, target changed (per prompt)
- **Transitions:** edges added or removed in the graph

Output format: human-readable text with `+`/`-`/`~` prefixes (like a git diff but for workflow structure).

```
$ loom diff v1/ v2/
States:
  + new_queue "New Queue"
  - old_action "Old Action"
  ~ planning: display_name "Planning" -> "Plan Creation"

Outcomes (planning):
  + new_outcome -> some_state [ok]
  - removed_outcome [fail]

Profiles:
  + new_profile
  ~ autopilot: phases added [new_phase]
```

**Test:** Create a `v2` fixture that modifies knots_sdlc (add a state, remove an outcome, change a display name), diff against v1.

### Phase 11: Compatibility Checking

**Files:**
- `crates/loom-core/src/compat.rs` — compat checker
- `crates/loom-cli/src/commands/compat.rs` — CLI command

`loom check-compat <old-dir> <new-dir>` builds on the diff engine to classify changes per spec section 8.1:

| Classification | Condition |
|---|---|
| **Safe** | Adding new states, outcomes, profiles, or phases |
| **Breaking** | Removing states, outcomes, or transitions that existing consumers depend on. Renaming state identifiers. |
| **Migration required** | Changing outcome targets for existing outcomes |

Output:
```
$ loom check-compat v1/ v2/
safe: 2 additions
  + state: new_queue
  + profile: new_profile

breaking: 1 removal
  - state: old_action (may have active knots)

migration: 1 target change
  ~ outcome planning.plan_complete: ready_for_plan_review -> ready_for_new_review

Result: NOT backward compatible (1 breaking, 1 migration)
```

Also emit a state migration map (spec 8.2) with `--emit-map`:
```toml
[state_map]
ready_for_planning = "ready_for_planning"
new_queue = "new_queue"
# old_action = REMOVED
```

**Test:** Fixtures with safe-only, breaking, and migration-required changes. Assert classification is correct.

## Implementation Order

Phases 7 and 8 (Go/Python codegen) are independent of each other and of phases 9–11. Phases 10 and 11 are sequential (compat builds on diff).

```
Phase 7 (Go)  ──┐
Phase 8 (Python)─┤── can be parallel
Phase 9 (Sim)  ──┘
Phase 10 (Diff) ──> Phase 11 (Compat)
```

## Files to Create

| File | Phase | Purpose |
|------|-------|---------|
| `crates/loom-core/src/codegen/go.rs` | 7 | Go code generation |
| `crates/loom-core/src/codegen/python.rs` | 8 | Python code generation |
| `crates/loom-core/src/sim.rs` | 9 | Simulator engine |
| `crates/loom-cli/src/commands/sim.rs` | 9 | Simulator CLI command |
| `crates/loom-core/src/diff.rs` | 10 | Diff engine |
| `crates/loom-cli/src/commands/diff.rs` | 10 | Diff CLI command |
| `crates/loom-core/src/compat.rs` | 11 | Compat checker |
| `crates/loom-cli/src/commands/compat.rs` | 11 | Compat CLI command |
| `tests/fixtures/knots_sdlc_v2/` | 10–11 | Modified fixture for diff/compat tests |

## Files to Modify

| File | Phase | Change |
|------|-------|--------|
| `crates/loom-core/src/codegen/mod.rs` | 7, 8 | Add `Go`/`Python` to `CodegenTarget`, dispatch |
| `crates/loom-core/src/lib.rs` | 9, 10, 11 | Add `pub mod sim`, `pub mod diff`, `pub mod compat` |
| `crates/loom-cli/src/main.rs` | 7–11 | Add `Sim`, `Diff`, `CheckCompat` subcommands, add `go`/`python` to `--lang` |
| `crates/loom-cli/src/commands/mod.rs` | 9–11 | Add `pub mod sim`, `pub mod diff`, `pub mod compat` |
| `crates/loom-cli/src/commands/build.rs` | 7, 8 | Add `Go`/`Python` to `EmitFormat` |

## Verification

1. `cargo test` — all existing + new tests pass
2. `loom build --lang go tests/fixtures/knots_sdlc | go vet` — generated Go compiles
3. `loom build --lang python tests/fixtures/knots_sdlc | python -m py_compile` — generated Python compiles
4. `loom sim autopilot` on knots_sdlc — interactive walkthrough works
5. `loom diff tests/fixtures/knots_sdlc tests/fixtures/knots_sdlc_v2` — reports expected changes
6. `loom check-compat tests/fixtures/knots_sdlc tests/fixtures/knots_sdlc_v2` — classifies correctly
