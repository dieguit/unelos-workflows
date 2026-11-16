# Unelos OpenCode MCP adapter

Thin adapter for OpenCode V2. Shared instructions, execution
prompts, result validation and guarded progression live in Rust core. The model
dispatches configured native workers; the plugin does not intercept subagents.

## Install integration

Install the Unelos CLI on your machine, then run inside an existing project:

```sh
unelos init
unelos harness install opencode
```

`init` creates missing authoring scaffold and removes legacy project registrations
and known bundled assets (preserving custom files). It never installs a plugin.
`harness install opencode` installs v2 assets under the Unelos user config directory
and registers them in the OpenCode user `opencode.json` or `opencode.jsonc`, preserving
foreign entries and comments. Both config filenames existing is an error.
No npm, source checkout or network is required. The plugin uses a stable CLI copy
under the Unelos config directory; reinstall refreshes it and the integration.
Use `unelos harness uninstall opencode` to remove only our registration and recorded assets.
The stable CLI remains available. Reinstall removes the legacy `index.js` asset.

## Install Workflow and native agents

```sh
unelos --project /absolute/project install superpowers-dev --opencode v2 \
  --source /absolute/unelos/examples/superpowers-dev --json
```

With `--opencode v2`, `install` imports referenced authoring/Skills and generates
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
unelos --project /absolute/project install superpowers-dev --opencode v2 --json
```

Unrelated native agents/configuration and existing run state remain untouched.
No model is written when unauthored: native subagents inherit the parent model.
Required tools are not permission grants/allowlists. Agent `mcps: [id]` references
`.unelos/mcps/<id>.yaml` (source packages: `mcps/<id>.yaml`). Local/remote servers
are projected into project OpenCode config, disabled globally with `<id>_*` and
enabled only in referencing agent profiles. Missing definitions and user-config
collisions fail explicitly; reinstall cleans removed references. Environment/header
values live in project files: use OpenCode `{env:VAR}` substitution for secrets.
Third-party MCP executables and credentials are not installed.
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
The package ships raw TypeScript, loaded by OpenCode without a build step.

## Run

```sh
opencode --standalone /absolute/project
```

Enter `/unelos` or `/unelos <free text>`, select a Workflow and describe the change: it always starts fresh. Free text is passed as `requestedAction`; only the single words `off` and `resume` are keywords.
`/unelos resume` lists unfinished runs; selecting one injects its persisted context
and immediately continues ordinary work. Pending gates still need explicit decisions.
`/unelos off` disables only this session. Each session binds its own stable run ID;
independent runs do not isolate edits to shared source files.

Native tools: `unelos_next`, `unelos_start`, `unelos_resume`, `unelos_complete`, `unelos_approve`,
`unelos_reject`, `unelos_status`, `unelos_inspect`, `unelos_rename`.
Tools use the local `unelos mcp` server. The plugin replaces tool context with the
native session, canonical project and controller/worker role before dispatch.
Completion/decisions check full expected identity inside the
core runtime lock and return the resulting package. No shell-called JS helper,
status-then-next loop, worker tracking or per-model-request polling.

The package supplies `workerPrompt` unchanged and a mapped `nativeAgent`.
Workers use `subagent(agent, prompt)` and `sessionID`. Controller inspects reports/results,
relays questions and completes ordinary work. Native return alone is not success.

Parallel Nodes supply `workerPrompts[]`, each with its own mapped
`nativeAgent`. The controller starts every worker with `background=true` in one
turn, relays a worker's question through its native session, writes the
aggregate itself, then completes the parent with every worker's final frame.

Core validates result frames; gate decisions still require real human intent.
`mutation_uncertain` requires explicit run-list/state reconciliation, never retrying
start automatically. CLI run-specific commands require `--run RUN_ID`.

This is option A: native configuration applies settings, but the adapter does not
prove that the controller selected the right agent or forwarded the exact prompt.
Core guards progression; this is not a security sandbox. Only explicitly activated
main sessions can use controller tools; ordinary sessions produce no automatic reads.

Session bindings live in core, not OpenCode metadata. Existing metadata bindings
are ignored: use `/unelos resume` to rebind. Restart OpenCode after upgrading the
binary/plugin; running MCP processes do not hot-reload.

Historical evidence: [T15](../../docs/opencode-delivery-verification.md),
[T16](../../docs/opencode-execution-verification.md). Those reports describe earlier
architecture, not current acceptance. The thin-adapter verification report records
installed results separately; the user's interactive frontend run remains distinct.
