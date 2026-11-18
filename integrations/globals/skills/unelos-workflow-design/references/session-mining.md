# Mining past sessions

Session history shows how work really happens: which steps the user drives,
where agents fail, and what the user keeps repeating.

## Where history lives

- **OpenCode**: `~/.local/share/opencode/opencode.db` (SQLite). Newer versions
  use tables `session_v2` + `session_message`; older ones `session` + `message`.
  Sessions link to a project through `project.worktree` or `session.directory`.
  Child sessions (`parent_id` set) are subagent runs.
- **Claude Code**: `~/.claude/projects/<escaped-path>/*.jsonl`, one JSON event
  per line (`type: user|assistant`, `message.content`).

Read history only for the project the user asked about, and only with their
consent; transcripts can contain secrets.

## Digest, then analyze

Raw transcripts are too large to read. `scripts/digest-sessions.py` (Python, stdlib only)
writes one short markdown file per root session for OpenCode: title, subagent
titles, user messages, truncated assistant text, compaction summaries and tool
counts.

```sh
python3 scripts/digest-sessions.py <project-path-substring> <out-dir>
```

For Claude Code, do the same by hand: keep user text, truncate assistant text,
count tool uses.

Then split the digests into batches of ~15 and give each batch to a subagent
with this brief: for every session record goal, kind (bug, feature, design,
planning, research, refactor, ops), success, frictions, instructions the user
repeated, the step sequence the user drove, and agents used; write the results
as JSON; return the top repeated patterns with counts and session IDs.

## What to look for

- **Shapes**: the same step sequence across sessions (e.g. questions → spec →
  plan → implement → review → PR → CI fix). Each is a workflow candidate.
- **Skipped steps**: "done" claimed without a running-app check; CI failures
  the user reports; bugs the user finds right after. These become required
  tasks with evidence.
- **Repeated instructions**: rules the user restates ("no worktrees", "reuse
  the existing forms", "ask before planning"). These go into agent instructions
  or a project skill.
- **Environment friction**: missing MCP connections, stale installs, ports in
  use. These become `blocked` outcomes and setup checks.
- **Rework rounds**: how many times the user sent work back, and why.

Report counts honestly and say what the digests could not show (e.g. tool error
text is often missing).
