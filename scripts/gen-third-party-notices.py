#!/usr/bin/env python3
"""Generate THIRD-PARTY-NOTICES.md for everything compiled into shipped Rift.

Covers the Rust normal-dependency graph (what links into rift-tauri.exe on the
current target) and the npm production tree (what vite bundles into the UI).
Each package is listed under the single license Rift elects when several are
offered (permissive-first), with its copyright lines; full license texts live
in the appendix. Run from anywhere: paths resolve relative to this script.

    python scripts/gen-third-party-notices.py

Regenerate whenever dependencies change (release.ps1 ships the output file
alongside the exe).
"""

import json
import os
import re
import subprocess
import sys
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "THIRD-PARTY-NOTICES.md")

# Election order when a package offers a choice ("A OR B", "A/B").
PREFERENCE = [
    "MIT",
    "Apache-2.0",
    "BSD-3-Clause",
    "BSD-2-Clause",
    "ISC",
    "Zlib",
    "Unicode-3.0",
    "0BSD",
    "MIT-0",
    "CC0-1.0",
    "Unlicense",
    "BSL-1.0",
    "MPL-2.0",
    "OFL-1.1",
    "CDLA-Permissive-2.0",
]

LICENSE_FILE_RX = re.compile(r"^(LICEN[SC]E|COPYING|NOTICE)", re.IGNORECASE)
COPYRIGHT_RX = re.compile(r"^.{0,10}copyright (?:\(c\)|©|\d)", re.IGNORECASE)

# Short canonical texts embedded directly; long ones are sniffed from package
# license files on disk (see TEXT_SNIFFERS).
EMBEDDED_TEXTS = {
    "MIT": """MIT License

Copyright (c) <the copyright holders listed above>

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.""",
    "ISC": """ISC License

Copyright (c) <the copyright holders listed above>

Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
PERFORMANCE OF THIS SOFTWARE.""",
    "Zlib": """zlib License

Copyright (c) <the copyright holders listed above>

This software is provided 'as-is', without any express or implied warranty. In
no event will the authors be held liable for any damages arising from the use
of this software.

Permission is granted to anyone to use this software for any purpose,
including commercial applications, and to alter it and redistribute it
freely, subject to the following restrictions:

1. The origin of this software must not be misrepresented; you must not claim
   that you wrote the original software. If you use this software in a
   product, an acknowledgment in the product documentation would be
   appreciated but is not required.
2. Altered source versions must be plainly marked as such, and must not be
   misrepresented as being the original software.
3. This notice may not be removed or altered from any source distribution.""",
    "BSD-3-Clause": """BSD 3-Clause License

Copyright (c) <the copyright holders listed above>

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice,
   this list of conditions and the following disclaimer.
2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.
3. Neither the name of the copyright holder nor the names of its contributors
   may be used to endorse or promote products derived from this software
   without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.""",
    "BSD-2-Clause": """BSD 2-Clause License

Copyright (c) <the copyright holders listed above>

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice,
   this list of conditions and the following disclaimer.
2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.""",
}

# family -> (sniff regex on file content) for long texts pulled from disk.
TEXT_SNIFFERS = {
    "Apache-2.0": re.compile(r"Apache License\s*\n?\s*Version 2\.0", re.IGNORECASE),
    "MPL-2.0": re.compile(r"Mozilla Public License Version 2\.0", re.IGNORECASE),
    "OFL-1.1": re.compile(r"SIL OPEN FONT LICENSE Version 1\.1", re.IGNORECASE),
    "BSL-1.0": re.compile(r"Boost Software License - Version 1\.0", re.IGNORECASE),
    "Unicode-3.0": re.compile(r"UNICODE LICENSE V3", re.IGNORECASE),
    "CDLA-Permissive-2.0": re.compile(
        r"Community Data License Agreement\s*-\s*Permissive\s*-?\s*Version 2\.0",
        re.IGNORECASE,
    ),
    "Unlicense": re.compile(
        r"This is free and unencumbered software released into the public domain",
        re.IGNORECASE,
    ),
    "CC0-1.0": re.compile(r"CC0 1\.0 Universal", re.IGNORECASE),
    "0BSD": re.compile(
        r"Zero.Clause BSD|Permission to use, copy, modify, and/or distribute this software for any\s*purpose with or without fee is hereby granted\.",
        re.IGNORECASE,
    ),
}


