# B7 — Enterprise-grade schema review & redesign plan

- Status: in-progress
- Started: 2026-07-10   Owner: Ruben
- Mission (owner directive): the DB schema (`app/ehrbase/migrations/{ehr,ext}`)
  grew accretively through P10→B6 (10 ehr migrations + 1 ext) and must become
  the best-possible, enterprise-grade foundation for a clinical data
  repository — spec-correct (DOCS-FIRST against `docs/specs/openehr/`),
  PG18-native, secure, operable. System logging and adjacent operational
  concerns in scope.
- Method: Opus research fan-out (current-schema map · spec persistence
  requirements · PG enterprise best practices via web) → Fable synthesis into
  an ADR + migration redesign plan → **owner confirms design questions before
  any rewrite** → implementation behind the standing gates (workspace green +
  full ECC zero drift; baseline 341 executed · 315 passed · 0 failed).
- Blueprint upkeep: §2 map State column refreshed (done/partial/missing) as
  part of this phase.

## Tasks

- [ ] 1. Research fan-out: (a) precise current-schema inventory + code-usage
      map; (b) openEHR persistence-requirement extraction with citations;
      (c) PostgreSQL 18 enterprise best-practice + security research (web).
- [ ] 2. Synthesis: gap analysis current-vs-required-vs-best-practice; design
      questions to the owner; ADR-013 schema redesign decision.
- [ ] 3. Implementation per the confirmed ADR (re-authored baseline or ordered
      migrations — owner decides), node codec/storage layer updates, tests.
- [ ] 4. Blueprint §2 State-column refresh (done/partial/missing) + chapter
      state tables.

## Exit criteria

- [ ] ADR-013 accepted by the owner; schema implemented per it; workspace
      green; full ECC zero drift (341/315/0 baseline); blueprint current.
