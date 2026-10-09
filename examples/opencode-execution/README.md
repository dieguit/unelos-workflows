# Run your own page-change workflow

Interview → plan → human approval → implementer → separate reviewer → finish.
Review corrections return to implementation; plan rejection returns to planning.
Your page and feature are conversation inputs, not installation settings.

## Install locally

Build/package the [OpenCode integration](../../integrations/opencode/README.md).
Keep its extracted runtime together outside `.opencode/plugins`. Configure the
project-local plugin with the absolute CLI path; preserve all other settings.

Install the workflow and generate native OpenCode agents in one operation:

```sh
unelos --project /absolute/project install page-change \
  --source /absolute/unelos/examples/opencode-execution --opencode v2 --json
```

Use `--opencode v1` for V1. Existing differing authoring is not overwritten.
To regenerate native profiles from your installed Unelos definitions:

```sh
unelos --project /absolute/project install page-change --opencode v2 --json
```

Generated `.opencode/agents/unelos-*.md` profiles are overwritten on reinstall,
including manual edits. Customize `.unelos/project.yaml` instead. Unrelated agents,
project configuration and runs are preserved. No global installation is needed.
The example ships no runtime state, ledger or fixed feature.

Before starting, check the project's required frontend/review Skills and browser
tools in the actual OpenCode session. Missing prerequisites block relevant work;
this workflow does not waive project instructions or add those resources.

## Interactive walkthrough (V2)

```sh
opencode --standalone /absolute/path/to/your/project
```

1. Enter `/unelos`; select **page-change** and describe your change.
2. Answer the interview until the requirements reflect shared understanding.
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
