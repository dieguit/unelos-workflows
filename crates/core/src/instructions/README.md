# Instruction catalog

These Markdown files are the authoritative shared instruction bodies, embedded
by Rust with `include_str!`. Edit here, then rebuild/reinstall the CLI. No runtime
Markdown lookup, template engine, or duplicate editable copy is involved.
This catalog extraction preserves existing instruction text and behavior.

## Native execution

| Source | Audience / delivery |
| --- | --- |
| [controller.md](controller.md) | Coordinator policy in execution packages and the resume run catalog: entry, naming, delegation, completion, gates, recovery and finish announcement. |
| [task-outputs.md](task-outputs.md) | Main and delegated Tasks, including parallel workers: `completionInstructions` in resolved Task context. |
| [worker-result.md](worker-result.md) | Delegated serial workers: appended to the exact generated `workerPrompt`; result frame and controller/worker boundaries. |

## Raw work packets and inspection

These lower-level instructions also remain available to raw CLI consumers.
Native execution packages carry the full coordinator policy above; their worker
prompts exclude raw-packet controller metadata.

| Source | Delivery |
| --- | --- |
| [ledger.md](ledger.md) | Every runtime work packet's coordinator instructions. |
| [context-and-naming.md](context-and-naming.md) | Task, parallel and gate runtime packets. |
| [output-confirmation.md](output-confirmation.md) | Task and parallel runtime packets; `complete NODE` refers to the raw CLI, not the native guarded `unelos_complete` tool. |
| [parallel.md](parallel.md) | Additional parallel runtime packet instructions. |
| [human-gate.md](human-gate.md) | Additional gate runtime packet instructions. |
| [finished.md](finished.md) | Additional finished runtime packet instructions. |
| [parallel-inspection.md](parallel-inspection.md) | Parallel Node inspection, before runtime bindings exist. |
| [human-gate-inspection.md](human-gate-inspection.md) | Gate Node inspection, before runtime bindings exist. |

## Project-owned instructions and runtime data

- `.unelos/project.yaml`: `taskDefinitions.*.instructions`, `agents.*.instructions`,
  selected Skills, declared outcomes and agent resource requirements.
- `.unelos/workflows/<workflow>.yaml`: graph, Task/agent references, execution mode,
  logical input and output names. [Example workflow](../../../../examples/opencode-execution/.unelos/workflows/page-change.yaml).
- `.unelos/skills/<skill>/SKILL.md`: selected project Skill bodies.
  [Example definitions](../../../../examples/opencode-execution/.unelos/project.yaml)
  and [Skill](../../../../examples/opencode-execution/.unelos/skills/page-change/SKILL.md).

There is no separate shared input-reading instruction today: Task instructions
describe how to use inputs; core validates bindings and supplies their paths.
Core resolves inputs to recorded producer artifacts and allocates outputs under
`.unelos/runs/<run>/<node>/<declared-name>` (parallel worker reports have separate
worker directories). Names and exact paths travel in `inputs` / `outputs`, not
as placeholders inside these Markdown files. Other runtime data includes run/Task
identity, feedback, the agreed Brief and continuation context.

The generated serial worker prompt combines its JSON executor context, Brief,
continuation context and worker-result instructions. Structural section labels
remain in Rust; project-specific YAML and Skills remain project-owned. Mechanical
validation does not prove report quality or model adherence.
