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
test ! -e "$QA/.unelos/runs/$RUN_ID"
test -f "$QA/.unelos/runs/fix-save-button/plan/plan.md"
"$BIN" --project "$QA" complete plan --json
"$BIN" --project "$QA" next --json
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
"$BIN" --project "$QA" complete implement --json
"$BIN" --project "$QA" complete review --outcome approved --json
"$BIN" --project "$QA" approve implement --json
"$BIN" --project "$QA" reject implement --reason ' ' --json
"$BIN" --project "$QA" status --json > "$QA/unchanged.json"
```

Continue through implementation, requested correction, and approval:

```bash
python3 "$QA/write-report.py" "$QA/next.json" 'Implementation v1; manual runtime exercise, not real code verification.'
"$BIN" --project "$QA" complete implement --json
"$BIN" --project "$QA" next --json > "$QA/next.json"
python3 "$QA/write-report.py" "$QA/next.json" 'Review: request one correction.'
"$BIN" --project "$QA" complete review --outcome unknown --json
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

Unknown outcome must fail without progression. Final history has eight entries:
plan, rejected gate, revised plan, approved gate, implementation, review
requesting changes, correction implementation, and approved review. These
checks exercise CLI/runtime persistence after process restarts, not power-loss
guarantees beyond the store's atomic-write model.
