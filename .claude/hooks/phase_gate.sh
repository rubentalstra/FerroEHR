#!/usr/bin/env bash
# .claude/hooks/phase_gate.sh
#
# Claude Code Stop hook: blocks ending a session in which the worklist was not
# touched and no commit was made (CLAUDE.md worklist workflow).
#
# Uses .claude/.session-start-head written by inject_phase_context.sh at
# SessionStart. Exit 2 blocks the stop once; a second stop attempt (with
# stop_hook_active=true) is allowed through so purely informational sessions
# can still end.

set -uo pipefail

payload="$(cat)" || true

# Do not loop: if we already blocked once this stop, let the session end.
if printf '%s' "$payload" | grep -q '"stop_hook_active"[[:space:]]*:[[:space:]]*true'; then
  exit 0
fi

root="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
cd "$root" || exit 0

marker=".claude/.session-start-head"
[ -f "$marker" ] || exit 0 # no baseline recorded; cannot judge — do not nag

head_now="$(git rev-parse HEAD 2>/dev/null || true)"
[ -n "$head_now" ] || exit 0

if [ "$(cat "$marker")" != "$head_now" ]; then
  exit 0 # at least one commit was made this session
fi

# No commit yet: allow the stop only if the worklist/plans were edited in the
# working tree (a row updated, an item recorded).
if ! git diff HEAD --quiet -- docs/plans 2>/dev/null; then
  exit 0
fi

echo "worklist gate: no commit was made and docs/plans/WORKLIST.md was not touched this session. Follow the worklist workflow (CLAUDE.md): record/close the worklist row for what you did and commit on a claude/* branch. If this session was purely informational, stop again to end anyway." >&2
exit 2
