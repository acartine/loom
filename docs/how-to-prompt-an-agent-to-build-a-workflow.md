# How to Prompt an Agent to Build a Workflow

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

- [schema.md](../schema.md)

## A good default prompt

Use this as a starting point:

```text
Build a complete Loom workflow package for this process.

Use the Loom schema in schema.md from this repo checkout as the source of truth.

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

Use schema.md from this repo checkout as the source of truth.

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

Use schema.md from this repo checkout as the source of truth.
```

That kind of prompt is usually stronger than asking for a workflow from nothing.

## Recommended final sentence in your prompt

End with this:

```text
If anything is ambiguous, prefer a simpler workflow over a more clever one.
```

That one instruction usually improves the result.

## Next reads

- [Configure and Install Your Own Knots Workflow](configure-and-install-a-custom-knots-workflow.md)
- [Under the Hood: How Knots and Loom Work Together](under-the-hood-knots-and-loom.md)
- [Loom Language Specification](../schema.md)
