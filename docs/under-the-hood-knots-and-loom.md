# Under the Hood: How Knots and Loom Work Together

This guide explains the architecture behind custom Knots workflows.

The short version:

- Loom is the workflow compiler
- Knots is the workflow runtime

Loom defines and validates the workflow. Knots installs and executes it.

## Why there are two tools

Loom and Knots solve different problems.

### Loom

Loom is responsible for:

- workflow structure
- prompt routing
- profile definitions
- compile-time validation

It answers questions like:

- Is this state reachable?
- Does this prompt point to a real target?
- Does this profile include a valid set of phases?

### Knots

Knots is responsible for:

- loading installed workflows
- applying them to workspaces or projects
- advancing workflow state over time
- selecting the active profile
- coordinating the actual agent and human sessions

It answers questions like:

- Which workflow is active here?
- What state is this knot in right now?
- Which prompt should run next?
- Is this step agent-owned or human-gated?

## The install path

From the user’s perspective, installation is one command:

```bash
knots workflows install .
```

Under the hood, the flow looks like this:

```text
workflow.loom + prompts + profiles
        │
        ▼
   loom validate
        │
        ▼
   loom compile bundle
        │
        ▼
  knots imports bundle
        │
        ▼
  workflow registered in Knots
```

Knots does not parse ad hoc prompt files on its own. It imports a validated Loom-compiled workflow package.

## Why Knots does not dynamically import Rust source

This is an important design choice.

A user-installed workflow should not require:

- rebuilding Knots
- loading arbitrary Rust code at runtime
- matching Rust compiler toolchains on every machine
- unstable plugin boundaries

That approach is fragile and operationally expensive.

Instead, Knots loads a compiled Loom workflow bundle with a stable runtime format.

## So where does the strong typing live?

The strong typing lives in the Loom compiler and in the compiled bundle contract.

That gives you the benefits you actually want:

- invalid workflows are rejected before install
- outcome targets are fully resolved
- profiles are fully materialized
- prompt metadata is structured and typed
- Knots can execute the workflow quickly without re-deriving the graph every time

In other words:

- authoring remains high-level
- validation remains strict
- runtime loading remains fast

## Static embedding versus runtime install

Loom supports more than one output path.

### Static embedding

Use generated Rust, Go, or Python when the workflow is part of an application build.

That is best when:

- the workflow ships with your app
- you want compile-time integration into your service
- workflow changes happen through code review and deployment

### Runtime install into Knots

Use the Knots install path when:

- users create or customize workflows after installation
- teams want to swap workflows without rebuilding Knots
- the same Knots install should support many workflows

That path uses the Loom bundle, not dynamic Rust source loading.

## What Knots stores after install

After installation, Knots stores:

- the workflow identity and version
- the compiled graph
- profile definitions
- prompt metadata
- transition tables

This gives Knots everything it needs to run the workflow safely and quickly.

## What changes require a reinstall

Any change to the Loom workflow package should go through the same loop:

```bash
loom validate
knots workflows install .
```

That includes:

- changing prompt routing
- adding or removing phases
- changing profile ownership
- renaming states

Knots treats the installed workflow as a compiled artifact, not a live mutable directory.

## Why not use plain TOML as the integration layer

TOML is useful for inspection and interchange, but it is not the best primary runtime boundary for Knots.

Using a Loom-native bundle keeps:

- richer typing
- clearer versioning
- better validation guarantees
- less ambiguity around prompt semantics

TOML can still exist as an export format. It just should not be the main story for serious Knots workflow installs.

## Recommended mental model

Think of Loom workflows the same way you think about application code:

- source files are for humans
- compiled artifacts are for runtimes

For custom Knots workflows:

- `.loom` and prompt markdown are the source
- the Loom bundle is the installed artifact
- Knots is the runtime

## Next reads

- [Configure and Install Your Own Knots Workflow](/Users/cartine/loom/docs/configure-and-install-a-custom-knots-workflow.md)
- [How to Prompt Claude, Codex, Gemini, or OpenCode to Build a Workflow for You](/Users/cartine/loom/docs/how-to-prompt-an-agent-to-build-a-workflow.md)
- [Loom Language Specification](/Users/cartine/loom/schema.md)
