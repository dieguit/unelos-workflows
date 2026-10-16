---
name: backend-research
description: Trace server-side behavior from entry points to data and report findings with file references.
---
# Backend research

1. Find entry points (commands, handlers, CLI subcommands) relevant to the question.
2. Follow calls into domain logic and persistence; note types and error paths.
3. Report: `## Findings` (each with `path:line`), `## Open questions`, `## Risks`.
