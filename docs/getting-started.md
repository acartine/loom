# Getting Started With Loom

This guide takes you from a fresh install to a validated workflow and generated code.

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

To refresh an installed release later without re-running the shell installer:

```bash
loom update
loom update --check
```

If you do not want to install the binary yet, clone the repo and replace `loom` with `cargo run -p loom-cli --` in the commands below.

## 2. Inspect the bundled templates

```bash
loom templates list
```

Loom ships with:

- `minimal`: one produce step, one review step, one phase, one default profile
- `knots_sdlc`: planning, implementation, review, shipment, and multiple execution profiles

## 3. Scaffold the full Knots SDLC workflow

Create a new workflow directory from the bundled `knots_sdlc` template:

```bash
loom init knots_sdlc
cd knots_sdlc
```

If you want the same template under a different directory and workflow name, use:

```bash
loom init --template knots_sdlc my_team_flow
cd my_team_flow
```

The scaffold contains the full bundled workflow package:

- `workflow.loom`: the workflow definition
- `loom.toml`: package metadata
- `prompts/`: prompt files for planning, plan review, implementation, implementation review, shipment, and shipment review
- `profiles/`: bundled profiles such as `autopilot`, `autopilot_with_pr`, and `semiauto`

## 4. Read the scaffold

The generated workflow is the full Knots SDLC shape:

```loom
workflow knots_sdlc v1 {
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

    terminal shipped
    terminal abandoned
    escape   deferred

    * -> abandoned
    * -> deferred

    step plan -> planning
    step plan_rev -> plan_review
    step impl -> implementation
    step impl_rev -> implementation_review
    step ship -> shipment
    step ship_rev -> shipment_review

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
}
```

The generated template gives you:

- 6 steps across planning, implementation, and shipment
- 3 phases
- 6 execution profiles
- prompt files that already route outcomes to valid states

## 5. Validate your workflow

```bash
loom validate
```

This checks the workflow graph, prompt outcome targets, profile configuration, and unresolved references.

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

## 9. Start smaller if you need to

If you want a tiny starter instead of the full Knots SDLC workflow:

```bash
loom init my_workflow
cd my_workflow
```

That uses the default `minimal` template: one produce step, one review step, one phase, and one default profile.

## Next references

- [`README.md`](/Users/cartine/loom/README.md)
- [`docs/releasing.md`](/Users/cartine/loom/docs/releasing.md)
- [`schema.md`](/Users/cartine/loom/schema.md)
- [`CONTRIBUTING.md`](/Users/cartine/loom/CONTRIBUTING.md)
