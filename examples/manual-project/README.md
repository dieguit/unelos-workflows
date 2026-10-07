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

The caller supplies artifact and ledger paths. Workers update existing
artifacts in place and report changes; only the controller writes the
informational ledger. Core, not ledger content, owns progression. T07
inspection writes no runtime artifacts and does not implement dispatch,
artifact updates, or ledger writing.

## Manual QA: persisted runs (T08)

Linux/Bash, from the repository root. Each `cargo run` below is a fresh process;
the repeated `next` checks recovery/persistence, not transitions or harness
delivery. Only the disposable copy is modified.

```sh
mkdir -p /tmp/opencode
qa=$(mktemp -d /tmp/opencode/t08-qa.XXXXXX)
cp -a examples/manual-project/. "$qa/"
cargo run --locked -p unelos-core --example run_qa -- start "$qa" implement-feature change-button-color "Change the button color."
cargo run --locked -p unelos-core --example run_qa -- status "$qa"
cargo run --locked -p unelos-core --example run_qa -- next "$qa"
cargo run --locked -p unelos-core --example run_qa -- next "$qa"
cargo run --locked -p unelos-core --example run_qa -- start "$qa" implement-feature second-request "Another request."
```

The first four commands succeed. `status` and both `next` packets retain the
request, `task-1`, and the same output path
`$qa/.unelos/runs/change-button-color/plan/plan.md`. The last command exits 1
with an active-run error. The initial run does not create `plan.md`, a worker
report, or `ledger.md`.

For live-definition checks, edit the disposable `$qa/.unelos/project.yaml`:
change `create-plan.instructions` and `planner.instructions`, then rerun `next`.
The packet shows both edits; `status` still has the same request and identity,
and the output path is unchanged. Add a unique line to the selected Skill at
`$qa/.unelos/skills/planning/SKILL.md`; `next` includes it. Rename that file to
`SKILL.md.disabled`: `next` exits nonzero because the selected Skill is missing,
while `status` still succeeds. Restore the filename. Confirm
`$qa/.unelos/runs/change-button-color/ledger.md` and
`$qa/.unelos/runs/change-button-color/plan/plan.md` are absent; runtime creates
directories and allocates references only.

Optional checks on a second fresh copy: invoke `start` with an invalid name such
as `../bad` and expect exit 1 without an active pointer. Change its workflow
start to the input-free `done` end Node before starting; the run is finished,
and a subsequent differently named `start` succeeds. Remove disposable copies
when finished.

## Manual QA: confirmed artifacts and same-path rework (T09)

Linux/Bash, from the repository root. Run each command individually (do not use
`set -e`): the expected failures are part of the check. Start from a fresh
disposable copy, not the already-active T08 `$qa` project.

```sh
cargo build --locked -p unelos-core --example run_qa
mkdir -p /tmp/opencode
qa=$(mktemp -d /tmp/opencode/t09-qa.XXXXXX)
cp -a examples/manual-project/. "$qa/"
./target/debug/examples/run_qa start "$qa" implement-feature artifact-check "Change button color."
./target/debug/examples/run_qa verify "$qa" task-1
./target/debug/examples/run_qa confirm "$qa" task-1
mkdir "$qa/.unelos/runs/artifact-check/plan/plan.md"
./target/debug/examples/run_qa confirm "$qa" task-1
rmdir "$qa/.unelos/runs/artifact-check/plan/plan.md"
: > "$qa/.unelos/runs/artifact-check/plan/plan.md"
./target/debug/examples/run_qa verify "$qa" task-1
./target/debug/examples/run_qa confirm "$qa" task-1
./target/debug/examples/run_qa verify "$qa" task-1
./target/debug/examples/run_qa confirm "$qa" task-1
./target/debug/examples/run_qa confirm "$qa" task-99
./target/debug/examples/run_qa next "$qa"
./target/debug/examples/run_qa bindings-demo "$qa"
mv "$qa/.unelos/runs/artifact-check/plan/plan.md" "$qa/.unelos/runs/artifact-check/plan/plan.saved"
./target/debug/examples/run_qa verify "$qa" task-1
mv "$qa/.unelos/runs/artifact-check/plan/plan.saved" "$qa/.unelos/runs/artifact-check/plan/plan.md"
./target/debug/examples/run_qa verify "$qa" task-1
```

Expected: first `verify` and `confirm` exit 1 because the plan report is
missing. Confirming the directory also exits 1. Verification of the empty
report exits 1 until confirmation; then confirmation, verification, and
unchanged reconfirmation succeed. Stale `task-99` exits 1. `next` succeeds and
still describes `plan`/`task-1`; it does not transition. `bindings-demo`
requires that confirmed active plan, prints JSON with `sameOutputPath`,
`newExecutionId`, `staleOutputRejected`, `feedbackPreserved`, and
`persistedStateUnchanged` all `true`, and creates synthetic implementation and
review reports only at the allocated paths. `verify` success serializes as
JSON `null`. Renaming the confirmed report makes verify exit 1; restoring it
makes verify succeed.

