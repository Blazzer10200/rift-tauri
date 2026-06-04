#!/usr/bin/env python3
"""Emit an edit-swarm batch from the durable worklist, excluding done + held findings.

Usage:
  python scripts/edit-batch.py <bucket> [--limit N]
    <bucket>  state-ts | comp-assistant | comp-other
    --limit   cap count (canary)

Writes .tmp/batch-<bucket>.json = {"findings":[...]} and prints a one-line summary.
Held back (never auto-swarmed): all Rust (src-tauri), all security-flavored findings,
the 3 highs, dependency bumps, dead-code DELETIONS (flag-not-delete), package.json /
lockfile edits, anything in HELD_IDS, and anything already in .tmp/edit-done.json.
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

# Dependency / supply-chain findings — need npm install + migration review, not a patch.
DEP = (
    "vitest",
    "ghsa",
    "cve-",
    "esbuild",
    "vite-node",
    "npm install",
    "save-dev",
    "semver-major",
    "advisory",
    "dependency",
    "devdependenc",
)

# 3 highs (one is a frontend dup at medium) + explicit specials caught by hand last run.
# F39 = AssistantPage:205 pane-keying high; F237 = vitest; F233/F234 = dead-CSS deletes.
HELD_IDS = {"F39", "F237", "F233", "F234"}


def norm(f):
    return f.replace("\\", "/")


def is_rust(f):
    return "src-tauri" in norm(f)


def is_pkg(f):
    f = norm(f).lower()
    return f.endswith(("package.json", "package-lock.json", "cargo.toml", "cargo.lock"))


def is_sec(x):
    s = (x["title"] + " " + x.get("description", "")).lower()
    return any(k in s for k in SEC)


def is_dep(x):
    s = (
        x["title"] + " " + x.get("description", "") + " " + x.get("suggested_fix", "")
    ).lower()
    return any(k in s for k in DEP)


def is_deletion(x):
    """Flag-not-delete: a fix whose core action is removing dead code/CSS."""
    s = (x["title"] + " " + x.get("suggested_fix", "")).lower()
    return ("dead " in s or "unreferenced" in s or "orphan" in s) and (
        "delete" in s or "remove all" in s or "trim" in s or "strip" in s
    )


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

    def held_reason(x):
        if x["id"] in HELD_IDS:
            return "held-id"
        if is_rust(x["file"]):
            return "rust"
        if is_pkg(x["file"]):
            return "pkg/lockfile"
        if is_sec(x):
            return "security"
        if is_dep(x):
            return "dependency"
        if is_deletion(x):
            return "deletion"
        return None

    held = {}
    fe = []
    for x in d:
        if x["id"] in done or bucket_of(x) != want:
            continue
        r = held_reason(x)
        if r:
            held.setdefault(r, []).append(x["id"])
        else:
            fe.append(x)
    if limit:
        fe = fe[:limit]

    os.makedirs(os.path.join(ROOT, ".tmp"), exist_ok=True)
    out = os.path.join(ROOT, f".tmp/batch-{want}.json")
    json.dump({"findings": fe}, open(out, "w", encoding="utf-8"), indent=2)
    print(f"bucket={want} count={len(fe)} done-excluded={len(done)} -> {out}")
    print("ids:", ",".join(x["id"] for x in fe))
    if held:
        print("held:", {k: len(v) for k, v in held.items()})


if __name__ == "__main__":
    main()
