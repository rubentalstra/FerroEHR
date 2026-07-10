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
- [x] 5. ADR reconciliation (owner directive): the ADR set carries layered
      amendments and contradictions (ADR-006 §3/§4 vs ADR-008; ADR-007's
      schema replaced; historical parity framing inside ADR-004/005; ADR-010
      packaging amended by ADR-011). Audit every ADR, fix status headers /
      supersession banners so each file states its current truth, and add a
      docs/ADRs/README.md index with a what-is-current table. ADR-013 (this
      phase's schema decision) lands consistent with the cleaned set.
      *Done 2026-07-10: ADR-010 supersession banner (packaging → ADR-011);
      CLAUDE.md + architecture.md conformance-script reverse-drift flipped;
      dated notes on ADR-008/012; docs/ADRs/README.md current-truth index.
      Audit verdict: 001–007/009/011 already correctly annotated.*

## Exit criteria

- [ ] ADR-013 accepted by the owner; schema implemented per it; workspace
      green; full ECC zero drift (341/315/0 baseline); blueprint current.