def elect(expr: str) -> list[str]:
    """Resolve an SPDX-ish expression to the license(s) Rift elects."""
    expr = expr.strip()
    # Top-level AND components must ALL be honored.
    and_parts = re.split(r"\s+AND\s+", expr)
    chosen = []
    for part in and_parts:
        part = part.strip().strip("()")
        options = [o.strip() for o in re.split(r"\s+OR\s+|\s*/\s*", part) if o.strip()]
        pick = None
        for pref in PREFERENCE:
            if pref in options:
                pick = pref
                break
        if pick is None:
            # e.g. sole "Apache-2.0 WITH LLVM-exception"
            pick = options[0]
        chosen.append(pick)
    # De-dup while preserving order
    seen = set()
    return [c for c in chosen if not (c in seen or seen.add(c))]


def family_of(license_id: str) -> str:
    return "Apache-2.0" if license_id.startswith("Apache-2.0 WITH") else license_id


def license_files(pkg_dir: str) -> list[str]:
    try:
        return [
            os.path.join(pkg_dir, f)
            for f in os.listdir(pkg_dir)
            if LICENSE_FILE_RX.match(f) and os.path.isfile(os.path.join(pkg_dir, f))
        ]
    except OSError:
        return []


def read_text(path: str) -> str:
    try:
        with open(path, encoding="utf-8", errors="replace") as fh:
            return fh.read()
    except OSError:
        return ""


def copyright_lines(pkg_dir: str, cap: int = 3) -> list[str]:
    lines: list[str] = []
    for lf in license_files(pkg_dir):
        for line in read_text(lf).splitlines():
            line = line.strip().lstrip("/#*- ").strip()
            if COPYRIGHT_RX.match(line) and len(line) < 200:
                if line not in lines:
                    lines.append(line)
            if len(lines) >= cap:
                return lines
    return lines


def collect_rust() -> tuple[list[dict], dict[str, str]]:
    meta = json.loads(
        subprocess.check_output(
            [
                "cargo",
                "metadata",
                "--format-version",
                "1",
                "--manifest-path",
                os.path.join(ROOT, "src-tauri", "Cargo.toml"),
            ],
            text=True,
            encoding="utf-8",
        )
    )
    workspace = set(meta["workspace_members"])
    by_key = {(p["name"], p["version"]): p for p in meta["packages"]}

    # Only what actually links into the shipped binary: the normal-edge graph
    # on the current target.
    tree = subprocess.check_output(
        [
            "cargo",
            "tree",
            "-e",
            "normal",
            "--prefix",
            "none",
            "--manifest-path",
            os.path.join(ROOT, "src-tauri", "Cargo.toml"),
        ],
        text=True,
        encoding="utf-8",
    )
    wanted: set[tuple[str, str]] = set()
    for line in tree.splitlines():
        m = re.match(r"^(\S+) v(\S+)", line.strip())
        if m:
            wanted.add((m.group(1), m.group(2)))

    pkgs, texts = [], {}
    for key in sorted(wanted):
        p = by_key.get(key)
        if p is None or p["id"] in workspace:
            continue
        lic = p.get("license") or ""
        pkg_dir = os.path.dirname(p["manifest_path"])
        if not lic:
            lic = "SEE-LICENSE-FILE"
        cop = copyright_lines(pkg_dir) or (
            [
                f"Copyright the {p['name']} authors"
                + (f" ({', '.join(p['authors'][:3])})" if p.get("authors") else "")
            ]
        )
        elected = (
            [family_of(e) for e in elect(lic)]
            if lic != "SEE-LICENSE-FILE"
            else ["SEE-LICENSE-FILE"]
        )
        pkgs.append(
            {
                "name": p["name"],
                "version": p["version"],
                "expr": lic,
                "elected": elected,
                "copyrights": cop,
            }
        )
        harvest_texts(pkg_dir, texts)
    return pkgs, texts


