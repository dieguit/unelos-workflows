# Simple plan, execute, review

Interview → plan → human approval → implementer → separate reviewer → finish.
Review corrections return to implementation; plan rejection returns to planning.
The requested work is a conversation input, not an installation setting.

## Workflow and package layout

The optional workflow YAML `description` appears beside its name in `/unelos`:
`0` continues without Unelos; workflows are choices `1..N`. Choose by number or name.

| Step | Runs as | Output / decision |
| --- | --- | --- |
| `interview` | Main controller | `requirements.md`: confirmed scope and acceptance checks |
| `plan` | Main controller | `plan.md`: smallest implementation and verification plan |
| `approve-plan` | Human gate | Explicit approval → implement; rejection → revise plan |
| `implement` | Implementer subagent | `implementation-report.md`: changes and verification evidence |
| `review` | Independent reviewer subagent | `review.md`: passed → finish; changes_requested → implement |
| `finish` | Controller | Announces completion and artifact paths |

```text
simple-plan-execute-review/
├── README.md
├── workflows/simple-plan-execute-review.yaml
└── agents/
    ├── simple-controller/{agent.yaml,instructions.md}
    ├── simple-implementer/{agent.yaml,instructions.md}
    └── simple-reviewer/{agent.yaml,instructions.md}
```

This is a flat installable package, not a project: no `.unelos/` wrapper or runtime
state. The workflow embeds its four local `taskDefinition` bodies. Agents are
discovered by folder ID and load Markdown using `instructionsFile: instructions.md`.
Templates/resources can accompany an agent. Instructions paths are relative to
their containing YAML; escaping the authoring root and symlinked paths are rejected.
Place shared instruction files outside task/agent registries (for example `shared/`)
so installation does not need to import unrelated definitions.

For reusable tasks, add `tasks/<id>/task.yaml` and use `task: <id>` instead of inline
`taskDefinition` (also supported for parallel workers). Skills stay in
`skills/<id>/SKILL.md`. Never supply both task forms or both instruction forms.
Installed projects keep these folders under `.unelos/`; `project.yaml` is reserved
for project settings and currently contains a commented empty mapping.

## Install locally

With the Unelos CLI installed, initialize the target project:

```sh
unelos --project /absolute/project init --opencode v2
```

This bundles and configures the [OpenCode integration](../../integrations/opencode/README.md)
automatically, preserving unrelated configuration and authored files. Rerun init
to repair missing integration assets; it does not restore deleted runs or authoring.

Install the workflow and generate native OpenCode agents in one operation:

```sh
unelos --project /absolute/project install simple-plan-execute-review \
  --source /absolute/unelos/examples/simple-plan-execute-review --json
```

Use `init --opencode v1` for V1. Install infers the initialized version.
Existing differing authoring is not overwritten.
To regenerate native profiles from your installed Unelos definitions:

```sh
unelos --project /absolute/project install simple-plan-execute-review --json
```

Generated `.opencode/agents/unelos-*.md` profiles are overwritten on reinstall,
including manual edits. Customize `.unelos/agents/<id>/agent.yaml`, its Markdown,
or inline tasks in `.unelos/workflows/simple-plan-execute-review.yaml` instead. Unrelated agents,
project configuration and runs are preserved. No global installation is needed.
The example ships no runtime state, ledger, fixed feature or Skill dependency.

Before starting, check the project's required Skills and tools in the actual
OpenCode session. Missing prerequisites block relevant work;
this workflow does not waive project instructions or add those resources.

## Interactive walkthrough (V2)

```sh
opencode --standalone /absolute/path/to/your/project
```

1. Enter `/unelos`; select **simple-plan-execute-review**. The controller asks what
   needs to be done if you have not already described it.
2. Answer the interview and confirm shared understanding of the requirements.
   The controller writes requirements and a plan, completing ordinary Tasks itself.
3. Review the presented plan. To exercise revision, reject with a concrete reason,
   then explicitly approve the revised plan. Interview answers are not gate approval.
4. An implementer and a separate reviewer run through native subagents. You do
   **not** issue manual `complete` commands.
5. Answer any worker question in the main conversation. The same Task/worker
   continues; meaningful answers are saved in the report for downstream work.
6. Review findings loop through corrections; the controller reports finish and
   the run's artifact paths: `requirements.md`, `plan.md`,
   `implementation-report.md`, `review.md`, and controller-owned `ledger.md`.
7. For recovery, enter `/unelos resume` and select an unfinished run. Selection
   injects its existing context and immediately continues work; no second instruction
   is needed. Pending gates still need explicit approval/rejection.
8. `/unelos` always begins fresh. Different main sessions bind independent runs;
   shared source edits are not automatically isolated. `/unelos off` affects only
   the current session.

V1 uses the packaged Markdown command and native `task`/`task_id`; V2 uses
`subagent`/`sessionID`. Server checks do not establish V1 interactive TUI support.
See the [execution verification report](../../docs/opencode-execution-verification.md) for
observed versions, blockers and acceptance status. The user's own interactive
run remains an acceptance checkpoint until actually exercised.
