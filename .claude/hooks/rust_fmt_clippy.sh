#!/usr/bin/env bash
# .claude/hooks/rust_fmt_clippy.sh
#
# Claude Code PostToolUse hook (matcher: Write|Edit).
# Formats an edited .rs file with rustfmt, then makes a best-effort scoped
# `cargo clippy --fix` pass on the owning crate.
#
# NEVER blocks and swallows all failures: during Phases P1-P16 the code is not
# required to compile (PORT_MASTER_PLAN.md section 4.1), so rustfmt failing to
# parse a draft or clippy failing to compile is expected and fine. Before P17
# the crate module trees are tiny, so the clippy attempt is cheap; after P17 it
# earns its keep.

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

# Walk up to the owning crate (a Cargo.toml directly under crates/).
dir="$(dirname "$file_path")"
while [ "$dir" != "/" ] && [ ! -f "$dir/Cargo.toml" ]; do
  dir="$(dirname "$dir")"
done
if [ -f "$dir/Cargo.toml" ] && [ "$(basename "$(dirname "$dir")")" = "crates" ]; then
  (cd "$dir" && cargo clippy --fix --allow-dirty --allow-staged --quiet >/dev/null 2>&1) || true
fi

exit 0
