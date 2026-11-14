# Harness MCP contract

Shared contract for the Unelos controller MCP server and harness adapters (OpenCode, Claude Code).
"One server" = one implementation/entrypoint; each harness spawns its own process.

## Controller MCP server

- Entry: `<unelos-bin> mcp` (stdio, rmcp). MCP server name: `unelos` (reserved; workflow MCP ids `unelos` are rejected).
- Exposed names: OpenCode `unelos_<tool>`; Claude Code `mcp__plugin_unelos_unelos__<tool>`.
- Tools (names, descriptions, argument schemas exactly as `integrations/opencode/index.ts` `toolSchemas`/`descriptions` today):
  `next, start, resume, complete, approve, reject, status, inspect, rename`.
  Each schema additionally allows optional property `unelos_context` (advertised optional; mandatory at dispatch).
- Stateless per request: reload binding + run state every call; no cached session/run, no cwd changes. Blocking fs/lock work off the async executor.
- Success: one text content item = JSON of the CLI-equivalent result plus adapter fields
  (`nativeAgent`, `workerPrompts[i].nativeAgent` = `unelos-<agent>`, `harnessInstructions` per harness).
- Domain failure: `isError: true`, text `{"error":{"code","message"}}`, preserving `ExecutionError::code()` and `mutation_uncertain`.
  JSON-RPC errors only for protocol failures. Once a mutation starts, cancellation is not rollback.
- Binary upgrades require restarting the harness/MCP process (documented, no hot reload).

## `unelos_context` (injected by harness hooks, never trusted from the model)

```json
{"harness":"claude|opencode","sessionId":"<1..256 chars>","project":"<absolute path>","role":"controller|worker"}
```

- Hooks REPLACE the whole key (never merge model-supplied values).
- Missing/malformed → `context_missing`. Unsupported harness → `context_invalid`. `role: worker` → `child_controller`.
- `project` is canonicalized (realpath) and must equal the binding's project, else `project_changed`.
- OpenCode: if `_meta["ai.opencode/sessionID"]` is present it must equal `sessionId`, else `context_invalid`.
- Claude: `role = worker` iff PreToolUse input has `agent_id`. OpenCode: `role = worker` iff `session.get(sessionID).parentID != null`.
- Threat model: prevents accidental misrouting inside trusted local harness execution; not authentication against hostile same-user processes.

## Session binding (core)

- Store: `<unelos config dir>/sessions/<harness>/<sha256(sessionId) hex>.json`, lock `<same>.lock` (std `File::lock`, never deleted/replaced).
- Record: `{"version":1,"harness","sessionId","project","enabled":true,"mode":"new|resume","entryId":"<uuid>","runId":null|"<id>","pendingStart":null|{"entryId","at"}}`. Unknown version → `binding_version_unsupported`.
- Lock order: session lock → project runtime lock. Never take the runtime lock externally and then call `execution_start()` (it locks itself).
- Guards (ported from `bridge()` in `integrations/opencode/index.ts`): `controller_inactive`, `child_controller`, `project_changed`, `entry_superseded`, `invalid_entry`, `run_unbound`, `stale_execution`.
- start: under session lock write `pendingStart`, call start, then write `runId` and clear pending. Leftover `pendingStart` → `mutation_uncertain` ("explicitly reconcile with /unelos resume; never retry start") on every tool until activation (`new` or `resume`) clears it. Never auto-retry.
- Two controller sessions may select the same run; execution identity guards stale advancement, not ownership.
- Legacy OpenCode session metadata is ignored; users rebind with `/unelos resume`.
- No TTL cleanup. `off` deletes the record under lock. Never delete runs.

## Session CLI

- `unelos --project <dir> session activate --harness <id> --session <id> [--mode new|resume|off] --json`
  - new → `execution new` catalog; resume → `execution runs` list; both plus `requestedAction`, adapter fields (`harnessInstructions`).
  - off → `{"disabled":true,"message":"Unelos disabled for this session."}`.
  - `--request <text>` (exclusive with `--mode`): raw user args; exactly `off` → off, exactly `resume` → resume, else new; echoed as `requestedAction`. `--project` defaults to cwd.
  - Errors: existing stderr `{"error":{code,message}}` + non-zero exit.
