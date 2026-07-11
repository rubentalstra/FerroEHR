#!/usr/bin/env bash
# .claude/hooks/block_dangerous.sh
#
# Claude Code PreToolUse hook (matcher: Bash). Blocks destructive commands:
#   - rm -rf / rm -fr (delete specific files, use git rm, or work under /tmp)
#   - force-pushes touching main/master/develop, and bare force-pushes
#   - deletion of docs/plans/current-phase.md or README.md (the live pointer + guide)
#   - deletion of the read-only reference/v1 ref (Stage 2 archaeology source)
#
# Reads the tool-call JSON on stdin. Exit 2 blocks; exit 0 allows.

set -euo pipefail

payload="$(cat)"

if command -v jq >/dev/null 2>&1; then
  cmd="$(printf '%s' "$payload" | jq -r '.tool_input.command // empty' 2>/dev/null || true)"
else
  cmd="$payload"
fi
[ -n "${cmd:-}" ] || exit 0

# rm with both -r and -f (combined or separate flags), unless scoped to /tmp.
if printf '%s' "$cmd" | grep -qE '(^|[;&|[:space:]])rm[[:space:]]+-[a-zA-Z]*([rR][a-zA-Z]*f|f[a-zA-Z]*[rR])' ||
  printf '%s' "$cmd" | grep -qE '(^|[;&|[:space:]])rm[[:space:]]+(-[a-zA-Z]+[[:space:]]+)*-[a-zA-Z]*[rR][a-zA-Z]*([[:space:]]+-[a-zA-Z]+)*[[:space:]]+-[a-zA-Z]*f'; then
  if ! printf '%s' "$cmd" | grep -qE 'rm[[:space:]]+-[a-zA-Z]+[[:space:]]+"?(/private)?/tmp/'; then
    echo "BLOCKED: 'rm -rf' is not allowed (block_dangerous hook). Delete specific files with 'git rm' or 'rm <file>', or operate under /tmp." >&2
    exit 2
  fi
fi

# Force pushes: never to main/master/develop; bare force-pushes refused too.
if printf '%s' "$cmd" | grep -qE 'git[[:space:]]+push[^;|&]*(--force([^-]|$)|--force-with-lease|[[:space:]]-f([[:space:]]|$)|[[:space:]]\+[[:alnum:]])'; then
  if printf '%s' "$cmd" | grep -qE '(main|master|develop)'; then
    echo "BLOCKED: force-push touching main/master/develop is forbidden (CLAUDE.md hard rule)." >&2
    exit 2
  fi
  if ! printf '%s' "$cmd" | grep -q 'claude/'; then
    echo "BLOCKED: bare force-push refused. Force-push (prefer --force-with-lease) only an explicit claude/* branch." >&2
    exit 2
  fi
fi

# Never delete the live phase pointer or the plans guide. Completed phase files
# may be pruned once their close is recorded in docs/PROGRESS.md.
if printf '%s' "$cmd" | grep -qE '(^|[;&|[:space:]])(git[[:space:]]+rm|rm)[^;|&]*docs/plans/(current-phase\.md|README\.md)'; then
  echo "BLOCKED: docs/plans/current-phase.md and docs/plans/README.md are the live pointer + guide and must not be deleted. Completed phase files may be pruned once recorded in docs/PROGRESS.md." >&2
  exit 2
fi

# Never delete the read-only v1 reference ref.
if printf '%s' "$cmd" | grep -qE 'git[[:space:]]+(branch[[:space:]]+(-D|-d|--delete)[^;|&]*reference/v1|update-ref[[:space:]]+-d[[:space:]]+refs/heads/reference/v1|push[^;|&]*:[[:space:]]*reference/v1)'; then
  echo "BLOCKED: reference/v1 is the read-only pre-v2 reference (consulted only in Stage 2). It must not be deleted or overwritten." >&2
  exit 2
fi

exit 0
