#!/usr/bin/env bash
#
# Derive the per-release Artifact Hub annotations from CHANGELOG.md, so the hub
# listing has ONE source of truth rather than a second changelog that drifts.
#
# The hub's change `kind` values — added, changed, deprecated, removed, fixed,
# security — ARE Keep a Changelog's subsection names
# (https://artifacthub.io/docs/topics/annotations/helm/), which this project
# already maintains and already guards (scripts/checks/changelog-structure.sh),
# so the mapping is one-to-one and needs no editorial decision.
#
# Usage:
#   deploy/helm/artifacthub-changes.sh <version|Unreleased>   # print the YAML
#   deploy/helm/artifacthub-changes.sh 3.17.4 --inject        # write into Chart.yaml
#
# Emits `artifacthub.io/changes` and, when the section has a `### Security`
# subsection, `artifacthub.io/containsSecurityUpdates: "true"`.
#
# WHY the whole release section rather than only the chart's own entries: a chart
# version is published at a release and its `appVersion` names that release, so
# what a consumer is deciding about is the release the chart deploys. There is no
# machine-readable way to split "chart change" from "server change" in the
# changelog, and guessing from wording would silently drop entries.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
CHANGELOG="${REPO_ROOT}/CHANGELOG.md"
CHART="${SCRIPT_DIR}/ferroehr/Chart.yaml"

VERSION="${1:-Unreleased}"
INJECT=0
[[ "${2:-}" == "--inject" ]] && INJECT=1

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is required (CHANGELOG parsing + YAML emission)" >&2
  exit 1
}

RENDERED="$(python3 - "$CHANGELOG" "$VERSION" <<'PY'
import re, sys

changelog, version = sys.argv[1], sys.argv[2]
text = open(changelog, encoding="utf-8").read()

# The section for this version: from its own `## [x]` heading to the next one.
heading = re.compile(r"^## \[([^\]]+)\]", re.M)
spans = [(m.group(1), m.start(), m.end()) for m in heading.finditer(text)]
body = None
for i, (name, _start, end) in enumerate(spans):
    if name.lower() == version.lower():
        stop = spans[i + 1][1] if i + 1 < len(spans) else len(text)
        body = text[end:stop]
        break
if body is None:
    names = ", ".join(n for n, _, _ in spans[:6])
    sys.exit(f"no '## [{version}]' section in CHANGELOG.md (found: {names})")

KINDS = {"added", "changed", "deprecated", "removed", "fixed", "security"}

def plain(md: str) -> str:
    """Markdown down to the plain text the hub renders."""
    md = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", md)   # links → their text
    # chr(96) is a backtick. Spelled this way because bash parses a heredoc
    # nested in $( ) far enough to trip over an unbalanced one.
    md = md.replace("**", "").replace(chr(96), "")
    return re.sub(r"\s+", " ", md).strip()

def summary(entry: str) -> str:
    """The entry's own one-line summary.

    This changelog's house style opens most entries with a bolded claim, and that
    IS the summary its author wrote — so it is used verbatim when present. Only
    the remaining entries fall back to a first-sentence split, and that split
    ignores periods inside brackets, because the text is full of them
    (`EVENT.offset`, `(POST /admin/…)`)."""
    flat = plain(entry)
    bold = re.match(r"\*\*(.+?)\*\*", entry.strip(), re.S)
    if bold:
        return plain(bold.group(1))
    depth = 0
    for i, ch in enumerate(flat):
        if ch in "([":
            depth += 1
        elif ch in ")]":
            depth = max(0, depth - 1)
        elif ch in ".!?" and depth == 0 and i + 1 < len(flat) and flat[i + 1] == " ":
            return flat[: i + 1]
    return flat

# Entries are hard-wrapped: a bullet continues on any following two-space-indented
# line until a blank line, the next bullet, or the next subsection. Reading only
# the first physical line truncates the summary mid-sentence.
changes, has_security = [], False
kind, current = None, None

def flush():
    global current
    if current is not None:
        changes.append((kind, current))
        current = None

for line in body.split("\n"):
    sub = re.match(r"^### (\w+)", line)
    if sub:
        flush()
        kind = sub.group(1).lower()
        if kind not in KINDS:
            sys.exit(f"'### {sub.group(1)}' is not an Artifact Hub change kind")
        has_security = has_security or kind == "security"
        continue
    if kind is None:
        continue
    if line.startswith("- "):
        flush()
        current = line[2:]
    elif current is not None and line.startswith("  ") and line.strip():
        current += " " + line.strip()
    elif not line.strip():
        flush()
flush()

if not changes:
    sys.exit(f"the '## [{version}]' section lists no changes")

def quote(s: str) -> str:
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'

out = ["  artifacthub.io/changes: |"]
for kind, entry in changes:
    out.append(f"    - kind: {kind}")
    out.append(f"      description: {quote(summary(entry))}")
if has_security:
    out.append('  artifacthub.io/containsSecurityUpdates: "true"')
print("\n".join(out))
PY
)"

if [[ "$INJECT" -eq 0 ]]; then
  printf '%s\n' "$RENDERED"
  exit 0
fi

# Append under the existing `annotations:` mapping. Refuses rather than
# duplicating a key: a second `artifacthub.io/changes` would make the packaged
# chart's metadata depend on YAML key-collision behaviour.
grep -q '^annotations:' "$CHART" || { echo "no 'annotations:' block in ${CHART}" >&2; exit 1; }
if grep -q 'artifacthub.io/changes:' "$CHART"; then
  echo "${CHART} already carries artifacthub.io/changes — it is injected at package time, not committed" >&2
  exit 1
fi
printf '%s\n' "$RENDERED" >> "$CHART"
echo "injected the ${VERSION} changes into ${CHART}" >&2
