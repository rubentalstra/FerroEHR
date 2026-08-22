---
name: default-style-guard-untracked-blindspot
description: the default-style guard's single-reader DEFAULT_* rule runs only under --all and only over git-tracked files, so new untracked files pass locally and fail in CI
metadata:
  type: project
---

`scripts/checks/default-style.sh` rule (3) — *"`const DEFAULT_<X>` with
exactly ONE reference"* — is gated on `[ "${1:-}" = "--all" ]` and enumerates
declarations with `git grep`. Consequences when reviewing a branch whose files
are still **untracked**:

- a per-file invocation (`default-style.sh path/to/new.rs`) reports **OK** —
  rule (3) never runs;
- `--all` cannot see the file either, because `git grep` skips untracked paths.

So a new private `const DEFAULT_FOO` with one reader passes every local check
and fails the `default-style` CI job the moment the file is committed.
Counting is exact and easy to reproduce for a PRIVATE const:
`grep -cow DEFAULT_FOO file.rs` minus 1 must be `> 1`.

**How to apply:** on any review of new/untracked Rust, grep for
`const DEFAULT_` yourself and count readers — do not trust a green
per-file guard run. Renaming away from the `DEFAULT_` prefix (the tree already
does this: `SEEDED_RM_VERSION`) or inlining the value both satisfy it.
