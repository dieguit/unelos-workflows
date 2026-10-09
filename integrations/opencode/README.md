# Unelos native OpenCode tools

Thin adapter for OpenCode V1 ≥1.18.29 and V2. Shared instructions, execution
prompts, result validation and guarded progression live in Rust core. The model
dispatches configured native workers; the plugin does not intercept subagents.

## Install integration

Build `cargo build --locked -p unelos-cli`. Run `npm pack` in this directory and
extract into a dedicated directory outside `.opencode/plugins`. Keep packaged
files together. Node stdlib transport; V1 uses its harness-installed native SDK
schema helper. V2 does not import that helper.

Merge into existing project configuration without replacing unrelated settings:

V2:
```jsonc
{"plugins":[{"package":"/absolute/package","options":{"binary":"/absolute/unelos"}}]}
```

V1:
```jsonc
{"plugin":[["/absolute/package/index.js",{"binary":"/absolute/unelos"}]]}
```

Copy `commands/unelos.md` to project `.opencode/commands/unelos.md` for V1.
V1 normally resolves `@opencode-ai/plugin/tool` from the native configuration's
installed modules. For isolated extracted archives, set `v1ToolModule` to the
absolute harness-installed `@opencode-ai/plugin/dist/tool.js`. Missing native SDK
fails explicitly; no custom schema shim or automatic/global installation.

## Install Workflow and native agents

```sh
unelos --project /absolute/project install page-change \
  --source /absolute/unelos/examples/opencode-execution --opencode v2 --json
```

Use `--opencode v1` for V1. This imports referenced authoring/Skills and generates
`.opencode/agents/unelos-<agent-id>.md`. Differing authored source collisions stop;
generated native files overwrite on reinstall, including manual edits. Customize
Unelos definitions, then regenerate without `--source`:

```sh
unelos --project /absolute/project install page-change --opencode v2 --json
```

Unrelated native agents/configuration and existing run state remain untouched.
No model is written when unauthored: native subagents inherit the parent model.
Required tools are not permission grants/allowlists. Unsupported declared MCP
requirements fail explicitly; third-party Skills/MCP/credentials are not installed.

## Run

```sh
opencode --standalone /absolute/project
```

Enter `/unelos`, select a Workflow and describe the change: it always starts fresh.
`/unelos resume` lists unfinished runs; selecting one injects its persisted context
and immediately continues ordinary work. Pending gates still need explicit decisions.
`/unelos off` disables only this session. Each session binds its own stable run ID;
independent runs do not isolate edits to shared source files.

Native tools: `unelos_next`, `unelos_start`, `unelos_resume`, `unelos_complete`, `unelos_approve`,
`unelos_reject`, `unelos_continue`, `unelos_status`, `unelos_inspect`, `unelos_rename`.
Each calls CLI once. Completion/decisions check full expected identity inside the
core runtime lock and return the resulting package. No shell-called JS helper,
status-then-next loop, worker tracking or per-model-request polling.

The package supplies `workerPrompt` unchanged and a mapped `nativeAgent`.
V1 dispatches `task(subagent_type, prompt)` and continues `task_id`; V2 uses
`subagent(agent, prompt)` and `sessionID`. Controller inspects reports/results,
relays questions and completes ordinary work. Native return alone is not success.
Core validates result frames; gate decisions still require real human intent.
`mutation_uncertain` requires explicit run-list/state reconciliation, never retrying
start automatically. CLI run-specific commands require `--run RUN_ID`.

This is option A: native configuration applies settings, but the adapter does not
prove that the controller selected the right agent or forwarded the exact prompt.
Core guards progression; this is not a security sandbox. Only explicitly activated
main sessions can use controller tools; ordinary sessions produce no automatic reads.

Historical evidence: [T15](../../docs/opencode-delivery-verification.md),
[T16](../../docs/opencode-execution-verification.md). Those reports describe earlier
architecture, not current acceptance. The thin-adapter verification report records
installed results separately; the user's interactive frontend run remains distinct.
