# T17 parallel dispatch — OpenCode V2 verification

Checked 2026-10-16. One real-model run of `examples/parallel-research` on
OpenCode V2. All operator choices (workflow selection, interview answers, the
worker's view question, gate rejection and approval) were **assistant-scripted
QA**, not user decisions. Raw transcripts, event stream and probes are private
under `/tmp/opencode/t17-evidence/`; excerpts below are trimmed/redacted.

## Environment

- OpenCode v2.0.30, private `opencode serve` on loopback, driven over its HTTP
  API (`session.create`, `session.command` name `unelos`, `session.prompt`,
  `experimental.session.wait`, `session.message.list`) plus `/api/event` capture.
- CLI built from commit `55d39db` (`cargo build --locked -p unelos-cli`).
- Model: `openai/gpt-6.1-sol#low` for controller and both workers (worker
  sessions inherited it; no authored model). The default configured model
  (OpenRouter) failed with HTTP 402 before any tool call; that attempt is
  discarded.
- Project: throwaway copy of this repo's `src/`, `src-tauri/src/`,
  `crates/core/src/`, `package.json`, `Cargo.toml` at `/tmp/opencode/t17-run`;
  `unelos init --opencode v2`, then `install parallel-research --source
  examples/parallel-research`. Generated agents:
  `unelos-backend-researcher.md`, `unelos-frontend-researcher.md`,
  `unelos-research-lead.md`.
- `opencode run "/unelos"` does not invoke the slash command (model saw plain
  text; `unelos_next` → `controller_inactive`). The API `session.command` was used.

## Steps

1. `/unelos` → menu → `1` (parallel-research).
2. Interview answer: question "How does a Unelos run get persisted and shown in
   the app?", backend `crates/core/src` + `src-tauri/src`, frontend `src/`, and
   "frontend researcher: ask which view to focus on" deliberately left open.
3. Controller wrote `scope/brief.md` and ledger `## Brief`, completed `scope`
   (task-1), received parallel package task-2 and dispatched both workers.
4. Frontend NEEDS_CONTEXT relayed; answer: dashboard view.
5. Controller completed task-2, wrote synthesis (task-3), presented gate-4.
6. Rejected with rework feedback → parallel task-5 → synthesis task-6 → gate-7.
7. Approved → `finished`.

Run `r-000001`, main session `ses_…3COtilS`; worker sessions `ses_…lrQsreGUgyto`
(worker-1, backend) and `ses_…pwMzAt6H14aiHd` (worker-2, frontend).

## Observations

### 1. Concurrent dispatch — PASS (with one deviation)

Both `subagent` calls had `background:true`, used the package's `nativeAgent`,
and their prompts were byte-identical to `workerPrompts[i].prompt`. Each
returned immediately with "working in the background (sessionID …)".
Timestamps (UTC, from tool `time`, child session messages and frames):

| Event | Pass 1 (task-2) | Pass 2 (task-5) |
| --- | --- | --- |
| worker-1 child receives prompt | 01:22:30.939 | 01:27:19.521 |
| worker-2 child receives prompt | 01:22:46.607 | 01:27:37.075 |
| first worker result | 01:23:01.355 (w2 NEEDS_CONTEXT) | 01:27:38.211 (w1 FINISHED) |
| other worker result | 01:23:42.217 (w1 FINISHED) | 01:27:59.915 (w2 FINISHED) |

Both workers started before either result in both passes; in pass 1 they ran
overlapped for ~15 s and backend kept running for 41 s after the frontend
question. **Deviation:** the controller emitted the two calls in two consecutive
model steps of the same user turn (separate assistant messages, ~15 s apart
while generating the second long prompt), not two calls in one message. In
pass 2 the margin was ~1.1 s. This is model behavior, not adapter/core: the
package contains both prompts and the harness instruction says "in one turn …
for every entry before waiting". Concurrency is shown by start/result
timestamps, not by file existence.

### 2. NEEDS_CONTEXT relay — PASS

worker-2 frame (trimmed):

```text
{"kind":"parallel-worker","runId":"r-000001","nodeId":"parallel-research","taskId":"task-2",
 "workerKey":"worker-2","status":"NEEDS_CONTEXT",
 "question":"Which app view should frontend research focus on for fetching and displaying Unelos runs?"}
```

Controller relayed the question; operator (assistant-scripted) answered
"dashboard view". Controller then:

```text
unelos_continue({expected:{kind:"parallel",runId:"r-000001",nodeId:"parallel-research",taskId:"task-2"},
                 workerKey:"worker-2", answer:"User selected the dashboard view: …", progress:[…]})
→ expected {kind:"parallel",…,taskId:"task-2"}; workerPrompts = [worker-2 only, nativeAgent unelos-frontend-researcher]
subagent({agent:"unelos-frontend-researcher", sessionID:"ses_…pwMzAt6H14aiHd", background:true, prompt:<exact continuation prompt>})
```

The continuation prompt carried the answer in `Continuation context`. Parent
identity stayed `task-2` (no new parent Task). The controller did not call
`unelos_next`; identity is evidenced by the `continue` response `expected` and
the later accepted `complete` on `task-2`.

### 3. Guarded completion and aggregate — PASS

Controller read both reports, wrote `parallel-research/research-summary.md`,
then called `unelos_complete` with `expected` parallel task-2 and `resultText`
containing both FINISHED `parallel-worker` frames (worker-1, worker-2). Returned
package: `synthesis` task-3 with inputs `brief.md` and
`…/parallel-research/research-summary.md`, feedback artifact the same aggregate.

Aggregate excerpt:

```text
## worker-1 — backend-researcher
- Report: `backend-researcher/result.md` (relative to this summary).
## worker-2 — frontend-researcher
- Report: `frontend-researcher/result.md` (relative to this summary).
- Questions/decisions: worker returned NEEDS_CONTEXT instead of guessing. User selected dashboard …
```

The controller never attempted a partial completion, so "only with both" was
probed separately via the CLI against a **copy** of the run at task-5 (paths
rewritten to the copy; real run untouched):

| Probe `resultText` | Result |
| --- | --- |
| worker-1 FINISHED only | `invalid_worker_result`: "Missing or unfinished worker results: worker-2" |
| worker-1 FINISHED + worker-2 BLOCKED | `invalid_worker_result`: "Worker result does not match the expected execution" |
| both FINISHED | accepted → `synthesis` task-6 |

Copy state stayed `parallel task-5` after both rejections. The BLOCKED message
is generic rather than naming the unfinished key (minor).

### 4. Ledger written only by controller — PASS

Worker session file writes (all sessions, both passes): `patch` Add/Update only
on their own `…/<agent>/result.md`; no `bash`/shell, no ledger or aggregate
writes. Workers stated "parent aggregate untouched". All 7 ledger patches came
from the main session.

### 5. Rejection re-entry and approval — PASS

`unelos_reject` on gate-4 (assistant-scripted, actionable feedback) returned
parallel `task-5` with both worker prompts. Persisted history: task-2 and task-5
have identical `research-summary.md` and worker report paths
(`backend-researcher/result.md`, `frontend-researcher/result.md`). Reports,
aggregate and synthesis were updated in place: diffs against the pass-1 copies
only append `## Rework notes`; prior content preserved. Controller completed
task-5 with both task-5 frames → synthesis task-6 → gate-7. Assistant-scripted
approval → state `current: {kind:"finished", nodeId:"done"}`.

The controller re-dispatched rework by continuing the existing worker sessions
(`sessionID` set) with the exact task-5 prompts, rather than starting fresh
sessions. Contract permits this; noted for review.

## Limitations

- V2 only; V1 parallel dispatch is unsupported and was not exercised.
- Single run, single model; operator was the assistant, not a human.
- Concurrency evidence: OpenCode tool timestamps, child-session message times
  and `/api/event` capture. Dispatch was two sequential steps in one turn, not
  one multi-call message; pass-2 overlap margin was ~1 s.
- Partial-completion rejection was a CLI probe on a copied run, not a
  controller-issued attempt.
- Run used the API rather than the TUI; TUI interaction was not exercised.
- Research findings are static; no tests were run in the throwaway project.
