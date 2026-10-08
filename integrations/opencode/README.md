# Unelos OpenCode delivery

Opt-in native plugin for V1 ≥1.18.29 and V2. See
[`docs/opencode-delivery-verification.md`](../../docs/opencode-delivery-verification.md)
for tested versions and remaining limits. No automatic workflow runner.

## Install

Build the CLI (`cargo build --locked -p unelos-cli`) and put `target/debug/unelos`
on PATH, or supply its absolute path as `binary`. Node stdlib only; no runtime
dependencies. From this directory run `npm pack`, then extract the archive into
a dedicated installation directory outside `.opencode/plugins` (to prevent
automatic discovery of supporting modules). Keep all packaged files together.

Merge into project `opencode.json(c)` for project installation, or
`~/.config/opencode/opencode.json(c)` for global installation. Preserve existing
settings/plugins. Do not install a second plugin/command with the same name.

V2:
```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "plugins": [{"package":"file:///absolute/path/to/package/index.js",
    "options":{"binary":"/absolute/path/to/unelos"}}]
}
```

V1:
```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "plugin": [["file:///absolute/path/to/package/index.js",
    {"binary":"/absolute/path/to/unelos"}]]
}
```
V1 also needs the packaged `commands/unelos.md` copied to
`.opencode/commands/unelos.md` (project) or
`~/.config/opencode/commands/unelos.md` (global). Check for an existing file first;
never overwrite it. V2 registers its command natively, with no Markdown copy.
Default `binary` is `unelos`; it is executed directly, never interpreted by a shell.
Restart/reload your native project's plugins after configuration changes.

## Use

- `/unelos`: activate this controller; offer Workflow choices if idle, continue
  existing work if running/waiting, report finished if finished.
- `/unelos start`: offer choices, including after a finished run.
- `/unelos start WORKFLOW`: explicitly select a Workflow; an existing unfinished
  run is continued, never replaced.
- `/unelos off`: Continue without Unelos; clear only this session's activation.
  The run and unrelated native metadata stay unchanged.

Supply your goal in ordinary conversation. Controller starts with a descriptive
name and records the agreed goal/constraints under `## Brief` in the supplied
`ledger.md`. CLI request text remains unchanged. On recovery read fresh `next`,
then ledger/reports; missing brief means ask, not invent. Only the controller
writes the ledger; decisions needed downstream belong in declared reports.
Native activation persists across restart. Fresh sessions require `/unelos`.
Moved sessions require explicit reactivation in the new project. Children never
inherit controller bootstrap.

## Delivery and limits

Exact selected Task/Agent instructions, ordered Skill bodies/directories,
identities and artifact paths are delivered before main work. Skills are already
loaded; do not resolve ambient names again. Read referenced resources using native
tools, respecting permission prompts. Existing output files are updated in place.
Only the owned current block is replaced; historical tool results remain.

Configured main models are applied at native prompt admission and verified at
request time. A model change during a tool loop can require a fresh prompt; never
rewrite global defaults. No configured model means preserve native selection.
Authored variants require an actual native variant inventory; V1 may lack one.

Tools are **required availability**, not an allowlist. `read-file` maps to `read`;
`run-command` maps to V1 `bash` / V2 `shell`. V2 uses effective request tools.
V1 nonempty required-tool lists fail closed (`effective_tools_unverified`), since
effective inspection is unverified. Resource permission prompts are never bypassed.
All nonempty MCP requirements currently block (`unsupported_mcp`). Failures do not
advance core state or inject a cached packet. V1 textless recovery requires an
ordinary continuation message.

T15 provides deterministic worker preparation through `unelos-opencode/packet`:
`executorPacket(packet, workerKey?)`, `executorText(packet, brief, workerKey?)`,
and `requireConfiguration(packet, evidence)`. Workers exclude controller metadata.
The native consumer must set/verify that child's model and requirements using its
own request evidence, not the parent's inventory. Production dispatch, worker
continuation/question relay and completion coordination are T16, not this plugin.
Do not reconstruct/paraphrase worker packets manually.

CLI mutations remain Node-only: packet IDs do not add stale-callback protection.
Never blindly retry mutations or infer gate consent from ledger/worker results.

Run package tests: `npm test`.
