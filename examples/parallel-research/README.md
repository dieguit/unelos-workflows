# parallel-research example

The research goal lives in the declared `brief.md` artifact, not a ledger `## Brief` section.

Scope a question (main) → backend and frontend researchers run in parallel
(subagents, separate Skills and reports) → controller writes
`parallel-research/research-summary.md` → synthesis (main) → human gate.
Rejecting the gate re-runs both researchers, updating the same reports.

    unelos --project /abs/project install parallel-research \
      --source /abs/unelos/examples/parallel-research --json

Parallel dispatch is supported on OpenCode V2 only.
