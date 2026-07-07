# Phase S2-03 — openEHR CNF Conformance Framework

- Status: in-progress
- Started: 2026-07-07   Owner: —
- Consumes (spec/layer): openEHR CNF Platform Conformance Test Schedule
  (`docs/specs/openehr/CNF/`); ITS-REST 1.0.3; AQL 1.1 (ADR-008 acceptance
  instrument)
- Compile required: yes (compiling, tested increments)

## Objectives

Build the ADR-008 acceptance instrument: a runner that executes the official
openEHR Platform Conformance Test Schedule against a running server, a committed
per-test-case results matrix, and a generated, honestly-scoped Conformance
Statement. Binding design: `docs/design/conformance-framework.md` (v2).

## Preconditions

- [x] Access-control (RBAC/ABAC) + version-signing implemented (the STANDARD
      profile's Signing capability is reachable).
- [x] Seed scaffold `crates/ehrbase-conformance` (Cargo.toml + lib.rs + module
      seams).

## Scope

In: the `ehrbase-conformance` crate (case model, schedule parser, registry +
coverage guard, SUT client + modes, assertions, report generation, CLI,
`scripts/conformance.sh`), the transcribed schedule cases (grown
chapter-by-chapter), the runner-defined `SIGN-*` cases, and the generated
`docs/conformance/` artifacts.

Out: CI wiring (the two tiers — done by the orchestrator after review);
FLAT/STRUCTURED/EhrScape (not CNF-gated — `openehr-flat` suite); benchmarks
(P20).

## Tasks (design §8)

- [x] 1. Crate scaffold + case model + registry + coverage guard + inventory
      snapshot (the honest zero state: every schedule id classified, 0/322
      implemented).
- [x] 2. SUT client + modes (External / SelfHosted) + CLI + `scripts/conformance.sh`;
      prove both modes with one hand-picked case
      (`I_EHR_SERVICE.create_ehr-main`) end-to-end incl. RESULTS.md / badge.
- [~] 3. master06 (EHR + EHR_STATUS, **21/21 done**), master09 (DIRECTORY,
      **11/11 core done**), master05 (query provisioning, **3 done**), master04
      (OPT provisioning, **upload-valid + invalid + list done**) — all
      fixture-driven, green e2e against the self-hosted SUT. master07/08/11/
      content/admin/demographic/SIGN modules scaffolded (design §4.1 layout) with
      empty `entries()`. Also landed: `fixtures.rs` — typed read-only access to
      the ENTIRE vendored `test_data_sets` corpus (valid + invalid, every
      category) + the RM-1.2.0 `EHR_STATUS` overlay (§6) + the
      `composition_validation_lib` typed mutators (`content/mutate.rs`); corpus
      pinned by a guard test.
- [~] 4. master04/05 (DEFINITION, 22) and master08 (CONTRIBUTION, 31).
      **master07 (COMPOSITION): 28/31 transcribed** (the 3 `has_composition-*`
      cases have no ITS-REST endpoint → `NotYetTranscribed`); event/persistent
      create + read round-trips run under JSON **and** XML. **master08
      (CONTRIBUTION): 22/31 transcribed** (`has_contribution-*` ×4 +
      `list_contributions-*` ×5 have no endpoint → `NotYetTranscribed`).
      Both fixture-driven (compositions `CANONICAL_JSON`/`CANONICAL_XML`,
      contributions `valid` + `invalid`, the OPTs they reference) and green
      e2e against the self-hosted SUT except the recorded `F-open-3..8`
      findings.
- [ ] 5. master09 (DIRECTORY, 37) — completes the CORE + directory surface.
- [ ] 6. Query: master11's real cases + the `QUERY-FIXTURE-*` corpus with
      golden-result diffing.
- [x] 7. Content chapters (master15–17, **118/119 transcribed**; the 119th is the
      `CONT-DV_TEXT-validate_open#2` upstream duplicate → `Excluded`). All 118
      registered + cited against the truth tables via the typed mutation catalogue
      (`content/mutate.rs`) + driver (`content/drive.rs`). **22 driven** — 12 at
      the RM/schema level (mandatory-attribute rows, all surface **F-open-9**) plus
      8 re-driven against the full constraint-carrying corpus
      (`all_types`/`clinical_content_validation` OPTs + their canonical
      compositions), plus **2 FLAT-/instance-backed** (2026-07-08, below): 4
      **pass** (DV_QUANTITY units / DV_ORDINAL / DV_CODED_TEXT local-codes /
      **DV_QUANTITY units+magnitude-range** — our validator enforces these), 6 open
      findings (**F-open-30** C_DATE_TIME pattern, **F-open-31** ITEM_STRUCTURE
      narrowing ×4, **F-open-40** DV_PROPORTION `type` C_INTEGER.list). **96
      Skipped** (no OPT both constrains the leaf and ships a committable instance
      our stack can provision — OPTs searched named per case; **F-open-41** the
      `ehrn_vital_signs` OPT is unparseable). Self-host e2e on PG18: 4 passed,
      18 failed (findings), 96 skipped, 0 transport errors.
- [ ] 8. OPTIONS chapters we implement (master12 admin subset, master10
      demographic) + the `SIGN-*` capability cases; wire the two CI tiers;
      first committed `docs/conformance/` + README badge.
- [ ] 9. The first STANDARD-profile Conformance Statement (REST, JSON + XML, RM
      1.2.0; RBAC on, ABAC off; deviations register).

## Exit criteria

- [ ] The CNF schedule passes with documented exceptions only; the deviation
      register is complete (the phase-19 finish line, ADR-008).
- [ ] `docs/conformance/` (results.json, RESULTS.md, CONFORMANCE_STATEMENT.md,
      badge.json) is generated from a run and committed.

## Decisions made this phase

- The `master03` documentation-template heading (angle-bracket id) is dropped
  from the inventory (322 real cases from 323 raw heading matches); the 57
  `aaaa`/`bbbb` placeholders are `Excluded(UpstreamPlaceholder)`; the one real
  duplicate (`CONT-DV_TEXT-validate_open`, master17.2) keeps its first
  occurrence and keys the second `…#2` → `Excluded(UpstreamDuplicate)`.
- master10/12/13 are 100% placeholder headings upstream — they carry no real
  transcribable ids (so the demographic/admin/messaging capabilities are
  entirely runner-defined or OPTIONS-scoped work, not schedule transcription).
- No `I_DEFINITION_ADL2` ids exist in the current vendored schedule, so the
  `Adl2Returns501` exclusion rule matches zero cases today (kept for a future
  re-vendor).
- The fixture-derived negative case
  `FIXTURE-I_EHR_SERVICE.create_ehr-invalid_status` is a **FixtureDerived**
  case (outside the 322 schedule inventory by design §3.4); the coverage guard
  therefore only requires `Schedule`-provenance registry ids ⊆ inventory.

## Findings (design §4.5 — failures are findings, never exclusions)

- **F-open-1 (create_ehr, invalid `EHR_STATUS`):** of the 11 vendored
  `ehr/invalid` data sets, only 2 are rejected by the SUT — 9 invalid
  `EHR_STATUS` payloads (e.g. missing `_type`, missing/empty subject id or
  namespace, missing `is_modifiable`/`is_queryable`) are accepted with `201`.
  CNF master06 §Test Data Sets (INVALID class 2) requires rejection. Needs an
  `F-AA-NN` write-up + fix in EHR create validation before the CORE claim can
  be made. Surfaced by the runner, not fabricated.
- **F-open-2 (upload_opt, invalid OPT):** of the 18 vendored invalid `.opt`
  templates, 12 are rejected — 6 invalid OPTs (e.g. `alien_tags`,
  `multiple_elements` duplicate template-id/concept/definition,
  `removed_mandatory_elements`) are accepted by OPT 1.4 upload. CNF master04
  `upload_opt-invalid_opt` requires rejection. Needs an `F-AA-NN` + a stricter
  OPT ingest validation pass.

### master07 (COMPOSITION) — surfaced by the transcribed cases

- **F-open-3 (create/update_composition, mandatory RM attribute not enforced):**
  a COMPOSITION missing a mandatory RM attribute (`composer` [1]) is **accepted**
  with `201` on commit. The commit path validates `data` as a raw `Value`
  (`validate_composition_for_commit` → `openehr_flat::validate_rm_and_terminology`
  + template conformance), which does not enforce mandatory-attribute *presence*
  (that would come from typed `Composition` deserialization, which the commit
  path never performs). Surfaces master07 `create_composition-invalid_event`'s
  intent and master08 `commit_contribution-invalid_composition`/
  `two_commits_second_invalid` (below). CNF master07 §create_composition-invalid_*
  + RM `COMPOSITION` invariants require rejection (`composition_create.yaml` 422).
- **F-open-4 (update_composition-wrong_template):** updating an existing event
  COMPOSITION with a body referencing a **different** `template_id`
  (`persistent_minimal.en.v1` over `nested.en.v1`) is accepted with `200`. CNF
  master07 `update_composition-wrong_template` requires rejection on the
  `template_id` mismatch (`composition_update.yaml` 422). No template-continuity
  check on update.
- **F-open-5 (create_composition-same_opt_twice):** a second `create` for the
  same persistent OPT in one EHR is accepted with `201` (persistent
  single-instance not enforced). CNF master07 `create_composition-same_opt_twice`
  expects a negative response. NOTE: the schedule itself flags this as
  spec-ambiguous ("under debate in the openEHR SEC … lack of information in the
  openEHR specifications; some implementations permit … and some others not"), so
  this is a divergence from the CNF case's *stated* criterion, recorded honestly
  rather than weakened away.
- **F-open-6 (get_versioned_composition, XML):** `GET versioned_composition` with
  `Accept: application/xml` returns `406` ("canonical XML for this response is
  available once typed payloads land (P12)"). Canonical XML is a claimed
  STANDARD-profile data format; `VERSIONED_COMPOSITION` has no canonical-XML
  serializer yet. The JSON variant passes. CNF master07
  `get_versioned_composition` under XML requires `200`
  (`versioned_composition_get.yaml`).

### master08 (CONTRIBUTION) — surfaced by the transcribed cases

- **F-open-3 (shared):** `commit_contribution-invalid_composition` and
  `-two_commits_second_invalid` are accepted with `201` — same root cause as
  F-open-3 (a COMPOSITION VERSION missing a mandatory RM attribute is not
  rejected on the CONTRIBUTION commit path). CNF master08 C.2/C.8 require
  rejection (and C.8 requires the whole commit to fail atomically).
- **F-open-7 (commit_contribution-ehr_status_invalid_change_type):** a
  CONTRIBUTION with a `VERSION<EHR_STATUS>` whose `change_type = 249|creation|`
  is accepted with `201` even though the EHR already has its (mandatory,
  singleton) `EHR_STATUS`. CNF master08 D.3 requires rejection ("the `EHR_STATUS`
  already existing for the EHR"); RM `EHR.ehr_status` is `[1]`. The commit path
  does not reject a second `EHR_STATUS` creation.
- **F-open-8 (commit_contribution-fail_create_existing_directory):** a
  CONTRIBUTION creating a directory (`VERSION<FOLDER>`, `change_type =
  creation`) when the EHR already has a root directory is accepted with `201`.
  CNF master08 E.2 requires rejection ("wrong `change_type` because the root
  `FOLDER` already exists"). The dedicated `directory_create` endpoint returns
  `409` for this, so the CONTRIBUTION path is inconsistent with it.

All eight `F-open-*` are surfaced by the runner asserting the ITS-REST/CNF
expectation, never fabricated and never weakened to green a run (design §4.5).

### master15/16/17 (content / data validation) — surfaced by the transcribed cases

The 118 `CONT-*` cases (the 119th, `CONT-DV_TEXT-validate_open#2`, stays
`Excluded(UpstreamDuplicate)`) are transcribed against the vendored schedule
truth tables. **106 are `Skipped`** with a documented reason: the constraint the
case exercises (content/context cardinality; HISTORY events cardinality + summary
existence; EVENT / ITEM_STRUCTURE class narrowing; every `validate_range` /
`validate_list` / `validate_pattern` / `validate_constraint` / `validate_property*`
/ `validate_ratio*` / `DV_INTERVAL` bound / `C_BOOLEAN` / `C_STRING` /
`C_CODE_PHRASE`) needs a **constraint-expressing OPT that the vendored corpus does
not contain** — upstream ships none (master15 §Implementation notes: the
archetypes "should be generated"), and the framework does not generate archetypes.
Those cases are cited but not executable as specified: a `Skipped` (design §2.2a),
never a fabricated pass and never a masked failure (without the constraining OPT
the SUT correctly accepts data no template forbids — there is nothing to reject).

**12 cases are drivable** at the RM/schema level — a mandatory RM attribute whose
absence any conformant server must reject regardless of archetype — against the
known-committable `nested` / `persistent_minimal` bases. **All 12 fail (findings),
one shared root cause:**

- **F-open-9 (mandatory RM attribute / value presence not enforced on commit —
  content-chapter confirmation of F-open-3):** committing a COMPOSITION in which a
  mandatory RM attribute is removed is **accepted with `201`** where the CNF truth
  tables (rows marked "(RM/schema constraint)" / "RM/Schema mandatory") require
  rejection (`composition_create.yaml` `422`). Surfaced across:
  `CONT-OBS-*` ×4 (OBSERVATION.data existence.lower — master16 §OBSERVATION),
  `CONT-EVENT-state_ex_opt|mand` ×2 (EVENT.data existence.lower — master16 §EVENT),
  `CONT-DV_TEXT-validate_open` (DV_TEXT.value — master17.2),
  `CONT-DV_COUNT-validate_open` (DV_COUNT.magnitude — master17.3),
  `CONT-DV_ORDINAL-validate_open` (DV_ORDINAL.value — master17.3),
  `CONT-DV_BOOLEAN-anything_allowed` (DV_BOOLEAN.value — master17.1),
  `CONT-DV_DATE_TIME-validate_open` (DV_DATE_TIME.value — master17.4),
  `CONT-DV_EHR_URI-validate_open` (DV_EHR_URI.value — master17.7). Root cause is
  the same as F-open-3: the commit path validates `data` as a raw `serde_json::Value`
  (`validate_composition_for_commit`) and never performs typed RM deserialization,
  so a missing mandatory attribute/leaf-value is not caught. RM invariants
  (`RM/data_types`, `RM/ehr` OBSERVATION/EVENT `data [1]`) + the master17.x tables
  require rejection. Fix once in the commit-time validation (typed presence checks)
  and all 12 flip green.

**Constraint-carrying OPTs re-drive the archetype-constraint cases (2026-07-07).**
The content chapters were re-driven against the *full* constraint corpus
(`all_types/Test_all_types{,_v2}.opt` + their bare canonical compositions under
`query/data_load/compositions/`, `clinical_content_validation.opt` + its
composition) via [`drive::drive_constraint`]. **Driven rose 12 → 20**; the
self-hosted PG18 run now reports **3 passed, 17 failed (findings), 98 skipped**
(0 transport errors). The three newly-**passing** cases prove our validator
already enforces those archetype constraints correctly:
`CONT-DV_QUANTITY-validate_property_units` (units off the `{mg,kg}` list rejected),
`CONT-DV_ORDINAL-validate_constraint` (symbol off the ordinal list rejected),
`CONT-DV_CODED_TEXT-validate_local_codes` (code off the `local` code_list rejected).
Two new findings:

- **F-open-30 (`C_DATE_TIME` field-validity pattern not enforced):**
  `CONT-DV_DATE_TIME-validate_constraint` drives `Test_all_types` `items[at0010]`
  (DV_DATE_TIME `value` constrained to `yyyy-mm-ddTHH:MM:SS`). A partial value
  `2021` (missing the mandatory month/day/time fields) is **accepted with `201`**
  where master17.4 §DV_DATE_TIME-validate_constraint requires rejection. The leaf
  validator (`openehr-flat` `leaf.rs`) documents temporal-range/pattern checks as
  deferred; this is the CNF confirmation. Fix in the leaf C_DATE_TIME validity check.
- **F-open-31 (ITEM_STRUCTURE type narrowing not enforced):**
  `CONT-ITEM_STR-type_item_{tree,list,table,single}` drive
  `clinical_content_validation` — four EVALUATION `data` slots the OPT narrows to a
  specific ITEM_STRUCTURE subtype. Swapping a slot's `_type` to a sibling subtype
  (e.g. ITEM_LIST where the slot is narrowed to ITEM_TREE) is **accepted with `201`**
  where master16 §ITEM_STRUCTURE requires rejection ("Class not allowed"). The
  WebTemplate archetype-conformance walk does not reject a sibling ITEM_STRUCTURE
  subtype in a narrowed slot. All four flip green when the walk enforces the slot's
  narrowed `rm_type`.

**FLAT-backed & instance-backed constraint cases (2026-07-08).** Two of the
cases the 2026-07-07 pass left `Skipped` as "FLAT-only / no canonical instance"
were re-examined and driven via our own FLAT→canonical converter
(`fixtures::flat_to_canonical` — the same `openehr_flat::from_flat` path the
SUT's FLAT endpoint uses, run deterministically in-harness; design §4.5 path
*b*) and the existing `minimal_action_2` canonical instance. **Driven rose
20 → 22:**

- **`CONT-DV_QUANTITY-validate_property_units_mag` (PASSES — enforced).** The
  original candidate `ehrn_vital_signs.v2.opt` does **not** parse (F-open-41), so
  the equivalent magnitude-range constraint was sourced from `time_series.opt`
  (C_DV_QUANTITY property `openehr::129`, units `{mm3}`, magnitude range `[0,∞)`),
  whose **only** committable instance is a FLAT one
  (`compositions/FLAT/time_series…flat.json`). Converted to canonical in-harness
  (path *b*), committed at `702.9 mm3` (accepted), then a below-range magnitude
  `-1.0` and an off-list unit `L` (both rejected 422). Our leaf validator enforces
  both the C_DV_QUANTITY units list **and** the per-unit magnitude range — 3/3.
- **`CONT-DV_PROPORTION-validate_any_fraction` (FAILS — F-open-40).** Driven via
  `minimal_action_2.opt` (C_INTEGER.list `{3,4}` on DV_PROPORTION `type`) + its
  vendored bare canonical composition (`type=3`, num=889, den=149 — accepted);
  mutating `type=0` (ratio, off the `{3,4}` list) is **accepted with `201`** where
  master17.3 §validate_any_fraction requires rejection (`C_INTEGER.list`).

Two new findings:

- **F-open-40 (DV_PROPORTION `type` `C_INTEGER.list` not enforced):**
  `CONT-DV_PROPORTION-validate_any_fraction` drives `minimal_action_2` (DV_PROPORTION
  `type` constrained to `C_INTEGER.list {3,4}`). `type=0` is **accepted with `201`**
  where master17.3 §DV_PROPORTION-validate_any_fraction requires rejection (`422`,
  `composition_create.yaml`). The WebTemplate archetype-conformance walk does not
  enforce a `C_INTEGER.list` on the DV_PROPORTION `type` leaf. Flips green when the
  leaf/primitive-list check covers DV_PROPORTION `type`.
- **F-open-41 (`opt14` parser rejects `ehrn_vital_signs.v2.opt`):** the **only**
  OPT that constrains a DV_COUNT `C_INTEGER` range/list — and one of the DV_QUANTITY
  magnitude-range templates — fails our `openehr_its::opt14::from_xml` with
  `xml parse error: missing element type`, so it cannot be provisioned on the SUT
  and its FLAT instance cannot be converted. Blocks
  `CONT-DV_COUNT-validate_range` / `CONT-DV_COUNT-validate_list` (no other OPT
  constrains a committable DV_COUNT). Needs a fix in the ITS canonical-XML/opt14
  reader (out of the content-suite scope); the two DV_COUNT cases stay `Skipped`
  until then.

Cases still `Skipped` name the OPTs searched and why none drives them (per-case
notes in `data_types.rs` / `entry.rs` / `composition.rs`): no vendored OPT both
constrains the leaf *and* ships a committable instance our stack can provision —
the `master15` COMPOSITION content-cardinality intervals (`cardinality_of_section`
constrains SECTION occurrences, not the six per-case content-cardinality
intervals), `master16` HISTORY cardinality / EVENT subtype narrowing (no OPT
narrows either), DV_COUNT range/list (only `ehrn_vital_signs`, unparseable —
F-open-41), the other DV_PROPORTION kind cases
(`validate_ratio/unitary/percent/fraction/integer_fraction` — only `proportion.opt`,
which parses but ships **no** instance, canonical or FLAT), DV_SCALE / DV_DATE /
DV_TIME / DV_DURATION-fields / DV_BOOLEAN / DV_IDENTIFIER / DV_MULTIMEDIA media-type
(no constrained committable canonical leaf; `obs_*` contribution instances omit
`archetype_details` on their content ENTRYs and fail the `Is_archetypeRoot` RM
invariant as a bare commit).

All content findings are surfaced by the runner asserting the CNF truth-table
`expected` column, never fabricated and never weakened to green a run (design §4.5);
the self-hosted PG18 run reports 0 transport errors (17 Failed = findings, 98
Skipped = non-executable-as-specified).

### master11 (QUERY) — surfaced by the `QUERY-FIXTURE-*` cases

Query findings use a distinct **F-open-20+** block to avoid colliding with the
concurrent content-chapter agent's numbering; if a collision survives merge the
orchestrator renumbers. All surfaced by the QUERY-FIXTURE golden diffs against a
self-hosted PG18 SUT (0 transport errors; the normalizer's per-diff rule labels
prove nothing was silently suppressed).

- **F-open-20 (RESULT_SET column `path` omitted for EHR/VERSION-scoped SELECT
  columns):** the AQL engine emits `columns: [{"name":"#0"}]` with **no `path`**
  for SELECT columns targeting EHR or VERSION pseudo-attributes
  (`e/ehr_id/value`, `e/time_created/value`, `e/system_id/value`), while emitting
  the path correctly for COMPOSITION/ENTRY data columns (`c/uid/value` →
  `path: "/uid/value"`). Both the vendored goldens and the ITS-REST
  `schemas/query/ResultSet.yaml` **example** carry `path: '/ehr_id/value'` for
  exactly this column. `RESULT_SET_COLUMN.path` is `0..1`
  (`SM/docs/UML/classes/result_set_column.adoc`: "RM path of data item for this
  column *as specified in query*"), so this is not a hard schema breach — but the
  path *is* specified in the query and is emitted for other column classes, so
  the **asymmetric omission** is the defect (`target_path_string` returns `None`
  for `PathTarget::Ehr`/`PathTarget::Version` in
  `crates/ehrbase/src/aql/sql.rs`). Impact: every group-A query (EHR selects) and
  the group-D EHR-column selects fail the golden column diff (`A/empty_db` 0/27,
  `A/loaded_db` 0/23, and master11 `execute_ad_hoc_query-empty_db` +
  `execute_stored_query-empty_db`); composition/entry projections pass
  (`B/empty_db` 17/18, `C/empty_db` 10/11). Needs an `F-AA-NN` + emitting the
  identified path for EHR/VERSION targets in the column metadata; the four
  QUERY-FIXTURE column-diff cases go green when it lands.
- **F-open-21 (corpus artifact — NOT a defect against the SUT): `TIMEWINDOW`
  queries rejected, which is spec-correct.** Corpus queries using the `TIMEWINDOW`
  clause (`A/109`, `B/103`, `C/103`, …) are rejected by our parser with `400
  invalid AQL`. This is **conformant**: `TIMEWINDOW` was *removed* from AQL
  (`QUERY/docs/AQL/master00-amendment_record.adoc`, SPECQUERY-20 "remove
  `TIMEWINDOW`"), and the corpus README lists these among the EHRSCAPE-failing
  queries. Recorded so the resulting per-query golden-diff failures are
  understood as a corpus-legacy artifact (EHRSCAPE extension predating the AQL
  cleanup), not a server bug — no fix needed. The spec, not EHRSCAPE, is the
  oracle (ADR-008).

The QUERY framework is demonstrably a real conformance instrument, not a rubber
stamp: it **passes** where the server is conformant (`smoke_test`,
`execute_ad_hoc_query-loaded_db`, `QUERY-FIXTURE-invalid` 2/2 rejected, and the
B/C COMPOSITION/ENTRY column projections) and **fails with a precise, cited
finding** where it is not (F-open-20). Golden diffing runs through the documented
[`query_golden`] normalizer (design §6), and each suppressed difference names its
rule in the failure/skip message.

### SIGN-* (runner-defined Signing capability, design §4.6) — the STANDARD Signing evidence

The five `SIGN-*` cases (`suites/sign.rs`, `Provenance::RunnerDefined`, pseudo-
chapter `Chapter::Signing`, **outside** the 322 inventory — coverage guard
confirmed green) specify the implemented behaviour in
`docs/design/version-signing.md`. Self-hosted PG18 e2e
(`sign_capability_cases_run_against_self_hosted_sut`, 0 transport errors):

- **SIGN-digest-present (JSON): PASS** — the served `ORIGINAL_VERSION` carries a
  `sha256:<32-byte-base64>` digest (version-signing.md §3.2/§4.4).
- **SIGN-digest-recomputes (JSON): PASS** — the strongest case: the served
  digest recomputes from the version's own RFC 8785 `canonical_form`
  (`openehr_rm::…::version_impl::canonical_form_of_json`), proving commit-time ==
  read-time object identity (§6.3).
- **SIGN-all-kinds (JSON): PASS (4/4)** — an EHR_STATUS-update version and both a
  create + modification COMPOSITION version via the CONTRIBUTION path recompute;
  the FOLDER write is driven (accepted) but its signature is **not
  API-observable** — the directory read surface serves the bare FOLDER (no
  `ORIGINAL_VERSION` wrapper). PORT NOTE in `sign.rs`: FOLDER version-signature
  verification via the API awaits a versioned-directory version-read surface; the
  storage-level signing is proven by the ehrbase `service_signing` SQL sweep.
- **SIGN-client-verbatim (JSON): PASS** — a CONTRIBUTION version carrying a
  client-supplied signature is served verbatim, never re-signed (§3.3).
- **SIGN-pgp-verifies: SKIPPED(SutConfig)** — the self-hosted SUT boots in
  `digest` mode (§3.4); a `pgp`-keyed self-host SUT (a boot-path change) is a
  **follow-up**. The four digest cases prove the capability.
- **SIGN-digest-present (XML): FAIL — shared root cause with F-open-6.** The
  `versioned_composition/{vo}/version/{ovid}` endpoint returns `406` for
  `application/xml` (same "canonical XML … once typed payloads land (P12)" gap as
  F-open-6, on the version-get endpoint rather than the versioned-object-get
  endpoint). The RM/serialization layer already emits `<signature>` in canonical
  XML (proven by ehrbase `service_signing::canonical_xml_carries_the_signature`);
  only the REST negotiation for versioned-object responses is missing. Recorded,
  not weakened: `SIGN-digest-present` keeps `formats = [Json, Xml]` so a real
  `conformance run` continues to surface the gap until it is fixed.

## Handoff for next session

Steps 1–2 landed: the crate parses the schedule (323 raw / 322 identified / 57
placeholders / 1 duplicate), classifies every id (coverage guard + committed
`inventory/schedule-cases.txt` snapshot), runs a SUT (external + self-hosted),
and generates the report set — proven end-to-end by the one implemented case
`I_EHR_SERVICE.create_ehr-main` (16/16 data sets). Next: step 3 — transcribe the
rest of master06 then master07, growing the registry and shrinking the not-yet
count. Failures are findings (`F-AA-NN`), never exclusions.
