# openEHR CNF — Coverage Gaps & Missing Work

Companion to the generated `RESULTS.md` / `CONFORMANCE_STATEMENT.md`. Those are
produced by a run and state *what happened*; this file is the hand-maintained
register of *what is missing and why*, and what closing each gap requires. It is
the honest backlog behind the `91/322` badge — nothing here is hidden inside a
"skip."

**Snapshot (self-hosted PG18 run):** 322 identified · 210 implemented · **91
passed · 36 failed** · the remainder excluded or skipped-with-reason.

There are three distinct kinds of "not passing," and they need different work:

| Kind | Count (approx.) | Root cause | How to close |
|---|--:|---|---|
| **A. Findings** (fail) | 36 | our **server** has a conformance gap | fix the server (§1) — the highest-value work |
| **B. Skipped** (can't drive) | ~90 | the **vendored corpus** ships no template that constrains the tested node | author minimal test OPTs (§2) — deferred |
| **C. Excluded** (structural) | ~112 | no runnable target exists (upstream placeholders / no ITS-REST endpoint / out-of-profile) | mostly nothing to do; documented (§3) |

---

## 1. Findings — real server gaps (kind A, the actionable conformance bugs)

Every failing case is a tracked `F-open-*` finding. These are **our** defects:
the case is correctly written, the server answers wrong. Fixing them converts
failures → passes with no new fixtures. Full detail in
`docs/plans/s2-phase-03-conformance-framework.md`.

| Finding | Gap | Cases affected | Priority |
|---|---|--:|---|
| **F-open-3 / F-open-9** | mandatory RM attribute / value presence **not enforced on commit** — the commit path validates composition `data` as a raw JSON value and never typed-checks presence | ~12 content + composition/contribution create/update | **highest** — one fix flips the most cases |
| **F-open-1** | 9/11 invalid `EHR_STATUS` data sets accepted (should be 422) | 1 (data-set aggregate) | high |
| **F-open-2** | 6/18 invalid `.opt` templates accepted on upload (should reject) | 1 (aggregate) | high |
| **F-open-31** | `ITEM_STRUCTURE` type narrowing not enforced (a sibling ITEM subtype accepted in a narrowed slot) | 4 | high |
| **F-open-40** | `DV_PROPORTION.type` `C_INTEGER.list` constraint not enforced | 1 | medium |
| **F-open-30** | `C_DATE_TIME` field-validity pattern not enforced (partial date accepted where full required) | 1 | medium |
| **F-open-4** | `update_composition` with a mismatched `template_id` accepted (no continuity check) | 1 | medium |
| **F-open-7** | CONTRIBUTION creating a 2nd `EHR_STATUS` accepted (`EHR.ehr_status` is 1..1) | 1 | medium |
| **F-open-8** | CONTRIBUTION creating a directory when one exists accepted (inconsistent with `directory_create` → 409) | 1 | medium |
| **F-open-20** | AQL `RESULT_SET` omits the `path` column for EHR/VERSION-scoped SELECTs (emitted for COMPOSITION/ENTRY) | ~4 query columns | medium |
| **F-open-6** | `GET versioned_composition` with `Accept: application/xml` → 406; no canonical-XML serializer for VERSION/versioned-object REST responses (the RM layer *does* emit `<signature>` in XML — only REST negotiation is missing) | 1 (+ SIGN-digest-present XML) | medium |
| **F-open-41** | `opt14` parser rejects `ehrn_vital_signs.v2.opt` ("missing element type") — blocks provisioning that template, which is the **only** vendored one constraining a committable `DV_COUNT` | blocks ~3 (see §2) | medium — an ITS/opt14 reader bug, also unlocks §2 cases |
| **F-open-5** | 2nd persistent `create` for the same OPT accepted — **spec-ambiguous**, recorded honestly, not necessarily a defect | 1 | low / review |
| **F-open-21** | `TIMEWINDOW` query rejected — **spec-correct** (removed from AQL); a corpus artifact, **NOT a defect** | 0 | none (documented) |
| **F-open-42** (SUT-strictness, from the benchmark) | `EHR_STATUS.subject` accepts `PARTY_IDENTIFIED` where the RM types it `PARTY_SELF [1]` (RM ehr) — EHRbase Java rejects it with 400; ehrbase-rs commits it | benchmark + invalid-status cases | high (same root cause as F-open-3/9) |

### Root-cause analysis (2026-07-08, deep dive)

F-open-3, F-open-9, F-open-42, and much of F-open-1 share **one** defect, found
by reading the commit-time validation path against the RM + ITS-REST spec:

> **`crates/openehr-rm/src/validate.rs::run<T>` silently swallows
> `serde_json::from_value` failures.** It runs a node's RM class invariants only
> when the node deserializes into its declared concrete type, and drops a failed
> deserialize on the comment's assumption that it is "caught by the codec/schema
> layer." **That is false on the commit path** — a versioned object is stored as
> its raw canonical-JSON fragment (ADR-008 node codec) and the ITS-JSON schema is
> never enforced at commit. So a COMPOSITION missing mandatory `composer` [1], or
> an `EHR_STATUS.subject` typed `PARTY_IDENTIFIED` (RM types it `PARTY_SELF [1]`),
> **fails to deserialize → is swallowed → committed 201** (must be 422). Worse,
> `EHR_STATUS`/`EHR_ACCESS` are not even in the `validate_rm_value` dispatch
> table, so they get no typed validation at all.

**Spec:** `RM/docs/ehr/` (`COMPOSITION.composer [1]`, `EHR_STATUS.subject:
PARTY_SELF [1]`; mandatory existence = BMM `is_mandatory`, exposed by
`openehr_rm::model`); `ITS-REST/.../responses/422_COMPOSITION.yaml` ("converts,
but does not validate" → 422).

**Fix (spec-grounded, in the hand-written `validate.rs` — not generated):**
`run<T>` surfaces the deserialize `Err` as an `InvariantViolation`; add
`EHR_STATUS`/`EHR_ACCESS` to the dispatch table; route the EHR_STATUS + FOLDER
commit paths through `validate_rm_and_terminology`. **Verification guard:** the
valid corpus (`openehr-its/tests/corpus.rs`) must still deserialize + pass — if
any *valid* case newly fails, it exposes a codegen field-optionality bug (a
spec-optional attribute emitted as non-`Option`), which is fixed in the
**emitter**, not the validator. Full plan + ordering:
`docs/plans/s2-phase-04-cnf-hardening.md`.

## 2. Skipped — the vendored-corpus ceiling (kind B, deferred by owner decision)

These cases are **not driven** because a validation case like *"DV_QUANTITY
constrained to [0..10]: value 5 → accept, 15 → reject"* needs a **template that
imposes that constraint** to commit against, and the official CNF corpus ships
no such template for these nodes. CNF's content chapters assume archetype/OPT
*generation tooling* that upstream does not include — even upstream's own Robot
harness does not drive most of them. Each skip carries a per-case justification
naming the OPTs searched (in `suites/content/*.rs` + the phase file).

**What is missing, by chapter:**

| Chapter | Skipped cases | Why un-drivable from the corpus |
|---|--:|---|
| **master15** (COMPOSITION) | 12 | no vendored OPT constrains `COMPOSITION.content` **cardinality**/occurrences (`cardinality_of_section.opt` constrains SECTION, not COMPOSITION.content) |
| **master16** (ENTRY) | ~16 | no OPT narrows `HISTORY.events` cardinality, an `EVENT` slot to POINT/INTERVAL, or leaves an `ITEM_STRUCTURE` slot open (`type_any`) |
| **master17.3/4/6/7** (data types) | ~62 | `DV_COUNT` range/list only in the unparseable `ehrn_vital_signs.v2.opt` (blocked by **F-open-41**); `DV_PROPORTION` variants only in `proportion.opt` (no committable instance); `DV_SCALE`, `DV_DATE`, `DV_TIME`, `DV_DURATION`-field-validity, `DV_BOOLEAN`, `DV_IDENTIFIER`, `DV_MULTIMEDIA` media-type have no constrained committable leaf anywhere in the corpus |

**How to close it (deferred — not being done now):** author a small library of
**minimal test OPTs**, each imposing exactly one constraint the truth-table row
references, and drive the skipped cases against them. The safe method is
**not** hand-writing OPT 1.4 XML (it is ~250+ lines each and our own parser
already rejects one vendored OPT — F-open-41); instead, **load a known-parsing
vendored OPT via `openehr_its::opt14::from_xml`, tighten one constraint in its
typed tree, and serialize back via `opt14::to_xml`** — every authored OPT then
round-trips through our own parser, so it is provably parseable. Estimated
unlock:

- one authored `COMPOSITION.content` cardinality OPT → the **12** master15 cases;
- a handful of per-data-type constraint OPTs (range/list/pattern) → most of the
  ~62 master17 cases;
- `HISTORY`/`EVENT`/`ITEM_STRUCTURE`-narrowing OPTs → the master16 block.

Authored OPTs would live under `crates/ehrbase-conformance/fixtures/opts/` with
a provenance note (marked `RunnerAuthored`, distinct from the vendored corpus —
`docs/specs/openehr/**` is never edited). Fixing **F-open-41** first also
unlocks the `DV_COUNT` cases without authoring.

> **Honesty note:** authored OPTs are *our* artifacts, not vendored CNF
> fixtures. They remain faithful to each case because the truth-table's
> accept/reject expectation is the oracle regardless of which template imposes
> the constraint — but the report will label them `RunnerAuthored` so a reader
> can see exactly which coverage rests on our own templates vs. the official
> corpus.

## 3. Excluded — no runnable target (kind C, mostly nothing to do)

These are correctly outside the driven set; the coverage guard still *counts and
classifies* every one, so they are visible, not hidden.

| Group | Count | Reason |
|---|--:|---|
| Upstream placeholders (`aaaa`/`bbbb` headings) | 57 | not real cases in the schedule — literal TBD stubs |
| **master10** demographic | 24 | 100% upstream placeholder headings; OPTIONS profile |
| **master12** admin | 18 | 100% upstream placeholder headings; OPTIONS profile |
| **master13** messaging (EHR Extract) | 14 | not implemented; OPTIONS profile |
| `has_*` / `list_*` ops (composition/contribution) | 12 | **no matching ITS-REST endpoint** — these abstract SM operations are not realized in the REST API we implement |
| master04 ADL2 / OPT2 provisioning | ~3 | we return 501 for `adl2` (OPTIONS profile); documented |
| `CONT-DV_TEXT-validate_open#2` | 1 | upstream **duplicate** case id |
| master17.5 (time specification) | 0 | upstream ships **zero** cases (empty chapter) |

Closing kind C is mostly **not applicable**: they are either upstream artifacts,
out-of-profile OPTIONS capabilities we do not implement, or SM operations with no
REST surface. The honest STANDARD-profile claim does not depend on them.

---

## 4. Priority order to raise the number

1. **Fix F-open-3/F-open-9** (typed mandatory-presence validation at commit) —
   the biggest single pass-count gain, no new fixtures.
2. **Fix F-open-1, F-open-2, F-open-31, F-open-40, F-open-30** — the remaining
   validation-enforcement findings; each flips its case(s) green.
3. **Fix F-open-41** (opt14 parser) — unblocks the `DV_COUNT` cases *and* removes
   a real ITS reader bug.
4. **Fix F-open-6, F-open-20, F-open-4, F-open-7, F-open-8** — the wire/semantic
   findings.
5. **(Deferred) author the missing OPTs** (§2) — the only lever for the
   structurally-unconstrained content cases; a bounded, well-defined task via the
   `from_xml → tighten → to_xml` round-trip method.

Kinds A + partial B are the realistic path to a materially higher pass count;
kind C is the honest, documented ceiling of the official test data.
