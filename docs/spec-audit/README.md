# A1 full spec audit — register index

Started 2026-07-11 on `claude/a1-spec-audit` (develop @ 717585c85, includes
the PARTY_SELF fix PR #69). Brief: `docs/plans/a1-spec-audit-PROMPT.md`.
Oracle: the vendored spec text at `docs/specs/openehr/` — every requirement
and finding cites file + section. A green test suite is not evidence.

## Methodology

Three audit phases, one agent per chapter per phase (multi-agent workflow,
durable per-chapter logging — each file written the moment it exists):

1. **Extract** (`<chapter>/requirements.md`) — read the whole chapter text;
   extract machine-checkable normative requirements (invariants, mandatory
   attributes + exact declared types, cardinality/occurrences, validity
   functions, mandated behaviours, and above all REJECTION DUTIES), numbered
   `<chapter>-R1…`, capped ~70/chapter keeping the highest-risk.
2. **Verify** (`<chapter>/verification.md`) — per requirement, find the
   ENFORCEMENT SITE in the code (a struct field existing does not prove
   runtime rejection). Classify: `verified` (file:line) / `defect` /
   `missing` / `partial` (some paths only) / `not-statically-checkable`
   (exact runtime probe named). Severity: critical (write-path integrity /
   missing mandated rejection) > major (conformance-visible) > minor.
   Records whether a negative test exists for every "must reject" rule.
3. **Skeptic** (`<chapter>/skeptic.md`) — adversarial refutation of every
   suspected defect/missing/partial: re-read the cited spec (misread?
   SHOULD vs MUST? relaxing clause?), re-read the code (enforced in another
   layer: derive, walker, REST extractor, DB constraint?). `refuted` needs
   concrete evidence; survivors = `confirmed`; statically undecidable =
   `uncertain` + exact probe.

Consolidated register: [`FINDINGS.md`](FINDINGS.md) — every confirmed defect
(id, severity, citation, location, fix status) + every uncertain finding
with its runtime probe.

## Chapter status

Extract completed 2026-07-11 (24/24 chapters, 1,126 requirements, 418
high-risk). Verification runs as separate per-chapter agents (owner ruling
2026-07-11: no monolithic 3-phase workflow — token discipline), highest
leniency-risk chapters first; the skeptic pass is the orchestrator's review
before a finding enters `FINDINGS.md`.

| # | Chapter | Extract (reqs / high-risk) | Verify | Skeptic |
|---|---|---|---|---|
| 1 | rm-common-change-control | done (56 / 13) | done — all fixed | orchestrator-reviewed |
| 2 | rm-ehr | done (41 / 13) | done — 8 fixed | orchestrator-reviewed |
| 3 | rm-composition | done (48 / 28) | done — 5 fixed (1 systemic) | orchestrator-reviewed |
| 4 | rm-data-structures | done (37 / 14) | done — 3 fixed | orchestrator-reviewed |
| 5 | rm-data-types-text-quantity | done (58 / 30) | done — 3 fixed + 1 tension noted | orchestrator-reviewed |
| 6 | rm-data-types-rest | done (45 / 20) | done — 2 fixed | orchestrator-reviewed |
| 7 | rm-support | done (40 / 17) | done — 4 fixed | orchestrator-reviewed |
| 8 | rm-demographic | done (42 / 20) | done — 3 fixed | orchestrator-reviewed |
| 9 | rm-ehr-extract | done (50 / 8) | done — 6 fixed | orchestrator-reviewed |
| 10 | rm-integration | done (20 / 6) | done — 1 fixed | orchestrator-reviewed |
| 11 | base-foundation | done (57 / 30) | done — 1 fixed | orchestrator-reviewed |
| 12 | base-base-types | done (37 / 11) | pending | pending |
| 13 | am-aom14-opt | done (56 / 28) | pending | pending |
| 14 | am-aom2-adl2 | done (70 / 18) | pending | pending |
| 15 | term | done (53 / 21) | pending | pending |
| 16 | query-aql | done (50 / 12) | pending | pending |
| 17 | sm-platform | done (56 / 24) | pending | pending |
| 18 | sm-tdd | done (47 / 7) | pending | pending |
| 19 | its-rest-general | done (40 / 10) | pending | pending |
| 20 | its-rest-ehr-composition | done (53 / 25) | pending | pending |
| 21 | its-rest-query-definition-admin | done (45 / 11) | pending | pending |
| 22 | its-json | done (42 / 12) | pending | pending |
| 23 | its-xml | done (60 / 29) | pending | pending |
| 24 | cnf-cross-check | done (23 / 11) | pending | pending |
