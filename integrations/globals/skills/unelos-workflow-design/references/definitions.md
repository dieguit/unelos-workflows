# Definition reference

All authoring lives under the project's `.unelos/`. Folder and file names are
the IDs. Unknown fields are rejected.

```
.unelos/
├── project.yaml                 {} for now
├── workflows/<name>.yaml
├── tasks/<id>/task.yaml         (+ instructions file, resources)
├── agents/<id>/agent.yaml       (+ instructions file, resources)
├── skills/<id>/SKILL.md         (+ any resources)
└── mcps/<id>.yaml
```

Global equivalents (workflows, tasks, agents, skills, MCPs) live under
`~/.config/unelos/` in the same layout and are referenced with a `global/`
prefix: `agent: global/workflow-lead`, `workflow: global/pr-and-ci`,
`unelos start global/workflow-design`. A global workflow may reference only
global definitions, so it behaves the same in every project; a project workflow
may use global agents, tasks, skills and child workflows.

## Task — `tasks/<id>/task.yaml`

```yaml
description: One line shown in the app.
skills: [app-dev]                  # skill IDs
outcomes: [pass, fail, blocked]    # omit for a single next step
instructions: |                    # or instructionsFile: instructions.md (exactly one)
  What to read, do, write, and how to choose the outcome.
```

## Agent — `agents/<id>/agent.yaml`

```yaml
instructionsFile: instructions.md  # or instructions: | ... (exactly one)
skills: [app-dev]
mcps: [playwright, global/pencil]  # MCP IDs
tools: [bash]                      # informational only, not a restriction
model: balanced                    # alias from model mappings, or {harness: opencode, model: provider/model}
effort: low
```

## MCP — `mcps/<id>.yaml`

```yaml
type: local
command: [npx, '@playwright/mcp@latest']
environment: {}
```
```yaml
type: remote
url: https://example.com/mcp
headers: {}
```

## Workflow — `workflows/<name>.yaml`

```yaml
name: ui-bugfix
description: One line.
start: intake
inputs: [brief.md]        # only for workflows called as children
nodes:
  <node-id>: ...
```

### Task node

```yaml
reproduce:
  task: ui-reproduce          # or taskDefinition: {instructions, skills, outcomes, ...} inline
  agent: ui-qa
  execution: subagent         # or main: the orchestrator does it in the user's session; default main
  inputs: [bug.md]            # artifacts produced earlier in the run
  outputs: [repro.md]         # artifacts this step must write
  next:                       # a node ID, or a map outcome -> node ID
    reproduced: fix
    not-reproduced: confirm-bug
    blocked: confirm-bug
```

Use an inline `taskDefinition` for a step used only in this workflow; a named
`task` when several workflows share it.

Only subagent steps get a native harness agent (`unelos-<id>`, or
`unelos-global-<id>`); users never select these in their harness. A main step's
agent is a role the orchestrator adopts, so its `model` and `mcps` are ignored.

### Human gate

```yaml
approve:
  type: human-gate
  inputs: [fix.md, ui-verification.md]   # what the human reviews
  next: {approved: done, rejected: fix}
```

A rejection's reason is delivered as feedback to the target step.

### Parallel node

```yaml
council:
  type: parallel
  inputs: [question.md]
  tasks:
    - {task: review, agent: reviewer-a}
    - {taskDefinition: {instructions: ...}, agent: reviewer-b}
  outputs: [responses.md]
  next: synthesis
```

Workers always run as subagents. Give them disjoint write scopes.

### Child workflow

```yaml
frontend:
  type: workflow
  workflow: react-component
  inputs: {brief.md: plan.md}               # child input <- parent artifact
  outputs: {report.md: frontend-report.md}  # child output -> parent artifact
  next: {done: review, abandoned: plan}     # child end-node IDs become outcomes
```

### End

```yaml
done: {type: end}
```

## Runtime facts that shape design

- Every declared output must exist before a step can complete.
- On a transition, the next step receives the previous step's outputs as
  feedback; this is how a failing check report reaches the fixer.
- Re-entering a step finds its earlier outputs in place: instruct it to update,
  not rewrite.
- Each run keeps its artifacts under `.unelos/runs/<run-id>/<node>/`.

## CLI for validation

```sh
unelos inspect <workflow>                 # load and validate
unelos inspect <workflow> --node <id>     # resolved packet for one step
unelos start <workflow>                   # start a run
unelos next --run <id>                    # current packet
unelos complete --run <id> [--outcome X] <node>
unelos approve --run <id> <node>
unelos reject --run <id> --reason "..." <node>
unelos status --run <id>
```
