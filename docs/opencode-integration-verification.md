# T14: OpenCode integration evidence

Verified 2026-10-08: **OpenCode V2.0.25** (isolated and existing-plugin setups),
**V1.18.29** (isolated). Disposable probes only; no production adapter installed.

**Decision:** V2's essential path is demonstrated. **V1's essential path is now
observed on a representative workflow:** core context, model-led delegation,
controller question relay and same-worker `task_id` continuation. The initial
question turn required manual abort after provider throttling; continuation
completed cleanly. The original synthetic fixture still fails. This establishes
an integration path, not reliable autonomous orchestration or an installed
adapter. MCP requirements remain unavailable/unverified and must fail preflight.

## Evidence locations

- **I:** `/tmp/opencode/unelos-t14-v2-isolated.t1cnmbll/evidence/`
- **C:** `/tmp/opencode/unelos-t14-v2-coexist.h79m0ogt/evidence/`
- **L:** `/tmp/opencode/unelos-t14-v1-isolated.cio68ja9/evidence/`
- **R:** `/tmp/opencode/unelos-v1-document-review.LlNfWY/evidence/` (representative
  V1 follow-up; original I/C/L results below remain historical evidence)
- Execution ledger/scripts: `/tmp/opencode/unelos-t14-execution.gC7cke/`
- Each cell retains `openapi.json`, `context.jsonl`, native transcripts, core
  packets and reports. Context snapshots are taken at the probe hook, not
  guaranteed to be the last hook before transport. Persisted transcripts alone
  omit transient injection. Private logs may contain server credentials: do not
  publish raw evidence without redaction.

## Exact mechanisms and observations

