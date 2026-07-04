# Phase 06 — ODIN + BMM reader

- Status: **done for codegen** (runtime ODIN parser moved to P13)
- Build order: complete (tooling foundation)
- Decisions: ADR-004

## Outcome

The **BMM reader** (`openehr-codegen::bmm`, hand-written tooling) loads the
vendored `*.bmm.json` meta-model that drives all spec-crate generation, and
`openehr-lang` (the runtime BMM/P_BMM object model) is generated from it. An
**ODIN reader** for the codegen path exists in `openehr-lang`.

The **runtime ODIN + ADL instance parsers** (for ingesting archetypes/templates
at runtime, not for codegen input) are **not** part of this phase — they belong
to **P13 (template ingestion)**, where ADL 1.4 / AOM 1.4 / OPT 1.4 XML parsing is
built.

## Verification

`cargo run -p openehr-codegen -- check` loads every vendored BMM schema;
`openehr-lang` builds clean.
