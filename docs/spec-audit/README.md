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

| # | Chapter | Extract | Verify | Skeptic |
|---|---|---|---|---|
| 1 | rm-common-change-control | pending | pending | pending |
| 2 | rm-ehr | pending | pending | pending |
| 3 | rm-composition | pending | pending | pending |
| 4 | rm-data-structures | pending | pending | pending |
| 5 | rm-data-types-text-quantity | pending | pending | pending |
| 6 | rm-data-types-rest | pending | pending | pending |
| 7 | rm-support | pending | pending | pending |
| 8 | rm-demographic | pending | pending | pending |
| 9 | rm-ehr-extract | pending | pending | pending |
| 10 | rm-integration | pending | pending | pending |
| 11 | base-foundation | pending | pending | pending |
| 12 | base-base-types | pending | pending | pending |
| 13 | am-aom14-opt | pending | pending | pending |
| 14 | am-aom2-adl2 | pending | pending | pending |
| 15 | term | pending | pending | pending |
| 16 | query-aql | pending | pending | pending |
| 17 | sm-platform | pending | pending | pending |
| 18 | sm-tdd | pending | pending | pending |
| 19 | its-rest-general | pending | pending | pending |
| 20 | its-rest-ehr-composition | pending | pending | pending |
| 21 | its-rest-query-definition-admin | pending | pending | pending |
| 22 | its-json | pending | pending | pending |
| 23 | its-xml | pending | pending | pending |
| 24 | cnf-cross-check | pending | pending | pending |