| Operation | Version / exact mechanism | Observed result / evidence | Source / limitation |
| --- | --- | --- | --- |
| Explicit activation | Both: `.opencode/commands/unelos.md`; V2 `session.command({name,text})`, V1 `session.command({command,arguments})` | Ordinary sessions had no Unelos block or core reads. I/C `/unelos` offered selection without starting; explicit start created one run; continuation reused it. `selection-result.json`, `start-result.json`, `continue-result.json` | [Commands][commands], local OpenAPI. **L failed selection adherence:** `/unelos` auto-selected the sole workflow. Marker-based activation here is probe wiring, not a production command-authentication boundary. |
| Bootstrap/current packet | Dual export `{id,setup,server}`. V2 `ctx.session.hook("context")` adds a tagged transient `event.system` block; V1 `experimental.chat.messages.transform` adds a tagged text part to a retained text-bearing user message. Fetch real CLI `status`/`next --json` | Selected core instructions, paths, skill contents and controller guidance appeared before resource reads in I/C/L. `context.jsonl`, `next-main.json` | [Plugins][plugins], [migration][migration], local contracts. Controller activation flags persisted under the disposable project; hooks only read core state. Bootstrap/Agent probe guidance is not claimed to be entirely core-generated. |
| Selected skills/resources | Deliver resolved `skills[].content` and `directory` directly; native `read` accesses `directory/resource.txt` | Selected/resource markers reached main and workers; unselected marker absent. Native reads used supplied paths. Synthetic ambient skill with the same ID did not replace the supplied body | Core `skills.rs`/`context.rs`; transcripts and `context.jsonl`. No native reload of `t14-selected`; unrelated ambient skills remained available. No promise of total ambient isolation. |
| Shell/main conversation | V2 native `shell`; V1 `bash`. Main prompts via V2 `session.prompt` + wait/context; V1 `/session/{sessionID}/message` | Native tools started/read/completed the fixture; main conversation accepted supplied answers. `start-result.json`, `complete-worker-result.json` | [API][api], [V1 SDK][v1sdk], local OpenAPI. V2 admission is not completion: wait, then inspect outcome/errors. |
| Focused dispatch/results | I/C native foreground `subagent({agent,description,prompt,model})`; L `/session/{sessionID}/message` with `SubtaskPartInput {type:"subtask",agent,description,prompt}`, invoking native `task` machinery | One child per probe, separate worker marker and selected packet; controller/private markers absent. `worker-handle.json`, `delegation-result.json`, L `native-subtask-result.json`/`native-children.json` | [Agents][agents], local `SubtaskPartInput` schema. L's default model refused both model-led delegation attempts; direct native dispatch is an alternate observed path, not automatic model compliance. |
| Missing-context relay | Task Definition asks for missing PR link; worker returns `NEEDS_CONTEXT`; controller does not complete | I/C relayed the exact question. L native worker result contained the question, but parent model refused relay. Core status before/after question was identical. `question-state-before.json`, `question-state-after.json` | Task instructions; core/CLI contract. **Original L fixture failed; R follow-up below succeeded.** No external-input schema or engine question queue. |
| Answer/follow-up | I/C `subagent(...,sessionID)` with explicit answer; L native prompt to the same child session ID | Same native worker and Unelos `r-000001` / `task-2` / `worker-review`; report contains supplied link and resource, link not fetched. `continuation-result.json`, L `worker-api-continuation.json`, `continuation-check.json` | [Agents][agents], [V1 SDK][v1sdk], local contracts. Handles established from tool/API records and child snapshots, not model assertions. No re-dispatch was necessary; unavailable-handle re-dispatch remains a proposed fallback. |
| Dedup | Replace the probe-owned tagged block; never dedup on another plugin's generic marker | Repeated `next` returned identical packets. I/C two native hook registrations on one model call still yielded one block. L same-draft double transform passed in a **manual** callback check, separately captured. `next-a.json`, `next-b.json`, `double-transform-check.json` | Plugin source and captured evidence. Historical tool results can still contain old packets; this proves owned-block dedup, not transcript-wide elimination. |
| Compaction/restart | V2 `session.compact` then wait; V1 `session.summarize` with the already-used model. Restart only owned private servers; fetch current `next` | Completed compactions; same logical identity/paths, unchanged report/state/sentinel; original worker handles survived restart. `compaction-result.json`, `restart-check.json`, `recovery-hashes.json` | [API][api], [V1 SDK][v1sdk]. Initial L recovery failed on a compaction-only first user message; corrected probe selects a text-bearing user message. Textless recovery not demonstrated. |
| Fresh session / finished work | Fresh session inactive until `/unelos`; recover via `next`, never `start`; explicit completion once | All core states/outputs preserved through fresh activation; after authorized completion, `next` returned finished and no run/write replay occurred. `fresh-activation-result.json`, `finished-check.json` | Core/runtime packets. **Controller limits:** fresh V2 mains treated delegated work as their own and asked again for a persisted answer; fresh L initially answered generically. Wiring recovery is not guaranteed correct autonomous resumption. |
| Model / tool requirements | V2 explicit session model (`session.switchModel`) and temporary `agents` definitions; V1 explicit `agent` and model on native prompts/definitions. Deny edit/shell via per-agent rules | I/C resolved default model and `read` present, `edit`/`write`/`shell` absent for main/worker snapshots. L actual `t14-main`/`t14-worker` traces read successfully, reported write tools absent and preserved sentinel. `effective-permission-check.json`, `permission-*-result.json`, L `permission-worker-messages.json` | [Agents][agents], local contracts. L **session creation's agent alone was insufficient**: a prompt without `agent` ran as `build` and edited the sentinel. Supplying `prompt.agent` corrected it. V1 tool absence lacks a raw final-request tool snapshot. Tests demonstrate filtering, not attempted-call rejection or a hard sandbox; other native/plugin tools remain. |
| MCP / unsupported configuration | Inspect native MCP status and required model/tool inventories; probe-only preflight compares real inspected core references | MCP inventories empty in I/C/L: positive invocation and per-MCP filtering blocked. Missing model/MCP/tool produced specific probe rejection and no Task submission/state mutation. `api-mcp.json` / L `mcp.json`, `preflight-check.json` | [Plugins][plugins], local API. No production preflight exists yet. Plugin-provided quota tools are not evidence of MCP support. `read-file` → `read`; `run-command` → V2 `shell` / V1 `bash` only if effectively available. |

## Compatibility and implementation prerequisites

1. **Keep the two native paths separate.** V1 ≥1.18.29 accepts `server()`;
   V2 calls `setup()`. Test a packaged installation in T15; this probe used
   local dependency-free files. Wait for location/plugin initialization before
   treating an initially empty agent/model catalog as a missing requirement.
2. **Keep configuration explicit.** No root model was configured in original
   I/C/L cells: observed native defaults were V2 `openrouter/anthropic/claude-haiku-5.5` and V1
   `perplexity-agent/anthropic/claude-sonnet-4-6`. Do not silently use an ambient
   `general` agent's different model. Short V2 requests needed transient
   `maxTokens: 2048`; its default 128k reservation initially failed with 402.
   R explicitly selected that same V1 model in temporary configuration, without
   replacing the provider or model.
3. **Harden controller/executor instructions before claiming process support.**
   Selected skills are already loaded; honor `execution`, read inputs/feedback
   and existing reports, obtain essential missing context, persist answers,
   retain Task identity during questions, and surface conflicts. Controller-only
   ledger creation needed explicit clarification in I/C. L model behavior and
   fresh-controller behavior above remain limitations, not fixed product code.
   The V1 failure was discussed and a representative path subsequently observed;
   retain its synthetic-fixture, provider-recovery and packet-fidelity limits.
4. **Preserve core authority.** CLI `start` stores empty request text; supplied
   context must travel in conversation/reports. Node-only CLI mutations cannot
   reject delayed results from earlier visits to the same Node. Native handle
   persistence in these probes is not a lifetime guarantee. Rework/gates/full
   workflow acceptance remain T16, not evidence from this two-Task probe.
