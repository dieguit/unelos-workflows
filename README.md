# React + TypeScript + Vite + shadcn/ui

This is a template for a new Vite project with React, TypeScript, and shadcn/ui.

## Adding components

To add components to your app, run the following command:

```bash
npx shadcn@latest add button
```

This will place the ui components in the `src/components` directory.

## Using components

To use the components in your app, import them as follows:

```tsx
import { Button } from "@/components/ui/button"
```

## Rust workspace and CLI

`crates/core` owns shared application logic. `crates/cli` owns the `unelos`
executable; `src-tauri` owns the desktop adapter. Neither adapter depends on the
other. Default Cargo commands build/test core and CLI without desktop dependencies.

```sh
cargo run -p unelos-cli -- --help
cargo run -p unelos-cli -- config show
cargo run -p unelos-cli -- --config settings.toml --json config show
UNELOS_LOG_LEVEL=info cargo run -p unelos-cli -- config show
cargo run -p unelos-cli -- config show --log-level debug
cargo run -p unelos-cli -- --project . start
cargo run -p unelos-cli -- --project . start implement-feature --name fix-button
cargo run -p unelos-cli -- --project . status
cargo run -p unelos-cli -- --project . next
cargo run -p unelos-cli -- --project . complete plan
cargo run -p unelos-cli -- --project . approve approve-plan
cargo run -p unelos-cli -- --project . reject approve-plan --reason 'Revise scope.'
cargo run -p unelos-cli -- --project . rename fix-save-button --run RUN_ID
```

Optional `settings.toml`:

```toml
log_level = "warn"
```

Settings precedence: defaults → explicit TOML file → `UNELOS_*` environment →
explicit CLI flags. `log_level` defaults to `warn`; allowed values are `off`,
`error`, `warn`, `info`, `debug`, and `trace`. `UNELOS_LOG_LEVEL` and `--log-level`
override it. Unknown settings are rejected. No config file is discovered
automatically; `--config` must name an existing UTF-8 TOML file exactly. Inspection
contains only safe settings; diagnostics never render raw config source contents.

Successful results go to stdout. Diagnostics and operational events go to stderr.
`--json` emits a settings object on stdout, application errors as
`{"error":{"code":"invalid_settings","message":"..."}}` on stderr, and
operational events as JSON lines. Application errors contain no ANSI styling.
Clap-generated help, version, and argument/usage errors retain standard text even
with `--json`. Broken output pipes terminate quietly.

Exit codes: `0` success, `2` usage/domain rejection (including invalid settings
or runtime state), `1` operational failure.

### Runtime CLI (T12)

The seven runtime commands are `start [WORKFLOW] [--name NAME]`, `status`,
`next`, `complete NODE [--outcome OUTCOME]`, `approve NODE`,
`reject NODE --reason REASON`, and `rename NAME --run RUN_ID`. `start` without
a Workflow lists sorted choices without creating a run; with a Workflow it
creates a run and returns its summary. `status` reports idle when no run exists;
`next` returns the current packet using the current authoring files: Definitions
and Skills are not snapshotted. CLI start persists empty request text; the
controller supplies conversation context and records required answers in reports.
Persisted progression alone does not preserve unsaved conversation context.
`complete NODE` explicitly confirms existing declared output files for current
work and advances after validation. Unchanged reviewed outputs are allowed; the
CLI does not automatically check content/mtime freshness or report quality.
Node-only mutations cannot identify a delayed callback to a later visit to the
same Node; inspect `status`/`next` after an uncertain result, never blindly retry.

See [coordinator instructions](docs/coordinator-instructions.md) and the
[manual runtime walkthrough](docs/manual-runtime-cli-qa.md). Instructions are
not automatically injected and no plugin or slash command is installed.

### Local Skill resolution (T03)

`unelos_core::resolve_skills(root, ordered_ids)` reads only selected local
`.unelos/skills/<id>/SKILL.md` files, deduplicating IDs in first-occurrence order.
Each result contains its ID, absolute directory, and unchanged UTF-8 content.
Resources remain available through that directory; they are not injected.
IDs must be nonempty, not `.` or `..`, and contain no `/`, `\`, `:`, or NUL.

No CLI Skill command exists yet. Terminal QA exercises real temporary files:

```sh
cargo test --locked -p unelos-core --test skills -- --nocapture
```

Expect five passing checks covering order/content/resources, missing files,
unreadable files, invalid IDs, and working-directory-independent roots.

### Workflow validation (T04)

`unelos_core::validate_project(&loaded_project)` checks only the selected
Workflow and returns a sorted map of logical output names to producer Node IDs.
Call `load_project(root, workflow_name)` first. Errors include the Workflow
source path, offending field/reference, and cause. Validation checks references,
transitions, outcome contracts, and artifacts; loops are valid. It does not read
Skills, connect MCPs, execute work, or check whether an input is completed yet.

Artifact names use relative forward-slash paths (e.g. `reports/plan.md`), with
nonempty components and no `.`, `..`, backslash, colon, or NUL. Each logical
output has one producing Node; every input must name a declared output.
Gates have exactly `approved` and `rejected` routes. Task outcome-map keys must
match the referenced definition's outcomes; definitions declaring outcomes
cannot use a single-successor transition.

There is no standalone validation-only CLI command: `inspect` validates the
selected Workflow before printing its normalized graph and reports authoring
diagnostics. See the [manual inspection walkthrough](examples/manual-project/README.md)
for the example and safe QA steps. Core validation terminal QA:

```sh
cargo test --locked -p unelos-core --test validation -- --nocapture
```

Expect ten passing checks: the T02 fixture and rework loops, missing
entry/targets, Task/Agent references, parallel workers, outcome maps, gate
routes, artifact producers/paths, unused definitions, and invalid Node fields.
The core API also returns these errors to direct callers; integration checks
assert their source path, field, and cause.

### Rust checks

```sh
cargo install cargo-nextest --locked
cargo fmt --all -- --check
cargo clippy --locked -p unelos-core -p unelos-cli --all-targets -- -D warnings
cargo nextest run --locked -p unelos-core -p unelos-cli
cargo test --locked -p unelos-core --doc
```

For the desktop adapter, use `bun run tauri dev` or `bun run tauri build` with the
platform's Tauri prerequisites installed. Rust artifacts and `Cargo.lock` now
live at the workspace root. `reqwest` is reserved in workspace dependencies for
shared HTTP operations when a real command needs them; no HTTP wrapper or
placeholder operation is included.
