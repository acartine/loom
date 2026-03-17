# Phase 1-6 Gap Analysis

This is a spec-compliance gap review of the current implementation against
[`schema.md`](./schema.md) and the positioning in [`README.md`](./README.md).
It is focused on tightening the current foundation before the final phases,
not on reviewing future feature work that is obviously still unimplemented.

## Summary

The current implementation is solid enough to parse, lower, validate the happy
path fixture, render graphs, and emit Rust/TOML artifacts. `cargo test` passes.

The main issue is not basic functionality. It is that several parts of the
implementation are narrower than the schema currently claims:

- validation is workflow-wide, not profile-subgraph-specific
- `loom build` does not enforce validation before emitting artifacts
- code generation is incomplete relative to the documented contract
- prompt parameter typing is only partially validated
- `loom.toml` metadata is parsed but mostly not enforced downstream
- implicit phase transitions are described in the schema but not implemented in
  the graph builder

Those are the areas worth tightening before adding the remaining CLI and
compatibility phases.

## Findings

### 1. Profile-specific validation is missing

Spec basis:

- `schema.md` says validation is performed on each profile's resolved subgraph.
- `README.md` calls out "profile-specific graph failures".

Current behavior:

- `loom_core::validate_workflow` validates the full workflow IR only.
- `graph::profile::extract_profile_subgraph` exists, but is only used by
  `loom graph --profile`.
- There is no validation pass that iterates over profiles and validates each
  extracted subgraph independently.

Why this matters:

- A full workflow can validate even when a specific profile is broken.
- "No planning" profiles can pull in outcome targets like
  `ready_for_planning` without validating whether the resulting profile graph is
  still coherent.

Relevant files:

- `/Users/cartine/loom/crates/loom-core/src/lib.rs`
- `/Users/cartine/loom/crates/loom-core/src/graph/profile.rs`
- `/Users/cartine/loom/crates/loom-core/src/graph/validate.rs`

Recommendation:

- Add a profile validation loop after full-workflow validation.
- Treat each profile as a first-class validation target.
- Fail `loom validate` if any profile subgraph fails.

### 2. `loom build` emits artifacts for invalid workflows

Spec basis:

- `schema.md` describes `loom build` as producing trusted compiled artifacts.

Current behavior:

- `crates/loom-cli/src/commands/build.rs` calls `loom_core::load_workflow`,
  not `validate_workflow`.
- This means `loom build` can emit Rust or TOML for workflows that would fail
  `loom validate`.

Why this matters:

- It weakens the main contract of Loom as a validating compiler.
- Invalid artifacts can leak into downstream consumers.

Observed behavior:

- A workflow with a declared step outside all phases still builds successfully.

Relevant file:

- `/Users/cartine/loom/crates/loom-cli/src/commands/build.rs`

Recommendation:

- Make `build` run the same validation path as `validate`.
- Emit warnings, but refuse to generate artifacts when errors exist.

### 3. Rust/TOML codegen is short of the documented contract

Spec basis:

- `schema.md` promises:
  - state enums
  - per-action outcome enums
  - a transition function `apply(state, outcome) -> Result<State>`
  - profile structs
  - prompt metadata including acceptance criteria, parameters, and outcome
    lists

Current behavior:

- Rust codegen emits state enums, executor/output enums, outcome enums,
  profiles, and a minimal `PromptMeta`.
- There is no generated transition function.
- `PromptMeta` contains only `name`, `accept`, and `body`.
- Prompt parameter definitions and outcome metadata are not emitted.
- TOML output omits prompt bodies, params, and package metadata such as
  `default_profile`.

Why this matters:

- The generated artifacts do not yet match the data model described in the
  schema.
- Downstream consumers cannot rely on generated metadata as documented.

Relevant files:

- `/Users/cartine/loom/crates/loom-core/src/codegen/rust.rs`
- `/Users/cartine/loom/crates/loom-core/src/codegen/toml_emit.rs`

Recommendation:

- Either implement the missing generated contract now, or narrow the schema so
  it only promises what the compiler actually emits today.