def collect_npm() -> tuple[list[dict], dict[str, str]]:
    out = subprocess.check_output(
        ["npm", "ls", "--omit=dev", "--all", "--parseable"],
        text=True,
        encoding="utf-8",
        cwd=ROOT,
        shell=(os.name == "nt"),
    )
    pkgs, texts, seen = [], {}, set()
    for line in out.splitlines():
        p = line.strip()
        if not p or os.path.normpath(p) == os.path.normpath(ROOT):
            continue
        pj = os.path.join(p, "package.json")
        if not os.path.isfile(pj):
            continue
        try:
            d = json.loads(read_text(pj))
        except json.JSONDecodeError:
            continue
        name, version = d.get("name", "?"), d.get("version", "?")
        if (name, version) in seen:
            continue
        seen.add((name, version))
        lic = d.get("license") or "SEE-LICENSE-FILE"
        if isinstance(lic, dict):
            lic = lic.get("type", "SEE-LICENSE-FILE")
        lic = str(lic).strip("()")
        cop = copyright_lines(p) or [f"Copyright the {name} authors"]
        elected = (
            [family_of(e) for e in elect(lic)]
            if lic != "SEE-LICENSE-FILE"
            else ["SEE-LICENSE-FILE"]
        )
        pkgs.append(
            {
                "name": name,
                "version": version,
                "expr": lic,
                "elected": elected,
                "copyrights": cop,
            }
        )
        harvest_texts(p, texts)
    pkgs.sort(key=lambda x: x["name"])
    return pkgs, texts


def harvest_texts(pkg_dir: str, texts: dict[str, str]) -> None:
    """Grab full texts for the long licenses from package license files."""
    missing = [f for f in TEXT_SNIFFERS if f not in texts]
    if not missing:
        return
    for lf in license_files(pkg_dir):
        content = read_text(lf)
        for fam in missing:
            if fam not in texts and TEXT_SNIFFERS[fam].search(content):
                texts[fam] = content.strip()


def render(rust_pkgs, npm_pkgs, texts) -> str:
    lines = [
        "# Third-Party Notices",
        "",
        "Rift's installer bundles open-source software: Rust crates compiled into",
        "`rift-tauri.exe` and npm packages bundled into the UI. Each package below is",
        "listed under the license Rift elects where the package offers a choice",
        "(original SPDX expression shown in parentheses when it differs). Full license",
        "texts are in the appendix; per-package copyright notices are listed inline.",
        "",
        "_Generated by `scripts/gen-third-party-notices.py` — regenerate when",
        "dependencies change. Do not edit by hand._",
        "",
    ]

    def section(title, pkgs):
        lines.append(f"## {title}")
        lines.append("")
        groups = defaultdict(list)
        for p in pkgs:
            groups[" AND ".join(p["elected"])].append(p)
        for fam in sorted(groups):
            lines.append(f"### {fam}")
            lines.append("")
            for p in sorted(groups[fam], key=lambda x: x["name"]):
                extra = f" (as {p['expr']})" if p["expr"] not in (fam, "") else ""
                lines.append(f"- **{p['name']}** {p['version']}{extra}")
                for c in p["copyrights"]:
                    lines.append(f"  - {c}")
            lines.append("")

    section("Rust crates (compiled into rift-tauri.exe)", rust_pkgs)
    section("npm packages (bundled into the UI)", npm_pkgs)

    lines.append("## Appendix — license texts")
    lines.append("")
    used = set()
    for p in rust_pkgs + npm_pkgs:
        used.update(p["elected"])
    for fam in sorted(used):
        body = EMBEDDED_TEXTS.get(fam) or texts.get(fam)
        lines.append(f"### {fam}")
        lines.append("")
        if body:
            lines.append("```text")
            lines.append(body)
            lines.append("```")
        else:
            lines.append(
                f"_Full text: <https://spdx.org/licenses/{fam.replace(' ', '-')}.html>_"
            )
        lines.append("")
    return "\n".join(lines) + "\n"


def main() -> int:
    rust_pkgs, rust_texts = collect_rust()
    npm_pkgs, npm_texts = collect_npm()
    texts = {**npm_texts, **rust_texts}
    md = render(rust_pkgs, npm_pkgs, texts)
    with open(OUT, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(md)
    missing_texts = sorted(
        {f for p in rust_pkgs + npm_pkgs for f in p["elected"]}
        - set(EMBEDDED_TEXTS)
        - set(texts)
    )
    print(f"Wrote {OUT}")
    print(f"  rust crates: {len(rust_pkgs)}  npm packages: {len(npm_pkgs)}")
    if missing_texts:
        print(
            f"  WARNING - no full text found for: {', '.join(missing_texts)} (SPDX link used)"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
