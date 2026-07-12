---
name: spec-audit-tracker
description: "Full-codebase openEHR spec audit (2026-07-06) — tracker location, what's fixed vs the 82 open findings"
metadata: 
  node_type: memory
  type: project
  originSessionId: 3a47c572-f8c4-4ffd-aa5c-5a002791eab8
---

2026-07-06: full 14-area spec audit vs the vendored openEHR specs, merged to
`develop` as PR #20 (squash). Tracker: `docs/spec-audit/SPEC_AUDIT.md`;
per-finding checkboxes in `docs/spec-audit/findings/`. 109/191 findings fixed
(all criticals + scheduled majors); **82 remain open** — mostly minors plus
deliberate deferrals: area 03 QUERY-execution items (P16 scope), area 05 XML
minors, area 07 spec-underdetermined AOM 1.4 decision points (need ADR + CNF
fixtures), area 13 openehr-flat builder refactors. ADR-009 records the
deliberate opt14↔am14 duplication (guarded by a divergence sentinel test).
Owner directives during this work: max 2 concurrent agents; full clean
rewrites over patches (see [[spec-adherence-mandate]], [[greenfield-pivot-adr-008]]).

2026-07-07: PR #21 merged (squash) — P16 AQL engine complete + three
owner-pulled-forward tracks: ATNA audit trail (ehrbase-audit crate,
docs/enterprise/atna-audit.md), GHCR container images
(docs/design/container-images.md; app + preconfigured PG18; Cargo.lock now
committed), full observability stack (docs/design/observability.md,
single-stage per owner directive — no deferred capability). Current phase:
P17 FLAT/EhrScape. Root Java residue deleted (LICENSE/NOTICE kept).
