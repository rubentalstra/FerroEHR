---
name: verify-crate-versions-live
description: Always verify crate versions/APIs from live sources before pinning — training data is wrong (e.g. lapin is 4.x not 2.x)
metadata: 
  node_type: memory
  type: feedback
  originSessionId: df0992bc-dba7-497a-b3b2-8614868c0600
---

When adding any new Rust dependency, verify the current version and API from
a live source first (`cargo add --dry-run <crate>`, `cargo search`, or
docs.rs) — never pin from training-data memory.

**Why:** The owner has corrected wrong version assumptions twice; concretely
lapin is at **v4.10.0** (2026-07), not the 2.x that training data suggests —
and major-version API drift follows. The CLAUDE.md stack list also marks
several pins *(verify)* for exactly this reason.

**How to apply:** Before writing a `Cargo.toml` line or telling a subagent a
version, run the dry-run check; instruct subagents adding deps to do the
same and to report the exact versions pinned. Related: [[official-cli-tooling-first]].

**The rule covers CAPABILITIES, not only versions (2026-08-24 incident).**
"Sonar's Rust support is limited" went into issue #2630 from training-data
memory and was flat wrong — SonarSource shipped official Rust support
(2025-04-17: 85 first-party Clippy rules, LCOV/Cobertura coverage, Rust
1.0–1.92). Any claim about what a third-party tool or service supports is
checked against its live official docs before it lands in an issue, a rule,
or prose — same law as crate versions.
