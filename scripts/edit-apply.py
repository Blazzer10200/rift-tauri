#!/usr/bin/env python3
"""Apply verified patches from the edit-swarm to the working tree.

Usage:
  python scripts/edit-apply.py <patch-source> [--apply]
    <patch-source>  edit-swarm task .output file (JSON) OR a raw patches JSON
    --apply         actually write files (default: DRY RUN — report only)

Patches are read from result.accepted: [{file, edits:[{old_string,new_string}], ...}].
Each old_string must appear EXACTLY ONCE in the current file or the edit is skipped and reported.
"""

import json
import sys
import os


def load_accepted(path):
    d = json.load(open(path, encoding="utf-8"))
    r = d.get("result", d)
    if isinstance(r, str):
        r = json.loads(r)
    return r.get("accepted", [])


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    src = sys.argv[1]
    do_apply = "--apply" in sys.argv[2:]
    accepted = load_accepted(src)

    # normalize any absolute / backslash path to a repo-relative one BEFORE grouping,
    # so all edits to one file land in a single read-modify-write cycle
    def norm(p):
        p = p.replace("\\", "/")
        for anchor in ("src-tauri/", "src/"):
            i = p.find(anchor)
            if i != -1:
                return p[i:]
        return p

    # group by normalized file
    by_file = {}
    for a in accepted:
        by_file.setdefault(norm(a["file"]), []).extend(a.get("edits", []))

    applied = miss = ambiguous = 0
    touched = []
    for f, edits in by_file.items():
        path = f.replace("\\", "/")
        # normalize possible absolute Windows path to repo-relative if it points into cwd
        if not os.path.exists(path):
            # try stripping everything up to 'src'
            for anchor in ("src-tauri/", "src/"):
                i = path.find(anchor)
                if i != -1 and os.path.exists(path[i:]):
                    path = path[i:]
                    break
        if not os.path.exists(path):
            print(f"  !! FILE NOT FOUND: {f}")
            miss += len(edits)
            continue
        text = open(path, encoding="utf-8").read()
        orig = text
        file_applied = 0
        for e in edits:
            old, new = e["old_string"], e["new_string"]
            c = text.count(old)
            if c == 1:
                text = text.replace(old, new, 1)
                applied += 1
                file_applied += 1
            elif c == 0:
                miss += 1
                print(f"  -- MISS  {path}: old_string not found ({old[:50]!r}...)")
            else:
                ambiguous += 1
                print(f"  -- AMBIG {path}: old_string x{c} ({old[:50]!r}...)")
        if file_applied and text != orig:
            touched.append(path)
            if do_apply:
                open(path, "w", encoding="utf-8").write(text)

    mode = "APPLIED" if do_apply else "DRY RUN (no files written)"
    print(f"\n=== {mode} ===")
    print(
        f"edits applied: {applied} | missed: {miss} | ambiguous: {ambiguous} | files touched: {len(touched)}"
    )
    for t in touched:
        print(f"   {'wrote' if do_apply else 'would write'}: {t}")
    if not do_apply and touched:
        print(
            "\nRe-run with --apply to write, then run the build-gate (npm run check / cargo check)."
        )


if __name__ == "__main__":
    main()
