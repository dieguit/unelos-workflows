# Handoff: create parallel steps in the workflow editor

## Goal

Users can add a parallel step from the workflow editor's "Add step" menu. Today the menu shows
"Parallel · Coming soon" (disabled). Parallel steps that already exist in YAML can be edited
(`ParallelPanel`), so most of the UI exists; what's missing is creating one and saving a new one.

## Done when

- "Add step" → Parallel inserts a parallel step between two steps (same split behavior as task/gate).
- The new step starts with 2 empty inline workers (agent + instructions), and a single `next` route.
- Saving writes valid YAML, the workflow reloads in the editor, and a run can start and reach it:

  ```yaml
  my-parallel:
    type: parallel
    tasks:
      - agent: researcher
        taskDefinition:
          instructions: Research A.
      - agent: researcher
        taskDefinition:
          instructions: Research B.
    inputs: [question.md]
    outputs: [summary.md]
    next: build
  ```

- Validation errors for an incomplete parallel step show in the panel like other steps
  (no workers, worker without agent/instructions, missing next).

## Where things are

Frontend
- `src/components/flow/flow-connector.tsx:25,61-79`: `AddKind` and the menu; `soon = kind === "parallel"` disables it.
- `src/components/flow/flow-pane.tsx:43,133` and `src/components/workflow-editor/workflow-editor.tsx:212-217`:
  `onAdd`/`addStep` typed as `"task" | "human-gate" | "workflow"`; widen to include `"parallel"`.
- `src/lib/step-edit.ts:65-93`: `insertStep` builds blank steps per kind; add the parallel case
  (`workers` via the existing `addWorker` at `:141`, `routes: [{ outcome: null, target: next }]`, id base
  e.g. `parallel`). Tests in `src/lib/step-edit.test.ts`.
- `src/components/workflow-editor/step-panel/parallel-panel.tsx`: existing editor for parallel steps
  (workers, inputs, outputs, next). Reuse; check it works for a step with `originalId: null`.
- `src/components/flow/parallel-card.tsx`: canvas card, already renders parallel steps.

Backend (core, Rust)
- `crates/core/src/workflow_document.rs:~456-525` (save): `type` is written only for `task` and
  `human-gate` (`:464-467`). A new parallel node would be saved without `type: parallel` and fail to parse
  (`definitions.rs:665-674` only infers `task`). Write `type: parallel` for parallel steps.
- Same function, `:468-475`: parallel `inputs` are only written when they changed vs the loaded doc;
  confirm this also works when there is no old step (new node).
- Load side: `workflow_document.rs:375-396` maps `NodeDefinition::Parallel` to `kind: "parallel"`.
- Schema: `crates/core/src/definitions.rs:611-618` (`ParallelNode`: `tasks`, `inputs`, `outputs`, `next`).
- Rules: `crates/core/src/validation.rs:89-119`: at least one worker, workers declare no outcomes,
  `next` must be a single successor.

## Runtime behavior (for UI copy, don't change)

Workers run as subagents in parallel; the coordinator writes the step's outputs (e.g. a summary) from
the worker results. Worker tasks can't have outcomes; the step has a single `next`.

## Checks

- `pnpm vitest run src/lib/step-edit.test.ts` (add a test for the parallel insert).
- `cargo test -p unelos-core` (add a save round-trip test: new parallel step → YAML has `type: parallel`
  → document loads back with the same workers).
- Manual: `pnpm tauri dev`, add a parallel step to a workflow, save, reopen, then start a run with the CLI
  (`unelos start <workflow> --json`, `unelos next --run <id> --json`) and confirm the package lists the
  workers.

## Out of scope

Converting an existing task step into a parallel step, dragging workers, per-worker outcomes.

## Project rules

See `AGENTS.md`: shared logic in `crates/core`, Tauri adapter thin; frontend code colocated; don't edit
`src/routeTree.gen.ts`. Don't commit `docs/superpowers/*`.
