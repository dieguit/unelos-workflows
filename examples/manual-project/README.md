# Workflow and Task inspection (T05a, T06)

This fixture has five Nodes and six edges. It starts at `plan`; approval routes
to `implement`, rejection loops back to `plan`, and review can route back to
`implement` before reaching `done`. `plan` uses the default `main` execution;
`implement` and `review` use `subagent`.

## Before testing (Linux, Bash)

Run all commands below in the same terminal, starting from the repository
root, so the `qa` variable remains available. They edit only a disposable copy,
never the shipped example.

```sh
cargo build -p unelos-cli --bin unelos
mkdir -p /tmp/opencode
qa=$(mktemp -d /tmp/opencode/manual-qa.XXXXXX)
mkdir "$qa/project"
cp -a examples/manual-project/. "$qa/project/"
cp "$qa/project/.unelos/workflows/implement-feature.yaml" "$qa/workflow.original"
cp "$qa/project/.unelos/project.yaml" "$qa/project.original"
```

Omit `--json` for readable, pretty-printed output. Add `--json` to either
inspection command when you want compact machine-readable JSON.

## Manual QA: workflow graph (T05a)

1. Inspect the workflow:

   ```sh
   ./target/debug/unelos --project "$qa/project" inspect implement-feature
   ```

   Expect `name: implement-feature`, `start: plan`, five Nodes, and six edges.
   Check the `rejected` edge goes from `approve-plan` to `plan`, and
   `changes_requested` goes from `review` to `implement`. `plan` shows
   `execution: main`; `implement` and `review` show `subagent`.

2. Test the default execution. Open
   `$qa/project/.unelos/workflows/implement-feature.yaml` in your editor and
   remove the `execution: subagent` line from `implement`. Save, then rerun
   the workflow inspection command above. Expect `implement` to show
   `execution: main`. Restore the original before continuing:

   ```sh
   cp "$qa/workflow.original" "$qa/project/.unelos/workflows/implement-feature.yaml"
   ```

3. Test an invalid transition. In the same workflow file, change the
   `implement` Node's `next: review` to `next: missing-node`. Rerun the
   workflow inspection command. Expect a nonzero exit and a diagnostic naming
   `missing-node`/`implement`; no graph is printed. Restore the file with the
   `cp` command in step 2 before continuing.

4. Test a missing project descriptor. Rename
   `$qa/project/.unelos/project.yaml` to `project.yaml.disabled` in your file
   manager or editor. Rerun the workflow inspection command. Expect a
   nonzero exit and an error naming the missing descriptor path. Rename it
   back to `project.yaml` before continuing.

5. Rerun the workflow inspection command twice. Expect the same graph both
   times; confirm `$qa/project/.unelos` has no `runs` directory or active-run
   file. Inspection is read-only.

## Manual QA: selected Task context (T06)

Use this shared command for the checks below:

```sh
./target/debug/unelos --project "$qa/project" inspect implement-feature --node implement
```

1. Run the command on the untouched copy. Expect Task and Agent instructions,
   `execution: subagent`, model/MCP/tool references, and logical `plan.md` and
   `implementation-report.md` names. The result has no run ID, Task ID,
   request, feedback, or concrete run paths. Rerun it and confirm the result
   is unchanged and no run files appeared under `.unelos`.

2. Open `$qa/project/.unelos/project.yaml` in your editor. In the `engineer`
   Agent, change `skills: [rust]` to `skills: [rust, verification]`. The
   `implement-plan` Task already selects `[verification]`. Save and rerun the
   shared command. Expect exactly two selected Skills in order:
   `verification`, then `rust`; this checks Task-then-Agent order and removes
   the Agent's duplicate `verification`. Each entry includes its full content
   and a directory under this disposable copy's `.unelos/skills`. Restore the
   project file before the next check:

   ```sh
   cp "$qa/project.original" "$qa/project/.unelos/project.yaml"
   ```

3. Open `$qa/project/.unelos/skills/code-review/SKILL.md` and add the unique
   line `UNRELATED_SKILL_MARKER`. This Skill is not selected by `implement`.
   Then open `$qa/project/.unelos/skills/verification/SKILL.md` and add the
   unique line `SELECTED_SKILL_MARKER`. Rerun the shared command. Expect the
   complete verification Skill text, including its marker, exactly once; the
   unrelated marker must not appear. Restore both files before continuing:

   ```sh
   cp examples/manual-project/.unelos/skills/code-review/SKILL.md "$qa/project/.unelos/skills/code-review/SKILL.md"
   cp examples/manual-project/.unelos/skills/verification/SKILL.md "$qa/project/.unelos/skills/verification/SKILL.md"
   ```

4. Rename `$qa/project/.unelos/skills/verification/SKILL.md` to
   `SKILL.md.disabled`, then rerun the shared command. Expect a nonzero exit
   and an error identifying `verification` and the expected file path. While
   it is still renamed, inspect the whole workflow by rerunning:

   ```sh
   ./target/debug/unelos --project "$qa/project" inspect implement-feature
   ```

   Expect the normal workflow graph: whole-Workflow inspection does not
   resolve Skills. Rename `SKILL.md.disabled` back to `SKILL.md`. Remove the
   disposable copy when finished if you no longer need it.

## Manual QA: parallel, gate, and end Node contexts (T07)

Build the CLI and run these inspections against the disposable project copy:

```sh
cargo build --locked -p unelos-cli --bin unelos
./target/debug/unelos --project "$qa/project" inspect research-feature --node parallel-research --json
./target/debug/unelos --project "$qa/project" inspect research-feature --node approve-plan --json
./target/debug/unelos --project "$qa/project" inspect research-feature --node done --json
```

1. `parallel-research` returns two ordered delegated workers: backend first,
   frontend second. Each worker contains its selected Skill text and logical
   `plan.md` input and `result.md` output. `BACKEND_ONLY_MARKER` appears exactly
   once only in the backend worker; `FRONTEND_ONLY_MARKER` appears exactly once
   only in the frontend worker. The parent aggregate output is the logical name
   `research-summary.md`. These are names, not concrete run paths.

2. `approve-plan` returns kind `human-gate`, logical inputs `plan.md` and
   `research-summary.md`, decisions `approved` and `rejected`, and requires a
   rejection reason. It has no executor. `done` returns kind `end`; this
   describes the Node, not a runtime `finished` state.

3. Repeat the parallel command and compare its output; it should be identical.
   Confirm the disposable project remains unchanged and has no `.unelos/runs`
   directory or active-run file.

4. Rename
   `$qa/project/.unelos/skills/backend-research/SKILL.md` to
   `SKILL.md.disabled`, then repeat the parallel command. Expect a nonzero exit
   and a diagnostic naming the selected Skill and its `SKILL.md` path. While
   it remains renamed, repeat the gate and end commands; both should still
   succeed because neither resolves worker Skills. Restore the Skill:

   ```sh
   mv "$qa/project/.unelos/skills/backend-research/SKILL.md.disabled" "$qa/project/.unelos/skills/backend-research/SKILL.md"
   ```

Illustrative flat runtime layout (not created or enforced by T07):

```text
.unelos/runs/change-button-color/
  ledger.md
  plan/plan.md
  parallel-research/backend-researcher/result.md
  parallel-research/frontend-researcher/result.md
  parallel-research/research-summary.md
```

The caller supplies artifact and ledger paths. Descriptive run-directory naming
is future controller-bootstrap behavior. Future workers update existing
artifacts in place and report changes; only the controller writes the
informational ledger. Core, not ledger content, owns progression. T07
inspection writes no runtime artifacts and does not implement dispatch,
artifact updates, or ledger writing.
