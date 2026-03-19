# How to Prompt Claude, Codex, Gemini, or OpenCode to Build a Workflow for You

You do not need to hand-write every Loom workflow from scratch.

A strong way to start is:

1. describe the process in plain language
2. point the agent at the Loom schema
3. ask for a complete workflow package
4. validate and refine

The important part is to ask for a full workflow package, not just a single `workflow.loom` file.

## What to ask the agent to produce

Ask for:

- `workflow.loom`
- `loom.toml`
- all needed prompt files under `prompts/`
- profile files under `profiles/` when useful
- a short explanation of the state machine

That keeps the output aligned with how Loom actually works.

## The minimum context to give the agent

Tell the agent:

- what the workflow is for
- what the key states are
- which steps are produce steps versus review gates
- what should happen on success
- what should happen on failure
- whether you want autopilot, semiauto, or both
- where the schema lives

In this repo, the source of truth is:

- [schema.md](/Users/cartine/loom/schema.md)

## A good default prompt

Use this as a starting point:

```text
Build a complete Loom workflow package for this process.

Use the Loom schema at /Users/cartine/loom/schema.md as the source of truth.

Produce:
- workflow.loom
- loom.toml
- all prompt files under prompts/
- profile files under profiles/ when needed

Requirements:
- keep the workflow easy to understand
- use clear state names
- include both success and failure routing
- include one autopilot profile and one semiauto profile when appropriate
- do not invent syntax that is not in the schema
- keep prompts concise and operational

Process to model:
[describe your workflow here]
```

## A better prompt for real work

The more concrete your process description is, the better the result.

Example:

```text
Build a Loom workflow package for our release process.

Use /Users/cartine/loom/schema.md as the source of truth.

We want these stages:
1. Release planning
2. Implementation
3. QA review
4. Production rollout review

Rules:
- planning and implementation are agent-owned
- QA review and production rollout review are human-gated
- failed QA returns to implementation
- failed rollout review returns to implementation unless the issue is deployment-only, in which case it returns to rollout
- successful rollout review ends in shipped

Produce a complete workflow package with:
- workflow.loom
- loom.toml
- prompts/*.md
- profiles/autopilot.loom
- profiles/semiauto.loom

Also include a short explanation of the states, phases, and profiles.
```

## What to avoid asking for

Avoid vague prompts like:

- “make me a workflow”
- “build something for software development”
- “use best practices”

That usually leads to generic state machines that do not match your real process.

Also avoid asking the agent to:

- invent Loom syntax
- skip prompt files
- flatten everything into one file
- use TOML as the main workflow definition

## A useful review loop

After the agent generates the files:

1. read `workflow.loom`
2. read the prompt frontmatter
3. run `loom validate`
4. simplify names that feel too abstract
5. tighten the success and failure routing

The best workflows are usually edited once after generation. They should feel obvious to the team that uses them.

## Ask for customization, not just generation

Agents are especially useful when you already have a base template.

Example:

```text
Starting from the knots_sdlc Loom template, customize it for a security incident response workflow.

Keep the package structure valid for Loom.

Changes we want:
- rename planning to triage
- rename implementation to remediation
- add a security review gate before shipment
- add a profile with all review gates owned by humans
- rewrite the prompts to match incident response language

Use /Users/cartine/loom/schema.md as the source of truth.
```

That kind of prompt is usually stronger than asking for a workflow from nothing.

## Per-agent guidance

### Claude

Claude usually does well when you ask it to:

- reason about the workflow structure first
- list states, steps, phases, and profiles before writing files
- then produce the final files

### Codex

Codex usually does best when you ask for:

- exact file outputs
- schema-constrained syntax
- no invented abstractions

### Gemini

Gemini usually benefits from:

- explicit examples of desired outcomes
- strict instructions not to invent unsupported syntax

### OpenCode

OpenCode works best when the prompt is concrete about:

- file names
- output format
- routing rules

## Recommended final sentence in your prompt

End with this:

```text
If anything is ambiguous, prefer a simpler workflow over a more clever one.
```

That one instruction usually improves the result.

## Next reads

- [Configure and Install Your Own Knots Workflow](/Users/cartine/loom/docs/configure-and-install-a-custom-knots-workflow.md)
- [Under the Hood: How Knots and Loom Work Together](/Users/cartine/loom/docs/under-the-hood-knots-and-loom.md)
- [Loom Language Specification](/Users/cartine/loom/schema.md)
