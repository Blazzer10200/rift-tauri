#!/usr/bin/env python3
"""Apply verified patches from the edit-swarm to the working tree.

Usage:
  python scripts/edit-apply.py <patch-source> [--apply]
    <patch-source>  edit-swarm task .output file (JSON) OR a raw patches JSON
    --apply         actually write files (default: DRY RUN — report only)

Patches are read from result.accepted: [{id, file, edits:[{old_string,new_string}], ...}].

ATOMIC PER FINDING: a finding's edits are applied all-or-nothing. If ANY edit of a
finding fails to match exactly once, the whole finding is skipped (its other edits are
NOT written) and it is reported for manual completion. This prevents coupled edits
(e.g. a type change + its call sites) from leaving a half-applied, broken build.

On --apply, also writes the touched repo-relative paths (one per line) to
.tmp/edit-last-touched.txt so the commit can be scoped: `git add $(cat ...)` instead
of `git add -A` (avoids sweeping up concurrent-session work).
"""

import json
import os
import sys


def load_accepted(path):
    d = json.load(open(path, encoding="utf-8"))
    r = d.get("result", d)
    if isinstance(r, str):
        r = json.loads(r)
    return r.get("accepted", [])


def norm(p):
    p = p.replace("\\", "/")
    for anchor in ("src-tauri/", "src/"):
        i = p.find(anchor)
        if i != -1:
            return p[i:]
    return p


def resolve(path):
    if os.path.exists(path):
        return path
    for anchor in ("src-tauri/", "src/"):
        i = path.find(anchor)
        if i != -1 and os.path.exists(path[i:]):
            return path[i:]
    return None


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    src = sys.argv[1]
    do_apply = "--apply" in sys.argv[2:]
    accepted = load_accepted(src)

    # group findings by normalized file, preserving the per-finding edit boundary
    by_file = {}
    for a in accepted:
        by_file.setdefault(norm(a["file"]), []).append(a)

    findings_applied = findings_skipped = edits_applied = 0
    touched = []

    for f, findings in by_file.items():
        path = resolve(f.replace("\\", "/"))
        if not path:
            print(f"  !! FILE NOT FOUND: {f}  (skipping {len(findings)} finding(s))")
            findings_skipped += len(findings)
            continue
        text = open(path, encoding="utf-8").read()
        orig = text
        for a in findings:
            fid = a.get("id", "?")
            edits = a.get("edits", [])
            # trial-apply ALL edits of this finding on a working copy
            trial = text
            ok = True
            for e in edits:
                old, new = e["old_string"], e["new_string"]
                c = trial.count(old)
                if c != 1:
                    why = "not found" if c == 0 else f"x{c} (ambiguous)"
                    print(f"  -- SKIP {fid} {path}: old_string {why} ({old[:48]!r}...)")
                    ok = False
                    break
                trial = trial.replace(old, new, 1)
            if ok:
                text = trial
                findings_applied += 1
                edits_applied += len(edits)
            else:
                findings_skipped += 1
        if text != orig:
            touched.append(path)
            if do_apply:
                open(path, "w", encoding="utf-8").write(text)

    mode = "APPLIED" if do_apply else "DRY RUN (no files written)"
    print(f"\n=== {mode} ===")
    print(
        f"findings applied: {findings_applied} | skipped (manual): {findings_skipped} "
        f"| edits: {edits_applied} | files touched: {len(touched)}"
    )
    for t in touched:
        print(f"   {'wrote' if do_apply else 'would write'}: {t}")
    if do_apply and touched:
        os.makedirs(".tmp", exist_ok=True)
        with open(".tmp/edit-last-touched.txt", "w", encoding="utf-8") as fh:
            fh.write("\n".join(touched) + "\n")
        print(
            "\nScoped commit: git add $(cat .tmp/edit-last-touched.txt) && git commit"
        )
    elif not do_apply and touched:
        print(
            "\nRe-run with --apply to write, then run the build-gate (npm run check)."
        )


if __name__ == "__main__":
    main()
