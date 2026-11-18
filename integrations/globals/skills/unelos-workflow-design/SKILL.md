---
name: unelos-workflow-design
description: Design Unelos workflows, tasks, agents and skills for a project from how the team actually works. Use whenever the user wants to propose, build, review or improve Unelos workflows, turn a repeated way of working (bug fixing, UI features, backend tasks, releases, reviews) into a workflow, find patterns in past agent sessions, or asks whether something should be a task, workflow, agent or skill, even if they do not say "workflow".
---

# Designing Unelos workflows

Unelos makes the **process** deterministic, not the work. Nobody can predict
what a bug fix or feature will touch, but you can fix the phases, the gates,
who does what, and the evidence each step must leave behind. Most failures in
agent-driven work are skipped steps (no browser check, no cross-platform check,
"done" without running anything), not wrong decompositions, so that is where a
workflow pays for itself.

Unelos ships these stages as the `global/workflow-design` workflow (gather →
mine → propose → approve → write → validate → review). When the user wants the
full process, suggest starting it; use this skill directly for one-off
questions or when you are a step inside that workflow.

Work through the stages below with the user. Figure out where they are and
start there; they may already know which workflow they want.

## 1. Gather evidence

Good workflows come from what the team repeatedly does, not from a generic
template. In order of value:

1. **Past sessions.** If the harness keeps history (OpenCode keeps a SQLite
   database, Claude Code keeps JSONL transcripts), digest it and look for
   repeated shapes. See `references/session-mining.md` and
   `scripts/digest-sessions.py`. For more than ~15 sessions, split the digests
   across parallel subagents and aggregate their findings.
2. **The repository.** CI config, package scripts, test commands, docs,
   AGENTS.md / CLAUDE.md, existing `.unelos/` definitions, design files. These
   tell you what "checked" means here.
3. **The user.** Ask what they keep re-explaining to agents and where agents
   keep failing them. One question at a time.

For each candidate shape write down: how often it happens, its usual step
sequence, where it breaks (rework rounds, bugs found by the user, CI failures),
and the instructions the user keeps repeating. Repeated instructions are
standing rules waiting to be written down.

## 2. Pick what to build

Rank shapes by frequency × rework. Start with one small, frequent workflow as a
pilot (a bug fix loop is a good one): it exercises the building blocks quickly
and gives fast feedback. Build larger ones after the blocks hold up.

## 3. Design building blocks first

Decide what is a task, workflow, agent, skill or MCP before writing any YAML.
The rules and the reasoning are in `references/building-blocks.md`; read it
before designing. The short version:

- **Task**: one unit of work, one agent, one outcome. Most reusable pieces are
  tasks.
- **Workflow**: only when there is routing between steps: a loop, a gate,
  parallel lanes, or a branch.
- **Agent**: a role (QA, check runner, fixer, lead) with a model sized to the
  job and only the MCPs it needs.
- **Skill**: knowledge several tasks or agents need (how to run this app, its
  CI pitfalls). Project knowledge lives here, not copied into each task.
- **MCP**: a tool server an agent needs. Project MCPs in `.unelos/mcps`;
  machine-specific ones (local paths) as global MCPs referenced `global/<id>`.

## 4. Compose the workflow

Sketch the graph for the user as text before writing YAML, with every outcome
and where it goes. Check that:

- every check that can fail routes back to the step that can fix it, and the
  failing report reaches it (transition feedback carries the previous step's
  outputs);
- `blocked` (could not check) goes to a human, not back to a fixer who cannot
  solve it;
- human gates sit where only a human can decide: agreeing the spec or plan,
  confirming an unreproducible bug, accepting a visual result;
- the main-conversation step talks to the user and writes what was agreed into
  an artifact; it does not implement.

`references/patterns.md` has proven shapes (bug fix, UI feature, backend task,
integration, research, review-to-fix) to adapt.

## 5. Write the definitions

Write them under the project's `.unelos/` (see `references/definitions.md` for
every field). Keep instructions short and explain why; a step that knows the
reason handles edge cases a rule list misses.

## 6. Validate

1. `unelos inspect <workflow>` loads and validates the graph; `unelos inspect
   <workflow> --node <id>` shows exactly what an agent will receive (resolved
   agent, model, skills, MCPs, outcomes, inputs).
2. Walk every branch in a throwaway copy with `scripts/walk-workflow.py`, which
   starts a run, writes placeholder outputs and completes, approves or rejects
   each step along a path you give it. Walk at least: the happy path, each
   failure loop, and each gate rejection.

## 7. Pilot and iterate

Run the workflow for real on a task whose answer you already know (a bug from
the session history, a feature already shipped). Compare with how the original
session went. Fix what the run exposed, then move to the next workflow.

## Present to the user

When proposing, show: the evidence (shape, frequency, failures, with session
references), the graph sketch, the building blocks with their agent/model/
skills/MCPs and outcomes, and what you deliberately left out. Ask before writing
files; ask again before installing anything globally.
