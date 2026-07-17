# Memory index

- [Owner work style](owner-work-style.md) — defer nothing; no quick fixes (proper rewrites welcome); orchestrator codes context-heavy work itself; big-bang rewrites converge once at the end (no intermediate stubs); specs re-read first-hand over ADR claims; rerun ECC after runner/validation merges
- [Official CLI/tooling first](official-cli-tooling-first.md) — always use the official CLI (sqlx-cli etc.) for tool-managed artifacts; never hand-name/hand-roll
- [Commit-subject attribution tokens](commit-subject-attribution-tokens.md) — commit-msg hook deletes lines containing "Claude Code" etc.; avoid the literal in commit/PR text
- [Concurrent sessions share this tree](concurrent-sessions-shared-tree.md) — explicit-path commits, scoped gates, worktree-isolate subagents; ONE ./target for everything incl. the IDE (all isolation schemes retired 2026-07-16 after a 394GB fill; clean >30GB)
- [ECC: our own conformance framework](ecc-own-conformance-framework.md) — own numbering/taxonomy, generated data sets, latest-versions-only, no Robot/Python/legacy-CNF mapping ever
- [Verify crate versions from live sources](verify-crate-versions-live.md) — never pin from training data; lapin is 4.x (owner-corrected twice)
- [Autonomous phase flow](autonomous-phase-flow.md) — standing: PR+merge each phase, checkout develop, start next without asking; never branch while finished work sits unmerged

Cleaned 2026-07-12: stale/duplicative memories deleted — anything the repo
already records (CLAUDE.md, ADR-004/005/008, docs/spec-audit, the blueprint)
is not repeated here.
- [Max 2 concurrent workers](max-two-concurrent-workers.md) — owner cap: implementation subagents run in pairs, never wider
- [Session workflow gotchas](session-workflow-gotchas.md) — background-task ~30min kill (nohup+caffeinate+Monitor), attribution-hook regex traps, changelog-guard label needs PR reopen
- [Pre-production migrations: edit the baseline directly](pre-production-migrations-edit-baseline.md) — never append ALTER/DROP migrations while nothing is deployed; minimum migration files, update the count guard in the same change
- [No task IDs in code](no-task-ids-in-code.md) — F-nn/S-nn/G-nn/W-nn tracker markers banned from all code/doc comments; only docs/specs/openehr citations
- [Served OpenAPI is native](served-openapi-is-native.md) — ehrbase-rest serves ONLY its own utoipa-generated document; vendored ITS-REST OAS = codegen input + behavioural oracle, never imported/served; update our #[utoipa::path] in the same PR as any wire change
