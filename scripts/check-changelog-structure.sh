#!/usr/bin/env bash
# Changelog structure guard (Keep a Changelog 1.1.0).
#
# Fails when any release section of CHANGELOG.md (including [Unreleased])
# violates the Keep a Changelog structure:
#   1. a duplicated `### <Type>` subsection inside one release section
#      (each type appears at most once per section — new entries merge into
#      the existing subsection, they never append a second header);
#   2. a subsection header outside the canonical type set
#      (Added / Changed / Deprecated / Removed / Fixed / Security).
#
# Wired into the CI changelog-guard job; runs unconditionally (the
# `no-changelog` escape label waives the entry REQUIREMENT, never the
# structural validity of the file).
set -euo pipefail

file="${1:-CHANGELOG.md}"

python3 - "$file" <<'PY'
import re, sys

CANON = {"Added", "Changed", "Deprecated", "Removed", "Fixed", "Security"}
path = sys.argv[1]
lines = open(path, encoding="utf-8").read().split("\n")

errors = []
section = None
seen: set[str] = set()
for n, line in enumerate(lines, 1):
    m = re.match(r"^## \[([^\]]+)\]", line)
    if m:
        section = m.group(1)
        seen = set()
        continue
    m = re.match(r"^### (.+?)\s*$", line)
    if m and section is not None:
        sub = m.group(1)
        if sub not in CANON:
            errors.append(
                f"{path}:{n}: '### {sub}' in [{section}] is not a "
                f"Keep-a-Changelog type (allowed: {', '.join(sorted(CANON))})"
            )
        if sub in seen:
            errors.append(
                f"{path}:{n}: duplicate '### {sub}' in [{section}] — merge "
                f"the entry into the existing subsection instead of adding "
                f"a second header"
            )
        seen.add(sub)

if errors:
    print("changelog structure check FAILED:", file=sys.stderr)
    for e in errors:
        print(f"  {e}", file=sys.stderr)
    sys.exit(1)
print(f"changelog structure OK ({path})")
PY
