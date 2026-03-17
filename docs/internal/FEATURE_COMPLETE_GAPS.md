# Feature Complete Gap Assessment

Assessed against the current implementation on `main`, using:

- `README.md`
- `schema.md`
- current CLI/codegen surface in `crates/loom-cli` and `crates/loom-core`

## Verdict

The current implementation is substantially tighter than the original phase
1-6 drop and the implemented surface appears stable. `cargo test` passes and
the major semantic gaps previously identified around validation/codegen
consistency have been addressed.

However, the repo is **not feature complete relative to the language spec**.
The remaining gaps are now mostly scope gaps, not correctness gaps.

## What Looks Complete

- PEG parsing for workflow and profile files
- Prompt frontmatter parsing and parameter extraction
- IR lowering with reference resolution
- Workflow validation plus per-profile subgraph validation
- Rust code generation
- TOML interchange emission
- Mermaid and DOT graph output
- CLI commands:
  - `init`
  - `validate`
  - `build`
  - `graph`

## Remaining Gaps

### 1. The schema still promises a larger CLI than the implementation provides

`schema.md` documents the following commands as part of the CLI surface:

- `loom sim <profile>`
- `loom diff <v1-dir> <v2-dir>`
- `loom check-compat <old> <new>`
- `loom graph ... ascii`
- `loom build [--lang <target>]` with `rust`, `go`, and `python`

Current implementation:

- `crates/loom-cli/src/main.rs` only exposes:
  - `init`
  - `validate`
  - `build`
  - `graph`
- `graph` supports only `mermaid` and `dot`
- `build` supports only Rust codegen or TOML emit

Impact:

- If "feature complete" means "feature complete against the spec", this is the
  main blocker.

Evidence:

- `schema.md` CLI reference
- `crates/loom-cli/src/main.rs`

### 2. Go and Python codegen are still unimplemented

The schema still describes multi-target code generation and includes a Go
example.

Current implementation:

- `crates/loom-core/src/codegen/mod.rs` has a single `CodegenTarget::Rust`
- there are no Go or Python codegen modules
- the CLI integration test suite explicitly asserts that
  `build --lang python` fails

Impact:

- "typed code generation" is currently Rust-only in practice
- the codegen contract is still narrower than the spec's advertised surface

Evidence:

- `schema.md` sections 4.3 and 5
- `crates/loom-core/src/codegen/mod.rs`
- `crates/loom-cli/tests/cli_integration.rs`

### 3. Compatibility and migration tooling are still design-only

`schema.md` includes dedicated sections for backward compatibility and state
mapping:

- `loom check-compat`
- migration/state-map emission

Current implementation:

- no CLI command
- no library entrypoint for compatibility analysis
- no emitted migration map

Impact:

- one of the more important "compiler" promises in the spec is still not
  implemented
- version-to-version workflow evolution remains manual

Evidence:

- `schema.md` sections 8.1 and 8.2
- absence of corresponding command/library modules in the codebase

### 4. README still says the missing feature set is "coming next"

The repo's own README does not currently claim feature completeness. It still
calls out the remaining feature set as future work:

- Go and Python codegen
- interactive simulator
- workflow diff
- backward compatibility checking

Impact:

- even the user-facing project status does not support a "feature complete"
  framing yet

Evidence:

- `README.md` status section
- `PHASE_7-11.md`

### 5. Minor documentation drift remains around implicit phase transitions

The current graph builder no longer generates implicit phase-link edges because
produce actions are required to declare explicit success outcomes. In practice,
prompt routing is authoritative.

But the docs still describe graph construction as including "implicit phase
transitions", which is broader than the current implementation model.

Impact:

- low severity
- mostly a docs/spec clarity issue now, not a product correctness issue

Evidence:

- `README.md` implemented-features list
- `crates/loom-core/src/graph/mod.rs`

## Assessment Summary

If the bar is:

- **Feature complete for the currently implemented core compiler surface**
  then the answer is close to yes.

- **Feature complete against the full spec and advertised roadmap surface**
  then the answer is no.

The remaining work is concentrated in the old phase 7-11 area:

- extra CLI commands
- compatibility analysis
- non-Rust codegen targets
- possible cleanup of spec/docs language to match the current architecture

## Recommended Next Step

Pick one of these paths explicitly:

1. **Narrow the contract**
   Update `schema.md` so it documents only the currently implemented surface.

2. **Finish the missing surface**
   Implement:
   - `sim`
   - `diff`
   - `check-compat`
   - ASCII graph output
   - Go codegen
   - Python codegen

Without one of those two moves, "feature complete" is ambiguous and likely
misleading.
