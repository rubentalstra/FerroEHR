# Phase S2-04 — CNF conformance hardening (spec-grounded)

- Status: in-progress
- Started: 2026-07-08   Owner: —
- Consumes: the CNF conformance framework (`crates/ehrbase-conformance`, PR #27)
  + the benchmark harness (`crates/ehrbase-bench`, PR #28), which between them
  surfaced the findings below. The vendored openEHR specs are the oracle.
- Compile required: yes — compiling, clippy-clean, tested increments; each fix
  re-verified by the CNF framework (`cargo nextest run -p ehrbase-conformance
  --features self-host`) and, where relevant, the benchmark's dual-stack run.

## Why this phase

The CNF framework and the ehrbase-rs-vs-EHRbase benchmark both proved the same
thing from different angles: **our server is too lenient — it accepts inputs the
RM and the ITS-REST contract require it to reject.** The benchmark made it
visceral (EHRbase Java rejects with 400/412 payloads ehrbase-rs happily commits,
e.g. `PARTY_IDENTIFIED` in a `PARTY_SELF` slot). This phase hardens the
validation surface against the *official spec text*, verified by the CNF suite —
not against EHRbase behaviour, though EHRbase's strictness is a useful smell
test.

## The deep root cause (the load-bearing finding)

One defect explains the largest cluster of failures (F-open-3, F-open-9, and the
benchmark PARTY_SELF gap):

**`crates/openehr-rm/src/validate.rs::run<T>` silently swallows
`serde_json::from_value` failures.** It runs a node's RM class invariants only
when the node deserializes into its declared concrete type, and its comment
(lines 287–289) justifies dropping a failed deserialize as "a structural error
caught by the codec/schema layer, not an invariant failure." **That assumption
is false for the commit path:** a versioned object is stored as its raw
canonical-JSON fragment (decompose → `node` rows, ADR-008) and the ITS-JSON
schema is *not* enforced at commit. So a deserialize failure is caught
**nowhere**:

- a COMPOSITION missing the mandatory `composer` (`COMPOSITION.composer [1]`,
  RM ehr) fails to deserialize into `openehr_rm::…::Composition` → swallowed →
  committed with 201 (must be **422**);
- an `EHR_STATUS` whose `subject` is `PARTY_IDENTIFIED` where the RM types it
  `EHR_STATUS.subject: PARTY_SELF [1]` (RM ehr) fails to deserialize into the
  typed `EhrStatus` → but `EHR_STATUS` isn't even in the `validate_rm_value`
  dispatch table, so it is never typed-checked at all.

**Spec grounding:**
- `docs/specs/openehr/RM/docs/ehr/` — `COMPOSITION.composer: PARTY_PROXY [1]`,
  `EHR_STATUS.subject: PARTY_SELF [1]` (mandatory existence + the concrete
  subject type). Mandatory existence for every RM attribute is the BMM
  `is_mandatory` flag, already exposed by `openehr_rm::model::attribute(...).is_mandatory`.
- `docs/specs/openehr/ITS-REST/specifications/responses/422_COMPOSITION.yaml` —
  a composition that "converts, but does not validate" is **422** (already the
  policy `composition.rs::validate_composition_for_commit` cites); a missing
  mandatory RM attribute is exactly that.

**The fix (spec-grounded, hand-written — `validate.rs` is editable ADR-003 glue,
not generated):**
1. In `run<T>`: when `from_value::<T>` returns `Err(e)`, **push an
   `InvariantViolation`** naming the RM type and the serde error (which
   identifies the missing mandatory field / wrong nested type), instead of
   dropping it. This flows through `validate_rm_and_terminology` → the service's
   422 mapping.
2. Add `EHR_STATUS` and `EHR_ACCESS` to the `validate_rm_value` dispatch table
   (`run::<EhrStatus>` / `run::<EhrAccess>`) so status/access commits get typed
   validation → surfaces the `PARTY_SELF` type mismatch and mandatory presence.
3. Confirm the `EHR_STATUS` update path (`service/ehr.rs`) and the FOLDER path
   (`service/directory.rs`) run `validate_rm_and_terminology` on the incoming
   object (composition already does via `composition.rs`); route them through
   the same seam.

**Mandatory verification (this fix can over-reject if a generated type has an
incorrectly non-`Option` field for a spec-*optional* attribute — a latent codegen
bug this change would surface):**
- The valid corpus (`openehr-its/tests/corpus.rs`, the fidelity gate) must still
  deserialize + pass — it proves valid inputs won't newly fail.
- Run the full CNF self-host suite + the composition/EHR service e2e; any newly
  *failing valid* case is a codegen-optionality bug → fix the **emitter**
  (`emit.rs` field-optionality), not the validator.

## Findings backlog (grouped by fix, ordered by leverage)

Cross-referenced to `docs/conformance/COVERAGE_GAPS.md`. `SUT-strictness` marks
the benchmark-surfaced ones.

### Cluster 1 — typed-deserialization enforcement (the root cause above)
- **F-open-3 / F-open-9** — mandatory RM attribute presence not enforced on
  commit (composition create/update + contribution path). *Highest leverage —
  ~12 content cases + create/update.*
- **F-open-42 (new, SUT-strictness)** — `EHR_STATUS.subject` accepts
  `PARTY_IDENTIFIED` where the RM types it `PARTY_SELF`. Same fix (dispatch +
  surface).
- **F-open-1** — 9/11 invalid `EHR_STATUS` data sets accepted (mostly
  mandatory-presence / type violations the surfaced-deserialize catches).

### Cluster 2 — archetype-constraint enforcement (WebTemplate walk)
- **F-open-31** — `ITEM_STRUCTURE` type narrowing not enforced (a sibling ITEM
  subtype accepted in a narrowed slot). Spec: AOM `C_OBJECT.rm_type_name` /
  `C_ATTRIBUTE` — the WebTemplate walk (`openehr-flat` `Validator::walk`) must
  reject a node whose concrete type is not the (narrowed) allowed type.
- **F-open-40** — `DV_PROPORTION.type` `C_INTEGER.list` not enforced (the walk
  doesn't apply a primitive `C_INTEGER` list to the `type` leaf).
- **F-open-30** — `C_DATE_TIME` field-validity pattern not enforced (partial
  `2021` accepted where `yyyy-mm-ddTHH:MM:SS` is required). Spec: AOM
  `C_DATE_TIME` validity + the ISO-8601 partial-precision rules already in
  `validate.rs`'s `is_valid_iso_date_time` — wire the pattern into the walk.

### Cluster 3 — version/commit semantics
- **F-open-4** — `update_composition` with a mismatched `template_id` accepted;
  no template-continuity check across versions. Spec: RM change control +
  ITS-REST update semantics (a new version must keep the versioned object's
  template).
- **F-open-7** — CONTRIBUTION creating a 2nd `EHR_STATUS` accepted
  (`EHR.ehr_status [1]`, RM ehr). Enforce the 1..1 cardinality at commit.
- **F-open-8** — CONTRIBUTION creating a directory when one exists accepted;
  inconsistent with `directory_create` (409). Unify the "already exists" guard.
- **F-open-5** — 2nd persistent `create` for the same OPT accepted; spec-ambiguous
  — resolve with an ADR + `// PORT NOTE:` (decide 201-new-version vs 409).

### Cluster 4 — serialization / wire
- **F-open-6** — `GET versioned_composition` (+ VERSION responses) with
  `Accept: application/xml` → 406; no canonical-XML serializer for
  versioned-object REST responses (the RM layer *does* emit `<signature>` in
  XML — only REST negotiation is missing). Wire the XML `respond_rm` path for
  `VERSIONED_OBJECT`/`VERSION`.
- **F-open-20** — AQL `RESULT_SET` omits the `path` column for EHR/VERSION-scoped
  SELECTs (emitted for COMPOSITION/ENTRY). Spec: QUERY `RESULT_SET` +
  `ResultSet.yaml` example carry `path`. Fix `aql/sql.rs` `target_path_string`
  for `PathTarget::Ehr`/`Version`.

### Cluster 5 — the opt14 reader
- **F-open-41** — `opt14::from_xml` rejects `ehrn_vital_signs.v2.opt`
  ("missing element type"). An ITS/opt14 reader bug; fixing it unlocks the
  DV_COUNT content cases. Spec: OPT 1.4 XSD.

### Not defects (documented, no action)
- **F-open-21** — `TIMEWINDOW` rejected is spec-correct (removed from AQL).

## Tasks

- [ ] **T1 — the root-cause fix (Cluster 1):** `run<T>` surfaces deserialize
      failures; `EHR_STATUS`/`EHR_ACCESS` added to the dispatch table; EHR_STATUS
      + FOLDER commit paths routed through `validate_rm_and_terminology`. Verify:
      valid corpus still passes; CNF cluster-1 cases flip to pass; benchmark
      EHR_STATUS PARTY_IDENTIFIED now 422. Record any surfaced codegen-optionality
      bug as its own emitter fix.
- [ ] **T2 — Cluster 2** (archetype-constraint enforcement in the WebTemplate walk).
- [ ] **T3 — Cluster 3** (version/commit semantics: template continuity, EHR_STATUS
      1..1, directory-exists 409, persistent-create ADR).
- [ ] **T4 — Cluster 4** (versioned-object XML negotiation; AQL `path` column).
- [ ] **T5 — Cluster 5** (opt14 reader `ehrn_vital_signs.v2.opt`).
- [ ] **T6 — Re-run + regenerate the conformance report** (`docs/conformance/`);
      update `COVERAGE_GAPS.md`, the badge, and this phase file; every flipped
      finding cited to its spec section in the commit.

## Exit criteria

- [ ] Cluster 1 lands and the valid corpus is unbroken (no false rejections).
- [ ] Every fixed finding's CNF case(s) pass; the generated report's pass count
      rises and the badge reflects it.
- [ ] Each fix commit cites the governing spec section (spec-adherence.md).
- [ ] Remaining findings are either fixed, or re-classified with a spec-cited
      reason in `COVERAGE_GAPS.md` (never masked).

## Decisions made this phase

- The commit-time validation gap is a *validator* defect (swallowed
  deserialize), not a storage defect — the fix is in `openehr-rm::validate`
  (hand-written) + the service commit seam, not the node codec.
- Strictness is driven by the **RM + ITS-REST spec text**, with EHRbase's
  behaviour as a smell test only (ADR-008). Where the spec is ambiguous
  (F-open-5), an ADR records the decision.

## Handoff for next session

Start with **T1** — it is the highest-leverage, most-spec-grounded fix and
unblocks the largest finding cluster. The precise change site is
`crates/openehr-rm/src/validate.rs::run<T>` (surface the `Err`) +
`validate_rm_value` (add EHR_STATUS/EHR_ACCESS). Verify against
`crates/openehr-its/tests/corpus.rs` first (must stay green), then the CNF
self-host suite, then the benchmark's EHR_STATUS path.
