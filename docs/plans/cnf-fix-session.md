# Next session (Fable 5) — drive ehrbase-rs to openEHR CNF CORE/STANDARD

**Paste this as the opening prompt of a fresh Fable-5 session.** It is the
orchestrator brief for closing the CNF findings and expanding real coverage.

---

You are Fable 5, orchestrating on `ehrbase-rs` (branch: cut a new `claude/cnf-*`
from `claude/cnf-hardening`). Read `CLAUDE.md`, `docs/ADRs/ADR-008`, and the CNF
rules (`.claude/rules/spec-adherence.md`, `serialization.md`, `rest-axum.md`)
first. The vendored spec + CNF schedule at `docs/specs/openehr/` is the oracle —
never resolve a spec question from memory or from EHRbase behaviour.

## Where we are (2026-07-08, self-host run)

`docs/conformance/RESULTS.md`: **322 identified · 263 implemented · 202 passed ·
105 findings.** The full CNF framework (`crates/ehrbase-conformance`) is built,
server-agnostic (`--base-url` or `--self-host`), and emits the guide's artefacts
(Test Execution Report + Conformance Statement + certificate). Head-to-head vs
EHRbase Java: `CNF_COMPARISON.md` (rs column filled; run
`docker/conformance/run.sh java` for the reference column).

## The "322" reality — read this before touching the denominator

**322 is the upstream inventory and is NOT fully reachable.** Of it, **57 are
`aaaa`/`bbbb` placeholder headings** the official CNF never wrote (master10
demographic 24, master12 admin 18, master13 messaging 14 — bodies are "Test
Environment: TBD"). Only **265 are real cases** (264 distinct + 1 dup). So:

- Honest PASS denominator = **265 real** (minus the handful with no ITS-REST verb,
  see bucket 3) **+ our 34 runner-defined** cases (`DEMO-*` 23, `ADMIN-*` 6,
  `SIGN-*` 5) that cover the placeholder chapters' real endpoints.
- **Task:** make the report state conformance against the *real testable set*, not
  a fraction of 322. Add a "reachable" line to the report/certificate
  (`report.rs`): `265 real + N runner-defined − M no-ITS-REST = reachable`, and
  express CORE/STANDARD ratings against the profile capabilities, not the raw 322.
- Do **not** fabricate cases to pad 322. master13 messaging has no server API →
  it stays honestly uncovered.

## Also expand real coverage (the "we need more tests" ask)

Many functional cases (master06–09) currently drive a **single** data set
(`DataSetReport::SINGLE`). The schedule truth tables specify **multiple** data
sets per case (success + failure + border). Widen the high-value ones to drive
all specified data sets (the content chapters already do this). This raises the
genuine assertion count without inventing cases.

## The 105 findings → fix in this order (CORE first)

**Bucket 1 — archetype value/cardinality constraints not enforced (71 findings, THE
CORE gate).** All `CONT-*` failures. The composition validator accepts values that
violate the OPT. Fix in `openehr-flat`:
- `webtemplate/builder.rs::requires_cardinality` returns `false` for `min ≤ 1` →
  content/events cardinality `1..*`/`0..1`/`1..1` never enforced. Surface these.
- `validation/leaf.rs`: enforce C_INTEGER/C_REAL **lists** (currently range-only);
  enforce **temporal** ranges/patterns (C_DATE/TIME/DATE_TIME/DURATION — currently
  deferred); enforce **C_CODE_PHRASE** external code lists (DV_MULTIMEDIA
  media_type, DV_CODED_TEXT ext_term).
- **DV_INTERVAL**: accept a valid `DV_INTERVAL<T>` as `ELEMENT.value` (28 findings
  currently reject it outright — "expected RM type conforming to…"), then the
  `Interval` invariant (`lower ≤ upper`).
- **subtype narrowing**: reject a sibling subtype in a narrowed ITEM_STRUCTURE /
  EVENT slot ("Class not allowed").
- Governing spec: AOM 1.4 `master04-constraint_model_package` + the master15–17
  truth tables. Cite sections. Verify each with the self-host run.

**Bucket 4 — service validation leniency (~5, also CORE).** In `ehrbase` service:
`create_composition-same_opt_twice` (→ should 4xx not 201),
`update_composition-wrong_template` (template must match on update),
`commit_contribution-ehr_status_invalid_change_type` + `-fail_create_existing_
directory` (reject), invalid EHR_STATUS partially accepted. Spec: RM Change
Control (`docs/specs/openehr/RM/docs/common/`) + the ITS-REST operation specs.

Closing **1 + 4 → CORE**.

**Bucket 2 — AQL (~9, STANDARD).** `openehr-query` parser: support `TIMEWINDOW`;
`ehrbase::aql` result-set: populate the column `path` metadata (golden mismatch
`columns differ: golden=[{name,path}], served=[{name}]`). Spec: AQL 1.1 +
`master11` query fixtures.

**Bucket 3 — missing REST realizations (~12, STANDARD/OPTIONS).** SM ops with no
verb on our surface: `delete_opt` (master04 ×4), `list_contributions` (master08
×5), `get_versioned_directory` (×1), `list_queries` (master05 ×2). For each:
check the vendored ITS-REST OAS — if the endpoint exists in the spec, add it to
`ehrbase-rest` + service; if it genuinely isn't in ITS-REST, mark the case
`Skipped(NotInITS)` with the guide citation (only ITS-expressed ops are testable)
rather than counting it as a finding.

**Bucket 5 — demographic CRUD (`DEMO-*` ×6, OPTIONS).** Inspect the 6 failing
rows in `docs/conformance/RESULTS.md`; fix in the demographic service/dispatch or
correct the case expectation if it over-asserts.

## Orchestration

Fable owns the hard bespoke logic (the validator hardening in bucket 1, AQL) and
architecture; fan bucket 3/5 wiring and mechanical edits to Opus `implementer`
subagents with the exact spec paths. Iterate on the **fast** loop:
`cargo run -p ehrbase-conformance --features self-host --bin conformance -- run
--self-host --out docs/conformance` after each fix, watch the pass count climb,
commit per bucket. Do the Docker two-container run + `run.sh java` at the end for
the official comparison.

## Hard rules

Spec is the authority (ADR-008); never weaken/skip a test to go green (a real
finding stays a finding until the *server* is fixed); cite spec/CNF sections in
commits; `claude/*` branches; **no AI attribution** in commits/PRs; keep every
crate compiling + clippy-clean + tested per change.

## Definition of done

CORE rating achieved (buckets 1+4 green), STANDARD in reach (2+3), the report
states conformance against the *real* reachable set (not raw 322), `CNF_COMPARISON.md`
has both columns, and `docs/conformance/COVERAGE_GAPS.md` lists any residual
non-ITS-REST cases with citations.
