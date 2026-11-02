#!/usr/bin/env python3
"""Prepend an SPDX + copyright header to every first-party Rust source file.

Idempotent: files that already carry an SPDX line in their first lines are left
untouched. The generated template (crates/frus-cli/template) is skipped so that
user-scaffolded apps never inherit the project copyright. Per-file newline style
(LF vs CRLF) is preserved.
"""
import sys
from pathlib import Path

ROOT = Path(sys.argv[1]).resolve()          # e.g. .../frus/crates
EXCLUDE_PARTS = {"target"}
EXCLUDE_DIRS = [ (ROOT / "frus-cli" / "template").resolve() ]

SPDX = "SPDX-License-Identifier: MIT OR Apache-2.0"
COPYRIGHT = "Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>"

changed, skipped_existing, skipped_template = 0, 0, 0

for path in ROOT.rglob("*.rs"):
    rp = path.resolve()
    if any(part in EXCLUDE_PARTS for part in rp.parts):
        continue
    if any(str(rp).startswith(str(d)) for d in EXCLUDE_DIRS):
        skipped_template += 1
        continue

    raw = path.read_bytes()
    text = raw.decode("utf-8")

    # already headered?
    head = "\n".join(text.splitlines()[:5])
    if "SPDX-License-Identifier" in head:
        skipped_existing += 1
        continue

    nl = "\r\n" if "\r\n" in text else "\n"
    header = f"// {SPDX}{nl}// {COPYRIGHT}{nl}{nl}"

    lines = text.split(nl)
    # Preserve a true shebang on line 1 (rare in .rs), header goes right after.
    if lines and lines[0].startswith("#!") and not lines[0].startswith("#!["):
        new_text = lines[0] + nl + header + nl.join(lines[1:])
    else:
        new_text = header + text

    path.write_bytes(new_text.encode("utf-8"))
    changed += 1

print(f"headers added:      {changed}")
print(f"already had SPDX:   {skipped_existing}")
print(f"template skipped:   {skipped_template}")
