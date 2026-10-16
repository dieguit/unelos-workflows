# Unelos native OpenCode tools

Thin adapter for OpenCode V1 ≥1.18.29 and V2. Shared instructions, execution
prompts, result validation and guarded progression live in Rust core. The model
dispatches configured native workers; the plugin does not intercept subagents.

## Install integration

Install the Unelos CLI on your machine, then run inside an existing project:

```sh
unelos init --opencode v2
```

This creates missing authoring scaffold, writes the bundled plugin into
`.unelos/opencode/unelos`, and updates the existing project OpenCode JSON/JSONC
configuration without replacing unrelated settings or comments. With no config it
creates `opencode.json`; multiple local config files require manual resolution.
No npm, source checkout or network is required at initialization time.

The plugin points to the absolute running CLI executable. Rerun `unelos init` to
repair missing managed assets or refresh that path after moving the CLI. Reruns
preserve authored workflows/skills/definitions, unrelated agents and run reports.
Legacy local `thin-plugin`/`plugin` registrations migrate to `unelos`; their old
directories are not deleted. Init does not install a sample workflow.

Use `unelos init --opencode v1` for V1; it also installs the slash command.
Without a flag, first init defaults to V2 and reruns retain the recorded version.
Version switches remove only an unchanged generated V1 command, never an edited
one. A differing command blocks V1 installation rather than being overwritten.
V1 normally resolves `@opencode-ai/plugin/tool` from the native configuration's
installed modules. For isolated installs, set `v1ToolModule` to the
absolute harness-installed `@opencode-ai/plugin/dist/tool.js`. Missing native SDK
fails explicitly; no custom schema shim or automatic/global installation.

## Install Workflow and native agents

```sh
unelos --project /absolute/project install superpowers-dev \
  --source /absolute/unelos/examples/superpowers-dev --json
```

After init, `install` infers the recorded OpenCode version. This imports referenced authoring/Skills and generates
`.opencode/agents/unelos-<agent-id>.md`. Differing authored source collisions stop;
generated native files overwrite on reinstall, including manual edits. Customize
Unelos definitions, then regenerate without `--source`:

Source packages contain `workflows/`, `agents/`, and required `tasks/`/`skills/`
directly, without a `.unelos` wrapper. Installed definitions live in
`.unelos/agents/<id>/agent.yaml` and `.unelos/tasks/<id>/task.yaml`; workflow-local
tasks use inline `taskDefinition`. Markdown `instructionsFile` paths are relative
to their YAML file. Agent/task resource folders are copied; destination settings
and package README files are not imported.

```sh
unelos --project /absolute/project install superpowers-dev --json
```

Unrelated native agents/configuration and existing run state remain untouched.
No model is written when unauthored: native subagents inherit the parent model.
Required tools are not permission grants/allowlists. Unsupported declared MCP
requirements fail explicitly; third-party Skills/MCP/credentials are not installed.
An explicit `install --opencode v1|v2` overrides agent generation without changing
the recorded integration. Without metadata or a flag, install remains core-only.
Missing recorded integration assets block install with guidance to rerun init.
Deleting `.unelos` destroys custom authoring and run history; init only recreates
scaffold and integration, not that data.

### Developer packaging

Build `cargo build --locked -p unelos-cli` (or install from the checkout with
`cargo install --locked --path crates/cli`). The CLI embeds integration runtime
files at build time. `npm pack` here is optional for distributing the plugin
separately; keep extracted package files together outside `.opencode/plugins`.

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

Parallel Nodes (V2 only) supply `workerPrompts[]`, each with its own mapped
`nativeAgent`. The controller starts every worker with `background=true` in one
turn, relays a worker's question with `unelos_continue` + `workerKey`, writes the
aggregate itself, then completes the parent with every worker's final frame.

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
