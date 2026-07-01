#!/usr/bin/env bash
# .claude/hooks/protect_java.sh
#
# Claude Code PreToolUse hook (matcher: Write|Edit).
# Protects the Java reference implementation during the port:
#   1. Blocks edits to Maven build files: pom.xml, mvnw, mvnw.cmd, anything
#      under .mvn/ (read-only until P99 cutover).
#   2. Blocks edits to any .java file that has no completed Rust counterpart
#      beside it (same directory, snake_case basename, containing a
#      "PORT STATUS" trailer).
#
# Reads the tool-call JSON on stdin. Exit 2 blocks the tool call and returns
# the stderr text to Claude. Exit 0 allows it.

set -euo pipefail

payload="$(cat)"

if command -v jq >/dev/null 2>&1; then
  file_path="$(printf '%s' "$payload" | jq -r '.tool_input.file_path // empty' 2>/dev/null || true)"
else
  file_path="$(printf '%s' "$payload" | sed -n 's/.*"file_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)"
fi
[ -n "${file_path:-}" ] || exit 0

base="$(basename "$file_path")"

# 1. Maven build files are read-only reference for the whole port.
case "$base" in
  pom.xml | mvnw | mvnw.cmd)
    echo "BLOCKED: '$file_path' is a Maven build file. pom.xml/mvnw/.mvn are read-only reference during the port and are deleted only at P99 (CLAUDE.md hard rule)." >&2
    exit 2
    ;;
esac
case "$file_path" in
  */.mvn/* | .mvn/*)
    echo "BLOCKED: '$file_path' is under .mvn/. Maven build files are read-only reference during the port (CLAUDE.md hard rule)." >&2
    exit 2
    ;;
esac

# 2. Java sources are read-only until a completed Rust counterpart sits beside them.
case "$base" in
  *.java)
    dir="$(dirname "$file_path")"
    stem="${base%.java}"
    # CamelCase -> snake_case (AqlSqlLayer.java -> aql_sql_layer.rs)
    snake="$(printf '%s' "$stem" | sed -E 's/([a-z0-9])([A-Z])/\1_\2/g; s/([A-Z]+)([A-Z][a-z])/\1_\2/g' | tr '[:upper:]' '[:lower:]')"
    rs="$dir/$snake.rs"
    if [ -f "$rs" ] && grep -q "PORT STATUS" "$rs" 2>/dev/null; then
      exit 0 # counterpart complete; edits (e.g. pre-deletion tidy) allowed
    fi
    echo "BLOCKED: '$file_path' has no completed Rust counterpart ('$snake.rs' with a PORT STATUS trailer) in the same directory. The Java reference is read-only until its port is complete (CLAUDE.md hard rule). Port it first; never edit the reference." >&2
    exit 2
    ;;
esac

exit 0
