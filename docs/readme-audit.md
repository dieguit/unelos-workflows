# README audit

Goal: a new user following README.md gets the experience it describes, through the GUI.
Two questions for every step:

1. **True?** Does each README claim hold in the current app (packaged build, not just source)?
2. **Complete?** Does the README tell users everything they need for that step?

Rules:

- GUI first. Use the CLI only to observe what the GUI does (both call the same `unelos_core` functions).
- Evidence over code reading: run the packaged build, inspect real files, run real harness commands.
- Never touch the user's real `~/.config`, `~/.claude`, `opencode.json`. Use an isolated `HOME`
  (see Test rig). Live harness sessions in the real home only with user approval (they spend tokens).
- One verdict per claim: ✅ true, ⚠️ partial, ❌ false, ❓ unverifiable here (goes to the manual checklist).
- Fix only what the user approves; otherwise record it under Known issues.
- Go step by step; report each step in a short table before moving on.

## Test rig

- Build: `pnpm tauri build --bundles appimage,deb` (≈6 min). Outputs in `target/release/bundle/`.
  CLI from the package: `target/release/bundle/deb/Unelos_0.1.0_amd64/data/usr/bin/unelos`.
- Isolated home: `H=$PWD/target/qa/hN` and run with
  `env -u UNELOS_CONFIG_DIR -u XDG_CONFIG_HOME -u XDG_DATA_HOME HOME=$H …`.
  `/tmp/opencode` is not writable; use `target/qa/` or `/tmp/qa-*`.
