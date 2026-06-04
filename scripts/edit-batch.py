#!/usr/bin/env python3
"""Emit an edit-swarm batch from the durable worklist, excluding done + held findings.

Usage:
  python scripts/edit-batch.py <bucket> [--limit N]
    <bucket>  state-ts | comp-assistant | comp-other
    --limit   cap count (canary)

Writes .tmp/batch-<bucket>.json = {"findings":[...]} and prints a one-line summary.
Held back: all Rust (src-tauri), all security-flavored findings, the 3 highs (already
excluded from the worklist), and anything already recorded in .tmp/edit-done.json.
"""

import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
WORKLIST = os.path.join(ROOT, "docs/audit-2026-06-04/edit-worklist.json")
DONE = os.path.join(ROOT, ".tmp/edit-done.json")

SEC = (
    "inject",
    "traversal",
    "scheme",
    "sanitiz",
    " xss",
    "dompurify",
    "allowlist",
    "url ",
    "releaseurl",
    "validate_path",
    "file://",
    "javascript:",
    "bypass",
    "capabilit",
    "permission-mode",
    "plaintext",
    "keychain",
    "wildcard",
)


def norm(f):
    return f.replace("\\", "/")


def is_rust(f):
    return "src-tauri" in norm(f)


def is_sec(x):
    s = (x["title"] + " " + x.get("description", "")).lower()
    return any(k in s for k in SEC)


def bucket_of(x):
    f = norm(x["file"])
    if f.endswith(".ts") and "components" not in f:
        return "state-ts"
    if "/assistant/" in f:
        return "comp-assistant"
    return "comp-other"


def load_done():
    if os.path.exists(DONE):
        return set(json.load(open(DONE, encoding="utf-8")))
    return set()


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    want = sys.argv[1]
    limit = None
    if "--limit" in sys.argv:
        limit = int(sys.argv[sys.argv.index("--limit") + 1])

    d = json.load(open(WORKLIST, encoding="utf-8"))["findings"]
    done = load_done()
    fe = [
        x
        for x in d
        if not is_rust(x["file"])
        and not is_sec(x)
        and x["id"] not in done
        and bucket_of(x) == want
    ]
    if limit:
        fe = fe[:limit]

    os.makedirs(os.path.join(ROOT, ".tmp"), exist_ok=True)
    out = os.path.join(ROOT, f".tmp/batch-{want}.json")
    json.dump({"findings": fe}, open(out, "w", encoding="utf-8"), indent=2)
    print(f"bucket={want} count={len(fe)} done-excluded={len(done)} -> {out}")
    print("ids:", ",".join(x["id"] for x in fe))


if __name__ == "__main__":
    main()
