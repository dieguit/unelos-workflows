# Workflow and Task inspection (T05a, T06)

Without `--node`, `inspect` prints the selected Workflow as normalized JSON;
it does not resolve instructions or Skills. With `--node`, it resolves only
that ordinary Task's instructions, selected Skills, and opaque configuration
references, without starting a run or creating runtime identities or paths.
This example starts at `plan` and has five Nodes and six edges: `plan` executes
in the main process;
the human gate routes `approved` to `implement` and `rejected` back to `plan`;
`implement` and `review` execute as subagents; and `changes_requested` routes
from review back to implement. The graph ends at `done`.

Run the following commands from the repository root:

```sh
cargo build -p unelos-cli --bin unelos
./target/debug/unelos --project examples/manual-project inspect implement-feature --json
./target/debug/unelos --project examples/manual-project inspect implement-feature --node implement --json
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

## Manual QA: T06 resolved Task context

Run from the repository root after building. All edits below are confined to
the temporary project copy. The first check verifies baseline fields, no
runtime-only IDs, deterministic bytes, and an untouched tree. The modified
copy adds `verification` after `rust` on the Agent; resolution must preserve
Task-then-Agent order and deduplicate the repeated Skill. Request and feedback
are tested in core; a later runtime caller supplies run/execution-scoped paths.

```sh
qa=$(mktemp -d /tmp/opencode/t06-qa.XXXXXX)
mkdir "$qa/project"
cp -a examples/manual-project/. "$qa/project/"
cp -a examples/manual-project "$qa/untouched"

./target/debug/unelos --project "$qa/project" inspect implement-feature --node implement --json >"$qa/baseline.json"
./target/debug/unelos --project "$qa/project" inspect implement-feature --node implement --json >"$qa/repeat.json"
cmp "$qa/baseline.json" "$qa/repeat.json"
python3 - "$qa/baseline.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
assert d["kind"] == "task" and d["nodeId"] == "implement"
assert d["execution"] == "subagent" and d["inputs"] == ["plan.md"]
assert d["outputs"] == ["implementation-report.md"]
assert d["model"] == "example-provider/example-model"
assert d["mcps"] == ["github"] and d["tools"] == ["read-file", "run-command"]
assert not {"runId", "taskId", "request", "feedback"}.intersection(d)
PY
diff -ru "$qa/untouched" "$qa/project"

# Mutate a temporary copy only; normalize CRLF before matching YAML text.
python3 - "$qa/project" <<'PY'
from pathlib import Path
import sys
root = Path(sys.argv[1])
config = root / ".unelos/project.yaml"
text = config.read_text().replace("\r\n", "\n")
old = "  engineer:\n    instructions: Follow existing project conventions.\n    skills: [rust]\n"
new = old.replace("skills: [rust]", "skills: [rust, verification]")
assert text.count(old) == 1
config.write_text(text.replace(old, new))
selected = root / ".unelos/skills/verification/SKILL.md"
selected.write_text(selected.read_text().replace("\r\n", "\n") + "\nSELECTED_SKILL_MARKER\n")
unrelated = root / ".unelos/skills/unrelated"
unrelated.mkdir()
(unrelated / "SKILL.md").write_text("UNRELATED_SKILL_MARKER\n")
PY

./target/debug/unelos --project "$qa/project" inspect implement-feature --node implement --json >"$qa/selected.json"
python3 - "$qa/project" "$qa/selected.json" <<'PY'
from pathlib import Path
import json, sys
root = Path(sys.argv[1])
d = json.load(open(sys.argv[2]))
assert [s["id"] for s in d["skills"]] == ["verification", "rust"]
selected = root / ".unelos/skills/verification/SKILL.md"
assert d["skills"][0]["content"] == selected.read_text()
assert str(root / ".unelos/skills/verification") == d["skills"][0]["directory"]
assert str(root / ".unelos/skills/rust") == d["skills"][1]["directory"]
assert d["skills"][0]["content"].count("SELECTED_SKILL_MARKER") == 1
assert "UNRELATED_SKILL_MARKER" not in json.dumps(d)
PY

mv "$qa/project/.unelos/skills/verification/SKILL.md" "$qa/project/.unelos/skills/verification/SKILL.md.disabled"
set +e
./target/debug/unelos --project "$qa/project" inspect implement-feature --node implement --json >"$qa/missing.stdout" 2>"$qa/missing.stderr"
status=$?
set -e
test "$status" -eq 2
test ! -s "$qa/missing.stdout"
python3 - "$qa/project" "$qa/missing.stderr" <<'PY'
from pathlib import Path
import json, sys
root = Path(sys.argv[1])
error = json.load(open(sys.argv[2]))["error"]
assert error["code"] == "context_resolution_failed"
assert "verification" in error["message"]
assert str(root / ".unelos/skills/verification/SKILL.md") in error["message"]
PY

# Whole-Workflow inspection remains independent of Skill resolution.
./target/debug/unelos --project "$qa/project" inspect implement-feature --json >"$qa/workflow.json"
python3 - "$qa/workflow.json" <<'PY'
import json, sys
assert json.load(open(sys.argv[1]))["name"] == "implement-feature"
PY
printf 'T06 manual QA passed: deterministic/read-only baseline, ordered deduplicated selected Skills, unrelated Skill excluded, full selected content, missing Skill status/path, whole-Workflow compatibility\n'
```
