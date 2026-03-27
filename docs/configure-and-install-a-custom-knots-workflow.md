# Configure and Install Your Own Knots Workflow

This guide shows the shortest path from idea to a working Knots workflow.

You do not need to learn the full Loom language before you start. The easiest path is:

1. start from a base template
2. make a few targeted changes
3. validate it
4. install it into Knots

## What you need

- `loom` installed
- `knots` installed

## Step 1. Start from a base template

Pick the closest workflow shape and initialize it:

```bash
loom init --template knots_sdlc my_team_flow
cd my_team_flow
```

The shorthand `loom init knots_sdlc` also works when you want the directory and workflow name to stay `knots_sdlc`.

This creates a complete workflow package:

- `workflow.loom`: the workflow structure
- `prompts/`: the prompt files for each action
- `profiles/`: execution modes such as autopilot or human-gated review
- `loom.toml`: package metadata

If you are not sure which template to start from:

```bash
loom template list
```

Common starting points:

- `minimal`: one produce step, one review step
- `knots_sdlc`: planning, implementation, review, and shipment

## Step 2. Set the workflow name for your team

The best option is to choose the final name when you run `loom init --template ... <name>`.

If you rename the scaffold after creation, update both:

- `loom.toml`
- the `workflow <name> v1` declaration in `workflow.loom`

Example:

```toml
[workflow]
name = "payments_sdlc"
version = 1
entry = "workflow.loom"
default_profile = "semiauto"
```

```loom
workflow payments_sdlc v1 {
    ...
}
```

Pick a name that describes the workflow, not the team member who created it.

## Step 3. Edit the workflow in plain language

Most teams make three kinds of changes first:

- rename states to match internal vocabulary
- remove phases they do not use
- adjust who owns each action

Example changes:

- rename `ready_for_shipment` to `ready_for_release`
- remove the planning phase if your team starts from implementation
- make review steps human-owned in the `semiauto` profile

You usually do this in two places:

- `workflow.loom` for states, steps, phases, and included profiles
- `profiles/*.loom` for execution modes

## Step 4. Rewrite the prompts to match your process

Each action has its own prompt file in `prompts/`.

You do not need to touch every field on day one. Focus on:

- the acceptance criteria
- success outcomes
- failure outcomes
- the body of the prompt

Example:

```yaml
---
accept:
  - Changes are safe to roll out
  - Monitoring and rollback notes are included

success:
  approved: ready_for_release

failure:
  changes_requested: ready_for_implementation

params: {}
---
```

If a step should pause until another knot ships, prefer `escape blocked` plus a
failure outcome like `blocked_by_dependency: blocked` over inventing a polling
action.

The important idea is simple: the prompt decides where the workflow goes next.

## Step 5. Validate before you install

```bash
loom validate
```

Loom checks the full workflow before Knots sees it:

- missing prompt files
- broken state targets
- dead states
- invalid profiles
- bad overrides

If validation passes, your workflow is safe to install.

## Step 6. Install it into Knots

```bash
knots workflows install .
```

Knots reads the workflow package, imports the compiled Loom bundle, and registers the workflow by name.

You can then list installed workflows:

```bash
knots workflows list
```

And set the active workflow for a workspace:

```bash
knots workflows use payments_sdlc
```

## Step 7. Test the flow in a safe mode

Before you roll it out broadly, run the workflow in a human-gated profile:

```bash
knots workflows use payments_sdlc --profile semiauto
```

That lets your team confirm:

- state names make sense
- prompts produce the right outcomes
- review gates are in the right places
- the handoffs feel natural

## The fast mental model

If you remember only one thing, remember this:

- Loom defines and validates the workflow
- Knots runs the workflow

You design in Loom once, then install the result into Knots.

## Common customization patterns

### Skip planning

Start from `knots_sdlc`, remove the planning phase from your default profile, and route failed reviews back to implementation instead.

### Add a security review

Add:

- a new queue state
- a new review action
- a new step
- a new phase
- a prompt file for the security review

Then include that phase in the profile that needs it.

### Keep one workflow, offer two operating modes

Use profiles:

- `autopilot` for agent-heavy operation
- `semiauto` for human review

That way the workflow logic stays the same while the ownership model changes.

## Next reads

- [Under the Hood: How Knots and Loom Work Together](under-the-hood-knots-and-loom.md)
- [How to Prompt an Agent to Build a Workflow](how-to-prompt-an-agent-to-build-a-workflow.md)
- [Loom Language Specification](../schema.md)
