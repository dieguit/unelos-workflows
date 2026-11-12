# Instruction catalog

These Markdown files are the authoritative shared instruction bodies, embedded
by Rust with `include_str!`. Edit here, then rebuild/reinstall the CLI. No runtime
Markdown lookup, template engine, or duplicate editable copy is involved.

## Native execution

| Source | Audience / delivery |
| --- | --- |
| [controller.md](controller.md) | Invariants in every execution package and resume run catalog. |
| [controller-idle.md](controller-idle.md) | Idle workflow selection, requestedAction and naming. |
| [controller-main-task.md](controller-main-task.md) | Main Task execution and completion. |
| [controller-delegated.md](controller-delegated.md) | Delegation and worker result handling. |
| [controller-parallel.md](controller-parallel.md) | Parallel dispatch and aggregation. |
| [controller-gate.md](controller-gate.md) | Human gate presentation and decisions. |
| [controller-finished.md](controller-finished.md) | Finished run announcement. |
| [controller-resume.md](controller-resume.md) | Resume run catalog and resumed packages (alongside their state block). |
| [task-outputs.md](task-outputs.md) | Main and delegated Tasks, including parallel workers: `completionInstructions` in resolved Task context. |
| [worker-result.md](worker-result.md) | Delegated serial workers: appended to the exact generated `workerPrompt`; result frame and controller/worker boundaries. |
| [parallel-worker-result.md](parallel-worker-result.md) | Parallel workers: appended to each generated `workerPrompts[].prompt`; frame grammar with `workerKey` and report-only ownership. |

Raw work packets and inspection carry no coordinator instructions.

## Project-owned instructions and runtime data

- `.unelos/project.yaml`: reserved project settings; empty mapping for now.
- `.unelos/tasks/<id>/task.yaml` and `.unelos/agents/<id>/agent.yaml`: discovered
  reusable definitions, Skills, outcomes and agent resource requirements.
- `.unelos/workflows/<workflow>.yaml`: graph, Task/agent references or inline
  `taskDefinition`, execution mode, logical input and output names.
  [Example workflow](../../../../examples/superpowers-dev/workflows/superpowers-dev.yaml).
- Task/agent `instructions` or `instructionsFile`: exactly one source. Markdown
  paths resolve relative to the containing YAML within the authoring root.
- `.unelos/skills/<skill>/SKILL.md`: selected project Skill bodies.
  [Example agents](../../../../examples/superpowers-dev/agents/)
  (the example bundles its Skills under `skills/`).

There is no separate shared input-reading instruction today: Task instructions
describe how to use inputs; core validates bindings and supplies their paths.
Core resolves inputs to recorded producer artifacts and allocates outputs under
`.unelos/runs/<run>/<node>/<declared-name>` (parallel worker reports have separate
worker directories). Names and exact paths travel in `inputs` / `outputs`, not
as placeholders inside these Markdown files. Other runtime data includes run/Task
identity, feedback, the agreed Brief and continuation context.

The generated serial worker prompt combines Markdown assignment sections, optional Brief,
feedback and worker-result instructions. Structural section labels
remain in Rust; project-specific YAML and Skills remain project-owned. Mechanical
validation does not prove report quality or model adherence.
