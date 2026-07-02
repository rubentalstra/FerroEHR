#!/usr/bin/env bash
# .claude/hooks/phase_gate.sh
#
# Claude Code Stop hook: blocks ending a session in which no phase checkbox was
# ticked and no commit was made (CLAUDE.md six-step loop, steps 4-5).
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

# No commit yet: allow the stop only if a checkbox was ticked in the working tree.
if git diff HEAD -- docs/plans 2>/dev/null | grep -qE '^\+.*- \[x\]'; then
  exit 0
fi

echo "phase gate: no phase checkbox was ticked and no commit was made this session. Follow the six-step loop (CLAUDE.md): tick the finished task in the current phase file under docs/plans/ and commit as 'phase-NN: <task>' on a claude/* branch. If this session was purely informational, stop again to end anyway." >&2
exit 2
