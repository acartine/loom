# Loom

![Coverage: 92%](https://img.shields.io/badge/coverage-92%25-brightgreen)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**A validating compiler for agent and human-in-the-loop workflows.**

Loom is for workflows where prompts, state transitions, and execution modes need to be explicit and safe. You define the workflow once, validate it before anything runs, and generate typed code that your runtime can consume directly.

## What Loom is for

Use Loom when you have:

- Multi-step agent workflows with real state, not one prompt at a time
- Prompt outcomes that should route to known states
- Multiple execution modes such as autopilot and human-gated review
- A need to catch broken transitions, missing prompts, and bad overrides before runtime

Loom is not a workflow engine. It does not execute work. It compiles workflow definitions into validated artifacts.

## Quick Start

Install with the standard curl flow:

```bash
curl -fsSL https://raw.githubusercontent.com/acartine/loom/main/install.sh | sh
```

Or build from source:

```bash
git clone https://github.com/acartine/loom.git
cd loom
cargo install --locked --path crates/loom-cli
```

Sanity-check the reference workflow that ships with the repo:

```bash
loom validate tests/fixtures/knots_sdlc
loom graph tests/fixtures/knots_sdlc --format ascii
loom build tests/fixtures/knots_sdlc --lang rust > /tmp/knots_workflow.rs
```

Then create your own workflow scaffold:

```bash
loom init support_triage
cd support_triage
loom validate
loom build --lang rust > workflow.rs
loom graph --format mermaid > workflow.mmd
```

The scaffold is intentionally small but complete: two queues, a produce action, a gate action, two prompt files, one phase, and a default profile. It validates cleanly without warnings.

For a fuller walkthrough, see [docs/getting-started.md](/Users/cartine/loom/docs/getting-started.md).

## How It Works

A workflow directory contains:

- `workflow.loom`: states, actions, steps, phases, and optional inline profiles
- `loom.toml`: package metadata and default profile
- `prompts/*.md`: markdown prompts with YAML frontmatter for outcomes and params
- `profiles/*.loom`: optional profile files included from `workflow.loom`

Minimal example:

```text
my_workflow/
  workflow.loom
  loom.toml
  prompts/
    work.md
    review.md
  profiles/
```

Larger workflows typically move profiles into separate files:

```text
knots_sdlc/
  workflow.loom
  loom.toml
  prompts/
    planning.md
    plan_review.md
    implementation.md
    implementation_review.md
    shipment.md
    shipment_review.md
  profiles/
    autopilot.loom
    semiauto.loom
```

## Step-By-Step

1. Define states and actions in `workflow.loom`.
2. Put each action prompt in `prompts/<prompt-name>.md`.
3. Declare success and failure routing in the prompt frontmatter.
4. Group steps into phases.
5. Select phases and executors with profiles.
6. Run `loom validate` until the graph is clean.
7. Generate code with `loom build`.

The fastest evaluation loop is:

1. Run `loom init <name>`.
2. Open `workflow.loom`, `prompts/work.md`, and `prompts/review.md`.
3. Run `loom validate`.
4. Run `loom build --lang rust`.
5. Run `loom graph --format mermaid`.
6. Compare the scaffold to `tests/fixtures/knots_sdlc` for a richer multi-phase example.

Example workflow fragment:

```loom
workflow my_workflow v1 {
    queue ready_for_planning "Ready for Planning"
    queue ready_for_review   "Ready for Review"

    action planning "Planning" {
        produce agent
        prompt planning
    }

    action review "Review" {
        gate review agent
        prompt review
    }

    terminal done      "Done"
    escape   deferred  "Deferred"

    step plan     { ready_for_planning -> planning }
    step plan_rev { ready_for_review   -> review }

    phase main_phase {
        produce plan
        gate plan_rev
    }
}
```

Example prompt frontmatter:

```yaml
---
accept:
  - Implementation steps are concrete
  - Risks are called out

success:
  plan_complete: ready_for_review

failure:
  insufficient_context: ready_for_planning

params:
  complexity:
    type: enum
    values: ["small", "medium", "large"]
---
```

## CLI

| Command | Description |
|---------|-------------|
| `loom init <name>` | Scaffold a new workflow directory |
| `loom validate [dir]` | Parse, load prompts, validate the full graph, and print warnings |
| `loom build [dir] --lang rust\|go\|python` | Validate and generate code |
| `loom build [dir] --emit toml` | Emit TOML interchange output |
| `loom graph [dir] --format mermaid\|dot\|ascii` | Render the full graph or a profile subgraph |
| `loom sim [dir] --profile <name>` | Walk the workflow interactively |
| `loom diff <old-dir> <new-dir>` | Show structural changes between workflow versions |
| `loom check-compat <old-dir> <new-dir>` | Check backward compatibility and optionally emit a state map |

## Why This Is Useful

Without a compiler, workflow logic tends to sprawl across prompt templates, runtime handlers, and application code. Loom keeps the routing model in one place and validates:

- Dead or unreachable states
- Missing or orphaned prompt files
- Invalid outcome targets
- Bad profile overrides
- Phase composition errors
- Per-profile graph issues

## Docs

- [Getting started guide](/Users/cartine/loom/docs/getting-started.md)
- [Configure and install a custom Knots workflow](/Users/cartine/loom/docs/configure-and-install-a-custom-knots-workflow.md)
- [Under the hood: Knots and Loom](/Users/cartine/loom/docs/under-the-hood-knots-and-loom.md)
- [How to prompt an agent to build a workflow](/Users/cartine/loom/docs/how-to-prompt-an-agent-to-build-a-workflow.md)
- [Release guide](/Users/cartine/loom/docs/releasing.md)
- [Language specification](/Users/cartine/loom/schema.md)
- [Contributing guide](/Users/cartine/loom/CONTRIBUTING.md)

## Status

The current repo is feature-complete enough to evaluate end-to-end: the reference fixture parses, validates, renders, diffs, checks compatibility, and generates Rust, Go, Python, and TOML output. The repository now also includes CI, tagged GitHub release automation, release tarballs, and a curl-based installer.

## License

[MIT](LICENSE)
