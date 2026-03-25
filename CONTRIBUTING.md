# Contributing to Loom

## Prerequisites

- Rust stable 1.75+
- `cargo`
- Optional: Go 1.21+ and Python 3.10+ if you want to sanity-check generated code in those targets

## Development Setup

```bash
git clone https://github.com/acartine/loom.git
cd loom
cargo build
cargo test
cargo run -p loom-cli -- validate tests/fixtures/knots_sdlc
```

If you change the CLI, language semantics, or generated output, also run:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
```

## Contributor Flow

1. Change the smallest sensible layer: grammar, lowering, validation, graph, codegen, or CLI.
2. Add or update focused tests.
3. Run `cargo test`.
4. If user-facing behavior changed, update [README.md](README.md) or [docs/getting-started.md](docs/getting-started.md).
5. If language semantics changed, update [schema.md](schema.md) in the same change.
6. If install or release behavior changed, update [docs/releasing.md](docs/releasing.md) and `install.sh`.

## Project Layout

```text
loom/
  Cargo.toml
  README.md
  CONTRIBUTING.md
  schema.md
  crates/
    loom-core/
      src/
        lib.rs
        grammar.pest
        error.rs
        config.rs
        parse/
        prompt/
        ir/
        graph/
        codegen/
        sim.rs
        diff.rs
        compat.rs
    loom-cli/
      src/
        main.rs
        commands/
  tests/
    fixtures/
      knots_sdlc/
      knots_sdlc_v2/
```

## Architecture

Loom has two crates:

- `loom-core`: parser, IR lowering, graph construction, validation, code generation, simulation, diff, and compatibility logic
- `loom-cli`: the `loom` executable and command wiring

Compiler pipeline:

```text
.loom files + prompts/*.md + loom.toml
        │
     parse      grammar -> AST
        │
     lower      AST -> IR + prompt loading
        │
     graph      IR -> workflow graph
        │
     validate   graph checks on the full workflow and each profile
        │
     codegen    Rust / Go / Python / TOML
```

Important design constraints:

- The compiler accumulates diagnostics instead of stopping at the first error.
- Name resolution is two-phase so forward references are legal where the language allows them.
- Wildcard transitions are expanded during graph construction.
- Profiles are validated as filtered subgraphs of the full workflow.
- `loom build` is intentionally validation-gated.
- `schema.md` is the source of truth for language behavior.

## Reference Fixtures

`tests/fixtures/knots_sdlc/` is the main end-to-end fixture. It should continue to:

- Parse cleanly
- Validate cleanly
- Render graphs
- Simulate correctly
- Generate code for all supported targets

`tests/fixtures/knots_sdlc_v2/` exists primarily for `diff` and `check-compat` coverage.

## Common Work

### Add a validation rule

1. Add or update the diagnostic in `crates/loom-core/src/error.rs`.
2. Implement the rule in `crates/loom-core/src/graph/validate.rs` or `crates/loom-core/src/ir/lower.rs`.
3. Add a focused negative test.
4. Re-run the reference fixture validation.

### Add or change code generation

1. Update the relevant file under `crates/loom-core/src/codegen/`.
2. Wire the target in `codegen/mod.rs` and `crates/loom-cli/src/commands/build.rs` if needed.
3. Update CLI argument handling in `crates/loom-cli/src/main.rs` if the surface area changes.
4. Add or update tests for generated output.

### Change the language

1. Update `schema.md` first or in the same change.
2. Keep `grammar.pest` and parser behavior aligned with the spec.
3. Update README or `docs/getting-started.md` if the onboarding story changes.
4. Add fixture coverage for any new syntax or semantics.

## Documentation Expectations

Launch-facing docs should stay aligned with the shipped CLI. In practice that means:

- README examples must be copy-pasteable against the current repo
- New commands or flags must be reflected in the README and spec when user-facing
- `loom init` behavior should be documented as it actually scaffolds, not as an idealized future layout
- Public docs should use repo-relative Markdown links, not local filesystem paths
- If the onboarding flow changes, update [README.md](README.md) and [docs/getting-started.md](docs/getting-started.md) in the same change
- If release assets or install behavior change, update [docs/releasing.md](docs/releasing.md) in the same change

## Style

- Prefer direct code over premature abstraction.
- Keep tests close to the code in `#[cfg(test)] mod tests` where practical.
- Make diagnostics specific and actionable.
- Do not leave the implementation and spec disagreeing.

## Commits

- Use clear, imperative titles under 72 characters.
- Add a scope prefix when it helps, for example `validate:`, `codegen:`, or `docs:`.
- Do not commit generated artifacts or secrets.
