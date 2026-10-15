# T15: packaged OpenCode delivery evidence

Checked 2026-10-08: **OpenCode V1.18.29**, **V2.0.25** isolated and alongside
existing plugins. The opt-in package is implemented; **full acceptance is not
green**. V1 recovery replay remains a model-adherence blocker, not fixed by
delivery hooks. Subsequent execution implementation and user-run setup are in
[T16 verification](opencode-execution-verification.md); that report does not
replace the historical failures below.

## Installation and evidence

[Setup](../integrations/opencode/README.md) ·
The historical delivery fixture has been removed; the current
[workflow example](../examples/simple-plan-execute-review/README.md) replaces it
for new installations, not as evidence for these past observations.

Actual `npm pack` archive installed/extracted outside the source tree in each
cell, not workspace-linked. Final archive SHA-256:
`3ec8ebcb18c19ca445fe50f8bb8cbae450bee8e8f9e2217a02d2c6ecd2be0f37`.
Seven runtime files, no dependencies/test fixtures. V2 configured paths must
name the package **directory**; V1 uses `index.js` plus the copied Markdown
command. Native registration/selection verified, including `binary` options.

Private roots: `/tmp/opencode/unelos-t15-native-s3m057a0/`, with
`v2-isolated/evidence`, `v2-coexist/evidence`, `v1-isolated/evidence`.
Driver scripts/owned-server manifests retained at that root. **Raw logs contain
private context and may contain authentication details: do not publish them.**
No credentials copied, shared services restarted, or global integration installed.
217 pre-existing configuration/Skill/plugin file hashes remained unchanged.

Models stayed unchanged: V2 `openrouter/anthropic/claude-haiku-5.5`; V1
`perplexity-agent/anthropic/claude-sonnet-4-6`. Controlled children explicitly
received the parent's actual selected model; V1 prompts explicitly selected
their native Agent. Observer-only V2 `maxTokens:2048` avoided the known default
128k reservation issue, including compaction. No production token cap.

## Observed checks

| Check | Result / evidence |
| --- | --- |
| Native activation and selection | Ordinary requests received no owned block; `/unelos` offered choices without starting. V2 used its native question form; wait timed out while awaiting the operator's explicit selection, not a plugin failure. V1 offered choices in prose. `selection*`, `ordinary*` |
| Main context | Exact selected outline Skill, Task/Agent bodies, identities and paths captured before native reads/writes. Controller authored the ledger brief and outline. V1 also completed `outline` despite the probe stop instruction: adherence failure retained in `start.json` / `adherence-limit.json`. `main-next.json`, `context.jsonl` |
| Worker delivery | Installed `executorText` prepared exact review context; controlled native child received JSON identity/feedback/path/Skill bytes and relevant brief, without `ledgerPath` or controller duties. Actual checklist read and report observed on all three cells. Initial V2 observer failed on the catalogue envelope; same child retried after correction. `worker-session`, `worker*`, `verdict-input`, `tool-calls` |
| Dedup / refresh | Request snapshots show one owned current block. Native same-identity authoring refresh delivered the changed selected body. Double-callback/other-plugin preservation and V1 textless rejection covered by runnable package tests; manual callbacks are not final-transport proof. `refresh-verdict.json` |
| Compaction / owned restart | Compaction returned on both versions; persisted activation survived owned-server restart. Current context/ledger were read. Recovery adherence failures below prevent claiming all state/report-preservation checks passed. `compact`, `restart-recovery`, `fresh-activation`, before/after snapshots |
| Configured model | Exact already-selected configured model accepted at admission and observed in requests; missing model rejected before work. V2's native `model.list` uses `{location,data}`, corrected from the plan's array assumption with RED→GREEN test. `model-supported*`, `preflight-model*` |
| Required tools | V2 `read-file → read` accepted when available and rejected with native session deny policy; unrelated tools not restricted and state unchanged. Unknown tool rejected. V1 nonempty requirements fail closed because effective availability remains unverified. `read-required-*-verdict`, `preflight` |
| MCP / failures | Nonempty MCP requirements rejected on both versions; model/tool/MCP negative checks did not start executor work or change core state. Native HTTP often hides hook exception details (generic/empty 500); request-linked server cause logs contain required reference/version/cause. Normalized codes in `preflight.json` classify those observed causes, not an HTTP diagnostics guarantee. |
| Finished / opt-out | Finished activation did not create another run; `/unelos off` removed only owned activation, preserved finished state and restored ordinary delivery. `finished-optout-verdict.json` |

Capture stages: V2 `context` after the packaged integration; V1
`experimental.chat.messages.transform` after the packaged integration, with native
tool/API records. These are model-facing hook snapshots, **not necessarily the
last transport boundary**. Normalization extracts native text/tool records;
private raw context and native transcripts remain available for comparison.

## Recovery failures and retest

The aggregate `node integrations/opencode/test/evidence.mjs EVIDENCE_DIRECTORY`
passed for the original **V2 coexistence** cell. It correctly failed for:

- **V2 isolated:** review report bytes changed during recovery, though core
  `r-000001 / task-2 / review` identity stayed unchanged.
- **V1:** model-driven tools advanced review to `finished` and changed report/
  ledger during recovery. This also invalidated the same-execution check.

Explicit recovery guidance was added: read ledger/reports, do not automatically
redispatch, replay writes or advance; wait for authorization to resume work.
The V2 isolated fresh-session retest preserved packet/status/report/ledger.
V1 retest still failed; it used a separate real-core run seeded with previously
native-written reports/brief (not a repeat main/worker-generation acceptance).
`retest-next-*`, `retest-recovery`, `retest-verdict.json` retain both results.
Original failed snapshots were not overwritten or called passing.

Follow-up manual-QA preparation corrected the existing-run native command from
“Continue the existing run” to recovery-only guidance that explicitly does not
authorize Task work/progression and honors the latest user constraints. The V1
Markdown template was aligned; both native command paths have regression checks.
This removes a conflicting prompt, not an enforcement boundary. Earlier failures
remain evidence; native adherence with the corrected command awaits manual QA.

Subsequent manual QA found unnumbered choices and repeated CLI help discovery.
Command/context delivery now share a deterministic numbered list and accept
number/name selection instructions. Bootstrap now includes exact CLI grammar,
Node-only mutation/approval rules and project definition locations. Regression
checks verify delivered payloads, not that every model follows the guidance.
The installed synthetic `qa-draft` fixture/run was archived outside the test
project for a future realistic workflow; automatic orchestration remains T16.

Hooks invoke only read-only CLI operations. These failures are actual native
model tool behavior; the package does not add a permission sandbox or guarantee
instruction adherence. **Do not claim full V1 recovery support or T15 acceptance**
from the successful packet/resource delivery checks. Other versions/providers,
V1 effective-tool inspection and authored variant inventory remain unverified.

Package regression suite: `node --test integrations/opencode/test/*.test.js`.
Core/CLI request handling and progression grammar remain unchanged; CLI mutations
are still Node-only, without stale-same-Node callback protection.
# Historical report

Current architecture/evidence: [thin native adapter](opencode-thin-adapter-verification.md).
