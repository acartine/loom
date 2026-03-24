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

Update an installed release in place:

```bash
loom update
loom update --check
```

List the bundled templates and scaffold the full Knots SDLC workflow:

```bash
loom templates list
loom init knots_sdlc
cd knots_sdlc
loom validate
loom build --lang rust > workflow.rs
loom graph --format mermaid > workflow.mmd
```

`loom init knots_sdlc` creates the complete bundled workflow package: planning, implementation, review, shipment, six prompt files, and six execution profiles. If you want that same template under a different directory name, run `loom init --template knots_sdlc my_team_flow` instead.

For a fuller walkthrough, see [docs/getting-started.md](/Users/cartine/loom/docs/getting-started.md).

## How It Works

A workflow directory contains:

- `workflow.loom`: actions, phases, profiles, and terminal/escape states
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
4. Group actions into phases.
5. Select phases and executors with profiles.
6. Run `loom validate` until the graph is clean.
7. Generate code with `loom build`.

The fastest evaluation loop is:

1. Run `loom templates list`.
2. Run `loom init knots_sdlc`.
3. Run `cd knots_sdlc && loom validate`.
4. Run `loom build --lang rust`.
5. Run `loom graph --format mermaid`.
6. If you want a smaller starter instead, run `loom init my_workflow` to use the default `minimal` template.

Example workflow fragment:

```loom
workflow my_workflow v1 {
    action planning {
        produce agent
    }

    action review {
        gate review agent
    }

    terminal done
    escape   deferred

    phase main_phase {
        produce planning
        gate review
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
| `loom init [--template <id>] <name>` | Scaffold a new workflow directory |
| `loom templates list` | List bundled workflow templates |
| `loom validate [dir]` | Parse, load prompts, validate the full graph, and print warnings |
| `loom build [dir] --lang rust\|go\|python` | Validate and generate code |
| `loom build [dir] --emit toml` | Emit TOML interchange output |
| `loom graph [dir] --format mermaid\|dot\|ascii` | Render the full graph or a profile subgraph |
| `loom sim [dir] --profile <name>` | Walk the workflow interactively |
| `loom diff <old-dir> <new-dir>` | Show structural changes between workflow versions |
| `loom check-compat <old-dir> <new-dir>` | Check backward compatibility and optionally emit a state map |
| `loom update [--check] [--force]` | Self-update an installed release binary using GitHub release asset redirects |

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
