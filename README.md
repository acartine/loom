# Loom

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

Install a published release with the standard curl flow:

```bash
curl -fsSL https://raw.githubusercontent.com/acartine/loom/main/install.sh | sh
```

This path is for:

- Linux `x86_64` and `aarch64`
- macOS Apple Silicon (`aarch64`)

The installer requires `curl`, `tar`, and `install`.

Build from source if you are on another platform or want a local dev build:

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

If you want to run from the repo without installing a binary, stay in the repo root and replace `loom` below with `cargo run -p loom-cli --`.

List the bundled templates and scaffold the full Knots SDLC workflow:

```bash
loom template list
loom init knots_sdlc
cd knots_sdlc
loom validate
loom build --lang rust > workflow.rs
loom graph --format mermaid > workflow.mmd
```

`loom init knots_sdlc` creates the complete bundled workflow package: planning, implementation, review, shipment, six prompt files, and six execution profiles. If you want that same template under a different directory name, run `loom init --template knots_sdlc my_team_flow` instead.

For a fuller walkthrough, see [docs/getting-started.md](docs/getting-started.md).

## How It Works

A workflow directory contains:

- `workflow.loom`: actions, phases, profiles, and terminal/escape states
- `loom.toml`: package metadata and default profile
- `prompts/*.md`: markdown prompts with YAML frontmatter for outcomes and params
- `profiles/*.loom`: optional profile files included from `workflow.loom`

Escape states are non-terminal waiting states, so they are valid prompt
targets without becoming claimable actions.

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
    ...
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

1. Run `loom template list`.
2. Run `loom init knots_sdlc`.
3. Run `cd knots_sdlc && loom validate`.
4. Run `loom build --lang rust`.
5. Run `loom graph --format mermaid`.
6. If you want a smaller starter instead, run `loom init my_workflow` to use the default `minimal` template.

Example workflow fragment:

```loom
workflow my_workflow v1 {
    action work {
        produce agent
    }

    action review {
        gate review human
    }

    terminal done

    step do_work -> work
    step review_work -> review

    phase main {
        produce do_work
        gate review_work
    }

    profile default "Default" {
        phases [main]
    }
}
```

Example prompt frontmatter:

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

## CLI

| Command | Description |
|---------|-------------|
| `loom init [--template <id>] <name>` | Scaffold a new workflow directory |
| `loom template list` | List bundled workflow templates |
| `loom validate [dir]` | Parse, load prompts, validate the full graph, and print warnings |
| `loom build [dir] --lang rust\|go\|python` | Validate and generate code |
| `loom build [dir] --emit toml\|knots-bundle` | Emit TOML or Knots bundle output |
| `loom graph [dir] --format mermaid\|dot\|ascii` | Render the full graph or a profile subgraph |
| `loom sim [dir] --profile <name>` | Walk the workflow interactively |
| `loom diff <old-dir> <new-dir>` | Show structural changes between workflow versions |
| `loom check-compat <old-dir> <new-dir>` | Check backward compatibility and optionally emit a state map |
| `loom update [--check] [--force]` | Self-update an installed release binary using GitHub release asset redirects |
| `loom doctor [--fix]` | Check the local install and optionally fix shell completions |
| `loom uninstall [--force] [--purge]` | Remove an installed Loom binary |
| `loom completions <shell>` | Generate shell completion scripts |

## Why This Is Useful

Without a compiler, workflow logic tends to sprawl across prompt templates, runtime handlers, and application code. Loom keeps the routing model in one place and validates:

- Dead or unreachable states
- Missing or orphaned prompt files
- Invalid outcome targets
- Bad profile overrides
- Phase composition errors
- Per-profile graph issues

## Docs

- [Getting started guide](docs/getting-started.md)
- [Configure and install a custom Knots workflow](docs/configure-and-install-a-custom-knots-workflow.md)
- [Under the hood: Knots and Loom](docs/under-the-hood-knots-and-loom.md)
- [How to prompt an agent to build a workflow](docs/how-to-prompt-an-agent-to-build-a-workflow.md)
- [Release guide](docs/releasing.md)
- [Language specification](schema.md)
- [Contributing guide](CONTRIBUTING.md)

## Status

The reference fixture currently parses, validates, renders, diffs, checks compatibility, and generates Rust, Go, Python, TOML, and Knots bundle output. The repository also includes CI, tagged GitHub release automation, release tarballs, and a curl-based installer.

## Related Projects

Loom is the workflow definition and validation layer for [Knots](https://github.com/acartine/knots), which lets Knots support different workflow shapes instead of hard-coding a single one.

[Knots](https://github.com/acartine/knots) is the workflow engine used by [Foolery](https://github.com/acartine/foolery), a web-based orchestrator for agent-driven software work.

## License

[MIT](LICENSE)
