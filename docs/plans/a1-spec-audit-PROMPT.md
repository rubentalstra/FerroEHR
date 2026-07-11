# A1 — Full spec audit: the launch prompt

This file IS the prompt for the A1 full-spec-audit session. Kick it off from a
fresh Fable 5 session with the one-liner at the bottom; the session then reads
this file and executes it verbatim. (Authored 2026-07-11 after the X1.2
upstream diff exposed the PARTY_SELF leniency gap.)

---

Run the FULL openEHR spec audit of ehrbase-rs (repo root: the current working
directory) as a multi-agent workflow — use the Workflow tool to orchestrate
it. This is a deep, chapter-by-chapter audit of the ENTIRE vendored spec
surface against the actual Rust code, not a fast scan. Token cost is
accepted; correctness and durable logging are the priorities.

## Why (context)

The project claims openEHR conformance (own ECC suite: 341 executed · 315
passed · 0 failed) — yet diffing against upstream EHRbase Java exposed a real
RM violation the suite never caught: our server accepted
`EHR_STATUS.subject` typed `PARTY_IDENTIFIED` where RM ehr master04 mandates
`PARTY_SELF` (a monomorphic slot). The audit's job is to find every remaining
gap of that kind: places where the code is MORE LENIENT than the spec
(accepts what must be rejected), LESS capable (mandated behaviour missing),
or WRONG (different semantics). A green test suite is NOT evidence — leniency
hides in openehr-derive (de)serialization, the validation walker,
hand-written `*_impl.rs` files, service logic in `app/`, and the REST layer.
Generated crates (`// @generated`) encode structure only.

## Ground rules (repo hard rules — non-negotiable)

- The vendored spec text at `docs/specs/openehr/` is the ONLY authority.
  Never answer from memory or from EHRbase behaviour. Every requirement and
  finding cites file + section.
- Never hand-edit a `// @generated` file (fix the emitter in
  `crates/openehr-codegen` and regenerate).
- Never weaken, skip, or delete a test. Never edit an ECC case to route
  around a finding.
- No AI/Claude attribution in any commit, PR, or file. Branches are
  `claude/*`.
- ONE cargo command at a time in the main session; any agent that must build
  uses an isolated `CARGO_TARGET_DIR` under /tmp and deletes it afterwards.
  Audit-phase agents are READ-ONLY on code.
- Work on branch `claude/a1-spec-audit` cut from up-to-date `develop`. Read
  `docs/plans/x1-comparison.md` and
  `docs/conformance/upstream-ehrbase/TRIAGE.md` first for the PARTY_SELF
  background; check whether branch `claude/x1-upstream-run` (or develop)
  already fixed it — do not duplicate that work.

## DURABLE LOGGING — the most important process requirement

Every agent's findings must be persisted to disk the moment they exist, so
nothing is lost to context limits or crashes. Structure (all files committed
on the audit branch, incrementally — commit after each phase, not only at the
end):

```
docs/spec-audit/README.md                  index: methodology, date, per-chapter status table
docs/spec-audit/<chapter>/requirements.md  Phase-1 output: numbered requirements + citations
docs/spec-audit/<chapter>/verification.md  Phase-2 output: per-requirement verdict table
                                           (classification, code file:line evidence, severity,
                                           negative-test-exists?, fix sketch)
docs/spec-audit/<chapter>/skeptic.md       Phase-3 output: per-finding confirmed/refuted/
                                           uncertain + reasoning
docs/spec-audit/FINDINGS.md                the consolidated register: every confirmed defect
                                           (id, severity, spec citation, code location, fix
                                           status) + every uncertain finding with the exact
                                           runtime probe it needs
```

Have each workflow agent BOTH return structured JSON (use schemas) AND Write
its chapter file directly (distinct paths per chapter = no write conflicts).
The orchestrator commits after each phase so partial progress survives
anything.

## The workflow (3 audit phases + orchestrated fix wave)

**Phase 1 — Extract** (one agent per chapter, model 'opus', effort 'high',
parallel via pipeline): Read the WHOLE chapter text. Extract
machine-checkable normative requirements: class invariants, mandatory
attributes + exact declared types (monomorphic slots especially),
cardinality/occurrence rules, validity functions, mandated behaviours
(versioning rules, status codes, header rules) and above all REJECTION
DUTIES ("must reject X" rules are the highest value). Skip prose philosophy.
Cap ~70/chapter, keep the highest-risk. Number them `<chapter>-R1`…

