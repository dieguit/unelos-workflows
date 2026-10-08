# Unelos coordinator instructions

The raw CLI does not install or inject these instructions. The optional
[OpenCode package](../integrations/opencode/README.md) supplies explicit
`/unelos`, `/unelos start`, `/unelos start WORKFLOW`, and `/unelos off`.
Activate only the current main session. When idle, offer discovered Workflow
choices plus **Continue without Unelos** (`/unelos off`) and wait for selection.
Continue running/waiting work; report finished rather than silently starting
again. `/unelos start` may offer new choices after finished. Ordinary sessions
and children do not inherit controller bootstrap. See
[native verification limits](opencode-delivery-verification.md).

For conversational requests, run `unelos start` to discover choices. Offer
suitable choices and **Continue without Unelos**; wait for the user's selection.
For the latter, do not use Unelos progression commands. Start the selected
Workflow with `unelos start WORKFLOW --name DESCRIPTIVE_NAME` after agreeing on
the goal. No CLI interview exists.

## Running work

- Bring relevant conversation context into the current work. Ask for missing
  information according to the Task instructions and persist answers in reports.
  Empty request text is not permission to invent requirements.
- CLI start persists empty request text. The controller supplies conversation
  context and records the agreed goal/constraints under `## Brief` in the supplied
  run ledger before Task work (create that file if absent at initial start).
  Record required downstream answers in reports; Definitions and Skills load
  from current authoring files, not snapshots. Persisted progression alone does
  not preserve unsaved conversation context.
- Fetch `unelos next` before execution or recovery. Treat persisted state as the
  progression authority. Honor selected Skills, placement, and configuration;
  surface harness capabilities that are unsupported.
- Recover by reading the ledger brief and applicable reports after fresh `next`.
  Missing/unclear brief means ask, not invent. Ledger prose never authorizes
  destinations or gates. Do not automatically redispatch, replay report writes,
  rename or advance on recovery; wait for authorization to resume Task work.
- Supplied selected Skill bodies are already loaded. Never re-resolve ambient
  names. Worker context is code-prepared with the relevant brief separately;
  exclude controller ledger metadata/duties. Production worker dispatch is T16.
- Read existing outputs and update them in place, preserving useful content.
  Only the coordinator writes ledger progress, decisions received, and artifact
  change notes. Workers do not edit ledgers or authorize progression.
- `complete NODE` explicitly confirms existing declared output files for current
  work and advances after validation. Unchanged reviewed outputs are allowed;
  there is no automatic hash/mtime freshness check or report-quality validation.
  After an uncertain mutation result, inspect `status` and `next`; Node-only
  mutations cannot identify a delayed callback to a later visit to the same
  Node, so never blindly retry.
- Present gate materials and obtain explicit human consent. A user's answer
  during a Task is not gate approval. Use `approve NODE`, or
  `reject NODE --reason 'actionable feedback'`.

## Naming and paths

Check `unelos status` for `namingPending`. Once enough context is known, choose a
descriptive, safe request-based name once and run
`unelos rename NAME --run RUN_ID` using the summary's run ID. Fetch a fresh
`unelos next` afterward. Do not automatically rename during recovery or later
clarification. Rename before dispatching workers, or after all workers holding
old paths finish; otherwise defer it. A rename relocates paths, so previously
delivered paths become stale. Re-read artifacts at paths in the fresh packet.

When no active run exists, `status` reports idle and `next` errors. Starting
without a Workflow lists choices; it does not create a run. No request-file,
separate confirmation command, execution ID, or completion reason is used.