- Desktop-launch PATH simulation: `env -i HOME=$H PATH=/usr/bin:/bin …`.
- OpenCode in an isolated HOME clashes with the user's running service port; set
  `opencode service set port 49399` under that HOME, and do not run it from inside the repo tree
  (it picks up the repo's `opencode.json`).
- GUI launch: `timeout 15 Unelos_0.1.0_amd64.AppImage` with isolated env; look for
  `main webview finished loading` in stdout.
- No WebDriver (`tauri-driver`, `WebKitWebDriver`) installed. GUI clicks are verified by reading the
  route code + calling the same core function via CLI, and listed in the manual checklist for the user.
  Installing `webkit2gtk-driver` + `cargo install tauri-driver` would allow real GUI automation; ask first.
- Scratch project used so far: `/tmp/qa-proj` (agents `coder`, `plain`; workflows `demo`, `demo2`).

## Done

### Step 1: Install from Releases

| Claim / need | Verdict |
|---|---|
| Releases exist | ❌ 0 releases, 0 tags; release workflow never ran; repo private (404 for others). User will tag `v0.1.0` and make public at the end. |
| Linux `.deb` / `.AppImage` build, launch | ✅ |
| CLI bundled | ✅ `/usr/bin/unelos` in deb; copied to `~/.config/unelos/bin/unelos` on app start |
| Windows portable had no CLI | Fixed: portable is now a zip with `unelos-app.exe` + `unelos.exe` |
| Naming ("unelos-app", "A Tauri App", `tauri-native`) | Fixed: productName `Unelos`, binary `unelos-app`, descriptions |
| macOS / Windows packages | ❓ manual |

README must add: Releases link, which asset per OS, Linux deps (`libwebkit2gtk-4.1-0`, `libgtk-3-0`),
Windows WebView2, macOS unsigned/unnotarized workaround (`xattr -dr com.apple.quarantine`), no auto-update
(re-download, then Harnesses → Update, restart harness), uninstall steps.

### Step 2: Configure harness(es)

| Claim / need | Verdict |
|---|---|
| Adds plugin + MCP to global config | ✅ Claude: plugin enabled, MCP connected. OpenCode: plugin in `opencode.json`, MCP connected (real home) |
| Creates default agents | ⚠️ only the 4 workflow-design subagents (+ lead), no generic starters |
| Creates model mappings | ⚠️ `smart/balanced/fast` seeded empty on first Model mappings visit |
| GUI launched from menu/Finder finds harness | Fixed: `src-tauri/src/shell_path.rs` loads login-shell PATH (before: Claude install failed "not on PATH", OpenCode model list empty) |

README must add: only OpenCode + Claude Code installable (10 others detect-only), harness must be installed
first, files written outside Unelos (`~/.claude/agents`, `~/.config/opencode/agents`, settings), bundled
workflow-design, restart harness after install, fill the `...` at README line 163.

### Step 3: Model aliases

| Claim / need | Verdict |
|---|---|
| Alias resolves per harness | ✅ |
| Codex example (luna/terra/sol) | ❌ Codex not installable |
| Aliases usable out of the box | ⚠️ empty; unmapped alias skips build |

Accepted for now (user): pinned-to-other-harness model errors on OpenCode but inherits on Claude
(contradicts `docs/harness-mcp-contract.md`); one bad agent skips that harness's whole build;
OpenCode model list silently empty when its service is unhealthy.

### Step 4: Agents

| Claim / need | Verdict |
|---|---|
| Create/edit/duplicate/remove agents in GUI | ✅ verified manually by user |
| "Agents define the LLM model" | ⚠️ only for `execution: subagent` steps (known issue 4) |

README must add: model/effort/MCPs/tools apply only to subagent steps; agent fields not editable in GUI.

### Step 5: Tasks and skills

Real run in `/tmp/qa-proj` (workflow `t5`: inline `plan` → reusable `review` with skill `checklist`, loop back).

| Claim / need | Verdict |
|---|---|
| Inline and reusable tasks | ✅ both run; package shows `taskDefinition` source |
| Inputs/outputs passed as artifact paths | ✅ |
| "Engine ensures agents produce artifacts" | ✅ missing output blocks `complete`; unchanged output on rework rejected; outcome required. Errors read "invalid persisted run state: …" (confusing wording) |
| Task skills | ✅ injected into the package (full SKILL.md content), not installed in the harness |
| Rename outcome / skill updates references (project) | ✅ code path `save_task_with_outcome_renames`, `save_skill_at` |
| Rename global skill | Fixed: now also updates global agents and global workflows |
| Remove global skill | Fixed: blocked while a global agent, task or workflow uses it |

README must add: task skills are injected per step, not installed; outcomes drive branching and the agent must pick one.

### Step 6: First workflow

Real runs in `/tmp/qa-proj` (`readme-ex`, `ex-a`, `parent`/`child`).

| Claim / need | Verdict |
|---|---|
| README example (Implement `in: plan.md, review.md`) | Fixed in README: Implement takes `plan.md` only; `review.md` arrives as feedback on rework |
| Same loop with Implement `in: plan.md` only | ✅ on rework `review.md` arrives as feedback artifact automatically. README example + sequence diagram (L61, L107) must change to this |
| "Sequence of tasks executed in order" | ⚠️ it's a graph: outcomes branch, loops allowed |
| Sub-workflows | ✅ parent → child → parent, artifacts mapped. Child sees the parent's artifact name (`q.md`), not its own (`question.md`) |
| GUI "Add step": task, human gate, sub-workflow | ✅ |
| GUI parallel step | ❌ "Coming soon": can't create; only edit existing ones from YAML |
| GUI End steps | ⚠️ only the default `end`; no way to add another (e.g. `abandoned`) |
| Stray folder in `.unelos/runs/` | ❌ blocks every `start` (`runtime_io_failed … runs/<dir>/state.json`) |

### Step 7: Parallel tasks and human gates

Real run `t7` in `/tmp/qa-proj`: parallel (2 workers) → gate → reject → rerun → approve → end.

| Claim / need | Verdict |
|---|---|
| Dispatches parallel tasks with subagents | ✅ package has one worker per task, each `execution: subagent`, own `result.md` |
| "Automatically gathers outputs in a summary" | ⚠️ the coordinator (main session model) writes the step's outputs from worker reports (`controller-parallel.md`); engine only enforces every worker result + aggregate exists |
| Gate pauses for approval | ✅ run status `waiting`, package lists artifacts; coordinator shows summary and asks (`controller-gate.md`) |
| Approve / reject | ✅ via chat reply → MCP; reject reason reaches the rework step and every worker as feedback |
| Approve/reject in GUI | ❌ none; Runs shows "Waiting" badge only (known issue 1) |
| Gate question text | ✅ `description` used if set; not editable in GUI (known issue 7) |

README must add: answer gates in the harness chat (any change request = reject with that reason); the
parallel summary is written by the coordinator; parallel steps can't be created in the GUI yet
(`docs/handoff-parallel-gui.md`).

### Step 8: Project vs global

| Claim / need | Verdict |
|---|---|
| Project vs global storage, "Make global" | ✅ verified manually by user |

Still to reflect in README (from code, known issue 8): "Make global" exists for tasks/skills/MCPs only;
only global agents are rendered to global harness dirs.

### Step 9: Import

| Claim / need | Verdict |
|---|---|
| Import (local, GitHub/GitLab, git URL, conflicts, MCP trust, warnings, examples) | ✅ verified manually by user |

Still to reflect in README: import doesn't rebuild harness agents (build after import); examples need aliases mapped.

### Step 10: Workflow-design tool

| Claim / need | Verdict |
|---|---|
| Discoverable, launchable, works | ✅ verified manually by user |

Still to reflect in README: Python 3 requirement.

## Remaining plan

Each session: resume from `.slim/deepwork/readme-audit/progress.md`, do the listed steps, append results
here under Done, update Known issues.

### Session 2: Authoring in the GUI (README "Getting started" 4–5, "Concepts")

Step 4: Agents (README L121–126, L167)
- Create, edit, duplicate, remove an agent (project + global) via the GUI code path; confirm YAML written
  (`.unelos/agents/<id>/agent.yaml`) and native files after auto-build.
- Fields: instructions, skills, MCPs, model, effort editable; `tools` and `instructionsFile` not (known).
  Verify claim "agents define the LLM model": only true for `execution: subagent` steps; main steps ignore
  model/effort/MCPs/tools (`crates/core/src/workflow_document.rs:95-113`). Check what the GUI tells the user.
- Global agent may only use global skills/MCPs: confirm enforcement and message.

Step 5: Tasks and skills (README L128–136)
- Inline vs reusable task; task skills; outcomes; rename outcome updates workflows.
- Claim "engine will generate prompts to ensure agents produce the expected artifacts": inspect worker
  prompt in a real package (`unelos next --json` on a started run in the scratch project).
- Skills: confirm they are injected into the package, not installed in the harness (README implies install
  at L54 "skills"). Check skill rename updates references.

Step 6: First workflow (README L115–119, L169)
- Build the README's own example (Plan → Implement ⇄ Review) in the GUI path. Expect failure: Implement needs
  `review.md` before Review ran (`runtime/artifacts.rs:155-190`). Confirm with a real run via CLI.
- Node types reachable in GUI: task, parallel, human gate, sub-workflow, end. Human-gate description not
  editable (known). Validation messages shown.
- "Sequence of tasks executed in order" (L117) vs graph with branches/loops.
- Composable sub-workflows (L119): create parent calling child, run it.

Step 7: Parallel tasks and human gates (README L138–144)
- Parallel: who writes the summary (coordinator model, `controller-parallel.md`), "automatically gathers"
  claim. Fixed worker list only.
- Gate: approve/reject via MCP; no GUI approve button; what the user sees.

Step 8: Project vs global (README L146–151)
- "Make global" button: exists for tasks/skills/MCPs only; not workflows/agents. Test it moves dependencies.
- Storage: `.unelos/`, `~/.config/unelos/`; "compiled to global harness settings": only global agents are
  rendered to global harness dirs; global skills/MCPs are not. Check `.unelos/runs/` sharing/gitignore advice.

### Session 3: Importing and workflow-design (README L170–171, L185–187)

Step 9: Import
- Local folder (absolute, `~`, Browse), GitHub/GitLab folder link, plain git URL; private repo; no git
  installed. Use `examples/superpowers-dev` and `examples/council`.
- One-workflow-per-folder rule, conflict rename, MCP trust checkbox, warnings count.
- Import does not trigger harness build (user decided: keep). Confirm what the user must do next, and that the
  imported workflow's subagents are missing in the harness until a build.
- Imported examples need aliases (`council` uses all three) → build skipped. Document.

Step 10: Workflow-design tool
- Is it discoverable in the GUI (Global workflows)? How to launch it (`/unelos` → pick it).
- Session mining reads OpenCode SQLite and Claude JSONL; consent step; Python 3 required (`digest-sessions.py`)
  — README never says Python is needed. Verify on this machine with isolated copies, not real transcripts,
  unless user approves.

### Session 4: Running a workflow live (README L173–183, "How it works" L21–109)

Needs user approval: real harness sessions spend tokens. Use a scratch project in `/tmp`, real home.

Step 11: `/unelos` flows, per harness (OpenCode `/unelos`, `/unelos resume`, `/unelos off`;
Claude `/unelos`, `/unelos-resume`, `/unelos-off`)
- `/unelos` alone lists workflows (numbered, plus "continue without Unelos").
- `/unelos I want to…` picks a matching workflow; one clear match may start immediately.
- Inline workflows (L182): chain of existing workflows offered for approval.
- Execution package contents vs README L35 (task, agent, skills, MCPs, inputs, outputs).
- Output validation: missing output blocks `complete`; unchanged output on rework is rejected.
- Runs screen shows the live run (2 s polling), artifacts viewer; no start/approve/stop in GUI.
- Resume after closing the harness; `mutation_uncertain` recovery.

Step 12: "How it works" accuracy
- Diagram: `unelos install` arrow is wrong (GUI auto-build / Project settings → Build now; `unelos install`
  imports a workflow). Sequence diagram lacks `start`; `complete` returns next package.
- Determinism/cost claims (L37): routing deterministic; dispatch, summaries, gate interpretation are model-led.
- Agent MCPs reach subagents only; main-session agents' MCPs are dropped (`install.rs:123-135`).

### Session 5: Contributing, cross-platform, README rewrite

Step 13: CONTRIBUTING.md: follow it on a clean checkout (prereqs, `pnpm tauri dev`, tests).
Step 14: Manual checklist for the user: macOS dmg (both arches), Windows installer + portable zip, WSL note
(L158), GUI clicks not automated.
Step 15: Rewrite README from the Done tables: install per OS, supported harnesses, quickstart
(install harness → map aliases → import `examples/superpowers-dev` → restart harness → `/unelos`),
GUI screen tour incl. Runs, definitions reference, data locations, commands per harness, troubleshooting,
typos (L37, L117, L125, L144, L151, L158, L161, L180, L182, L187, "GitHub/GitLab").
Step 16: Release: user tags `v0.1.0`, publishes draft, makes repo public; then verify downloads unauthenticated.

## Known issues (ranked)

1. No GUI way to start/approve/resume/stop runs; Runs empty state doesn't say where runs start.
2. No onboarding: nothing guides harness → aliases → workflow → `/unelos`.
3. Aliases seeded empty; unmapped alias skips the whole harness build (toast only).
4. `execution: main` silently ignores agent model/effort/MCPs/tools; GUI doesn't explain.
5. One bad agent blocks all native agents for that harness (accepted for now).
6. OpenCode errors on Claude-pinned models; Claude inherits (accepted for now; doc mismatch).
7. GUI can't edit agent `tools`/`instructionsFile`, task `instructionsFile`, gate description; no raw YAML.
8. "Make global" missing for workflows and agents.
9. No example gallery; workflow-design not surfaced in GUI.
10. MCP command field splits on whitespace (no quoted args).
11. Harness "Update" is integration update; no app update check.
12. `integrations/opencode/README.md:33-37` says generated files are overwritten; builder now skips edited ones.
13. Old OpenCode verification docs in `docs/` describe the retired JS architecture.
14. Release is a draft (`releaseDraft: true`); macOS ad-hoc signed only.
