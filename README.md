# Loom

![Coverage: 92%](https://img.shields.io/badge/coverage-92%25-brightgreen)

**A language and compiler for agentic workflows.**

Loom lets you define complex agent workflows as structured, typed programs — then validates them at compile time and generates typed code. No runtime surprises. No implicit routing. No dead states hiding in production.

```
loom validate    # catch every mistake before anything runs
loom build       # generate typed code from your workflow
loom graph       # visualize the full state machine
```

## The problem

Agent workflows today are a mess:

- State machines hardcoded across multiple files
- Prompt strings buried in application code
- Routing logic scattered through runtime handlers
- Transitions that only fail in production
- No way to know if a change breaks existing flows

When an agent workflow is 15 states deep with 6 different execution profiles, you need a compiler — not a config file.

## What Loom does

Loom treats a workflow as a **typed program**, not a bag of YAML.

You write `.loom` files that declare states, steps, phases, and profiles. Prompts live in markdown files with structured frontmatter that defines outcome routing. The compiler validates the entire graph, resolves every reference, and generates self-contained code.

```
knots_sdlc/
  workflow.loom       # states, steps, phases, profiles
  loom.toml           # package metadata
  prompts/
    planning.md       # prompt + outcome routing
    plan_review.md
    implementation.md
    ...
  profiles/
    autopilot.loom    # full agent autonomy
    semiauto.loom     # human-gated reviews
```

### Prompts own routing

The prompt is the routing table. Each prompt declares its own outcomes and where they lead:

```yaml
---
success:
  plan_complete: ready_for_plan_review
failure:
  insufficient_context: ready_for_planning
  out_of_scope: ready_for_planning
---
```

This keeps transition logic next to the action that produces it, not scattered through runtime code.

### Profiles are subgraphs

Profiles select which phases are active and override who executes each action. They're not patches on a default — they're independent subgraphs, validated independently.

```
profile autopilot "Autopilot" {
    phases [planning_phase, implementation_phase, shipment_phase]
    output remote_main
}

profile semiauto "Semi-automatic" {
    phases [planning_phase, implementation_phase, shipment_phase]
    output remote_main
    override plan_review { executor human }
    override implementation_review { executor human }
}
```

### Compile-time validation

The compiler catches mistakes that would otherwise surface at runtime:

- Dead states with no inbound transitions
- States that can never reach a terminal
- Missing or orphaned prompt files
- Outcome targets that don't exist
- Prompt parameters referenced but not declared
- Invalid profile overrides
- Phase composition errors
- Per-profile subgraph validation

Every profile is validated as an independent graph. A workflow can define phases that only make sense in combination — the compiler checks each profile's view of the world separately.

## CLI

```bash
loom init <name>               # scaffold a new workflow
loom validate [dir]            # full validation pipeline
loom build [--lang rust]       # generate typed code
loom build --emit toml         # TOML interchange format
loom graph [--profile <name>]  # mermaid or DOT output
```

### Quick start

```bash
cargo install --path crates/loom-cli

# Create a new workflow
loom init my_workflow
cd my_workflow

# Validate it
loom validate

# Generate Rust code
loom build --lang rust > src/workflow.rs

# Visualize the graph
loom graph --format mermaid
```

## Code generation

`loom build` produces self-contained code with no runtime dependency on Loom:

- **State enum** with all workflow states
- **Outcome enums** per action with `target()` and `is_success()` methods
- **Transition function** `apply(state, outcome) -> Result<State>`
- **Profile constants** with phase lists, output kinds, and executor maps
- **Prompt metadata** including acceptance criteria, typed parameters, and outcome routing

The generated code compiles on its own. Your runtime system consumes it directly.

## Architecture

Loom is a Rust workspace with two crates:

- **`loom-core`** — library: PEG parser, AST, IR with two-phase name resolution, petgraph-based graph analysis, validation, codegen
- **`loom-cli`** — binary: the `loom` command

The compiler pipeline:

```
.loom files + prompts/*.md + loom.toml
        |
     [ parse ]      PEG grammar -> AST
        |
     [ lower ]      AST -> IR (name resolution, prompt loading)
        |
     [ graph ]      IR -> petgraph (implicit transitions, wildcards)
        |
     [ validate ]   12 error checks + 5 warning checks, per-profile
        |
     [ codegen ]    IR -> Rust / TOML
```

## Spec

The full language specification lives in [`schema.md`](./schema.md). It covers:

- File and package structure
- The `.loom` PEG grammar
- Prompt frontmatter schema
- Profile selection and overrides
- Compiler semantics and implicit transitions
- Graph validation rules
- Code generation contracts
- A complete reference workflow (Knots SDLC with 6 profiles)

## Status

The core compiler is implemented and working. The reference Knots SDLC workflow (15 states, 6 steps, 3 phases, 6 profiles) parses, validates, and generates compilable Rust code.

**Implemented:**
- Full PEG parser for `.loom` files and profile files
- YAML frontmatter prompt parser with parameter extraction
- IR lowering with two-phase name resolution
- Graph construction with implicit phase transitions and wildcard expansion
- 12 error checks + 5 warning checks with per-profile subgraph validation
- Rust code generation (state enums, outcome enums, transition function, profiles, prompt metadata)
- TOML interchange format
- Mermaid and DOT graph output
- CLI: `init`, `validate`, `build`, `graph`

**Coming next:** Go and Python codegen, interactive simulator, workflow diff, backward compatibility checking. See [PHASE_7-11.md](./PHASE_7-11.md).

## License

MIT