### 4. Prompt parameter validation is too shallow

Spec basis:

- `schema.md` defines typed prompt params, enum values, defaults, and required
  flags.
- It explicitly calls out validation of prompt/body parameter usage.

Current behavior:

- Prompt frontmatter is deserialized into typed structs.
- Lowering checks only one semantic rule: every `{{ param }}` used in the body
  must appear in `params`.
- There is no semantic validation that:
  - enum params include `values`
  - defaults conform to declared types
  - defaults belong to enum domains

Why this matters:

- Prompt metadata can be structurally accepted while remaining semantically
  invalid.
- The typed parameter model is not yet trustworthy for downstream consumers.

Relevant files:

- `/Users/cartine/loom/crates/loom-core/src/prompt/mod.rs`
- `/Users/cartine/loom/crates/loom-core/src/ir/lower.rs`

Recommendation:

- Add prompt semantic validation after YAML parse and before IR finalization.
- Start with enum/value/default checks since those are highest-signal.

### 5. `loom.toml` metadata is parsed but barely enforced

Spec basis:

- `schema.md` defines package metadata including `default_profile`.

Current behavior:

- `loom.toml` is loaded and parsed.
- `entry` is used to locate the root workflow file.
- `default_profile` is parsed but not validated against the defined profiles.
- Config metadata is not carried through IR or emitted into TOML output.

Why this matters:

- Package metadata currently behaves more like loose input than part of the
  compiled contract.
- Invalid `default_profile` values pass validation.

Relevant files:

- `/Users/cartine/loom/crates/loom-core/src/config.rs`
- `/Users/cartine/loom/crates/loom-core/src/lib.rs`
- `/Users/cartine/loom/crates/loom-core/src/codegen/toml_emit.rs`

Recommendation:

- Validate that `default_profile` exists.
- Decide whether config metadata should live in IR and interchange output.

### 6. Implicit phase transitions are documented but not implemented

Spec basis:

- `schema.md` says the compiler generates implicit phase transitions from a
  produce action's success outcomes to the gate queue, unless explicit routing
  overrides them.

Current behavior:

- `EdgeKind::PhaseLink` exists.
- `graph::build_graph` has a placeholder block for phase links.
- No phase-link edges are actually added.

Why this matters:

- The schema describes semantics that the graph builder does not currently
  enforce.
- Today, the fixture works because prompts explicitly route produce success
  outcomes to the next queue.

Relevant file:

- `/Users/cartine/loom/crates/loom-core/src/graph/mod.rs`

Recommendation:

- Either implement the implicit edges, or revise the schema to state that phase
  progression is currently explicit via prompt routing.

## Test Coverage Gaps

The current suite is heavily weighted toward the happy-path fixture.

Missing or weak areas:

- no targeted negative tests for duplicate identifiers
- no targeted negative tests for unresolved references
- no targeted negative tests for invalid overrides
- no targeted negative tests for invalid `default_profile`
- no targeted negative tests for prompt param typing rules
- no test asserting that `build` fails when validation fails
- no test asserting per-profile validation behavior

Recommendation:

- Add focused unit tests around each validation rule before expanding the
  feature surface further.

## Tighten Before Final Phases

1. Make `loom build` validation-gated.
2. Validate every extracted profile subgraph, not only the full workflow.
3. Clarify the semantics of skipped phases and outcome targets outside the
   active profile.
4. Bring codegen and TOML emit in line with the documented contract, or narrow
   the docs immediately.
5. Add semantic validation for prompt parameter typing.
6. Validate `default_profile` and decide how config metadata propagates.
7. Add negative tests for the above so future phases do not stack on a loose
   foundation.

## Still-Unimplemented Later-Phase Surface

These are not gaps in phases 1-6 so much as clearly remaining work:

- `loom sim`
- `loom diff`
- `loom check-compat`
- ASCII graph output
- Go codegen
- Python codegen

Those are fine to leave for later, but the validation and contract issues above
should be tightened first so the remaining phases build on something strict.
