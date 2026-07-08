#!/usr/bin/env bash
# .claude/hooks/rust_fmt_clippy.sh
#
# Claude Code PostToolUse hook (matcher: Write|Edit).
# Formats an edited .rs file with rustfmt. Never blocks; swallows all failures
# (rustfmt failing to parse a draft is expected and fine).
#
# NOTE: this hook used to also run a scoped `cargo clippy --fix` on the owning
# crate after every edit. That was removed 2026-07-08: with the full workspace
# built, it triggered a check-build of the crate + its dependency cone per
# file edit — and because it ran per-package (from the crate dir), resolver-v3
# feature unification differed from the workspace build, invalidating shared
# artifacts and thrashing the cargo cache (target/ grew past 190 GB; dev
# builds hit 10+ minutes). Clippy remains a per-phase gate the agent runs
# explicitly (`cargo clippy --workspace --all-targets`), not a per-edit hook.

set -uo pipefail

payload="$(cat)" || true

if command -v jq >/dev/null 2>&1; then
  file_path="$(printf '%s' "$payload" | jq -r '.tool_input.file_path // empty' 2>/dev/null)" || true
else
  file_path="$(printf '%s' "$payload" | sed -n 's/.*"file_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)"
fi

case "${file_path:-}" in
*.rs) ;;
*) exit 0 ;;
esac
[ -f "$file_path" ] || exit 0

rustfmt --edition 2024 "$file_path" >/dev/null 2>&1 || true

exit 0
