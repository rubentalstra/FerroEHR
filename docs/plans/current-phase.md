# Current phase

**The roadmap is `docs/blueprint/00-THE-BLUEPRINT.md`** — read it first. It is
the single source of truth for the trajectory toward "first fully
spec-compliant openEHR CDR". This file is the live pointer under it; the
consolidated gap surface is the blueprint §2 (proven foundations + ECC
breakdown + spec-area map).

## Active work — B2: Validation depth

**Phase file: `docs/plans/b2-validation-depth.md`** (branch
`claude/b2-validation-depth`). The single biggest gap: **81 failing ECC
ArchetypeValidation cases (~76 % of all failures)** — template/archetype
constraint-validation depth, with the ECC data sets as the oracle. Contents in
dependency order (blueprint §3 B2): constraint-evaluation primitives
(`Multiplicity_interval`/`Cardinality`/`Interval` `*_impl.rs`), closed-world
ADR, slot enforcement, leaf completion (temporal/precision/ordinal/C_STRING),
BMM type conformance, ingestion-side artefact validity (AOM2 codes), commit
path guards (`is_modifiable`, 553 lifecycle, identifier equality, `Day_valid`),
spec-audit area-07/12 reconciliation.

**B1 closed 2026-07-09** (PR #36): ADR-011 rebuild converged; ECC re-baselined
at **211/318, zero drift**; conformance runs only against the Docker-composed
server (`scripts/conformance.sh` — the in-process self-host mode was removed).
From here every phase ends with an ECC run showing zero drift; the baseline
only ratchets upward (blueprint §4 rule 4).

## Priority order (from the blueprint build order, §3)

1. **B2 — ArchetypeValidation depth** (this phase).
2. B3 — SM-4 wave 3 Admin dump/load → SM-5 (Message / EHR Extract / TDD) →
   SM-6 (Subject Proxy) — designed in `docs/design/sm-platform/`.
3. B4 — terminology-server integration + its test harness.
4. B5 — the conformance-instrument corrections (ch 7 D1–D5).
5. B6/P19 — full conformance; then P20 optimization, P99 cutover.

**Read first:** `docs/blueprint/00-THE-BLUEPRINT.md`, then
`docs/plans/b2-validation-depth.md` and
`docs/ADRs/ADR-011-app-crate-redesign.md` (current app-crate reality) +
`docs/ADRs/ADR-008-greenfield-pg18-storage.md` (own PG18 internals).
