---
name: commit-subject-attribution-tokens
description: "The repo's commit-msg hook deletes any line containing \"Claude Code\" (and other attribution tokens) — keep those literals out of commit subjects/bodies entirely."
metadata: 
  node_type: memory
  type: project
  originSessionId: a2670f8c-322a-431c-b609-f598b3195b9c
---

The tracked `.githooks/commit-msg` hook in ferroehr strips whole lines matching attribution patterns, including the literal phrase "Claude Code" (case-insensitive), Co-authored-by lines, "Generated with/by", and the robot emoji. A legitimate subject like "register Claude Code hooks" gets its entire subject line deleted (observed 2026-07-02: commit landed body-only and needed `--amend`). A fully-matching single-line message strips to empty and aborts the commit.

**Why:** the hook is a hard no-attribution guarantee and cannot distinguish attribution from a technical mention of the tool name.

**How to apply:** in commit messages and PR text for this repo, write "agent harness", "hooks", or similar instead of the tool's literal name; never rely on the hook "knowing what you meant". The PreToolUse guard (`.claude/hooks/no_attribution_guard.sh`) blocks such commands up front in sessions started after it was registered.
