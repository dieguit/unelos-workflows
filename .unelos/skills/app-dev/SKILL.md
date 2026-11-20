---
name: app-dev
description: Run, inspect and check the unelos-app desktop UI (Vite + React + Tauri) in a browser for QA.
---

# unelos-app for QA

## Run the UI

- `pnpm dev` serves the frontend at `http://localhost:3000`. Reuse it if the port already answers; never start a second one.
- In a plain browser the Rust backend is absent. `src/lib/dev-ipc-mock.ts` answers every Tauri command with fixture data. Unknown commands throw `dev mock: unhandled command <name>`: report that as `blocked` (mock missing), not as an app bug.
- `pnpm tauri dev` runs the real backend, but Playwright cannot drive the Tauri webview. Browser QA always runs against the mock.

## Mock modes

Add `?mock=<mode>` to any URL:

| mode | effect |
|---|---|
| (none) | normal fixture project |
| `empty` | project with no workflows/agents/skills/tasks |
| `none` | no recent projects |
| `unavailable` | recent project folder missing |
| `error` | project and harness reads fail |
| `save-error` | saves fail (read-only project) |
| `switch` | slow project switch, prefixed names |
| `update` | OpenCode integration update available |

`?pick=cancel|plain` controls the Open folder dialog. Mock state lives in memory: reload resets it.

## Routes

Project: `/` (home), `/workflows`, `/agents`, `/skills`, `/tasks`, `/mcps`, `/runs`, `/runs/<runId>`, `/settings`.
Global: `/global-skills`, `/global-tasks`, `/global-mcps`, `/model-mappings`, `/harnesses`.

Screens sharing a component should behave the same: lists (workflows, agents, skills, tasks, MCPs) and right-side inspectors/editors. When checking one, check its siblings.

## Shared building blocks

Reuse before building new (all under `src/components`):

- Lists: `list-table`, `list-search`, `list-actions`, `empty-state`, `page-header`
- Inspectors and editors: `skill-inspector`, `task-inspector`, `markdown-editor`, `body-editor`, `step-panel/`
- Pickers: `model-picker`, `effort-picker`, `skill-picker`, `picker-status`, `used-by`
- Forms: `ui/field` with TanStack Form + zod, as in `src/routes/agents/-hooks/use-agent-form.ts` and `src/routes/model-mappings/-hooks/use-mapping-form.ts`
- Primitives: `ui/` (shadcn)

## Design

The Pencil file is `docs/ignored/design.pen`. Frames are named like `Skills list — code-review inspector open`. The Pencil MCP only works while the Pencil desktop app is open; if it is not reachable, say so.

## Evidence

Save screenshots under the run's output folder next to the report and reference them by relative path. Record the URL (with mock mode), viewport, and browser console errors for every check.
