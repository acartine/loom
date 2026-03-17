# Getting Started With Loom

This guide takes you from a fresh clone to a validated workflow and generated code.

## Before you start

- Rust stable 1.75 or newer
- Git
- Optional: Go 1.21+ or Python 3.10+ if you want to compile generated artifacts for those targets

## 1. Install the CLI

Standard curl install:

```bash
curl -fsSL https://raw.githubusercontent.com/acartine/loom/main/install.sh | sh
```

Or build from source:

```bash
git clone https://github.com/acartine/loom.git
cd loom
cargo install --locked --path crates/loom-cli
```

If you do not want to install the binary yet, replace `loom` with `cargo run -p loom-cli --` in the commands below.

## 2. Validate the reference workflow

Before writing your own workflow, confirm that the bundled fixture works:

```bash
loom validate tests/fixtures/knots_sdlc
loom graph tests/fixtures/knots_sdlc --format ascii
loom build tests/fixtures/knots_sdlc --lang rust > /tmp/knots_workflow.rs
```

This proves the compiler, prompt loading, graph validation, and code generation pipeline are working in your environment.

## 3. Scaffold a workflow

Create a new workflow directory:

```bash
loom init support_triage
cd support_triage
```

The scaffold contains:

- `workflow.loom`: the workflow definition
- `loom.toml`: package metadata
- `prompts/work.md`: the produce prompt with outcome routing
- `prompts/review.md`: the gate prompt with outcome routing

## 4. Read the scaffold

The generated workflow is intentionally small:

```loom
workflow support_triage v1 {
    queue ready_for_work "Ready for Work"
    queue ready_for_review "Ready for Review"

    action work "Work" {
        produce agent
        prompt work
    }

    action review "Review" {
        gate review human
        prompt review
    }

    terminal done "Done"

    step do_work {
        ready_for_work -> work
    }

    step review_work {
        ready_for_review -> review
    }

    phase main {
        produce do_work
        gate review_work
    }

    profile default "Default" {
        description "Default profile"
        phases [main]
        output local
    }
}
```

The produce prompt defines the first action's outcomes:

```yaml
---
accept:
  - Work is complete
  - Handoff notes are ready for review

success:
  completed: ready_for_review

failure:
  blocked: ready_for_work

params: {}
---
```

The review prompt routes approval to `done` and changes back to `ready_for_work`. Loom validates all of these files together, so prompt routing errors are caught before code generation.

## 5. Validate your workflow

```bash
loom validate
```

This checks:

- workflow graph structure
- prompt outcome targets
- profile configuration
- unresolved references

## 6. Generate code

Choose a target language:

```bash
loom build --lang rust > workflow.rs
loom build --lang go > workflow.go
loom build --lang python > workflow.py
```

If you want a machine-readable intermediate form:

```bash
loom build --emit toml > workflow.toml
```

## 7. Render the graph

```bash
loom graph --format mermaid > workflow.mmd
```

Other formats:

- `loom graph --format ascii`
- `loom graph --format dot`

## 8. Simulate transitions

```bash
loom sim
```

Use the simulator to walk the workflow and confirm that outcomes and wildcard transitions match the behavior you expect.

## 9. Graduate to phases and profiles

The scaffold proves the basic loop. Real workflows usually add:

- more queue and action states
- phases that pair produce and gate steps
- multiple profiles such as `autopilot` and `semiauto`
- richer prompt acceptance criteria and parameters

The best complete example in this repository is [`tests/fixtures/knots_sdlc`](/Users/cartine/loom/tests/fixtures/knots_sdlc). It includes:

- 15 states
- 6 steps
- 3 phases
- 6 execution profiles
- prompt files for planning, implementation, review, and shipment

## Next references

- [`README.md`](/Users/cartine/loom/README.md)
- [`docs/releasing.md`](/Users/cartine/loom/docs/releasing.md)
- [`schema.md`](/Users/cartine/loom/schema.md)
- [`CONTRIBUTING.md`](/Users/cartine/loom/CONTRIBUTING.md)
