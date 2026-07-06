#!/usr/bin/env bash
# .claude/hooks/protect_vendored_specs.sh
#
# Claude Code PreToolUse hook (matcher: Write|Edit|NotebookEdit). Blocks
# hand-edits to the vendored openEHR spec text under docs/specs/openehr/ —
# it is upstream-verbatim reference material (the conformance oracle), only
# ever refreshed by scripts/vendor-spec-docs.sh. The single exception is the
# top-level README.md (our own index).
#
# Reads the tool-call JSON on stdin. Exit 2 blocks; exit 0 allows.

set -euo pipefail

payload="$(cat)"

if command -v jq >/dev/null 2>&1; then
  path="$(printf '%s' "$payload" | jq -r '.tool_input.file_path // .tool_input.notebook_path // empty' 2>/dev/null || true)"
else
  path="$payload"
fi
[ -n "${path:-}" ] || exit 0

case "$path" in
  */docs/specs/openehr/README.md | docs/specs/openehr/README.md)
    exit 0
    ;;
  */docs/specs/openehr/* | docs/specs/openehr/*)
    echo "BLOCKED: docs/specs/openehr/** is vendored upstream openEHR spec text (the conformance oracle) and must never be hand-edited. Re-vendor with scripts/vendor-spec-docs.sh; pins live in that script + docs/VERSIONS.md." >&2
    exit 2
    ;;
esac

exit 0
