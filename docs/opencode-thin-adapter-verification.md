# Thin native OpenCode adapter verification

2026-10-08–09. This replaces the earlier JS orchestration architecture, not its
historical evidence. No commits, push or human frontend acceptance claimed.

## Verified implementation

- Core owns controller instructions, exact execution/continuation prompts,
  result-frame parsing, output checks and locked expected-identity progression.
- CLI exposes typed, bounded JSON-stdin execution controls and native profile
  installation. Generated profiles overwrite on reinstall; unrelated profiles,
  configuration, authoring and run state are preserved.
- Adapter registers nine native tools on both versions. Each calls CLI once;
  successful progression returns its resulting package. No JS mutation helper,
  dispatch interception, worker-session database or per-request polling remains.
- V1 loads its actual harness SDK schema helper lazily. Missing helper fails
  explicitly. The five-file package is index.js, v1.js, package.json, README.md
  and commands/unelos.md; extracted runtime bytes match workspace files.

Verification: 217 core/CLI Cargo tests; 10 workspace Node tests; 8 extracted-package
Node tests; native graph inspection; CLI build; formatting and whitespace checks.
The installed test set omits historical evidence-checker tests intentionally.

One independent correctness review found a partial-import issue: unsafe generated
agent destinations were checked after authoring import. A failing regression now
passes with all destinations preflighted before import. Writer checks again.
Deferred minor: empty worker-result `summary` is accepted; identity/file/outcome
guards still apply. This does not attest report quality.

## Native mechanism QA

All operator selections, approvals, rejection and question answers below are
**assistant-scripted QA**, not decisions by the user's human frontend operator.
Disposable change: exact bytes of greeting.txt, not a NORTE frontend change.
Private transcripts, observer traces and authentication remain outside the repo.

V2 discovered all three installed generated profiles after native location
initialization. Actual native controller tools reached plan approval. Rejection
of gate-3 returned plan task-4; revised planning presented gate-5. Explicit QA
approval returned implement task-6. A deliberately introduced *real* baseline
mismatch caused NEEDS_CONTEXT and a relayed overwrite question; answer used
`unelos_continue` and the same native worker session. A separate native reviewer
identified a deliberately introduced *real* missing final newline and returned
changes_requested, producing a new implementation identity task-8. Fault injection
belongs only to the private observer, never the runtime or shipped workflow.

Each observed workflow dispatch matched the exact core-provided prompt. No
shell-launched helper/progression occurred. Ordinary source verification shell
commands are expected and are not progression.
V2.0.25 reached persisted **finished**, after same-handle continuation of the
correction worker task-8 and a new separate reviewer task-9 returned passed.
Final greeting bytes were exactly `Welcome back\n`. The workflow used seven
native completions, one rejection, one approval, two continuations and six worker
dispatches; all six dispatch prompts matched core output exactly. The additional
`unelos_next` in combined observer counts was explicitly requested recovery QA,
not workflow polling. Reports/ledger retained correction history in place.

V2 fresh-main recovery of a deliberately unfinished run reported identity, Brief
and missing report without writing/dispatching/advancing. Native compaction then
one explicitly requested `unelos_next` recovered the same Task. All project hashes
remained unchanged. An ordinary, unactivated native session used zero tools and
zero Unelos calls.

Actual inherited V2 worker requests used parent openai/gpt-6.1-sol#low. A separate
authored-profile QA used a medium parent and observed a low worker request,
confirming openai/gpt-6.1-sol#low profile application rather than installation-time
model capture.

V1 discovered the three generated profiles and parsed the authored engineer model
google/gemini-3.1-flash-lite with variant low. Google requests could not execute:
API key unavailable. Real schema-backed native tools worked with a free model,
but opencode/nemotron-3.5-lightning-free ignored required result frames, repeatedly
submitted invalid identities/results, and claimed completion incorrectly. Core
rejected these requests; persisted execution remained at interview. Its apparent
gate/finish transcript is **not acceptance**. V1 actual worker/model progression
is unverified; test success must not be substituted for native end-to-end success.
V1.18.29 alternate free-model recovery with mimo-v2.6-flash-free timed out after
240 seconds and is also not acceptance. No credentials were installed or changed.

## Limits and handoff

Option A remains intentional: native configuration plus tools, not interception
or a security sandbox. Correct dispatch/forwarding and truthful human intent still
depend on controller adherence. Core guards stale/invalid progression, not report
quality or every unauthorized model edit.

The user's interactive NORTE frontend workflow remains the acceptance checkpoint.
Required project Skills/browser MCP and provider credentials must be available;
this installation does not add third-party resources or waive project rules.
Existing worker sessions retain old native configuration; use a fresh main session
and explicit user-directed recovery, not automatic redispatch.

NORTE handoff: regenerated profiles from resident authoring, installed the extracted
plugin alongside its retained predecessor, and updated only its owned plugin path
and CLI. Finished run r-000002 (`finances-red-buttons`), full status, 625
source/authoring/report hashes and global configuration were unchanged. Local
START_HERE.md documents launch/recovery, prerequisites and limitations. No work on
the user's changed finance components was performed by this refactor.
After native location initialization, NORTE discovered all three generated roles.
Its Skill inventory contains ponytail-review/fallow but lacks required
impeccable/frontend-review/code-review; MCP inventory is empty. Cold inventory
responses were initially empty and were not used as final availability evidence.

## Rulings made

In decision order; these are implementation choices, not additional features:

1. Resolve the resulting package before publication, return that candidate under
   lock. Avoids post-commit rendering failures; external authoring edits remain
   outside the runtime filesystem transaction.
2. Retain the existing isolated worktree and earlier uncommitted work until
   replacements pass. Cost: legacy modules temporarily coexist during transition.
3. Use existing workspace serde directly in CLI for typed stdin. Cost: one direct
   dependency/lock entry, no newly downloaded package.
4. Use existing YAML library for native frontmatter and shared core installation
   transport. Cost: direct dependency and small public filesystem helpers.
5. Resolve real V1 native schemas lazily with explicit isolated-install override.
   Cost: V1 cannot initialize until its installed SDK is resolvable.
6. Review the workspace, including untracked files, rather than an empty commit
   range. Cost: reviewer must explicitly read untracked source; no commits authorized.
7. Installed acceptance excluded from code review is owned by private native QA.
   Cost if wrong: unsupported acceptance claims; distinguish observed results and
   failures instead of extrapolating from unit tests.

Deferred minor: empty result summaries; tighten validation separately if needed.
# Latest architecture

See [independent-run verification](opencode-independent-runs-verification.md) for
fresh/resume/session-local behavior. This report describes the preceding singleton
thin adapter and remains historical evidence.
