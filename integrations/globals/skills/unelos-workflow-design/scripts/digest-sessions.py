#!/usr/bin/env python3
"""Digest OpenCode sessions for one project into short markdown files.

Usage: digest-sessions.py <project-path-substring> <out-dir> [--db PATH]
Writes one <date>_<session-id>.md per root session and prints a summary.
"""
import argparse
import json
import os
import sqlite3
import sys
from datetime import datetime, timezone


def iso(ms):
    return datetime.fromtimestamp(ms / 1000, timezone.utc).strftime("%Y-%m-%d")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("project")
    ap.add_argument("out")
    ap.add_argument("--db", default=os.path.expanduser("~/.local/share/opencode/opencode.db"))
    args = ap.parse_args()
    if not os.path.exists(args.db):
        sys.exit(f"No OpenCode database at {args.db}")
    db = sqlite3.connect(f"file:{args.db}?mode=ro", uri=True)
    db.row_factory = sqlite3.Row
    tables = {r[0] for r in db.execute("SELECT name FROM sqlite_master WHERE type='table'")}
    like = f"%{args.project}%"
    if "session_v2" in tables:
        sessions = db.execute(
            "SELECT s.id, s.parent_id, s.title, s.agent, s.time_created FROM session_v2 s "
            "LEFT JOIN project p ON p.id = s.project_id "
            "WHERE p.worktree LIKE ? OR s.directory LIKE ? ORDER BY s.time_created",
            (like, like),
        ).fetchall()
        messages = lambda sid: db.execute(
            "SELECT type, data FROM session_message WHERE session_id = ? ORDER BY seq", (sid,)
        ).fetchall()
    else:
        sessions = db.execute(
            "SELECT id, parent_id, title, '' AS agent, time_created FROM session "
            "WHERE directory LIKE ? ORDER BY time_created",
            (like,),
        ).fetchall()
        messages = lambda sid: [
            (json.loads(r["data"]).get("role", ""), r["data"])
            for r in db.execute("SELECT data FROM message WHERE session_id = ?", (sid,))
        ]

    os.makedirs(args.out, exist_ok=True)
    roots = [s for s in sessions if not s["parent_id"]]
    total = 0
    for s in roots:
        kids = [f'{k["agent"]}: {k["title"]}' for k in sessions if k["parent_id"] == s["id"]]
        out = [f'# {s["title"]} ({iso(s["time_created"])}) {s["id"]}', f'children: {" | ".join(kids)}']
        tools, errors = {}, 0
        for kind, raw in messages(s["id"]):
            try:
                d = json.loads(raw)
            except (TypeError, ValueError):
                continue
            if kind == "user":
                out.append(f'USER: {str(d.get("text", ""))[:1500]}')
            elif kind == "assistant":
                content = d.get("content") or []
                text = "\n".join(e.get("text", "") for e in content if e.get("type") == "text")
                if text:
                    out.append(f"AI: {text[:400]}")
                for e in content:
                    if e.get("type") not in (None, "text", "reasoning"):
                        name = e.get("name") or e.get("tool") or e["type"]
                        tools[name] = tools.get(name, 0) + 1
                        errors += bool((e.get("state") or {}).get("status") == "error" or e.get("error"))
            elif kind == "summary":
                out.append(f'SUMMARY: {str(d.get("summary", ""))[:1500]}')
            elif kind != "reasoning":
                name = d.get("tool") or d.get("name") or kind
                tools[name] = tools.get(name, 0) + 1
                errors += bool((d.get("state") or {}).get("status") == "error" or d.get("error"))
        out.append(f"TOOLS: {json.dumps(tools)} errors={errors}")
        text = "\n".join(out)
        total += len(text)
        with open(os.path.join(args.out, f'{iso(s["time_created"])}_{s["id"]}.md'), "w") as f:
            f.write(text)
    print(f"{len(roots)} root sessions ({len(sessions)} incl. subagents), {total} chars -> {args.out}")


if __name__ == "__main__":
    main()