**Phase 2 — Verify** (same pipeline, second stage, 'opus'/'high'): for each
requirement, find the ENFORCEMENT SITE in the code (a struct field existing
does not prove runtime rejection). Classify: `verified` (file:line of the
enforcement) / `defect` (code contradicts spec or accepts what must be
rejected) / `missing` / `partial` (enforced on some paths only — e.g.
composition commit but not EHR_STATUS PUT) / `not-statically-checkable`
(name the exact runtime probe). Also record whether a negative test exists
for every "must reject" rule. Severity: critical (write-path integrity /
missing mandated rejection) > major (conformance-visible) > minor.

**Phase 3 — Skeptic** (barrier: collect all suspected defect/missing/partial
findings, then one adversarial agent per chapter-with-findings,
'opus'/'high'): try to REFUTE each finding — re-read the cited spec section
(misread? SHOULD vs MUST? another clause relaxes it?), re-read the code (is
it enforced in another layer: derive, walker, REST extractor, DB
constraint?). `refuted` requires concrete evidence; survivors = `confirmed`;
statically undecidable = `uncertain` + exact probe.

**Fix wave** (after the workflow returns — orchestrator-controlled, NOT
inside the workflow):

1. Write FINDINGS.md; review every confirmed defect yourself; present the
   owner a summary table (count by severity/chapter) BEFORE fixing.
2. Fix confirmed defects in severity order via implementer agents (worktree
   isolation if parallel; spec citations in every prompt; regression test per
   fix — negative tests for every leniency fix). Derive/emitter-level fixes
   (systemic `_type` strictness etc.) get extra care: run the FULL workspace
   suite + the openehr-its fidelity gates; investigate any corpus fallout
   honestly.
3. Uncertain findings: write + run the named runtime probes (testcontainers
   PG18), reclassify.
4. Gates after the wave: `cargo nextest run --workspace` green; clippy clean;
   full ECC via `scripts/conformance.sh` — pass/fail must only IMPROVE from
   341/315/0; any newly-failing ECC case means our suite was asserting a bug —
   fix the server, never the case.
5. PR per logical chunk to develop, merge, then update `docs/PROGRESS.md` +
   `docs/plans/` (phase file `a1-full-spec-audit.md`: create at start, tick
   as you go) + `CHANGELOG.md` [Unreleased] for user-visible behaviour
   changes (stricter validation IS user-visible).

## Chapter map (spec paths under docs/specs/openehr/ · code map to start from)

