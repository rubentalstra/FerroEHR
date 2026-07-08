# Next session (Fable 5) — drive ehrbase-rs to openEHR CNF CORE/STANDARD

**Paste this as the opening prompt of a fresh Fable-5 session.** It is the
orchestrator brief for closing the CNF findings and expanding real coverage.

---

You are Fable 5, orchestrating on `ehrbase-rs`. **Stay on the current branch
`claude/cnf-hardening` — do NOT cut a new branch.** Read `CLAUDE.md`,
`docs/ADRs/ADR-008`, and the CNF rules (`.claude/rules/spec-adherence.md`,
`serialization.md`, `rest-axum.md`) first. The vendored spec + CNF schedule at
`docs/specs/openehr/` is the oracle — never resolve a spec question from memory or
from EHRbase behaviour.

## Where we are (2026-07-08, self-host run)

`docs/conformance/RESULTS.md`: **322 identified · 263 implemented · 202 passed ·
105 findings.** The full CNF framework (`crates/ehrbase-conformance`) is built,
server-agnostic (`--base-url` or `--self-host`), and emits the guide's artefacts
(Test Execution Report + Conformance Statement + certificate). Head-to-head vs
EHRbase Java: `CNF_COMPARISON.md` (rs column filled; run
`docker/conformance/run.sh java` for the reference column).

## Full 322 coverage is the target — every identified case must be driven

openEHR CNF identifies **322** cases; the goal is the **best CNF framework**, so
**all 322 must have a runner and be driven** (implemented = 322), then maximize
passed. Today `implemented = 263`; the gap is the **59 not-yet-bound slots**,
which are the **57 upstream `aaaa`/`bbbb` placeholders** (master10 demographic 24,
master12 admin 18, master13 messaging 14) plus 2 stragglers.

The placeholders have no upstream *body* (bodies read "Test Environment: TBD"), so
we supply our own **runner-defined** cases against the real ITS-REST + Service
Model surface and **bind them to the placeholder inventory slots** so the coverage
guard counts every one of the 322 as implemented:

- **master10 demographic (24 slots):** we already have 23 `DEMO-*` cases — bind
  them to the 24 `PLACEHOLDER-master10-*` slots (add/adjust to fill all 24).
- **master12 admin (18 slots):** we have 6 `ADMIN-*` — expand to fill all 18
  (per-EHR delete variants, bulk-delete variants, disabled→404 config case,
  role-forbidden 403, etc.) and bind to the slots.
- **master13 messaging (14 slots):** author `MSG-*` cases driving the ITS-REST
  **messaging** surface; the server exposes no messaging API yet, so they drive
  the expected endpoints and record as **findings** (endpoint missing) — driven,
  never skipped, so the slot is covered.
- Wire the coverage guard / registry so a placeholder slot is satisfied by its
  bound runner-defined case (see `schedule.rs` inventory keys
  `PLACEHOLDER-<file-stem>-<n>` and `registry.rs`).

**Definition of "implemented = 322":** every identified case (real + placeholder)
has a runner and executes an assertion against the SUT. Do not fabricate passes —
a placeholder whose endpoint is unimplemented is a driven **finding**, which still
counts as implemented/covered.

## Also expand data-set coverage (the "we need way more tests" ask)

Many functional cases (master06–09) currently drive a **single** data set
(`DataSetReport::SINGLE`). The schedule truth tables specify **multiple** data
sets per case (success + failure + border). Widen every such case to drive all
specified data sets (the content chapters already do this) — this multiplies the
genuine assertion count within the 322 without inventing cases.

## Priority: FULLY close CORE first

**Do CORE 100% before anything else** — finish buckets **1 + 4** and get every
CORE-profile capability (EHR ops, EHR status, composition ops, change sets,
versioning, **archetype validation**, ADL 1.4 archetype/OPT provisioning, EHR API,
DEFINITION API) to **all-pass**. Only once CORE is fully green move to the full
322-coverage work (placeholder binding + messaging + data-set expansion) and
STANDARD (buckets 2 + 3). Do not spread effort — CORE is the gate and must be
complete first.

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

1. **CORE fully green first** — buckets 1 + 4 closed; every CORE-profile capability
   all-pass (the certificate rates **CORE**).
2. **All 322 implemented/driven** — the 57 placeholder slots bound to runner-defined
   cases (demographic/admin/messaging), messaging driving the real ITS-REST surface
   (findings until the API exists); `implemented = 322` in `RESULTS.md`.
3. **STANDARD in reach** — buckets 2 + 3 addressed (AQL + REST realizations).
4. Data-set coverage widened on the single-data-set functional cases.
5. `CNF_COMPARISON.md` has both columns (run `docker/conformance/run.sh java`);
   `docs/conformance/COVERAGE_GAPS.md` lists any residual gaps with spec citations.

Stay on `claude/cnf-hardening` throughout. Commit per bucket; no AI attribution.
