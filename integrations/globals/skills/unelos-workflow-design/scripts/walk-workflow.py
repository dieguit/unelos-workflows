#!/usr/bin/env python3
"""Walk one path through a Unelos workflow in a throwaway copy of the project.

Usage:
  walk-workflow.py <project> <workflow> <step> [<step> ...] [--unelos BIN]
  walk-workflow.py <project> --steps-file FILE <step> [<step> ...] [--unelos BIN]

Each step is <node>, <node>:<outcome>, <node>:approve or <node>:reject.
Example:
  walk-workflow.py . ui-bugfix intake reproduce:reproduced fix:ui-only \
      frontend-checks:fail fix:ui-only frontend-checks:pass verify:pass approve:approve

Inline (composed) run: node ids are <workflow>/<node>, gate steps gate-1, gate-2, ...
  walk-workflow.py . --steps-file steps.yaml council/assignment council/council \
      council/synthesis council/approve-council:approve gate-1:approve pr-and-ci/open-pr:opened

The project's .unelos authoring is copied to a temp dir (runs are not), a run is
started, placeholder outputs are written before each step, and the node each
step leads to is printed together with the feedback it received.
"""
import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("project")
    ap.add_argument("first")
    ap.add_argument("rest", nargs="*")
    ap.add_argument("--steps-file")
    ap.add_argument("--unelos", default="unelos")
    args = ap.parse_args()
    if args.steps_file is not None:
        args.steps_file = os.path.abspath(args.steps_file)
        steps = [args.first, *args.rest]
    else:
        if not args.rest:
            ap.error("a workflow and at least one step are required")
        workflow = args.first
        steps = args.rest
    if os.sep in args.unelos:
        args.unelos = os.path.abspath(args.unelos)

    temp = tempfile.mkdtemp(prefix="unelos-walk-")
    shutil.copytree(
        os.path.join(args.project, ".unelos"),
        os.path.join(temp, ".unelos"),
        ignore=shutil.ignore_patterns("runs", "run-sequence", "runtime.lock"),
    )

    def cli(*argv):
        r = subprocess.run([args.unelos, "--json", *argv], capture_output=True, text=True, cwd=temp)
        try:
            return json.loads(r.stdout or r.stderr)
        except ValueError:
            sys.exit(f"unelos {' '.join(argv)} failed:\n{r.stdout}{r.stderr}")

    started = cli("start", "--steps-file", args.steps_file) if args.steps_file is not None else cli("start", workflow)
    if "error" in started:
        sys.exit(f"start failed: {started['error']}")
    run = started.get("runId") or cli("next", "--run", "r-000001").get("runId", "r-000001")

    failed = False
    for step in steps:
        node, _, action = step.partition(":")
        packet = cli("next", "--run", run)
        if packet.get("nodeId") not in (None, node):
            print(f"✗ expected {node}, run is at {packet.get('nodeId')}")
            failed = True
            break
        outputs = list(packet.get("outputs") or [])
        for worker in packet.get("workers") or []:
            outputs += worker.get("outputs") or []
        for out in outputs:
            os.makedirs(os.path.dirname(out["path"]), exist_ok=True)
            with open(out["path"], "a") as f:
                f.write(f"placeholder from {node}\n")
        feedback = [a["name"] for a in ((packet.get("feedback") or {}).get("artifacts") or [])]
        if action == "approve":
            result = cli("approve", "--run", run, node)
        elif action == "reject":
            result = cli("reject", "--run", run, "--reason", "walk rejection", node)
        elif action:
            result = cli("complete", "--run", run, "--outcome", action, node)
        else:
            result = cli("complete", "--run", run, node)
        if "error" in result:
            print(f"✗ {step}: {result['error']}")
            failed = True
            break
        after = cli("next", "--run", run)
        where = after.get("nodeId") or after.get("status") or "finished"
        print(f"✓ {step:<28} -> {where:<20} feedback in: {', '.join(feedback) or '-'}")

    print(f"\nTemp project: {temp}")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
