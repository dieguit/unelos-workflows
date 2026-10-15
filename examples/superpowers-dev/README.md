# superpowers-dev

The superpowers development pipeline as a Unelos workflow: brainstorm a spec,
approve it, write and approve a plan, execute it task by task with a review per
task, review the whole branch, then finish. Skills are adapted from
[superpowers](https://github.com/obra/superpowers) (MIT, see `LICENSE-superpowers`).
The requested work is a conversation input, not an installation setting.

## Workflow and package layout

The workflow YAML `description` appears beside its name in `/unelos`:
`0` continues without Unelos; workflows are choices `1..N`. Choose by number or name.

| Step | Runs as | Output / decision |
| --- | --- | --- |
| `brainstorm` | Controller (main) | `spec.md`: agreed design, self-reviewed |
| `approve-spec` | Human gate | Approved → plan; rejected → revise spec |
| `write-plan` | Controller (main) | `plan.md`: tasks with exact code and commands |
| `approve-plan` | Human gate | Approved → worktree; rejected → revise plan |
| `setup-worktree` | Controller (main) | `worktree.md`: worktree, branch, base branch, merge-base, green baseline, preflight rulings |
| `implement-task` | Implementer subagent | `implementation-report.md`: one task or one fix round appended |
| `review-task` | Task reviewer subagent | `review.md`: full task review, or scoped re-review of a fix round |
| `advance` | Controller (main) | `progress.md`; outcome `fix` / `next_task` → implement-task, `all_done` → final-review |
| `final-review` | Final reviewer subagent | `final-review.md`: whole branch from merge-base; scoped re-review of the fix wave on return |
| `advance-final` | Controller (main) | `final-rulings.md`; outcome `final_fix` → fix-final, `finish` → finish |
| `fix-final` | Implementer subagent | `final-fix-report.md`: one wave for all must-fix findings → final-review |
| `finish` | Controller (main) | `finish.md`: verification, rulings made, merge / PR / keep / discard |
| `done` | End | Run complete |

```text
superpowers-dev/
├── README.md
├── LICENSE-superpowers
├── workflows/superpowers-dev.yaml
├── agents/
│   ├── sp-controller/{agent.yaml,instructions.md}
│   ├── sp-implementer/{agent.yaml,instructions.md}
│   ├── sp-task-reviewer/{agent.yaml,instructions.md}
│   └── sp-final-reviewer/{agent.yaml,instructions.md}
└── skills/
    ├── brainstorming/SKILL.md
    ├── writing-plans/SKILL.md
    ├── using-git-worktrees/SKILL.md
    ├── sdd-controller/SKILL.md
    ├── finishing-a-development-branch/SKILL.md
    ├── test-driven-development/SKILL.md
    ├── verification-before-completion/SKILL.md
    ├── receiving-code-review/SKILL.md
    ├── systematic-debugging/SKILL.md
    └── code-review/SKILL.md
```

This is a flat installable package, not a project: no `.unelos/` wrapper or runtime
state. Workflow nodes embed their `taskDefinition` bodies; agents are discovered by
folder ID and load Markdown with `instructionsFile: instructions.md`. Install copies
only the referenced `skills/<id>/` folders, so each `SKILL.md` carries its own
attribution header. Installed projects keep these folders under `.unelos/`.

## Loop state

Each artifact has one producer node. Loop state that cannot be a declared input
travels as transition feedback: `progress.md` reaches `implement-task` and
`final-fix-report.md` reaches `final-review` re-reviews via transition feedback.
`implement-task` reads `## Open findings` and `## Carry forward` from `progress.md`;
rulings under Carry forward bind over the plan text. With no `progress.md` feedback it
implements Task 1. Reports and reviews are appended in place across visits.

Fix rounds are capped at 5 per task; at the cap `advance` adjudicates the remaining
findings (park with a ruling, or rule the smallest unblocking change) and the task
counts as complete. The final loop allows one fix wave. Rulings are collected at
`finish` in "Rulings I made".

The implementer's `DONE` / `DONE_WITH_CONCERNS` are status lines in its report; the
Unelos result for both is `FINISHED`. `NEEDS_CONTEXT` and `BLOCKED` are returned as-is.

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
unelos --project /absolute/project install superpowers-dev \
  --source /absolute/unelos/examples/superpowers-dev --json
```

Use `init --opencode v1` for V1. Install infers the initialized version.
Existing differing authoring is not overwritten.
To regenerate native profiles from your installed Unelos definitions:

```sh
unelos --project /absolute/project install superpowers-dev --json
```

Generated `.opencode/agents/unelos-*.md` profiles are overwritten on reinstall,
including manual edits. Customize `.unelos/agents/<id>/agent.yaml`, its Markdown,
`.unelos/skills/<id>/SKILL.md`, or inline tasks in
`.unelos/workflows/superpowers-dev.yaml` instead. Unrelated agents, project
configuration and runs are preserved.

## Models

`model:` is unset in every agent. Set `model:` in `agents/sp-final-reviewer/agent.yaml`
to the strongest available model, and a cheaper one in `agents/sp-implementer/agent.yaml`.

## Interactive walkthrough (V2)

```sh
opencode --standalone /absolute/path/to/your/project
```

1. Enter `/unelos`; select **superpowers-dev**. The controller asks what needs to
   be done if you have not already described it.
2. Brainstorm with the controller and agree the design. It writes `spec.md`.
3. Approve or reject the spec at the gate. Conversation approval is not gate approval.
4. Review the plan the same way; approve it at the gate.
5. The controller sets up an isolated worktree. From here the task loop runs
   without check-ins: implementer, reviewer and controller `advance` repeat per
   task, followed by the final review and at most one final fix wave. You do
   **not** issue manual `complete` commands.
6. Answer any worker question in the main conversation; the same Task/worker continues.
7. At `finish`, choose merge locally, push + PR, keep the branch, or discard
   (discard needs typed confirmation). `finish.md` records the result and rulings.
8. For recovery, enter `/unelos resume` and select an unfinished run. `/unelos`
   always begins fresh; `/unelos off` affects only the current session.

V1 uses the packaged Markdown command and native `task`/`task_id`; V2 uses
`subagent`/`sessionID`. See the
[execution verification report](../../docs/opencode-execution-verification.md) for
observed versions and acceptance status.

## Deliberate simplifications

- No model escalation after repeated fix rounds; each agent's model is fixed.
  Add an `sp-implementer-escalation` agent and an `escalate` outcome if needed.
- No per-task brief or review-package files; `plan.md`, `progress.md` and git
  ranges replace them.
- No implementer resume across fix rounds: each fix round is a fresh worker that
  reads the report file.