5. **Preserve ambient setup.** C loaded herdr, ponytail, RTK, Superpowers,
   google-agy and the probe. No user config/skill/plugin edits, credential copies,
   global installations or commits. Temporary V1 snapshots/updates were disabled;
   its empty Git project otherwise reported worktree `/`. Never introduce
   adapter variables under `UNELOS_*`: core settings reject unknown keys; the
   probe's original `UNELOS_T14_BIN` caused `invalid_settings`, corrected to
   `T14_BIN`. General ambient conflict enforcement remains unverified.

Verification and scope audit results are recorded in the execution ledger.
This note proves concrete integration paths and lists their limits; it does not
claim T15 installed, reliable autonomous V1 orchestration, or T16 acceptance.

## V1 blocker investigation (same day)

Compared the installed Superpowers V1 implementation (`superpowers.js:219–264`):
it uses the **same first-user-message injection hook**, inserts the full
`using-superpowers` skill/native mapping, and skips child bootstrap via `parentID`.
No hidden system-message escalation or special delegation API was found.

Fresh V1 sessions using the same `perplexity-agent/anthropic/claude-sonnet-4-6`
successfully invoked `task` and relayed a missing-PR-link question **both without
bootstrap and with installed Superpowers**. The original marked/JSON packet
reproduced rejection without Unelos injection and also with Superpowers. With
Unelos injection, one fresh session asked the question itself instead of
delegating; natural parent wording and an explicit fixture-purpose explanation
also failed to obtain delegation. Model responses cited synthetic/private
markers, opaque skill resources, empty request text and context-stripping.

This narrows the failure to the artificial packet/probe context, **not a general
V1 delegation failure or proof that the hook placement is wrong**. It does not
isolate which packet feature is causal or establish a fix. At this stage V1
packet-based controller support remained unverified. The user then approved the
representative Task/skill retest below; no stronger priority claims or model
changes were used to make the old fixture pass.

Evidence: `/tmp/opencode/unelos-v1-investigation.pbM4nC/` (`progress.md`,
`baseline/evidence/verdict.json`, `superpowers/evidence/*-verdict.json`,
`unelos/evidence/*-verdict.json`, native messages and hook snapshots).

### Representative documentation-review retest

Same V1.18.29, provider/model and first-user-text injection hook. Used a real
core `review-docs` workflow: a bound setup guide, a selected `readability-review`
skill with substantive `checklist.md`, a required missing audience, and a review
report. `/unelos` explicitly instructed the controller to honor `execution:
subagent`. Ordinary prose replaced synthetic controller/resource markers;
the CLI packet shape and empty stored request were unchanged. The fixture was
named and prepared by setup, not automatically started by the model.

| Operation | Observed R result | Evidence / limit |
| --- | --- | --- |
| Delivery/dispatch | Parent received the real current core packet, read guide/checklist, invoked native `task(subagent_type:"general",prompt)` and supplied the selected skill body, directories, bound input and declared output to one child | `context.jsonl`, `current-worker.json`, `aborted-controller-messages.json`. Parent rendered worker context into prose, **not a byte-for-byte JSON packet**; raw `runId`/`taskId` were not forwarded. T15 must preserve explicit identities deterministically. |
| Question | Worker asked who the guide was for; controller relayed that exact audience question. No report or Unelos progression occurred | `worker-question-messages.json`, `question-check.json`. Parent turn subsequently hit provider rate limits and many empty `finish:unknown` responses; HTTP timed out and the owned request was aborted. Do not call this a clean initial turn. |
| Follow-up/result | Controller received audience "engineers who are new to this project", invoked `task` with the **same native `task_id`**, and returned the worker's written report path. Native worker and parent both completed this follow-up with `finish:stop` | `answer-messages.json`, `worker-answer-messages.json`, `continuation-check.json`. Report states audience and cites actual checklist criteria. Same core `runId`/`taskId`/Node and one native child; no core completion. |

The essential model-led question/continuation path is now observed; the original
synthetic-packet refusal is **not fixed or disproved**. The exact triggering
feature remains unknown because this retest changed Task/skill wording, markers,
and explicit dispatch guidance together. Throttling recovery required operator
intervention. No production change, safety override or claim of universal model
adherence follows from this successful exemplar.

[plugins]: https://opencode.ai/v2/docs/build/plugins/
[migration]: https://opencode.ai/v2/docs/build/plugins/migrate-v1/
[commands]: https://opencode.ai/v2/docs/commands/
[agents]: https://opencode.ai/v2/docs/agents/
[api]: https://opencode.ai/v2/docs/api/
[v1sdk]: https://opencode.ai/docs/sdk/
