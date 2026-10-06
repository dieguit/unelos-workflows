# Workflow inspection (T05a)

`inspect` prints the selected Workflow as normalized JSON. It is read-only: it
does not start a run or resolve instructions or Skills. This example starts at
`plan` and has five Nodes and six edges: `plan` executes in the main process;
the human gate routes `approved` to `implement` and `rejected` back to `plan`;
`implement` and `review` execute as subagents; and `changes_requested` routes
from review back to implement. The graph ends at `done`.

Run the following commands from the repository root:

```sh
cargo build -p unelos-cli --bin unelos
./target/debug/unelos --project examples/manual-project inspect implement-feature --json
cargo run -p unelos-cli --bin unelos -- --project examples/manual-project inspect implement-feature --json
```

Omit `--project` to inspect the current directory; run these from
`examples/manual-project`:

```sh
../../target/debug/unelos inspect implement-feature --json
cargo run -p unelos-cli --bin unelos -- inspect implement-feature --json
```

Pretty JSON is the default; omit `--json` (run from the repository root):

```sh
./target/debug/unelos --project examples/manual-project inspect implement-feature
```

## Manual QA: safe failures and read-only checks

Run from the repository root. Each check restores from the untouched copy
before mutating. Output, diagnostics, and exit status are captured separately.
Omitting execution is valid and defaults the task to `main`; the invalid
transition and missing descriptor should return status `2`, empty stdout, and
actionable stderr.

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
set +e
./target/debug/unelos --project "$qa/project" inspect implement-feature --json >"$qa/stdout" 2>"$qa/stderr"
result=$?
set -e
if ! test "$result" -eq 0 || test -s "$qa/stderr"; then exit 1; fi
python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); assert next(n for n in d["nodes"] if n["id"] == "implement")["execution"] == "main"' "$qa/stdout"
printf 'omitted execution: status=%s stdout_bytes=%s stderr_bytes=%s (defaults to main)\n' "$result" "$(wc -c <"$qa/stdout")" "$(wc -c <"$qa/stderr")"

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

The fixture source is `crates/core/tests/fixtures/project/.unelos`; the CLI
parity test compares its parsed result with this shipped example.
