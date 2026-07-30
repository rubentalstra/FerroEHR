---
name: pause-after-767-for-rust-practices-update
description: "Owner hold-point 2026-07-30: after #767 closes, PAUSE the AM program — rust best practices + clippy settings are being updated in another session; re-read the rules before resuming"
metadata: 
  node_type: memory
  type: project
  originSessionId: 2112365c-704d-4141-82ea-d4ba3f4154d9
  modified: 2026-07-30T10:35:36.288Z
---

Owner directive 2026-07-30: finish #1308 and close #767 (ADL1.4 ch.5 cADL),
then **do NOT start #768 or any further chapter** until GitHub issue #1311
(the Rust best-practices + clippy settings update, worked in another
session) is CLOSED. Monitor it; no verbal go needed (owner revision
2026-07-30).

**How to apply:** when #1311 closes, FIRST study its changes (the issue +
its closing PRs) until the new strict practices are fully understood, and re-read
`.claude/rules/rust-style.md`, `.claude/rules/reliability.md`, the root
`Cargo.toml` `[workspace.lints]`, and any new/changed rules files — then
resume the v3.15.0 AM program (#768, ADL1.4 ch.6 — Assertions) under the new
conventions. Delete this memory once the resume has happened and the new
rules are internalized.

Related: [[spec-chapter-audit-programs]]