| # | Chapter | Spec | Code map |
|---|---|---|---|
| 1 | rm-common-change-control | RM/docs/common/master06 (+master05, audit parts) | app/ehrbase storage/service (vobject, contribution, versioning), openehr-rm/src/common |
| 2 | rm-ehr | RM/docs/ehr/master03 + master04 (EHR, EHR_STATUS, EHR_ACCESS, VERSIONED_*) | app/ehrbase service, ehrbase-sm ehr*.rs, openehr-rm/src/ehr |
| 3 | rm-composition | RM/docs/ehr/master05 + master06 (COMPOSITION/SECTION/ENTRY invariants) | openehr-rm composition+content, validation walker |
| 4 | rm-data-structures | RM/docs/data_structures/master.adoc | openehr-rm/src/data_structures + *_impl.rs, validation |
| 5 | rm-data-types-text-quantity | RM/docs/data_types/master03 + master04 (DV_TEXT/CODED_TEXT, DV_ORDERED/QUANTITY/PROPORTION/ORDINAL/SCALE incl. accuracy) | openehr-rm/src/data_types + *_impl.rs, ext.openehr_magnitude (migrations/ext/0001), validation |
| 6 | rm-data-types-rest | RM/docs/data_types/master01+05+06+07+08 (basic, dates incl. partial precision, timespec, DV_MULTIMEDIA/PARSABLE, DV_URI) | same + app/ehrbase/src/multimedia |
| 7 | rm-support | RM/docs/support/master.adoc (OBJECT_VERSION_ID etc, terminology interfaces) | openehr-rm/src/support + *_impl.rs, id handling in app |
| 8 | rm-demographic | RM/docs/demographic/master.adoc | openehr-rm demographic, app demographic service + routes |
| 9 | rm-ehr-extract | RM/docs/ehr_extract/master.adoc | openehr-rm ehr_extract, message/extract service (SM-5) |
| 10 | rm-integration | RM/docs/integration/master.adoc (GENERIC_ENTRY, FEEDER_AUDIT) | openehr-rm integration, fhir service provenance |
| 11 | base-foundation | BASE/docs/foundation_types/master.adoc (Interval/Multiplicity_interval/Cardinality functions, ISO8601 validity) | openehr-base foundation_types + *_impl.rs |
| 12 | base-base-types | BASE/docs/base_types/master.adoc (base classes, TERMINOLOGY_CODE, identifier equality/case) | openehr-base base_types + *_impl.rs, storage canonicalisation |
| 13 | am-aom14-opt | AM/docs AOM1.4/ADL1.4/OPT14 (C_OBJECT occurrences, C_ATTRIBUTE existence/cardinality, C_DV_*, ARCHETYPE_SLOT, CONSTRAINT_REF) | openehr-am am14+opt14, openehr-flat validation/webtemplate (the constraint evaluator) |
| 14 | am-aom2-adl2 | AM AOM2+ADL2 masters (validity codes VCOC/VACMCO/VATID…) | openehr-am am24, ADL2 store + ingestion validity |
| 15 | term | TERM/docs (+ support terminology interfaces) | openehr-term, terminology service + providers |
| 16 | query-aql | QUERY/docs master03 + grammar (CONTAINS, paths, operators/matches, functions, ORDER BY over DV_ORDERED, LIMIT/OFFSET, parameters, TERMINOLOGY()) | openehr-query, app AQL engine (IR + SQL lowering) |
| 17 | sm-platform | SM/docs/openehr_platform master02 (transactional equivalence) + master03..12 (interface pre/post-conditions, CALL_STATUS) | ehrbase-sm services (the literal catalog) + Platform impl |
| 18 | sm-tdd | SM TDS/TDD docs | openehr-flat from_tdd, message import_tdd |
| 19 | its-rest-general | vendored overview OAS + shared components (auth, negotiation, Prefer, ETag/Last-Modified/Location, openEHR-VERSION/AUDIT_DETAILS committal headers, status vocabulary) | ehrbase-rest (negotiation/headers/errors), openehr-its/src/rest |
| 20 | its-rest-ehr-composition | ehr-html.openapi.yaml (every EHR/EHR_STATUS/VERSIONED_*/COMPOSITION/DIRECTORY/CONTRIBUTION op: params, schemas, status codes) | ehrbase-rest handlers + openehr-its rest/generated |
| 21 | its-rest-query-definition-admin | query/definition/admin/demographic OAS bundles | ehrbase-rest handlers |
| 22 | its-json | canonical JSON rules (_type discipline: when mandatory, subtype-in-supertype rules, MONOMORPHIC SLOT STRICTNESS, empty omission, number formats) | openehr-derive (the derive), openehr-its/src/json |
| 23 | its-xml | XSDs + canonical XML (element order, xsi:type discipline, attributes, namespaces) | openehr-codegen xml emitter, openehr-its/src/xml |
| 24 | cnf-cross-check | CNF/docs/platform_test_schedule + Robot fixtures: sample 20 normative cases NOT covered by our ECC catalogue (tools/conformance) and verify server behaviour statically | tools/conformance + the relevant handlers |

## Final deliverable (single message at the end)

The audit register summary: chapters audited, requirements
extracted/verified counts, confirmed defects by severity with the top 10
detailed (citation + location), what was fixed + gate results (workspace
tests, ECC delta), uncertain findings still open with their probes, and the
PR list.

---

## The small launch prompt (send this in the fresh session)

> Read `docs/plans/a1-spec-audit-PROMPT.md` and execute it exactly as
> written — it is the full brief for the A1 spec audit. Start now, work
> autonomously through the audit phases, and pause for my review at the
> point the brief marks (after FINDINGS.md, before the fix wave).
