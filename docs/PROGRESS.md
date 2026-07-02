# Progress

One row per phase, updated when a phase's exit criteria are met (see
`docs/plans/current-phase.md` for the live pointer and the per-phase files
under `docs/plans/` for detail). Status values: `not-started`, `in-progress`,
`blocked`, `done`.

| Phase | Title | Status | Note |
|---|---|---|---|
| P0 | Scaffolding | done | 13-crate workspace green (build/fmt/clippy/deny); 428 Java files relocated per §9.1; harness + hooks live; reference/v1 = v0.32.0; archaeology recorded. Research dossier texts still to be committed. |
| P1 | Foundation + Identification (BASE 1.2.0) | done | 69 classes transcribed (foundation 39, base 30), trailers verified, ADR-001 shapes fixed; spec cached at Release-1.2.0 @ 9064413. |
| P2 | Terminology bundle (TERM 3.x) + service API | not-started | |
| P3 | RM transcription | not-started | data_types → data_structures → common → ehr → demographic → integration. |
| P4 | Canonical JSON serialization (ITS-JSON) | not-started | insta golden vectors. |
| P5 | Canonical XML serialization (ITS-XML) | not-started | 1.0.2 + 2.0.0. |
| P6 | REST skeleton (axum) | not-started | Every ITS-REST 1.0.3 endpoint + admin + item tags. |
| P7 | Persistence schema | not-started | Flyway migrations copied verbatim; sea-query tables; testcontainers PG18. |
| P8 | ODIN + BMM parsers | not-started | |
| P9 | ADL 1.4 + AOM 1.4 + OPT 1.4 XML | not-started | ADL 2/AOM 2 in parallel behind `adl2`. |
| P10 | WebTemplate builder | not-started | OptVisitor equivalent. |
| P11 | Composition validation | not-started | ValidationWalker equivalent + terminology binding. |
| P12 | AQL parser + AST + semantic path analysis | not-started | |
| P13 | AQL engine: AST → ASL → SQL | not-started | |
| P14 | rm-db-format bridge | not-started | RM ↔ decomposed row-per-locatable. |
| P15 | Service layer | not-started | Orchestration, transactions, versioning, contributions, audit. |
| P16 | FLAT / STRUCTURED / Web Template + EhrScape | not-started | |
| P17 | Make it compile | not-started | Drive `cargo check` to zero across the workspace, leaf-first. |
| P18 | Test parity | not-started | Target ≥99% parity on Linux x86_64 first. |
| P19 | Optimization | not-started | PG18 AIO tuning, hot-read pipelining, JSON_TABLE codegen. |
| P99 | Cutover | not-started | Delete remaining ported-out Java and residual Maven config; tag first pure-Rust release. |
