#!/usr/bin/env python3
"""Extract a slim edit-swarm work-list from an audit result.

Usage:
  python scripts/edit-worklist.py <audit-output-file> [--canary N] [--out FILE]
    default: writes full swarmable list (low+medium, excludes high) to .tmp/edit-worklist.json
    --canary N: instead print N hand-picked mechanical findings as JSON (for a test run)
"""

import json
import sys
import os

# keywords that signal a clearly-mechanical, low-risk fix (good canary material)
CANARY_HINTS = (
    "aria-label",
    "clearTimeout",
    "onDestroy",
    ".catch(",
    "reduced-motion",
    "keyed by index",
    "key by",
    "aria-disabled",
    "console.warn",
)


def load(path):
    d = json.load(open(path, encoding="utf-8"))
    r = d.get("result", d)
    if isinstance(r, str):
        r = json.loads(r)
    return r


def slim(f, i):
    return {
        "id": f"F{i}",
        "file": f.get("file", ""),
        "line": f.get("line", 0),
        "severity": f.get("severity", ""),
        "title": f.get("title", ""),
        "description": (f.get("description", "") or "")[:600],
        "evidence": (f.get("evidence", "") or "")[:400],
        "suggested_fix": (f.get("suggested_fix", "") or "")[:500],
    }


def main():
    src = sys.argv[1]
    canary = 0
    out = ".tmp/edit-worklist.json"
    if "--canary" in sys.argv:
        canary = int(sys.argv[sys.argv.index("--canary") + 1])
    if "--out" in sys.argv:
        out = sys.argv[sys.argv.index("--out") + 1]

    r = load(src)
    raw = r.get("rawConfirmed", [])
    swarmable = [
        slim(f, i) for i, f in enumerate(raw) if f.get("severity") in ("low", "medium")
    ]

    if canary:
        # prefer findings whose suggested_fix matches a mechanical hint, smallest files first
        scored = sorted(
            swarmable,
            key=lambda f: (
                0 if any(h in f["suggested_fix"] for h in CANARY_HINTS) else 1,
                f["file"],
            ),
        )
        pick = scored[:canary]
        print(json.dumps({"findings": pick}, indent=2))
        return

    os.makedirs(os.path.dirname(out) or ".", exist_ok=True)
    json.dump({"findings": swarmable}, open(out, "w", encoding="utf-8"), indent=2)
    print(f"wrote {out}: {len(swarmable)} swarmable findings (of {len(raw)} confirmed)")


if __name__ == "__main__":
    main()
