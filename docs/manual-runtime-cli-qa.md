# Manual runtime CLI QA

Prerequisites: Rust/Cargo, Python 3, and the repository's fixture. This uses no
network, model, or external agent. The report text is only test data: it does
not prove agent quality, code implementation, verification, automatic
instruction injection, or harness-managed ledger writes. Initial user context
is supplied manually. Keep the disposable directory for inspection.

```bash
cargo build --locked -p unelos-cli
BIN="$PWD/target/debug/unelos"
QA=$(mktemp -d /tmp/opencode/unelos-t12-qa.XXXXXX)
cp -a crates/core/tests/fixtures/project/. "$QA/"
"$BIN" --project "$QA" status --json
"$BIN" --project "$QA" start --json
"$BIN" --project "$QA" start implement-feature --json > "$QA/start.json"
cat "$QA/start.json"
RUN_ID=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["runId"])' "$QA/start.json")
"$BIN" --project "$QA" next --json > "$QA/next.json"
"$BIN" --project "$QA" next --json > "$QA/next-again.json"
cmp "$QA/next.json" "$QA/next-again.json"
```

Before any report write, prove status/next do not alter the authoritative state
or active-run pointer:

```bash
STATE=$(python3 -c 'import json,pathlib,sys; print(pathlib.Path(json.load(open(sys.argv[1]))["runDirectory"])/"state.json")' "$QA/start.json")
cp "$STATE" "$QA/state-before.json"
cp "$QA/.unelos/active-run" "$QA/pointer-before"
"$BIN" --project "$QA" status --json
"$BIN" --project "$QA" next --json > "$QA/next.json"
cmp "$STATE" "$QA/state-before.json"
cmp "$QA/.unelos/active-run" "$QA/pointer-before"
```

Expect idle status, selection containing `implement-feature`, generated short
name with `namingPending: true`, and byte-identical repeated packets. Every CLI
call above is a fresh process.

Write only to packet-declared output paths:

```bash
cat > "$QA/write-report.py" <<'PY'
import json, pathlib, sys
packet = json.loads(pathlib.Path(sys.argv[1]).read_text())
for output in packet["outputs"]:
    pathlib.Path(output["path"]).write_text(sys.argv[2] + "\n")
PY
python3 "$QA/write-report.py" "$QA/next.json" 'Plan: gather the goal; implement a small change; verify behavior.'
"$BIN" --project "$QA" rename fix-save-button --run "$RUN_ID" --json
"$BIN" --project "$QA" next --json > "$QA/next.json"
STATE="$QA/.unelos/runs/fix-save-button/state.json"
expect_runtime_error() {
  expected_code=$1
  shift
  if "$BIN" --project "$QA" "$@" --json > "$QA/error-stdout" 2> "$QA/error-stderr"; then
    echo "Expected command to fail: $*" >&2
    exit 1
  else
    test "$?" -eq 2
  fi
  test ! -s "$QA/error-stdout"
  python3 - "$QA/error-stderr" "$expected_code" <<'PY'
import json, sys
assert json.load(open(sys.argv[1]))["error"]["code"] == sys.argv[2]
PY
  cmp "$STATE" "$QA/state-before.json"
  cmp "$QA/.unelos/active-run" "$QA/pointer-before"
}
test ! -e "$QA/.unelos/runs/$RUN_ID"
test -f "$QA/.unelos/runs/fix-save-button/plan/plan.md"
"$BIN" --project "$QA" complete plan --json
"$BIN" --project "$QA" next --json
cp "$STATE" "$QA/state-before.json"
cp "$QA/.unelos/active-run" "$QA/pointer-before"
expect_runtime_error invalid_runtime_state reject approve-plan --reason ' '
mv "$QA/.unelos/runs/fix-save-button/plan/plan.md" "$QA/plan-saved.md"
for decision in approve reject; do
  cp "$STATE" "$QA/state-before.json"
  cp "$QA/.unelos/active-run" "$QA/pointer-before"
  if [ "$decision" = approve ]; then
    expect_runtime_error invalid_runtime_state approve approve-plan
  else
    expect_runtime_error invalid_runtime_state reject approve-plan --reason 'Revise scope.'
  fi
done
mv "$QA/plan-saved.md" "$QA/.unelos/runs/fix-save-button/plan/plan.md"
"$BIN" --project "$QA" reject approve-plan --reason 'Explain rollback.' --json
"$BIN" --project "$QA" next --json > "$QA/next.json"
python3 "$QA/write-report.py" "$QA/next.json" 'Revised plan: explain rollback and retain the agreed scope.'
"$BIN" --project "$QA" complete plan --json
"$BIN" --project "$QA" approve approve-plan --json
"$BIN" --project "$QA" next --json > "$QA/next.json"
```

Expect preserved run ID, settled naming, relocated plan, waiting gate, rejection
feedback with the exact reason and presented path, revised same plan path, then
implementation. Before writing implementation output, run each negative
command separately. Each must exit 2, print no stdout, and leave progression
unchanged; `status` should still identify `implement` as current:

```bash
STATE="$QA/.unelos/runs/fix-save-button/state.json"
cp "$STATE" "$QA/state-before.json"
cp "$QA/.unelos/active-run" "$QA/pointer-before"
expect_runtime_error invalid_runtime_state complete implement
expect_runtime_error wrong_node complete review --outcome approved
expect_runtime_error invalid_runtime_state approve implement
expect_runtime_error invalid_runtime_state reject implement --reason ' '
"$BIN" --project "$QA" status --json > "$QA/unchanged.json"
```

Continue through implementation, requested correction, and approval:

```bash
python3 "$QA/write-report.py" "$QA/next.json" 'Implementation v1; manual runtime exercise, not real code verification.'
"$BIN" --project "$QA" complete implement --json
"$BIN" --project "$QA" next --json > "$QA/next.json"
python3 "$QA/write-report.py" "$QA/next.json" 'Review: request one correction.'
cp "$STATE" "$QA/state-before.json"
cp "$QA/.unelos/active-run" "$QA/pointer-before"
expect_runtime_error invalid_runtime_state complete review --outcome unknown
"$BIN" --project "$QA" complete review --outcome changes_requested --json
"$BIN" --project "$QA" next --json > "$QA/next.json"
python3 "$QA/write-report.py" "$QA/next.json" 'Implementation v2; correction recorded in the same report.'
"$BIN" --project "$QA" complete implement --json
"$BIN" --project "$QA" next --json > "$QA/next.json"
python3 "$QA/write-report.py" "$QA/next.json" 'Review: approved.'
"$BIN" --project "$QA" complete review --outcome approved --json
"$BIN" --project "$QA" next --json
"$BIN" --project "$QA" status --json > "$QA/final.json"
python3 - "$QA/final.json" "$RUN_ID" <<'PY'
import json, sys
state = json.load(open(sys.argv[1]))
assert state["status"] == "finished"
assert state["runId"] == sys.argv[2]
assert state["name"] == "fix-save-button"
assert state["namingPending"] is False
assert len(state["history"]) == 8
PY
```

Unknown outcome must fail without progression. The final history has eight entries:
plan, rejected gate, revised plan, approved gate, implementation, review
requesting changes, correction implementation, and approved review. These
checks exercise persistence, binding, feedback, explicit output confirmation,
and unchanged progression on failures after process restarts. They do not prove
content freshness, safe delayed callbacks to a later visit to the same Node,
stored CLI request context, agent quality, or automatic instruction injection.
There is no automatic hash/mtime freshness or report-quality validation.
T13 is not claimed complete. These checks do not establish power-loss guarantees
beyond the store's atomic-write model.
