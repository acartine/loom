# Contributing to Loom

## Prerequisites

- Rust stable (1.75+)
- `cargo` (comes with Rust)

Optional, for testing codegen output:
- Go 1.21+ (for `go vet` on generated Go code)
- Python 3.10+ (for `py_compile` on generated Python code)

## Getting started

```bash
git clone https://github.com/acartine/loom.git
cd loom
cargo build
cargo test
```

## Project layout

```
loom/
  Cargo.toml                    # workspace root
  schema.md                     # language specification (source of truth)
  crates/
    loom-core/                  # library crate
      src/
        lib.rs                  # load_workflow(), validate_workflow()
        grammar.pest            # PEG grammar (must match schema.md section 2)
        error.rs                # LoomError, LoomWarning, Diagnostics
        config.rs               # loom.toml parsing
        parse/
          mod.rs                # pest -> AST
          ast.rs                # raw AST types
        prompt/
          mod.rs                # YAML frontmatter + {{ param }} extraction
        ir/
          mod.rs                # WorkflowIR, StateDef, StepDef, etc.
          lower.rs              # AST -> IR (two-phase name resolution)
        graph/
          mod.rs                # petgraph construction
          validate.rs           # error checks + warning checks
          profile.rs            # profile subgraph extraction
          render.rs             # mermaid + DOT output
        codegen/
          mod.rs                # CodegenTarget dispatch
          rust.rs               # Rust code generation
          toml_emit.rs          # TOML interchange format
    loom-cli/                   # binary crate
      src/
        main.rs                 # clap CLI entrypoint
        commands/
          init.rs               # loom init
          validate.rs           # loom validate
          build.rs              # loom build
          graph.rs              # loom graph
  tests/
    fixtures/
      knots_sdlc/              # reference workflow from schema.md section 7
```

## Compiler pipeline

The compiler has a strict phase order. Each phase consumes the output of the previous one:

1. **Parse** — PEG grammar (`grammar.pest`) produces pest pairs, converted to AST types (`parse/ast.rs`)
2. **Lower** — AST is lowered to IR (`ir/lower.rs`). This is a two-phase process: first register all declarations, then resolve all references. Prompts are loaded from disk during lowering.
3. **Graph** — IR is converted to a petgraph (`graph/mod.rs`). Implicit transitions (step claims, phase links, wildcards) are materialized as edges.
4. **Validate** — The graph is checked for errors (`graph/validate.rs`). Validation runs on the full workflow and on each profile's subgraph independently.
5. **Codegen** — IR is emitted as Rust code or TOML (`codegen/`).

The IR is the hub. AST is discarded after lowering. All downstream operations work from `WorkflowIR`.

## Key design decisions

- **Error accumulation**: The compiler collects all errors before reporting. It does not bail on the first error. This is handled by the `Diagnostics` struct in `error.rs`.
- **Two-phase name resolution**: Register all names first, then resolve references. This handles forward declarations cleanly.
- **Wildcard expansion**: `* -> target` expands to N concrete edges during graph construction.
- **Profile subgraph as filtered view**: `graph::profile::extract_profile_subgraph` filters the full graph rather than building separate IRs.
- **Validation gates codegen**: `loom build` runs the full validation pipeline. It refuses to emit artifacts for invalid workflows.

## Running tests

```bash
# All tests
cargo test

# Specific module
cargo test --lib parse
cargo test --lib ir::lower
cargo test --lib graph::validate
cargo test --lib codegen::rust
```

The test suite includes:
- **Happy-path integration tests** against the `knots_sdlc` reference fixture
- **Negative tests** for each validation rule (duplicate identifiers, unresolved references, type mismatches, invalid overrides, bad prompt params, invalid default_profile)
- **Codegen verification** — generated Rust code is checked for expected output

## The reference fixture

`tests/fixtures/knots_sdlc/` contains the complete Knots SDLC workflow from schema.md section 7. It has:
- 6 queue states, 6 action states, 2 terminals, 1 escape
- 6 steps, 3 phases
- 6 profiles (autopilot, autopilot_with_pr, semiauto, and no-planning variants)
- 6 prompt files with frontmatter

This fixture is the primary integration test target. If you change the compiler, it should still parse, lower, validate, and generate compilable code for this workflow.

## Adding a new validation check

1. Add the error variant to `LoomError` in `error.rs` (or `LoomWarning` for non-fatal checks)
2. Implement the check in `graph/validate.rs` (graph-level) or `ir/lower.rs` (name-resolution-level)
3. Add a negative test that triggers the new error
4. Verify the knots_sdlc fixture still validates clean

## Adding a new codegen target

1. Create `crates/loom-core/src/codegen/<lang>.rs`
2. Add the variant to `CodegenTarget` in `codegen/mod.rs` and wire the dispatch
3. Add the lang option to `EmitFormat` in `crates/loom-cli/src/commands/build.rs`
4. Add the `--lang <name>` match arm in `crates/loom-cli/src/main.rs`
5. Test: generate code for knots_sdlc and verify it compiles in the target language

## Style notes

- No unnecessary abstractions. Three similar lines are better than a premature helper.
- Tests go in `#[cfg(test)] mod tests` at the bottom of each file, not in separate test files.
- Error messages should be specific and actionable. Include the identifier name and context.
- The `schema.md` spec is the source of truth. If the compiler disagrees with the spec, fix the compiler or update the spec — don't leave them out of sync.

## Commit messages

- Clear, imperative titles under 72 characters
- Scope prefix when helpful: `codegen: add Go target`, `validate: check enum param values`
- Never commit secrets or generated artifacts
