# Memory index

- [Greenfield pivot (ADR-008)](greenfield-pivot-adr-008.md) — 2026-07-05: own PG18 storage design, openEHR spec conformance replaces EHRbase parity; EHRbase = reference only

- [Official CLI/tooling first](official-cli-tooling-first.md) — always use the official CLI (sqlx-cli etc.) for tool-managed artifacts; never hand-name/hand-roll
- [Commit-subject attribution tokens](commit-subject-attribution-tokens.md) — commit-msg hook deletes lines containing "Claude Code" etc.; avoid the literal in commit/PR text
- [Codegen pivot & crate naming](codegen-pivot-and-crate-naming.md) — generate spec crates from BMM (not hand-transcribe); openehr-*=spec / ehrbase-*=app naming split
- [Codegen crate generation state](codegen-crate-generation-state.md) — what's generated (base, am14+am24), version pins, emitter behaviors, what's left
- [Spec-adherence mandate](spec-adherence-mandate.md) — user demands 100% spec/CNF adherence; vendored oracle at docs/specs/openehr + /spec-lookup + /spec-audit
- [Spec-audit tracker](spec-audit-tracker.md) — 2026-07-06 full audit merged (PR #20); docs/spec-audit/SPEC_AUDIT.md; 82 findings still open (P16-scope + deferred)
- [Concurrent sessions share this tree](concurrent-sessions-shared-tree.md) — explicit-path commits, scoped gates, worktree-isolate subagents
- [Per-agent CARGO_TARGET_DIR](per-agent-cargo-target-dir.md) — owner rule 2026-07-12: concurrent cargo uses fixed lanes target/agent-t1..t4 only; target-cli retired; >30 GB target → cargo clean
- [ECC: our own conformance framework](ecc-own-conformance-framework.md) — 2026-07-08: ECC catalogue replaces legacy-CNF mapping; own numbering/taxonomy, generated data sets, latest-versions-only, no Robot/Python ever
- [Verify crate versions from live sources](verify-crate-versions-live.md) — never pin from training data; lapin is 4.x (owner-corrected twice)
- [Autonomous phase flow](autonomous-phase-flow.md) — standing: PR+merge each phase, checkout develop, start next (E1→E5) without asking
- [A1 audit cadence](a1-audit-cadence.md) — one agent at a time, per chapter, fix everything in the file
- [A1 audit complete](a1-audit-complete.md) — merged PR #70 2026-07-12; ECC 341/315/0 re-baselined; H1 ADR-sweep follow-up; rerun ECC after runner/validation merges
- [Specs over ADRs](specs-over-adrs.md) — openEHR specs are leading; ADRs may be wrong, re-verify any spec-facing ADR claim
