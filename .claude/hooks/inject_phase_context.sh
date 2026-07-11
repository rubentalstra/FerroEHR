#!/usr/bin/env bash
# .claude/hooks/inject_phase_context.sh
#
# Claude Code SessionStart hook: prints the current phase pointer, git status,
# and the last 10 commits so every session starts oriented on the six-step loop
# (CLAUDE.md). Also records the session-start HEAD so phase_gate.sh (Stop hook)
# can tell whether a commit was made during the session.

set -uo pipefail

root="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
cd "$root" || exit 0

mkdir -p .claude
git rev-parse HEAD >.claude/.session-start-head 2>/dev/null || true

echo "=== spec oracle ==="
echo "Vendored openEHR spec text + CNF test schedule: docs/specs/openehr/ (index: its README.md; use /spec-lookup). Implement and review all spec-facing behaviour against that text — never from memory or EHRbase behaviour alone (.claude/rules/spec-adherence.md)."
echo
echo "=== current phase (docs/plans/current-phase.md) ==="
cat docs/plans/current-phase.md 2>/dev/null || echo "(docs/plans/current-phase.md missing — read docs/blueprint/00-THE-BLUEPRINT.md)"
echo
echo "=== git status ==="
git status --short --branch 2>/dev/null | head -40
echo
echo "=== last 10 commits ==="
git log --oneline -10 2>/dev/null

exit 0
