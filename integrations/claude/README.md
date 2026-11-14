# Unelos for Claude Code

Install the CLI, then run `unelos harness install claude`. This installs the local
`unelos-local` marketplace and user-scoped `unelos` plugin, using a stable CLI copy.
Restart Claude Code after installation or upgrade: running MCP processes do not
hot-reload the binary.

Use `/unelos` for a fresh run, `/unelos-resume` to explicitly select an unfinished
run, and `/unelos-off` to disable only this session (prefix with `unelos:` if
another plugin defines the same name). Free text after `/unelos` is delivered to
the model as a request, never interpolated into a shell command. Follow the returned controller
and harness instructions; workers cannot operate controller tools.

SessionStart recovers enabled sessions after compaction or resume without
activating or mutating them. Call the controller's `next` tool to recover the
current execution package. If no binding exists, activate explicitly.

Trust the project folder in Claude Code: generated agents' inline, agent-scoped
MCP servers need folder trust. Review third-party server commands before trusting.
The skill pre-approves only its installed CLI executable for activation.

Activation skills run fixed `--mode new|resume|off` commands. No user text enters
shell source. Executable paths are shell-quoted in skills/hooks and JSON-escaped
without shell quoting in MCP configuration. Activation uses the CLI's default
project (cwd).

Reinstall refreshes the marketplace and installed plugin cache. The rendered
plugin version includes an asset content hash so changed assets trigger updates.
