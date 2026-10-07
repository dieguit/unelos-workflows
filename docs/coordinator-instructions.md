# Unelos coordinator instructions

These are repository instructions for a future harness bootstrap. T12 does not
install or inject them. `/unelos`, `/unelos start`, and `/unelos start WORKFLOW`
are future entry points, not implemented slash commands: the first two should
offer discovered Workflow choices plus **Continue without Unelos**, wait for a
selection, then start it; the third should select that Workflow directly.

For conversational requests, run `unelos start` to discover choices. Offer
suitable choices and **Continue without Unelos**; wait for the user's selection.
For the latter, do not use Unelos progression commands. Start the selected
Workflow with `unelos start WORKFLOW`. No CLI interview exists.

## Running work

- Bring relevant conversation context into the current work. Ask for missing
  information according to the Task instructions and persist answers in reports.
  Empty request text is not permission to invent requirements.
- Fetch `unelos next` before execution or recovery. Treat persisted state as the
  progression authority. Honor selected Skills, placement, and configuration;
  surface harness capabilities that are unsupported.
- Read existing outputs and update them in place, preserving useful content.
  Only the coordinator writes ledger progress, decisions received, and artifact
  change notes. Workers do not edit ledgers or authorize progression.
- `complete NODE` confirms declared outputs were produced or reviewed and
  advances work. Use it only when the work/output is ready; unchanged reviewed
  outputs are allowed. After an uncertain mutation result, inspect `status` and
  `next`; never blindly retry a Node-based mutation.
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
