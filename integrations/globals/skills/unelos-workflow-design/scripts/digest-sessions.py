#!/usr/bin/env python3
"""Digest OpenCode or Claude Code sessions for one project into short markdown files.

Usage: digest-sessions.py <project-path-substring> <out-dir> [--db PATH]
       digest-sessions.py <project-path-substring> <out-dir> --claude [--claude-dir DIR]
Writes one <date>_<session-id>.md per root session and prints a summary.
"""
import argparse
import glob
import json
import os
import sqlite3
import sys
from datetime import datetime, timezone


def iso(ms):
    return datetime.fromtimestamp(ms / 1000, timezone.utc).strftime("%Y-%m-%d")


def opencode_sessions(args):
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
    for s in (s for s in sessions if not s["parent_id"]):
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
        yield iso(s["time_created"]), s["id"], out, len(kids)


def read_jsonl(path):
    with open(path, encoding="utf-8") as f:
        for line in f:
            try:
                yield json.loads(line)
            except ValueError:
                continue


def user_text(content):
    if isinstance(content, str):
        return content
    return "\n".join(e.get("text", "") for e in content if e.get("type") == "text")


def claude_sessions(args):
    files = sorted(glob.glob(os.path.join(args.claude_dir, "*", "*.jsonl")))
    if not files:
        sys.exit(f"No Claude Code transcripts under {args.claude_dir}")
    for path in files:
        events = list(read_jsonl(path))
        cwd = next((e["cwd"] for e in events if e.get("cwd")), "")
        stamps = [e["timestamp"] for e in events if e.get("timestamp")]
        if args.project not in cwd or not stamps:
            continue
        sid = os.path.splitext(os.path.basename(path))[0]
        title = next((e["aiTitle"] for e in reversed(events) if e.get("type") == "ai-title"), "")
        kids = []
        for kid in sorted(glob.glob(os.path.join(os.path.dirname(path), sid, "subagents", "*.jsonl"))):
            first = next((e for e in read_jsonl(kid) if e.get("type") == "user"), None)
            kids.append(user_text(first["message"]["content"])[:120].replace("\n", " ") if first else "?")
        date = stamps[0][:10]
        out = [f"# {title} ({date}) {sid}", f'children: {" | ".join(kids)}']
        tools, errors = {}, 0
        for e in events:
            if e.get("isSidechain") or e.get("isMeta"):
                continue
            if e.get("type") == "summary":
                out.append(f'SUMMARY: {str(e.get("summary", ""))[:1500]}')
                continue
            content = (e.get("message") or {}).get("content")
            if e.get("type") == "user" and content:
                if isinstance(content, list):
                    errors += sum(1 for c in content if c.get("type") == "tool_result" and c.get("is_error"))
                text = user_text(content)
                if text:
                    # Compacted sessions resume with the summary as a user message.
                    label = "SUMMARY" if e.get("isCompactSummary") else "USER"
                    out.append(f"{label}: {text[:1500]}")
            elif e.get("type") == "assistant" and isinstance(content, list):
                text = "\n".join(c.get("text", "") for c in content if c.get("type") == "text")
                if text:
                    out.append(f"AI: {text[:400]}")
                for c in content:
                    if c.get("type") == "tool_use":
                        tools[c.get("name")] = tools.get(c.get("name"), 0) + 1
        out.append(f"TOOLS: {json.dumps(tools)} errors={errors}")
        yield date, sid, out, len(kids)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("project")
    ap.add_argument("out")
    ap.add_argument("--db", default=os.path.expanduser("~/.local/share/opencode/opencode.db"))
    ap.add_argument("--claude", action="store_true", help="read Claude Code transcripts instead of OpenCode")
    ap.add_argument("--claude-dir", default=os.path.expanduser("~/.claude/projects"))
    args = ap.parse_args()
    os.makedirs(args.out, exist_ok=True)
    roots = kids = total = 0
    for date, sid, out, n in (claude_sessions if args.claude else opencode_sessions)(args):
        text = "\n".join(out)
        roots, kids, total = roots + 1, kids + n, total + len(text)
        with open(os.path.join(args.out, f"{date}_{sid}.md"), "w", encoding="utf-8") as f:
            f.write(text)
    print(f"{roots} root sessions ({roots + kids} incl. subagents), {total} chars -> {args.out}")


if __name__ == "__main__":
    main()