- `unelos hook claude pre-tool-use` — stdin Claude hook JSON → stdout
  `{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow","updatedInput":{...tool_input,"unelos_context":{...}}}}`.
  project = `CLAUDE_PROJECT_DIR` env, else hook `cwd`. Only for `mcp__plugin_unelos_unelos__*`.
- `unelos hook claude session-start` — read-only. source `compact`/`resume` with an enabled binding → `additionalContext` (bound run, call `next` to recover). Otherwise empty output, exit 0. Never activates/mutates.

## Adapter (core `harness`)

- `AgentExport.model` stays unresolved (`Option<AgentModel>`); each adapter resolves. Unmapped alias for one harness → that harness's build reports it in `skipped` and keeps existing generated files; other harnesses still build.
- `build_harness_config` iterates installed adapters. Each adapter owns agent rendering, workflow MCP projection, stale `unelos-*` deletion, collisions.
- Claude project build: `.claude/agents/unelos-<id>.md`, frontmatter `name`, `description`, optional `model`, `mcpServers` as LIST of inline `{<id>: <config>}` (needs folder trust).
- Claude model: `{harness: claude, model}` → model; alias → mapping `claude`; else omitted (inherit). `list_models`: `opus`, `sonnet`, `haiku`.
- Claude templates separate executable-path contexts: `{{UNELOS_BIN_JSON}}` = JSON-string-escaped raw absolute provisioned path (MCP exec); `{{UNELOS_BIN_SHELL}}` = bare path only for `^[A-Za-z0-9_./+-]+$`, otherwise single-quoted with each `'` escaped as `'\''`; `{{UNELOS_BIN_SHELL_JSON}}` = JSON-string-escaped shell form (hook command). Skills use the identical shell form in the command and `Bash(<path> *)` allowed-tools prefix.

## Claude plugin assets (`integrations/claude/`)

```
integrations/claude/.claude-plugin/marketplace.json   # marketplace "unelos-local", plugin "unelos" at ./unelos
integrations/claude/unelos/.claude-plugin/plugin.json
integrations/claude/unelos/.mcp.json                  # command uses {{UNELOS_BIN_JSON}}
integrations/claude/unelos/hooks/hooks.json           # PreToolUse matcher mcp__plugin_unelos_unelos__.* ; SessionStart
integrations/claude/unelos/skills/unelos/SKILL.md     # fixed --mode new
integrations/claude/unelos/skills/unelos-resume/SKILL.md # fixed --mode resume
integrations/claude/unelos/skills/unelos-off/SKILL.md  # fixed --mode off
integrations/claude/README.md
```

Install: copy rendered assets to `<unelos config>/integrations/claude/`, `claude plugin marketplace add <dir>`, `claude plugin install unelos@unelos-local --scope user`.
Reinstall: `claude plugin marketplace update unelos-local`, then `claude plugin update unelos@unelos-local --scope user`. Plugin manifest `{{UNELOS_PLUGIN_VERSION}}` renders as `<INTEGRATION_VERSION>-<8hex SHA256 of rendered assets before version replacement>` so asset changes invalidate Claude's plugin cache.

Activation: `/unelos`, `/unelos-resume`, `/unelos-off` (namespace `unelos:` if ambiguous). Each shell injection passes its fixed `--mode` and never interpolates user text. Only the `/unelos` Markdown body includes `User request: $ARGUMENTS`; user arguments are never shell source or `--request` input.

## OpenCode plugin (`integrations/opencode/`)

- Registers controller MCP via `ctx.mcp.transform(e => e.set('unelos', {type:'local', command:[binary,'mcp']}))`.
- `ctx.tool.hook('execute.before')` for `unelos_*` tools: replace `input.unelos_context`.
- `/unelos` command: CLI `session activate --harness opencode` then `ctx.session.prompt`.
- Native tool registration removed.
