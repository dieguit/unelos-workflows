# T16 execution: implementation and handoff evidence

Checked 2026-10-08. Implementation and project-local workflow are installed;
**full T16 acceptance remains pending**, including the user's interactive run.
This report adds evidence without replacing [T15 results/failures](opencode-delivery-verification.md).

## Delivered

- Originating Task/gate identity checks, exact native worker preparation, actual
  child model/configuration checks, FINISHED/NEEDS_CONTEXT/BLOCKED receipts.
- Same-Task native continuation with answers/progress; stale re-entry blocks.
- JSON-stdin guarded complete/approve/reject helper: fresh state, output files,
  outcome selection, single mutation and uncertainty reconciliation.
- Controller automatic ordinary completion, explicit human gates, recovery-only
  activation, controller-owned brief/ledger and in-place reports.
- [Reusable page-change workflow](../examples/opencode-execution/README.md):
  interview → plan → approval → implementation → independent review → finish.

Final packed archive SHA-256:
`008b3340cb5d6f9f4a5b80354900629e86c26e9cb61019ae1a915bf43078a0f3`.
Ten packaged files: six JS runtime modules, manifest, bootstrap, command, README.
No dependencies, test fixtures, runtime state, private logs or superpowers docs.

## Observed native smoke checks

Private root: `/tmp/opencode/unelos-t16-native-qaura712/`.
Raw prompts/logs/authentication details remain private; do not publish them.
The disposable request changed only `greeting.txt` to `Welcome back\n`.
This is **text-only smoke QA**, not the required NORTE frontend acceptance.
Gate messages were explicitly labelled assistant-scripted QA operator decisions,
not represented as human approval or the user's interactive run.

| Check | Observed result |
| --- | --- |
| V2.0.25 installed native flow | Controller created requirements/plan, stopped at gate, used separate native implementation/review sessions, inspected reports, invoked guarded completion and reached finish. Actual file bytes verified. |
| V2 worker boundary | Actual native before input, child prompt and executor request captured with a private observer. Native adds a banner before the exact prepared prompt. Initial archive missed admission; work/result association failed and controller stopped. Banner regression fixed RED→GREEN, revised packed archive admitted/verified workers successfully. Initial failed archive/evidence retained. |
| V1.18.29 installed native flow | Interview confirmation, plan gate, actionable scripted rejection, new plan/gate identity, scripted approval, separate native implementer/reviewer and guarded completion reached finish. Native child metadata and assistant model records match prepared bindings. |
| V1 failures retained | Initial provider 402 required a QA-only output limit and owned-server restart. Controller initially confused Unelos Agent IDs with native carriers; it also tried malformed worker/dispatch protocols before correcting them. Native checks rejected those calls. It later completed, but this is not a pristine controller-adherence pass. Integration now normalizes the native carrier to general, with regression coverage. |
| Recovery-only activation | Fresh V2 session recovered an unfinished interview run without resume authorization; only read operations occurred and run/ledger hashes stayed unchanged. V1's attempt preserved bytes but its model failed with insufficient credits, so V1 recovery adherence remains unverified. Neither check establishes compaction safety or erases historical failures. |
| TUI startup | V2 standalone NORTE and isolated V1 project launched under PTYs for ten seconds, remained running and emitted no detected fatal startup text. No interactive interview/gates were exercised through the TUI; startup is not interactive acceptance. |

Independent review found the banner admission bug and an owned prepared-dispatch
escape when current work became main/gate/finished/idle or activation was removed.
Both received RED→GREEN regression checks, including new-call continuation binding.
Normal unmarked helper calls remain unaffected.

## User's project-local handoff

`/home/dieguit/projects/norte_unelos` has the packed plugin and generic Workflow,
Task/Agent maps and selected Skill installed locally. Empty authoring maps were
merged only after collision checks. The previous baseline plugin/scaffold is backed
up privately. Existing CLI, native config, source changes and global settings were
not replaced. Core remains **idle**, with no assistant-created user-project run.

```sh
opencode --standalone /home/dieguit/projects/norte_unelos
```

Enter `/unelos`, select `page-change`, describe your page/feature, answer the
interview and explicitly approve/reject the presented plan. Ordinary Tasks should
complete without manual CLI commands. Reports live at paths delivered for the run.
Project rules may require confirmation before work on the current main branch;
no NORTE worktree was created.

**Prerequisite blocker:** NORTE requires `impeccable` for frontend/review,
`ponytail-review`, `code-review`, fallow and Playwright MCP. Queried native Skill
and MCP inventories were empty; these required resources were not demonstrated
available. The workflow can start/interview, but applicable frontend/review work
must block until prerequisites are available. No third-party installation or
global configuration change was silently made.

## Verification scope and remaining acceptance

Package suite: 62 test executions, zero failures (some shared packet tests execute
again when imported). Core/CLI Cargo tests and real graph/packet inspection passed.
Installed archive callbacks are exercised separately from workspace imports.
`test/execution-evidence.mjs` rejects incomplete/contradictory full-acceptance
inputs; its synthetic fixture proves the checker only. No full native/human
acceptance evidence directory passed that checker.

Still pending: actual NORTE frontend prerequisites and page QA, native question/
same-handle answer relay, genuine review corrections/re-entry, compaction/changed
authoring recovery and the human's TUI workflow run. Callback tests cover several
of these contracts, but are not native/user acceptance. Nonempty authored MCP
requirements and V1 required-tool lists still fail closed as in T15.

Deferred minor: unavailable continuation lookup lacks a distinct
`worker_unavailable` diagnostic; it fails without completion but gives a less
specific error. No commits/push/merge were authorized or performed.
# Historical report

Current architecture/evidence: [thin native adapter](opencode-thin-adapter-verification.md).