Confirmation is trusted controller attestation, not semantic proof. The
`bindings-demo` fabricates completed history only in memory; it does not persist
synthetic current/history. Production CLI has no runtime mutation commands yet.
This manual QA does not prove actual agent execution or output quality.

## Manual QA: completion and correction loop (T10)

Linux/Bash, from the repository root. Each command is a fresh process. Run
commands individually, without `set -e`; expected failures are part of the QA.
Only the disposable copy is modified. The loop uses the ungated
`completion-loop` workflow and synthetic report contents.

```sh
cargo build --locked -p unelos-core --example run_qa
qa=$(mktemp -d /tmp/opencode/t10-qa.XXXXXX)
cp -a examples/manual-project/. "$qa/"
./target/debug/examples/run_qa start "$qa" completion-loop color-loop "Change button color."
./target/debug/examples/run_qa next "$qa"
printf 'Plan v1\n' > "$qa/.unelos/runs/color-loop/plan/plan.md"
./target/debug/examples/run_qa confirm "$qa" task-1
./target/debug/examples/run_qa complete "$qa" color-loop task-1
./target/debug/examples/run_qa status "$qa"
./target/debug/examples/run_qa next "$qa"
printf 'Implementation v1\n' > "$qa/.unelos/runs/color-loop/implement/implementation-report.md"
./target/debug/examples/run_qa confirm "$qa" task-2
./target/debug/examples/run_qa complete "$qa" color-loop task-2
printf 'Review requests better contrast\n' > "$qa/.unelos/runs/color-loop/review/review.md"
./target/debug/examples/run_qa confirm "$qa" task-3
cp "$qa/.unelos/runs/color-loop/state.json" "$qa/before-invalid-label.json"
./target/debug/examples/run_qa complete "$qa" color-loop task-3 --outcome rejected
cmp "$qa/before-invalid-label.json" "$qa/.unelos/runs/color-loop/state.json"
./target/debug/examples/run_qa complete "$qa" color-loop task-3 --reason 'Adjust color contrast' --outcome changes_requested
./target/debug/examples/run_qa status "$qa"
./target/debug/examples/run_qa next "$qa"
cp "$qa/.unelos/runs/color-loop/state.json" "$qa/before-unconfirmed.json"
./target/debug/examples/run_qa complete "$qa" color-loop task-4
cmp "$qa/before-unconfirmed.json" "$qa/.unelos/runs/color-loop/state.json"
printf 'Implementation v2 with requested contrast fix\n' > "$qa/.unelos/runs/color-loop/implement/implementation-report.md"
./target/debug/examples/run_qa confirm "$qa" task-4
cp "$qa/.unelos/runs/color-loop/state.json" "$qa/before-old-task.json"
./target/debug/examples/run_qa complete "$qa" color-loop task-2
cmp "$qa/before-old-task.json" "$qa/.unelos/runs/color-loop/state.json"
cp "$qa/.unelos/runs/color-loop/state.json" "$qa/before-wrong-run.json"
./target/debug/examples/run_qa complete "$qa" wrong-run task-4
cmp "$qa/before-wrong-run.json" "$qa/.unelos/runs/color-loop/state.json"
./target/debug/examples/run_qa complete "$qa" color-loop task-4
printf 'Final review approved\n' > "$qa/.unelos/runs/color-loop/review/review.md"
./target/debug/examples/run_qa confirm "$qa" task-5
./target/debug/examples/run_qa complete "$qa" color-loop task-5 --outcome approved
./target/debug/examples/run_qa status "$qa"
cp "$qa/.unelos/runs/color-loop/state.json" "$qa/before-repeat.json"
./target/debug/examples/run_qa complete "$qa" color-loop task-5 --outcome approved
cmp "$qa/before-repeat.json" "$qa/.unelos/runs/color-loop/state.json"
```

The start response allocates `task-1` at
`$qa/.unelos/runs/color-loop/plan/plan.md`. Then `task-2` implements,
`task-3` reviews, `task-4` reimplements, and `task-5` reviews. The two
implementations reuse the same report path; the second `printf` changes its
contents. `status`/`next` after the first completion and after the
`changes_requested` re-entry must show fresh IDs and feedback; the latter
feedback has outcome `changes_requested`, reason `Adjust color contrast`, and
the review artifact. All `cmp` checks succeed: invalid label, unconfirmed
completion (even though the old implementation report exists), old task ID,
wrong run ID, and repeated finished completion leave `state.json` unchanged.
The final status is finished with five history entries, preserving both review
outcomes and the correction reason. Malformed option syntax (for example,
`complete "$qa" color-loop task-5 --outcome`) exits 2 with usage; core failures
above exit 1. Core writes no ledger; a human controller may append ledger notes
manually. This proves routing and persistence, not real model execution,
implementation quality, or gate decisions.

The existing `implement-feature` workflow remains gated. Independently verify
that its plan completion enters `approve-plan` and ordinary `complete` on that
gate fails; the QA workflow above is not a gate bypass for production.
