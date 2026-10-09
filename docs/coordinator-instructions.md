# Unelos controller instructions

Authoritative shared instructions: [instruction catalog](../crates/core/src/instructions/README.md)
and [core controller](../crates/core/src/instructions/controller.md).
The [OpenCode adapter](../integrations/opencode/README.md) delivers those instructions
and core-generated execution packages through native tools on V1/V2.

- `/unelos` always begins fresh in the current main session. Offer available workflows
  and Continue without Unelos; wait for selection. `/unelos resume` lists unfinished
  runs; selection injects context and immediately authorizes ordinary Tasks until
  missing essential context, a human gate or finish. Resume never approves a gate.
- Bind the returned stable run ID only to this session. `/unelos off` affects only
  this session. Independent runs do not isolate edits to shared source files.
- Save the agreed goal/constraints in a nonempty `## Brief` in the controller-owned
  ledger before delegation. Preserve existing reports in place.
- Use the returned package, not repeated `next` reads. Forward its exact
  `workerPrompt` to `nativeAgent`. Implementer and reviewer are separate native
  workers. Selected Skill bodies are already resolved; missing project-required
  resources block applicable work.
- Inspect reports and real verification before `unelos_complete`. Core validates
  framed FINISHED/NEEDS_CONTEXT/BLOCKED results, expected identity, files and outcomes
  under the runtime lock. Native worker return alone is not success.
- For every automated Task, main or delegated, complete an inspected valid FINISHED
  result through `unelos_complete` and continue from its returned package. Never
  hand completion to the user or stop at a success summary. Respect blockers,
  human gates and the user's latest stop/scope constraints.
- Only a returned `status: finished` confirms the run is done. Explicitly announce
  "Task done; Unelos run finished", with a brief result and remaining verification
  limitations; an accepted verification gap is not a passed check.
- Relay worker questions; `unelos_continue` prepares the same-Task prompt. Continue
  the same V1 `task_id` / V2 `sessionID` when available. A new-session resume may
  recover in a new designated worker after reading existing reports, without changing
  the Task ID or replaying edits. Save consequential answers in reports.
- Present gate artifacts and obtain an explicit decision bound to the presented
  activation before `unelos_approve` or `unelos_reject`. Task answers are not approval.
- Mutations return the resulting package. After `mutation_uncertain`, read state;
  never automatically retry. Read `next` for selection/recovery/reconciliation or
  after explicit rename, which changes artifact paths.
- `unelos_rename` requires expected run ID. Rename before worker dispatch or after
  all workers using old paths finish; otherwise defer.

Option A intentionally does not intercept dispatch or prove exact forwarding.
Native agent configuration supplies model/variant settings; unauthored models
inherit. Controller adherence remains required. Ordinary sessions do no automatic
Unelos reads. No shell-called JS helper, worker database or per-request polling.

The raw CLI supports `execution new|runs|start|resume|next|complete|approve|reject|continue
--json`; resume and run-specific reads/mutations require `--run RUN_ID`.
Mutation controls are typed JSON stdin. Legacy Node-only commands also require
explicit selectors, but do not supply the guarded native execution contract.

See [thin-adapter verification](opencode-thin-adapter-verification.md) for current
evidence and limitations; older T15/T16 reports are historical.
