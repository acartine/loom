# Loom

Loom is a **workflow language and compiler** for agentic work.

It is not a workflow engine.
It does not execute workflows.
It defines workflows, validates them, and compiles them into typed artifacts that a runtime system — such as **Knots** — can consume.

**Short version:** Loom authors workflows. Knots runs them.

## Why Loom exists

A lot of workflow logic ends up in bad places:
- hardcoded state machines
- prompt strings buried in code
- implicit routing conventions
- runtime checks that happen too late
- brittle transitions that are easy to break and hard to inspect

Loom exists to turn workflow definition into a first-class artifact.

A workflow should be:
- explicit
- typed
- validated before runtime
- inspectable by humans
- safe to evolve

## Core idea

Loom treats a workflow as a small program, not a bag of configuration.

A Loom workflow defines:
- states
- steps
- phases
- profiles
- prompts
- outcome routing

Prompts are structured documents, not just strings. Each prompt can declare:
- acceptance criteria
- success outcomes
- failure outcomes
- typed parameters
- a markdown body

That means routing behavior lives with the action that produces it.

## Current shape of the project

Right now this repo is centered on the language design itself.

The main artifact is:
- [`schema.md`](./schema.md) — the current Loom language specification

That spec defines:
- the file/package structure
- the `.loom` grammar
- prompt file structure
- profile selection and overrides
- compiler semantics
- graph validation rules
- code generation targets
- CLI direction
- a reference Knots SDLC workflow

## Why Loom is separate from Knots

Knots should stay lean.

Knots should not own:
- workflow authoring UX
- graph analysis
- prompt routing design
- compatibility reasoning
- workflow package validation

Knots should consume a trusted compiled workflow artifact.

Loom should own:
- language design
- workflow authoring
- compile-time validation
- profile resolution
- code generation
- interchange/export formats

In other words:
- **Loom is the authoring/compiler layer**
- **Knots is the runtime layer**

## Design highlights

### Workflow packages, not giant files
A workflow is a directory with:
- `workflow.loom`
- `loom.toml`
- `prompts/*.md`
- `profiles/*.loom`

This keeps prompts and profiles first-class.

### Prompts own routing
A key Loom idea is that **the prompt is the routing table**.

Prompt frontmatter defines outcomes and their target states.
That keeps edge behavior close to the action instead of scattering it through runtime code.

### Profiles are subgraphs
Profiles are not patches over a default workflow.
They select phases and optionally override ownership.

That makes it easy to represent modes like:
- autopilot
- semiauto
- PR-based flows
- no-planning variants

### Compile-time graph validation
Loom is designed to catch workflow mistakes before runtime, including:
- dead states
- unreachable terminals
- missing prompts
- dangling outcome targets
- invalid profile overrides
- bad phase composition
- profile-specific graph failures

## Intended CLI

The spec currently describes a CLI along these lines:

```bash
loom init <name>
loom validate
loom build [--lang <target>]
loom build --emit toml
loom graph [--profile <name>]
loom sim <profile>
loom diff <v1-dir> <v2-dir>
loom check-compat <old> <new>
```

Some of this is still design direction rather than finished implementation.

## Status

This project is currently in the **language/specification phase**.

The repo is intentionally light right now because the most important thing to get right first is the model:
- what a workflow is
- how prompts route outcomes
- how profiles carve out subgraphs
- what the compiler guarantees
- what runtime consumers should be able to trust

## Read next

Start here:
- [`schema.md`](./schema.md)

If you want the shortest possible framing:

> Loom is a language and compiler for authored agent workflows.
> It lets you define the workflow once, validate it properly, and hand a typed result to systems like Knots.
