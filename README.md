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

Exit codes: `0` success, `2` invalid arguments/settings, `1` operational failure.

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

No Workflow validation CLI command exists yet. Terminal QA:

```sh
cargo test --locked -p unelos-core --test validation -- --nocapture
```

Expect ten passing checks: the T02 fixture and rework loops, missing
entry/targets, Task/Agent references, parallel workers, outcome maps, gate
routes, artifact producers/paths, unused definitions, and invalid Node fields.
For author-facing errors, call the core API and display the returned error;
the integration checks assert its source path, field, and cause.

### Workflow inspection (T05a)

`inspect` prints the selected Workflow as normalized JSON. It is read-only: it
does not start a run or resolve instructions or Skills. The example starts at
`plan` and has five Nodes and six edges: `plan` executes in the main process;
the human gate routes `approved` to `implement` and `rejected` back to `plan`;
`implement` and `review` execute as subagents; and `changes_requested` routes
from review back to implement. The graph ends at `done`.

From the repository root, build and run it either way:

```sh
cargo build -p unelos-cli --bin unelos
./target/debug/unelos --project examples/manual-project inspect implement-feature --json
cargo run -p unelos-cli --bin unelos -- --project examples/manual-project inspect implement-feature --json
```

Omit `--project` to inspect the current directory (run these from
`examples/manual-project`):

```sh
../../target/debug/unelos inspect implement-feature --json
cargo run -p unelos-cli --bin unelos -- inspect implement-feature --json
```

Pretty JSON is the default; omit `--json`:

```sh
./target/debug/unelos --project examples/manual-project inspect implement-feature
```

#### Manual QA: safe failure and read-only checks

Run from the repository root. Each check restores from the untouched copy
before mutating. Output and diagnostics are captured separately. Omitting
execution is valid and defaults the task to `main`; the invalid transition and
missing descriptor should return status `2`, empty stdout, and actionable stderr.

```sh
qa=$(mktemp -d /tmp/opencode/t05a-qa.XXXXXX)
mkdir "$qa/project"
cp -a examples/manual-project/. "$qa/project/"
cp -a examples/manual-project "$qa/untouched"

check_failure() {
  expected=$1
  set +e
  ./target/debug/unelos --project "$qa/project" inspect implement-feature --json >"$qa/stdout" 2>"$qa/stderr"
  result=$?
  set -e
  if ! test "$result" -eq 2 || ! test ! -s "$qa/stdout" || ! test -s "$qa/stderr" || ! grep -q "$expected" "$qa/stderr"; then return 1; fi
  printf 'status=%s stdout_bytes=%s\n' "$result" "$(wc -c <"$qa/stdout")"
  cat "$qa/stderr"
}

# Remove implement's execution setting; omitted execution defaults to main.
cp -a "$qa/untouched/." "$qa/project/"
sed -i '/^  implement:/,/^  review:/ { /    execution: subagent/d; }' "$qa/project/.unelos/workflows/implement-feature.yaml"
./target/debug/unelos --project "$qa/project" inspect implement-feature --json >"$qa/stdout" 2>"$qa/stderr"
test ! -s "$qa/stderr"
python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); assert next(n for n in d["nodes"] if n["id"] == "implement")["execution"] == "main"' "$qa/stdout"
printf 'omitted execution: status=0, stdout JSON defaults implement.execution to main\n'

# Unknown transition target: make implement point at a nonexistent Node.
cp -a "$qa/untouched/." "$qa/project/"
sed -i 's/    next: review/    next: missing-node/' "$qa/project/.unelos/workflows/implement-feature.yaml"
check_failure missing-node

# Missing project descriptor: rename it, then restore the exact name.
cp -a "$qa/untouched/." "$qa/project/"
mv "$qa/project/.unelos/project.yaml" "$qa/project/.unelos/project.yaml.disabled"
check_failure project.yaml
mv "$qa/project/.unelos/project.yaml.disabled" "$qa/project/.unelos/project.yaml"

# Repeated output is byte-identical and inspection did not alter/create files.
cp -a "$qa/untouched/." "$qa/project/"
./target/debug/unelos --project "$qa/project" inspect implement-feature --json >"$qa/first.json"
./target/debug/unelos --project "$qa/project" inspect implement-feature --json >"$qa/second.json"
cmp "$qa/first.json" "$qa/second.json"
diff -ru "$qa/untouched" "$qa/project"
```

The fixture source is `crates/core/tests/fixtures/project/.unelos`; the shipped
example parity test checks its parsed inspection output against the example.

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
