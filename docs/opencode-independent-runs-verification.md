# Independent Unelos runs — verification (2026-10-09)

## Delivered behavior

- `/unelos` always begins fresh; no historical run/pointer lookup.
- `/unelos resume` lists unfinished runs. Explicit selection injects bounded persisted
  context and immediately authorizes ordinary work, never a pending gate decision.
- `/unelos off` changes only the current native main session.
- Stable IDs address every core/CLI operation. Native tools use their session binding;
  cross-run controls fail before CLI. Concurrent runs do not isolate shared source edits.
- Resume preserves Task identity, existing bindings/reports and history. Missing
  essential context blocks dispatch. Empty, missing, invalid and oversized text differ.

## Automated checks

- **235 Rust tests**, core/CLI suites; CLI build; formatting and whitespace checks.
- **22 Node tests**, workspace and extracted runtime; both V1/V2 bindings exercised,
  zero skips, using an existing harness-installed V1 SDK helper. No new dependencies.
- Archive contains exactly five runtime files; extracted bytes match workspace.
- Native graph remains six Nodes/seven edges, with separate implementation/review,
  human plan gate, plan rejection and review correction routes.
- Independent final review: one Important metadata check/write race, no Critical or
  Minor findings. Deterministic V1/V2 regression failed before the fix and passes
  afterward. Per-session metadata changes now serialize; CLI work does not hold that
  queue. Full suites pass after the fix. No second reviewer or commits.

## Installed OpenCode V2.0.25

Disposable private server, extracted package and generated native profiles. A local
scripted provider and explicit scripted operator decisions drove actual native API,
Code Mode/controller tools and native subagents; no shell progression helpers.
This verifies native integration plumbing, **not real-model adherence or UI acceptance**.

Observed checks:

- A/B begin fresh, start distinct unfinished runs and interleave planning. A's
  completion/rejection/approval leaves B's state/report hashes unchanged. B's rename
  leaves A unchanged and preserves B's stable identity.
- Fresh C selects an unfinished delegated run once; no second resume instruction.
  Native implementer/reviewer execute through finish, preserving existing report
  content. Five recorded dispatches exactly match core prompts; generated roles and
  inherited scripted model settings are visible in actual worker sessions.
- A historical finished run does not appear as the current run on `/unelos`.
- A newly created main session resumes a pending gate without changing its state or
  activation. A missing required input produces a context question, no progression.
  Unknown/finished selections leave the session unbound without fallback.
- An initial private driver failed because native worker prompts include a harness
  preamble. Child controller calls were rejected; the fixture was corrected and the
  full independence scenario rerun successfully. This was not a product fix.

## NORTE handoff

Before installation: all **645** source/config/runtime files or symlinks snapshotted,
user git changes recorded, prior CLI/plugin/profile bytes saved privately. Explicit
status identifies finished `r-000002` (`finances-red-buttons`); no run was advanced,
renamed, canceled or selected during installation.
The ignored root `opencode.json` was omitted by `git ls-files`, but had been read
explicitly before installation; its full bytes match that pre-install text exactly
and its checked baseline is retained separately. Final comparison covers 646 entries:
640 unchanged, six authorized CLI/runtime/START_HERE updates. Profiles are unchanged.

Installed verified CLI/five-file runtime and regenerated resident native profiles.
Existing plugin configuration and obsolete pointer retained. START_HERE documents
fresh/resume behavior. Post-install checks compare every unrelated source,
authoring, configuration and stored run/report byte against the snapshot.

Native NORTE discovery after initialization sees the plugin and all three generated
roles. Isolated inventory sees fallow, but not impeccable/frontend-review/code-review
or ponytail-review; MCP inventory is empty. Prior resident inventory had ponytail-review.
Recheck the user's actual session; do not bypass required resources or install third
parties implicitly. Use a fresh main session/command to establish a new binding.

## Remaining acceptance limits

- Current environment exposes V2 only. **New native V1 execution is unverified**;
  SDK-backed adapter tests are not V1 native/provider acceptance. Earlier Google
  credential/free-model protocol failures remain unresolved.
- No real-model or interactive frontend acceptance was obtained. Scripted report-only
  QA cannot establish frontend quality, browser checks, or human gate intent.
- Deferred empty worker-result summary validation remains outside this refactor.
- No commits, pushes, merges, global config edits or third-party installs.

Private transcripts, fixtures, archive, status/hash snapshots and rollback bytes:
`/tmp/opencode/unelos-independent-runs-S862rM/`. Authentication remains private.
