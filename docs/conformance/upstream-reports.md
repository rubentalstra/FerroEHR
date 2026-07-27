# Upstream reports — the outbound openEHR report ledger

The outbound spec-defect reports raised from the CNF ambiguity register
(`tools/cnf-runner/artifacts/registers/ambiguities.yaml`). The register never
silently absorbs a spec divergence or silence — it documents it and points here
at the report that pushes the fix back to openEHR, so the spec is corrected
rather than worked around forever.

## Grounding rules (non-negotiable)

- **Docs text only.** Every report cites the openEHR **docs text**
  (`docs/specs/openehr/…`) — the normative prose. The vendored **OAS is STALLED
  and is NOT an oracle** (owner ruling 2026-07-24): it is never a citation here.
  A "defect" that exists ONLY because the OAS is stale/incomplete is an
  OAS-regeneration item, NOT an openEHR spec report, and does not belong in this
  ledger.
- **Proven before listed.** A report appears here ONLY after its register entry
  is CONFIRMED first-hand against the docs text (the `cnf-triage` adjudication).
  An entry the docs text actually resolves is REFUTED — removed from the
  register, its case made gating — and is never reported upstream.
- **Lifecycle.** `draft` (a `UPR-<n>` id) → filed on the openEHR channel
  (Jira / spec repo) → the `UPR-<n>` id is replaced, in both this ledger and the
  register `upstream_ref`, with the returned key (`SPECPR-<n>` /
  `SPECQUERY-<n>` / the merged editorial PR).
- **Channels.** `SPECPR` (RM/BASE/SM semantic gap) · `SPECQUERY` (AQL/QUERY) ·
  `editorial` (schedule/spec text defect, no semantic change) · `ITS-REST`
  (ITS-REST API-definition gap) · `ITS-XML` (serialization/XSD gap) · `TERM`
  (terminology verification) · `SEC` (Specifications Editorial Committee
  decision).

## Reports

One report per register entry that carries an `upstream_ref` (every real
SM↔ITS realization gap plus every report-carrying spec silence/defect). Register
entries realized internally by an existing released endpoint, name-only
CNF↔SM divergences, and benign released-documented behaviours carry no
`upstream_ref` and do NOT appear here.

### UPR-01 — duplicate-`template_id` handling on the ADL 1.4 upload is unassigned

- **Register entry:** AMB-4
- **Channel:** SPECPR
- **Status:** draft
- **Spec citation:** SM `i_definition_adl14.adoc` §`upload_opt` — states
  `Pre_valid` and the error `invalid_template` and says nothing about
  duplicates, where the same file's §`upload_archetype` ("If an archetype with
  the same id already exists, replace it") and SM `i_definition_adl2.adoc`
  §`upload_artefact` ("If an artefact with the same physical identifier and
  namespace exists, replace it") both state the replace rule; ITS-REST
  `specifications/responses/409_template_already_exists.yaml` declares the
  conflict branch on the same operation; AM ADL 1.4 `master02-overview.adoc`
  §Templates defines no template versioning (all confirmed first-hand).
- **Problem:** A second upload under an existing `template_id` has two
  conformant answers — refuse it as a conflict, or replace the stored template
  — and no released sentence picks one. Declaring the 409 branch is not
  requiring it, and `upload_opt` is the only upload in the two Definition
  interfaces that omits the replace rule its siblings state. With no formal
  version for an ADL 1.4 template there is no third answer available either.
- **Ask:** State the duplicate rule on `upload_opt` (replace, as its siblings
  do, or conflict, as the ITS branch implies) so the suite can gate one
  behaviour rather than carry sibling option cases.
- **Note (2026-07-27):** this report was re-grounded with AMB-4. Its earlier
  framing — conflict vs a version-parameter branch, taken from the CNF
  master04 NOTE — is false for Release-1.1.0: the ADL 1.4 upload declares no
  version parameter, and the only released one (the ADL2 upload's `version`
  query parameter) is `deprecated: true` under SPECITS-87.

### UPR-02 — Persistent-COMPOSITION uniqueness per EHR is unspecified

- **Register entry:** AMB-5
- **Channel:** SEC
- **Status:** draft
- **Spec citation:** RM ehr COMPOSITION class §Invariants (Category_validity /
  Territory_valid / Language_valid / Content_valid / Is_archetype_root — none
  requires uniqueness) + RM ehr `master05-composition_package.adoc` §Persistent
  Compositions (silent on uniqueness) — silence confirmed first-hand.
- **Problem:** The RM defines `category = 431|persistent|` and `is_persistent()`
  but carries no invariant constraining a persistent COMPOSITION to be unique per
  EHR (per archetype); whether committing a second persistent COMPOSITION of the
  same archetype must be rejected is undecided (under SEC debate).
- **Ask:** Rule on whether persistent-COMPOSITION uniqueness per EHR is required;
  if so, add the RM invariant; until resolved these verdicts are reported but not
  gated.

### UPR-03 — Empty-directory retrieval representation is unspecified

- **Register entry:** AMB-8
- **Channel:** ITS-REST
- **Status:** draft
- **Spec citation:** RM ehr EHR class §directory (0..1 — a directory is optional)
  + released ITS-REST 1.1.0 directory retrieval docs (overview
  `Requests_and_responses.md`) — no empty-structure representation is defined for
  an EHR that never had a directory.
- **Problem:** `get_directory` on an EHR without a directory is empty-vs-error
  ambiguous — the RM allows no directory, and the released ITS-REST docs describe
  only a not-found outcome, with no empty-structure representation.
- **Ask:** Define the required response for directory retrieval on an EHR with no
  directory (an explicit empty structure, or a normative not-found), so both the
  option branches can be gated.

### SPECPR-368 — EHR_STATUS `incomplete` lifecycle_state

- **Register entry:** AMB-9
- **Channel:** SPECPR
- **Status:** open (filed as SPECPR-368)
- **Spec citation:** RM common `master06-change_control_package.adoc` §Version
  Lifecycle — `lifecycle_state` coded from the 'version lifecycle state' group,
  including `553|incomplete|`.
- **Problem:** The behaviour of committing/retrieving content in the `incomplete`
  lifecycle state is under-specified (the open upstream problem report SPECPR-368).
- **Ask:** Resolve SPECPR-368; affected cases report but do not gate until it
  resolves.

### UPR-04 — physical deletion is defined by SM + ITS-REST and forbidden by the RM, with no reconciliation

- **Register entry:** AMB-10
- **Channel:** SPECPR
- **Status:** draft
- **Spec citation:** RM common `master06-change_control_package.adoc`
  §Contributions ("Since a versioned repository (i.e. a collection of
  `VERSIONED_OBJECTs`) is by definition indelible, all logical changes including
  deletions … are achieved by physically committing new Versions") + §Logical
  Deletion ("Medicolegal and traceability requirements mean that information
  cannot be literally removed … Accordingly, information can only ever be
  logically deleted") CONTRASTED with SM `i_admin_service.adoc`
  §`physical_ehr_delete` ("Physical deletion of specified EHR.") and
  §`physical_party_delete`, and ITS-REST `responses/204_deleted_hard.yaml` ("the
  resource(s) identified by the request parameters has been physically deleted
  (i.e. hard-delete)") + `operations/admin_ehr_delete.yaml` /
  `operations/admin_ehr_delete_all.yaml` (all owned resources "will also be
  **permanently** and physically deleted, in compliance with applicable data
  protection regulations (e.g., the GDPR in the European Union)") — all
  confirmed first-hand.
- **Problem:** RE-SCOPED 2026-07-27 (group-14 audit): this is not a silence but
  a head-on contradiction between released components. The RM states
  indelibility twice, in absolutes, as a *medicolegal* requirement; SM and
  ITS-REST define operations that permanently destroy the record and every
  version of it. Nothing in RM, BASE or SM reconciles the two. The only
  justification offered anywhere is the data-protection clause quoted above, and
  a grep of the entire vendored spec tree finds it in exactly two sentences —
  the two admin operation descriptions (plus the three assembled
  `computable/OAS/admin-*.openapi.yaml` bundles, which repeat them verbatim) —
  so the whole of the RM's indelibility principle is set aside by an aside in an
  ITS operation description, in the ITS layer rather than the model layer.
- **Ask:** reconcile the two — state in RM common that regulator-compelled
  erasure is an exception to §Logical Deletion, or state in SM/ITS-REST that
  physical deletion is an out-of-model administrative facility — and put the
  data-protection justification somewhere normative rather than in two operation
  descriptions. (Ours: split by route family. The two ADMIN routes are physical,
  because the admin descriptions plus `204_deleted_hard.yaml` are the only
  released text describing them and a logical reading would leave them with no
  defined behaviour; every NON-admin delete stays RM-logical, including where SM
  says otherwise — UPR-99/AMB-135 defect (7) decides `I_PARTY.delete_party`
  against the SM and for the RM on exactly that test. The admin success kind is
  `ok_empty`, never `deleted`, because the closed vocabulary defines `deleted`
  as a new `523|deleted|` VERSION — which is what a physical delete does not
  produce.)

### UPR-05 — SM `I_DEFINITION_ADL14.delete_opt` has no ITS-REST wire

- **Register entry:** AMB-17
- **Channel:** ITS-REST
- **Status:** draft
- **Spec citation:** SM `i_definition_adl14.adoc` (`delete_opt` defines the
  operation) vs released ITS-REST 1.1.0 (no DELETE on `/definition/template/adl1.4`
  — overview `Resources.md`).
- **Problem:** The SM defines OPT deletion but the released ITS-REST API surfaces
  no wire for it, so the four official master04 `delete_opt` cases have no bindable
  realization.
- **Ask:** Add an OPT-delete endpoint to the ITS-REST API (SM↔ITS alignment);
  until then the cases verdict not-applicable-with-citation.

### UPR-06 — SM `I_EHR_CONTRIBUTION.list_contributions` has no ITS-REST wire

- **Register entry:** AMB-22
- **Channel:** ITS-REST
- **Status:** draft
- **Spec citation:** SM `i_ehr_contribution.adoc` (`list_contributions` defines
  the operation) vs released ITS-REST 1.1.0 (no collection GET on
  `/ehr/{ehr_id}/contribution`; only the single-CONTRIBUTION GET — overview
  `Resources.md`).
- **Problem:** The SM defines a contribution collection listing but the released
  ITS-REST API exposes only single-CONTRIBUTION retrieval by uid.
- **Ask:** Add a contribution-collection GET to the ITS-REST API (SM↔ITS
  alignment).

### UPR-07 — SM `I_EHR_DIRECTORY.get_versioned_directory` has no ITS-REST wire

- **Register entry:** AMB-24
- **Channel:** ITS-REST
- **Status:** draft
- **Spec citation:** SM `i_ehr_directory.adoc` (`get_versioned_directory` defines
  the operation) vs released ITS-REST 1.1.0 (no `versioned_directory` resource,
  unlike `versioned_composition` / `versioned_ehr_status` — overview
  `Resources.md`).
- **Problem:** The SM defines versioned-directory retrieval but the released
  ITS-REST API has no versioned_directory resource.
- **Ask:** Add a versioned_directory resource to the ITS-REST API (SM↔ITS
  alignment).

### UPR-08 — master17.3 interval range row contradicts BASE Interval invariants

- **Register entry:** AMB-27
- **Channel:** editorial
- **Status:** draft
- **Spec citation:** BASE `foundation_types interval.adoc` §Invariants (the
  four-invariant set — none requires a bound value on an absent
  `*_unbounded`-flagged limit).
- **Problem:** The master17.3 `DV_INTERVAL<DV_TIME>` range table verdicts a
  domain-constraint rejection for a row whose lower bound is absent with
  `lower_unbounded=true` — underivable, since no BASE Interval invariant makes a
  bound-value constraint bite on an absent optional limit. (The remaining
  master17.3 table typos — the accepted/[] contradiction, the millisecond-column
  omission, the integer_fraction type [3] vs [4], the forum "IMO should fail"
  notes — are guide-quality CNF-internal observations, recorded with the
  conversion, NOT part of this report.)
- **Ask:** Correct the underivable range row to accepted per the BASE interval
  invariants.

### UPR-09 — master17.1 DV_BOOLEAN row contradicts AM C_BOOLEAN

- **Register entry:** AMB-28
- **Channel:** editorial
- **Status:** draft
- **Spec citation:** AM aom14 `c_boolean.adoc` (`C_BOOLEAN.true_valid=false`
  disallows a `true` value).
- **Problem:** The master17.1 DV_BOOLEAN `only_false_allowed` table's first row is
  internally inconsistent (expected=accepted while naming a violated constraint);
  AM C_BOOLEAN makes the verdict derivable (rejected). (The master17.2 duplicate
  DV_TEXT `validate_open` heading is a guide-quality CNF-internal observation,
  recorded with the conversion, NOT part of this report.)
- **Ask:** Correct the DV_BOOLEAN row's verdict to rejected per AM C_BOOLEAN.

### UPR-10 — Reduced-precision temporal comparability is unspecified

- **Register entry:** AMB-29
- **Channel:** SEC
- **Status:** draft
- **Spec citation:** RM data_types `DV_ORDERED` class §less_than ("<") +
  §is_strictly_comparable_to (redefined in descendants) + RM data_types
  `master07-date_time_package.adoc` §Partial Date/Times (reduced-accuracy partials
  per ISO 8601; no mixed-precision ordering semantics defined) — silence confirmed
  first-hand.
- **Problem:** DV_ORDERED requires a `<` and strict comparability, but neither the
  RM date/time package nor BASE defines a total order across partial/reduced-precision
  date/times, so the ordering of mixed-precision temporal values is left open.
  (This is the comparability question — distinct from the `Time`→`Iso8601_time`
  typing defect once mislabelled SPECPR-380.)
- **Ask:** Have the SEC decide the comparison/ordering semantics for
  mixed-precision date/times; until resolved these verdicts are reported but not
  gated.

### SPECQUERY-20 — AQL dialect-superset acceptance is unspecified

- **Register entry:** AMB-30
- **Channel:** SPECQUERY
- **Status:** open (filed as SPECQUERY-20)
- **Spec citation:** QUERY AQL grammar + §TERMINOLOGY (Amendment Record
  SPECQUERY-20) + released ITS-REST query `Response.md` (400 defined only for
  invalid AQL, not for a dialect superset).
- **Problem:** No clause forbids a server from accepting an AQL dialect superset
  (a removed construct such as TIMEWINDOW, a non-grammar clause order, or an
  undefined `TERMINOLOGY()` form); interoperability argues for rejection but the
  spec does not mandate it.
- **Ask:** Rule (via SPECQUERY-20) on whether a conformant server must reject
  dialect-superset queries; until then those cases report but do not gate.

### UPR-11 — SM PARTY_RELATIONSHIP operations have no ITS-REST wire

- **Register entry:** AMB-32
- **Channel:** ITS-REST
- **Status:** draft
- **Spec citation:** SM `i_party_relationship.adoc` + `i_demographic_service.adoc`
  (the PARTY_RELATIONSHIP operations) vs released ITS-REST 1.1.0 (the Demographic
  API surfaces PERSON/ORGANISATION/GROUP/AGENT/ROLE but no PARTY_RELATIONSHIP
  resource).
- **Problem:** The SM defines the six PARTY_RELATIONSHIP operations but the
  released ITS-REST API has no relationship resource, so those cases have no
  bindable realization.
- **Ask:** Add PARTY_RELATIONSHIP resources to the ITS-REST Demographic API
  (SM↔ITS alignment).

### UPR-12 — ADMIN realization gaps in both directions

- **Register entry:** AMB-33
- **Channel:** ITS-REST
- **Status:** draft
- **Spec citation:** SM `i_admin_service.adoc` / `i_admin_dump_load.adoc` /
  `i_admin_archive.adoc` (`list_contributions`, `contribution_count`,
  `versioned_composition_count`, `composition_version_count`,
  `physical_party_delete`, `export_ehrs`, `load_ehrs`, `archive_ehrs`,
  `archive_parties`) and `docs/openehr_platform/master15-admin_service.adoc`
  (the chapter's full include list) vs released ITS-REST 1.1.0
  `admin.openapi.yaml` (exactly two paths, `/admin/ehr/{ehr_id}` and
  `/admin/ehr/all{?ehr_id*}`) + `operations/admin_ehr_delete_all.yaml` +
  `parameters/query/ehr_id_Admin.yaml` — confirmed first-hand.
- **Problem:** SM→ITS — of the ten operations the three SM admin interfaces
  declare, the released wire realizes one; counts, listing, party deletion,
  dump/load and archive have no endpoint. ITS→SM (WIDENED 2026-07-27, group-14
  audit) — the reverse gap was invisible until now: `admin_ehr_delete_all`,
  half the entire released Admin API, realizes NO SM operation. The only
  candidate, `physical_ehr_delete`, takes `an_ehr_id: UUID [1]` — one mandatory
  id, no list form and no all-EHRs form — so it can express neither the
  unfiltered call nor the multi-id subset; the two `I_ADMIN_ARCHIVE` operations
  do take `List<UUID> [0..1]` with the same optional-means-all shape, but they
  archive ("Move selected EHRs to archival storage"), which is the opposite of
  destroying. Nothing else in SM is set-scoped. Same class as UPR-91 and
  UPR-100.
- **Ask:** add the missing ADMIN operations to the ITS-REST API, and give the
  bulk delete an SM home — either a set-scoped `physical_ehr_delete` overload
  taking `List<UUID> [0..1]` (matching the archive operations' shape) or a
  distinct SM operation. (Ours: the unwired SM operations carry explicit
  `unrealized` bindings, so their master12 cases are not-applicable WITH
  CITATION and activate untouched if a release adds the endpoints; the bulk
  route is anchored to `physical_ehr_delete` under `variant: delete_all` — the
  same catalogue naming convention UPR-91/UPR-100 use, never an SM claim — so it
  is exercised and visible on the SM-keyed surface gate instead of falling
  through it.)

### UPR-13 — SM MESSAGE / EHR-Extract / TDD has no ITS-REST wire

- **Register entry:** AMB-34
- **Channel:** ITS-REST
- **Status:** draft
- **Spec citation:** SM `i_ehr_extract_service.adoc` / `i_tdd_service.adoc` /
  `i_message_service.adoc` (export/import EHR(_extract), import TDD(s)) vs released
  ITS-REST 1.1.0 (no MESSAGE / extract / tdd API).
- **Problem:** The SM defines the EHR-Extract and TDD services but the released
  ITS-REST API has no MESSAGE/extract/tdd wire, so every master13 case is
  unrealizable.
- **Ask:** Add a MESSAGE / EHR-Extract / TDD API to ITS-REST (SM↔ITS alignment).

### UPR-14 — SM ADL2 generic-artefact operations have no ITS-REST wire

- **Register entry:** AMB-37
- **Channel:** ITS-REST
- **Status:** draft
- **Spec citation:** SM `i_definition_adl2.adoc` (`upload_artefact` /
  `get_artefact` / `has_artefact` / `delete_artefact` + `list_*` / `*_count`) vs
  released ITS-REST 1.1.0 (OPT-only wire under `/definition/template/adl2`).
- **Problem:** The SM ADL2 interface is artefact-generic (archetypes, templates,
  OPTs), but the released ITS-REST API surfaces only OPT wire — no archetype
  provisioning, generic listing, DELETE, or `*_count`.
- **Ask:** Add ADL2 archetype provisioning + generic listing + delete + count +
  example-generation operations to the ITS-REST API (SM↔ITS alignment).

### UPR-15 — Deprecated SDT-era media-type handling is unspecified

- **Register entry:** AMB-39
- **Channel:** ITS-REST
- **Status:** draft
- **Spec citation:** ITS-REST simplified_formats `master02-overview.adoc` §MIME
  Types + §Overview (SDT rename NOTE) — only the current
  `application/openehr.wt.flat+json` / `application/openehr.wt.structured+json`
  MIME types are defined; SDT was superseded by Simplified Formats at Release 1.1.0.
- **Problem:** Whether a server must still accept the legacy pre-1.1.0
  "Simplified Data Template (SDT)"-era media types is implementation-defined; the
  spec neither mandates acceptance nor mandates rejection (415).
- **Ask:** State the required handling of the deprecated SDT media types
  (accept vs `415`), so the suite can gate one behaviour rather than carry sibling
  option cases.

### UPR-16 — SM ADL 1.4 archetype provisioning has no ITS-REST wire

- **Register entry:** AMB-41
- **Channel:** ITS-REST
- **Status:** draft
- **Spec citation:** SM `i_definition_adl14.adoc` (ADL 1.4 archetype provisioning:
  upload / get / list over archetypes) vs released ITS-REST 1.1.0 (OPT-only wire
  under `/definition/template/adl1.4`).
- **Problem:** The SM defines ADL 1.4 archetype provisioning alongside OPT
  provisioning, but the released ITS-REST API has no ADL 1.4 archetype endpoint.
- **Ask:** Add ADL 1.4 archetype-provisioning endpoints to the ITS-REST API
  (SM↔ITS alignment); recorded meanwhile as an explicit certificate scope
  exclusion.

### UPR-17 — AOM 1.4 date/time-constraint fields are unserializable in ITS-XML 1.0.2

- **Register entry:** AMB-42
- **Channel:** ITS-XML
- **Status:** draft
- **Spec citation:** AM aom14 `c_time.adoc` + `c_date_time.adoc`
  (`millisecond_validity`) + `c_duration.adoc` (`seconds_allowed` /
  `fractional_seconds_allowed`) + AM ADL1.4 `master05-cadl.adoc` §Patterns vs
  ITS-XML 1.0.2 `Archetype.xsd` (C_TIME/C_DATE_TIME/C_DURATION sequences carry no
  such elements).
- **Problem:** AOM 1.4 defines constraint fields the CNF content schedule tests,
  but the ITS-XML 1.0.2 OPT serialization has no element for them, so those
  ground OPTs cannot exist on the OPT 1.4 wire.
- **Ask:** Add the missing `millisecond_validity` / per-field duration elements to
  the ITS-XML `Archetype.xsd` so the AOM fields are serializable.

### UPR-18 — BASE Interval has no bound-presence invariant

- **Register entry:** AMB-43
- **Channel:** SPECPR
- **Status:** draft
- **Spec citation:** BASE `foundation_types interval.adoc` §Attributes
  (lower/upper 0..1) + §Invariants (the four-invariant set — none requires a bound
  value when the matching `*_unbounded` flag is false).
- **Problem:** An interval `{lower_unbounded: false, no lower; upper_unbounded:
  false, no upper}` violates no BASE 1.3.0 invariant, while the CNF content lineage
  expects such an interval to be rejected as malformed (older revisions carried a
  bound-required invariant; 1.3.0 does not).
- **Ask:** Confirm whether the bound-presence invariant should be restored to
  BASE Interval; until then a bounded-flag interval with absent limits is accepted.

### UPR-19 — DV_TIME literals: CNF T-marker vs BASE Iso8601_time

- **Register entry:** AMB-44
- **Channel:** editorial
- **Status:** draft
- **Spec citation:** BASE `foundation_types iso8601_time.adoc` §Description — the
  value syntax is `hh:mm:ss[(,|.)sss][Z|±hh[:mm]]` (extended) or `hhmmss…`
  (compact), with no leading `T` in either form.
- **Problem:** The master17.4 §DV_TIME NOTE ("our test data sets all include the
  T time marker") and the master17.3 interval-of-time tables print T-prefixed time
  literals, which BASE Iso8601_time (DV_TIME.value's type oracle) does not admit.
- **Ask:** Remove the leading `T` from the schedule's DV_TIME data sets (the `T`
  marks the date-time separator and duration designators only).

### UPR-20 — master17.4 DV_DATE_TIME T-precision block is arithmetically underivable

- **Register entry:** AMB-45
- **Channel:** editorial
- **Status:** draft
- **Spec citation:** BASE `foundation_types interval.adoc` §has() (total-order
  containment).
- **Problem:** The master17.4 DV_DATE_TIME range table's T-precision block tests
  values anchored at 2021-10-24 against ranges anchored at 1900-03-13 yet expects
  accepted — impossible under BASE Interval containment.
- **Ask:** Re-anchor the block's values to the ranges' 1900-03-13 date (preserving
  each row's precision axis and verdict) so every row is derivable.

### UPR-21 — Commit-time CONSTRAINT_REF enforcement is not derivable from AOM 1.4

- **Register entry:** AMB-46
- **Channel:** SPECPR
- **Status:** draft
- **Spec citation:** AM AOM1.4 `master04-constraint_model_package.adoc` §Reference
  Objects (C_REFERENCE_OBJECT / CONSTRAINT_REF — the constraint definition lives
  outside the archetype, in the `constraint_bindings` query into an external
  service).
- **Problem:** No AOM 1.4 clause mandates rejecting a committed instance whose
  terminology no binding covers, yet the schedule's `validate_ext_term` row 4
  verdicts rejected.
- **Ask:** Clarify whether commit-time terminology-binding enforcement of a
  CONSTRAINT_REF is required; until then the unmatched-terminology row asserts
  acceptance.

### UPR-22 — master16 HISTORY cells contradict RM Events_valid

- **Register entry:** AMB-51
- **Channel:** editorial
- **Status:** draft
- **Spec citation:** RM data_structures `history.adoc` §Invariants — Events_valid:
  `(events /= Void and then not events.is_empty) or summary /= Void`.
- **Problem:** Two master16 HISTORY table rows verdict "no events | absent |
  accepted" (a HISTORY with zero events and no summary), which violates
  Events_valid regardless of any archetype events cardinality.
- **Ask:** Correct the two cells to rejected (violation `rm_invariant(Events_valid)`)
  per RM history.adoc.

### UPR-23 — commit_contribution mismatched-change_type rejection has no assigned status code

- **Register entry:** AMB-54
- **Channel:** ITS-REST
- **Status:** draft
- **Spec citation:** RM common `master06-change_control_package.adoc` §Change
  Control (addition = a first ORIGINAL_VERSION, `preceding_version_uid` Void,
  `change_type 249|creation|`; modification = a new ORIGINAL_VERSION on an
  existing VERSIONED_OBJECT, `change_type 251|modification|`) requires the
  rejection; ITS-REST `overview/Requests_and_responses.md` §HTTP status codes
  assigns no specific code (permits 400 generic or 422 semantic); the ITS-REST
  docs text has no CONTRIBUTION status-code table.
- **Problem:** Committing a CONTRIBUTION whose `change_type` mismatches the
  version state (a `251|modification|` as the first version; a `249|creation|`
  naming a `preceding_version_uid`) must be rejected per RM change-control, but
  the released ITS-REST spec assigns no status code — servers may return 400 or
  422, so conformance cannot gate a single code.
- **Ask:** Assign a specific status code (400 or 422) to the
  mismatched-`change_type` rejection on contribution commit in the ITS-REST
  spec (docs text + OAS), so the behaviour is gateable.

### UPR-24 — DV_CODED_TEXT.value has no value==rubric invariant

- **Register entry:** AMB-55
- **Channel:** editorial (RM)
- **Status:** draft
- **Spec citation:** RM data_types `dv_coded_text.adoc` §Description ("A text
  item whose value must be the rubric from a controlled terminology") and
  `dv_text.adoc` §Attributes value ("For DV_CODED_TEXT, this is the rubric of
  the complete term") vs `dv_text.adoc` §Invariants (`Valid_value: not
  value.is_empty` — the sole invariant on value) and `dv_coded_text.adoc`
  §Invariants (none beyond `defining_code` 1..1).
- **Problem:** The "value must be the rubric" statement is Description prose,
  not an invariant, so a committed DV_CODED_TEXT whose value is a non-empty
  string that is not the coded rubric violates no RM invariant — conformance
  cannot require value==rubric (doing so was reverted as overreach in PR #263).
- **Ask:** Either promote the "must be the rubric" statement to a formal
  invariant, or clarify it is authoring guidance, so the value==rubric
  expectation is unambiguously (non-)gateable.

### UPR-25 — Accept quality-factor (q-value) negotiation is undefined

- **Register entry:** AMB-56
- **Channel:** ITS-REST
- **Status:** draft
- **Spec citation:** ITS-REST `overview/Resources.md` §Data representation
  ("The client SHOULD use the Accept ... request header to specify the expected
  ... response format. If the service cannot fulfil this aspect of the request,
  it MUST respond with HTTP status code 406 Not Acceptable") +
  `overview/Requests_and_responses.md` §Representation details negotiation
  (Prefer verbosity only). The ITS-REST corpus is silent on q-values (no "q=",
  "quality", or "weight").
- **Problem:** ITS-REST defines only that an unfulfillable Accept yields 406; it
  assigns no behaviour to a weighted or multi-type Accept (q-value ordering,
  `*/*` wildcard resolution), so q-value strictness cannot be gated.
- **Ask:** Define (or explicitly delegate to RFC 7231) the q-value / weighted
  Accept semantics a conformant server must apply, so the behaviour is gateable.

### UPR-26 — No simplified inner-data surface for commit_contribution

- **Register entry:** AMB-57
- **Channel:** ITS-REST
- **Status:** draft
- **Spec citation:** ITS-REST `simplified_formats/master02-overview.adoc`
  §Scope (covers "Mapping between the Simplified Formats and canonical openEHR
  RM (e.g. COMPOSITION)"; CONTRIBUTION not in scope) + §MIME Types (only the
  two wt COMPOSITION MIME types) vs the released ITS-REST contribution-create
  operation (CONTRIBUTION envelope body, canonical JSON/XML only).
- **Problem:** There is no ITS-REST 1.1.0 wire for committing a CONTRIBUTION
  whose inner versions carry simplified-format COMPOSITIONs, so the behaviour
  cannot be exercised as a normative case.
- **Ask:** Define a simplified-format inner-data surface for
  `commit_contribution` (or state explicitly that CONTRIBUTION commit is
  canonical-only), so the boundary is authoritative rather than inferred.

### UPR-27 — RM and ITS-REST disagree on the form of the uid copied into a top-level object

- **Register entry:** AMB-65
- **Channel:** SPECPR
- **Status:** draft
- **Spec citation:** RM common `master03-archetyped_package.adoc` §Unique Node
  Identification — for "the top-level types such as `COMPOSITION`,
  `EHR_STATUS`, `PARTY` etc" it is recommended "to set the `_uid_` value to a
  copy of the `_uid.object_id()_` value of the owning `VERSION` object …
  i.e. the leading Uid, which is normally a Guid", worked through as
  `87284370-2D4B-4e3d-A3F3-F303D2F4F34B::uk.nhs.ehr1::2` →
  `87284370-2D4B-4e3d-A3F3-F303D2F4F34B`; the RM ehr `EHR_STATUS` class
  §Description restates the same object_id()-only rule. ITS-REST overview
  `Resources.md` §Identifier types recommends the opposite for the same
  attribute: the inherited `uid` "be populated using the `uid` copied from the
  enclosing VERSION object", worked through — from the identical example value
  — as `87284370-2D4B-4e3d-A3F3-F303D2F4F34B::uk.nhs.ehr1::2` →
  `87284370-2D4B-4e3d-A3F3-F303D2F4F34B::uk.nhs.ehr1::2`.
- **Problem:** Two released components give conflicting recommendations for
  one attribute, using the same worked example and producing different values
  (a HIER_OBJECT_ID vs an OBJECT_VERSION_ID). `LOCATABLE._uid_` is typed
  `UID_BASED_ID`, so both are model-legal and nothing arbitrates. Implementers
  split, and a client cannot tell from the specification whether a served
  top-level object's `uid` carries a version coordinate — which ITS-REST's own
  §Identifier types then relies on for request addressing ("the COMPOSITION
  identifier `8849182c-…::openEHRSys.example.com::1`, taken from
  COMPOSITION.uid.value, which also implies that the VERSIONED_OBJECT
  identifier is `8849182c-…` and the latest version is `1`").
- **Ask:** Align the two sentences on one form. The three-part
  OBJECT_VERSION_ID is the one that keeps ITS-REST's addressing derivation
  sound and loses no information (the RM's Guid is its first segment); if the
  RM's object_id()-only rule is intended to stand, ITS-REST §Identifier types
  needs a different basis for deriving the version coordinate from a served
  body.

### UPR-28 — SM `create_ehr.Pre_no_subject` is unsatisfiable and contradicts RM + ITS-REST

- **Register entry:** AMB-66
- **Channel:** SPECPR
- **Status:** draft
- **Spec citation:** SM `i_ehr_service.adoc` §`create_ehr` and
  §`create_ehr_with_id` both state `Pre_no_subject`:
  `an_ehr_status.subject = Void`, and both follow it two lines later with "A
  default `_subject_` will be generating containing a `PARTY_SELF` object".
  RM ehr `EHR_STATUS` class §Attributes types `subject` as `1..1 PARTY_SELF`
  ("The subject of this EHR"). The released ITS-REST EHR API surfaces no
  for-subject create operation, so the optional `EHR_STATUS` request body is
  the only channel through which a client can ever give an EHR a subject.
- **Problem:** The precondition cannot be satisfied by any valid argument: an
  `EHR_STATUS` with a Void subject violates the RM's 1..1 cardinality, so for
  every well-formed `an_ehr_status` the precondition is false. It also
  contradicts the sentence immediately beneath it, and enforcing it would make
  subject-bearing EHRs uncreatable over the released REST wire — stranding the
  subject-keyed operations the same interface defines
  (`create_ehr_for_subject`, `get_ehrs_for_subject`, `has_ehr_for_subject`),
  which would have nothing to read.
- **Ask:** Remove `Pre_no_subject` from `create_ehr` / `create_ehr_with_id`,
  or restate it as the intended constraint (presumably that the caller need
  not pre-populate the subject, the server defaulting it to an anonymous
  `PARTY_SELF` when absent) so it is satisfiable and consistent with the RM
  cardinality and the REST surface.

### UPR-29 — no released text states whether a served top-level object carries a populated `uid` (non-COMPOSITION scope)

- **Components:** RM common (locatable, master03), ITS-REST (Resources.md §Identifier types)
- **Register:** AMB-67 (report_only)
- **Facts:** `LOCATABLE.uid` is 0..1; master03 §Unique Node Identification
  recommends the top-level copy yet states the field "will usually be empty
  in most EHR data in most openEHR EHR systems"; the ITS-REST note is scoped
  to "COMPOSITION objects" and is "strongly recommended"/"should". No
  released sentence assigns presence (or absence) for the other top-level
  types (EHR_STATUS, EHR_ACCESS, FOLDER, PARTY) on the wire.
- **Problem:** clients cannot rely on the served `uid` of a contained
  non-COMPOSITION object, and conformance instruments cannot test it — the
  behaviour is untestable recommendation territory outside COMPOSITION.
- **Ask:** extend the Resources.md §Identifier types note (or the RM
  master03 section) to state the wire expectation for ALL top-level types,
  with an RFC keyword.

### UPR-30 — RM REVISION_HISTORY class table contradicts itself on item ordering

- **Component:** RM common (revision_history)
- **Register:** AMB-68 (fixed_handling)
- **Facts:** `org.openehr.rm.common.revision_history.adoc` Purpose: "The list
  is in most-recent-first order"; the `items` attribute Meaning: "in
  most-recent-last order"; both `most_recent_version` and
  `most_recent_version_time_committed` postconditions read `items.last`.
- **Problem:** the Purpose sentence and the attribute Meaning cannot both
  hold; implementations picking by the Purpose sentence serve the reverse
  order.
- **Ask:** correct the Purpose sentence to most-recent-last (the order the
  postconditions require).

### UPR-31 — VERSIONED_OBJECT.owner_id wire values are unstated

- **Components:** RM common (versioned_object), BASE (object_ref), ITS-REST
- **Register:** AMB-69 (fixed_handling)
- **Facts:** `owner_id` is OBJECT_REF 1..1 ("the id of the containing EHR or
  other relevant owning entity"); BASE bounds `namespace` lexically; no
  released sentence assigns `namespace`/`type` values for a served
  VERSIONED_* container, while the reverse edge (EHR.ehr_status.type =
  "VERSIONED_EHR_STATUS") IS an RM invariant.
- **Problem:** clients cannot rely on the served `owner_id` shape, and
  conformance instruments cannot test it beyond OBJECT_REF well-formedness.
- **Ask:** state the expected `owner_id.namespace`/`type` for EHR-owned
  version containers (e.g. `local`/`EHR`) in the RM or the ITS-REST docs
  text.

### UPR-32 — "version extant at time T" has no defined semantics

- **Components:** RM common (versioned_object), SM (i_ehr_status), ITS-REST
- **Register:** AMB-70 (fixed_handling)
- **Facts:** RM gives only `version_at_time (a_time)` with
  `Pre: has_version_at_time`; no released text defines the anchoring
  instant, interval closure, branch participation, future-time behaviour, or
  whether the no-parameter "latest" is `latest_version()` or
  `latest_trunk_version()`.
- **Problem:** two conformant servers can return different versions for the
  same `version_at_time` request; the behaviour is untestable beyond
  self-consistency.
- **Ask:** define extant-at-time (anchor = commit_audit.time_committed,
  closure, branch rule, future-time = latest) and name the "latest"
  function for the parameterless read.

### UPR-33 — §Location and §Prefer disagree on Location for updates

- **Component:** ITS-REST (overview Requests_and_responses.md)
- **Register:** AMB-71 (fixed_handling)
- **Facts:** §Location: "The `Location` header MUST ONLY be used for
  resource creation (e.g., `201 Created`) or redirect responses." §Prefer
  (return=minimal): the response "SHOULD include a `Location` header
  pointing to the newly created or updated resource." Updates return
  200/204, never 201.
- **Problem:** a server cannot satisfy both sentences on an update
  response; conformance instruments cannot gate Location on updates either
  way.
- **Ask:** reconcile the two sentences (e.g. restate §Location as
  "creation, change-controlled writes, or redirects").

### UPR-34 — five editorial defects in SM I_EHR_STATUS

- **Component:** SM (i_ehr_status.adoc)
- **Register:** AMB-72 (editorial)
- **Facts/Problems:** (1) `get_versioned_ehr_status` carries
  `Pre_has_ehr_status_version (an_ehr_id, a_version_uid)` — `a_version_uid`
  is not a parameter; (2) `get_ehr_status_at_time` omits the `an_ehr_id`
  its precondition uses; (3) version identifiers are typed `UUID` where the
  value is an OBJECT_VERSION_ID; (4) the at-time/at-version functions
  return EHR_STATUS while the ITS-REST versioned_ehr_status/version routes
  serve the VERSION envelope (approximate realization); (5)
  `clear_ehr_modifiable` says "this ensures it is treated as active",
  contradicting its own postcondition `not …is_modifiable` (and misspells
  `_is_modifable_`). SM also defines no revision-history operation, leaving
  the REST route anchored only by RM `VERSIONED_OBJECT.revision_history()`.
- **Ask:** correct the preconditions/signatures/typing/wording; consider
  adding container + revision-history read operations to I_EHR_STATUS.

### UPR-35 — template stability across versions of one VERSIONED_COMPOSITION is unstated

- **Components:** RM (versioned_composition, common change_control), SM (i_ehr_composition), ITS-REST
- **Register:** AMB-73 (report_only)
- **Facts:** VERSIONED_COMPOSITION pins `archetype_node_id` and
  `is_persistent` across versions (its two invariants) but not
  `archetype_details.template_id`; no released sentence permits or forbids
  a template change on update. The CNF schedule expects rejection but is a
  stalled guide.
- **Problem:** servers legitimately diverge; the behaviour is untestable.
- **Ask:** state whether template_id is version-stable (an invariant beside
  Archetype_node_id_valid, or an explicit permission).

### UPR-36 — deleted-composition read branches are only partially assigned

- **Component:** ITS-REST (composition_get + the versioned_composition read ops)
- **Register:** AMB-74 (fixed_handling)
- **Facts:** the only deleted branch on the bare route
  (`204_deleted_at_time`) is textually scoped to "at specified
  `version_at_time`"; a GET of a deleted version by explicit version_uid,
  or of the implicit latest when the trunk head is deleted, has no branch;
  the versioned envelope routes declare 200/404 only, and the overview
  docs never mention logical deletion.
- **Problem:** the read behaviour for deleted content — the case
  medicolegal traceability exists for — is unassigned on five routes.
- **Ask:** assign the deleted-read branches (suggested: 204 on the bare
  resource routes for all addressing forms; 200 with the data-less
  ORIGINAL_VERSION on the VERSION-envelope routes).

### UPR-37 — the composition DELETE 204's ETag identity is unstated

- **Component:** ITS-REST (204_version_deleted + headers/ETag)
- **Register:** AMB-75 (fixed_handling)
- **Facts:** the delete commits a NEW `523|deleted|` version (RM master06
  §Logical Deletion); the 204's ETag references the generic header with no
  statement whether it carries the new version's uid or the addressed
  (preceding) one.
- **Problem:** clients chaining on the DELETE's ETag (e.g. to read the
  deletion audit back) cannot rely on which version it names.
- **Ask:** state the identity (suggested: the new deleted version's uid —
  the resource's current state).

### UPR-38 — no branch is assigned for lexically malformed identifier path segments

- **Components:** ITS-REST (overview + parameter files), BASE (base_types)
- **Register:** AMB-76 (fixed_handling)
- **Facts:** `ehr_id` / `versioned_object_uid` are HIER_OBJECT_IDs per the
  glossary; BASE's UID subtypes have "mutually exclusive string patterns"
  but no class-level lexical invariant beyond non-empty; the per-operation
  response maps declare no 400 for a malformed segment, while the overview
  400 row covers "syntactically invalid … parameter".
- **Problem:** 400-vs-404 for a garbage identifier token is unassigned and
  implementations diverge (the stalled Robot suite itself answers both
  ways across sibling cases).
- **Ask:** assign the malformed-identifier branch explicitly.

### UPR-39 — editorial defects in SM I_EHR_COMPOSITION + three ITS-REST doc typos

- **Components:** SM (i_ehr_composition, i_validity_checker), ITS-REST
- **Register:** AMB-77 (editorial)
- **Facts/Problems:** (1) `get_composition_latest` precondition references
  the undeclared `a_version_uid`; (2) `get_composition_at_version` breaks
  the interface's error-name vocabulary (`ehr_does_not_exist` /
  `object_version_does_not_exist`); (3) the same operation declares no
  preconditions; (4) `delete_composition` types `a_version_uid` as UUID
  while its Meaning requires an OBJECT_VERSION_ID; (5)
  `update_composition` lacks a `preceding_version_uid` parameter (flagged
  by the CNF schedule itself, master07); (6) the preconditions call
  `valid_content(...)` where I_VALIDITY_CHECKER declares
  `content_valid(...)`; (7) REST 412/409/400-already-deleted have no SM
  counterparts and SM's `composition_already_exists` has no wire branch.
  Typos: `headers/ETag_COMPOSITION.yaml` example closes `W/"…'` with a
  single quote; `operations/composition_get.yaml` "is be used";
  `Resources.md` §Multiple identifiers example writes a single colon
  before the version_tree_id.
- **Ask:** correct the signatures/preconditions/typing/error names and the
  three examples; consider adding the missing concurrency/conflict error
  vocabulary.

### UPR-40 — the docs text never states the body-uid vs URL rule for a COMPOSITION update

- **Component:** ITS-REST (overview Resources.md + Requests_and_responses.md)
- **Register:** AMB-78 (report_only)
- **Facts:** `Resources.md` §Identifier types derives the container from the
  served identifier ("which also implies that the VERSIONED_OBJECT identifier
  is `8849182c-…`") and calls populating a COMPOSITION's inherited `uid`
  "strongly recommended"/"should"; RM types `LOCATABLE.uid` 0..1. The
  normative prose never says what a service does when an update body's
  `uid` names a DIFFERENT container than the addressed one — the requirement
  exists only as an API-definition operation description.
- **Problem:** the rule that clients are actually held to is invisible in the
  normative text, and its branch is unassigned: 422 (well-formed but
  unfollowable), 400 (client error), or silent acceptance with the
  server's own uid are all defensible, so implementations diverge on a
  request that is trivially easy to send by accident (a read-modify-write
  client replaying a fetched COMPOSITION at the wrong URL).
- **Ask:** state the rule in the docs text and assign its branch (suggested:
  reject with 422 — the request is well-formed but cannot be followed).

### UPR-41 — create-on-existing-directory has no wire branch

- **Components:** ITS-REST (directory_create), SM (i_ehr_directory)
- **Register:** AMB-79 (fixed_handling)
- **Facts:** SM `Pre_no_directory` establishes the rule; the operation
  declares 201/400/404 only; the stalled Robot suite asserts 409 with the
  in-file admission "NOTE: @PABLO this is not (yet) in the SPEC".
- **Ask:** bind the branch (suggested: 409).

### UPR-42 — update/delete of a nonexistent directory has no wire branch

- **Components:** ITS-REST (directory_update/delete), SM (i_ehr_directory)
- **Register:** AMB-80 (fixed_handling)
- **Facts:** the only 404 is scoped "when an EHR with ehr_id does not
  exist"; 412 requires a latest version to mismatch; SM Pre_has_directory
  has no error code.
- **Ask:** widen the 404 scoping or bind another branch.

### UPR-43 — the directory `path` parameter grammar is under-specified

- **Components:** ITS-REST (parameters/query/path), SM (has_path), RM
  (master05 §Paths — a different grammar)
- **Register:** AMB-81 (fixed_handling)
- **Facts:** one released sentence; leading slash, root-implicitness,
  duplicate-sibling disambiguation, and escaping are all unstated; the
  RM's bracket uniqueness-modifier path convention is never adopted.
- **Ask:** define the grammar (root-implicit, encoding, disambiguation).

### UPR-44 — the is_modifiable refusal has no status code

- **Components:** RM (master04 §EHR Active Status), ITS-REST
- **Register:** AMB-82 (fixed_handling)
- **Facts:** RM requires content writes (Compositions AND Folders) to be
  refused on a deactivated EHR; no released wire text mentions
  is_modifiable outside the EHR-creation defaults.
- **Ask:** assign the refusal branch (suggested: 409).

### UPR-45 — I_EHR_DIRECTORY editorial defects + two response-file contract defects

- **Components:** SM (i_ehr_directory, i_validity_checker), ITS-REST
- **Register:** AMB-83 (editorial)
- **Facts:** ehr_id/an_ehr_id spelling mixed across one interface; UUID
  typing of version/ehr ids; valid_content vs content_valid recurring in a
  third interface; update_directory "Create or update" vs its own
  Pre_has_directory + no preceding_version_uid parameter;
  get_directory_at_version with no preconditions and the wrong error set;
  inconsistent error declaration on the boolean probes; write operations
  typed 0..1 with no return; no SM counterpart for 412; three boolean
  probes with no route. ITS-REST: the update's 200/204 branches select
  different Location/ETag header contracts; 412_directory declares a
  Location solely to deprecate it.
- **Ask:** normalize the interface; align the update response header
  contracts; drop the deprecated Location declaration from 412.

### UPR-46 — empty CONTRIBUTION versions list has no released rejection ground

- **Components:** ITS-REST (NewContribution schema), RM (contribution class)
- **Register:** AMB-84 (fixed_handling)
- **Facts:** the commit schema has no minItems (the read schema does); RM
  CONTRIBUTION declares no invariants; no branch covers the case.
- **Ask:** add minItems to the commit schema or a non-empty invariant.

### UPR-47 — three contribution commit rejections remain unassigned (AMB-54 residue)

- **Component:** ITS-REST (contribution_create)
- **Register:** AMB-54 (fixed_handling, narrowed 2026-07-27)
- **Facts:** `400_CONTRIBUTION` assigns first-version-of-a-MODIFICATION →
  400; creation-with-preceding, stale/unknown preceding (no If-Match/412 on
  this op), and out-of-group tokens remain unassigned.
- **Ask:** assign the three residual branches, and assign (or decline) an
  identifying header for the body-borne precondition failure (no single
  latest `version_uid` exists on a multi-member contribution).

### UPR-48 — contribution GET's simplified promise contradicts its declared schema

- **Component:** ITS-REST (contribution_get / 200_CONTRIBUTION / Contribution schema)
- **Register:** AMB-57 (fixed_handling, corrected 2026-07-27)
- **Facts:** the §Simplified Formats paragraphs promise `versions[i].data`
  serialization; the declared read body carries OBJECT_REFs with no data;
  no Prefer parameter binds resolve_refs.
- **Ask:** align the paragraph with the schema or bind
  `Prefer: return=representation, resolve_refs` explicitly.

### UPR-49 — the commit schema has no deletion shape

- **Components:** ITS-REST (UpdateVersion/Version schemas), RM (master06 §Logical Deletion)
- **Register:** AMB-85 (fixed_handling)
- **Facts:** RM deletion commits a data-less version; both released
  schemas make `data` required.
- **Ask:** make `data` conditional (absent/Void on `523|deleted|` members).

### UPR-50 — committal-header scope on the native contribution POST is unstated

- **Component:** ITS-REST (Requests_and_responses §openehr-version…)
- **Register:** AMB-86 (fixed_handling)
- **Facts:** the MUST-accept sentence is scoped in context to the
  convenience methods; the native body carries the audit; no precedence
  rule exists for header-vs-body conflict.
- **Ask:** state the header scope on the native route.

### UPR-51 — ETag/Last-Modified on the contribution GET is unassigned

- **Component:** ITS-REST (200_CONTRIBUTION vs the ETag/Last-Modified SHOULD)
- **Register:** AMB-87 (fixed_handling)
- **Facts:** the SHOULD names "unique state identifiers"; Resources.md
  classifies CONTRIBUTION non-versioned; the 200 declares neither header;
  the 201 carries the uid ETag.
- **Ask:** declare the headers on the 200 or scope the SHOULD.

### UPR-52 — CONTRIBUTION.versions ref type: RM wording vs the released example

- **Components:** RM (contribution class), BASE (object_ref), ITS-REST (Contribution schema)
- **Register:** AMB-88 (fixed_handling)
- **Facts:** RM: refs to "Versions"; the released example emits the data
  class as `type` with an OBJECT_VERSION_ID id; the RM's own second
  sentence leans to the data-class reading.
- **Ask:** reconcile.

### UPR-53 — IMPORTED_VERSION cannot be committed through the released wire

- **Components:** RM (master06 §Copying/§Contributions), ITS-REST (NewContribution), SM (i_ehr_contribution)
- **Register:** AMB-89 (fixed_handling)
- **Facts:** RM mandates import "as part of a Contribution"; the commit
  schema is UpdateVersion-only (no item/uid/oneOf); the SM envelope
  likewise; ImportedVersion appears only in read unions.
- **Ask:** define the import commit shape (with foreign-identity
  preservation).

### UPR-54 — I_EHR_CONTRIBUTION + commit-DTO editorial defect bundle

- **Components:** SM (i_ehr_contribution, update_version, update_audit, master03), ITS-REST, RM
- **Register:** AMB-90 (editorial)
- **Facts/Asks:** precondition/postcondition arity and undeclared
  identifiers; both error-name and parameter spellings in one interface;
  no SM validation/conflict/precondition errors (wire 400/409/412/422
  unmapped); audit-vs-commit_audit naming; missing SM signature +
  system_id (contradicted by released SPECITS-95/SPECPR-472); stale
  Terminology_code typing with a .defining_code-dereferencing invariant;
  description String/DV_TEXT/UDvText three-way divergence; no envelope uid
  parameter. Plus the minor silences: system_id validation criterion,
  glossary omission of contribution_uid, the 204-only Content-Type
  exemption vs the minimal 201, cross-EHR 404 by implication, atomicity as
  an unrestated "should", copy-down vs per-member committer. Normalize the
  interface and DTOs to the released ITS-REST 1.1.0 typing.

### UPR-55 — wrong-kind uid on a typed tag route is unassigned

- **Component:** ITS-REST (the 404 files of all seven typed tag families —
  `404_unknown_ehr_id_or_uid_based_id{,_or_key}.yaml` on the EHR side,
  `404_unknown_uid_based_id{,_or_key}.yaml` on the demographic side)
- **Register:** AMB-91 (fixed_handling, widened 2026-07-27)
- **Facts:** the released surface has seven typed tag families, not two — the
  EHR-side composition/ehr_status pair plus the five demographic party routes
  (byte-identical modulo the type name). The only released 404 text is "when
  the `uid_based_id` does not exist", so neither the within-space mismatch (a
  PERSON's uid on `/demographic/organisation/{uid_based_id}/tags`) nor the
  cross-space one (a COMPOSITION uid on a party tag route, or the reverse) is
  assigned an outcome.
- **Ask:** state whether a kind mismatch is the "does not exist" 404 (our
  reading, applied on GET/PUT/DELETE across all seven families and in both
  directions) or a 400.

### UPR-56 — return=identifier on a tag PUT cannot satisfy the identifier contract

- **Components:** ITS-REST (Prefer enum + §Prefer only identifier), RM (item_tag)
- **Register:** AMB-92 (fixed_handling)
- **Ask:** assign the branch or exclude the token for collection
  resources without a uid.

### UPR-57 — RM-invariant violations on the tag PUT have no status

- **Components:** RM (item_tag invariants), ITS-REST (the seven tag update
  responses + the five party updates that DO declare 422)
- **Register:** AMB-93 (fixed_handling, widened 2026-07-27)
- **Facts:** all seven tag PUTs carry the identical 200/204/400/404 set and
  none declares 422, although an invariant-violating body is well-formed JSON
  satisfying the schema — the case the overview's own 422 row describes. The
  party PUTs sharpen it: their `<t>_update.yaml` siblings on the same resource
  DO declare 422, so one service answers semantic invalidity two ways
  depending on whether the body is a PARTY or its tags.
- **Ask:** assign 422 (our handling, on all seven) or 400 explicitly.

### UPR-58 — the tag-list filter grammar is undefined

- **Component:** ITS-REST (`ehr_tags_get` and `demographic_tags_get` + the
  three shared query params)
- **Register:** AMB-94 (fixed_handling, widened 2026-07-27)
- **Facts:** both released list routes reference the SAME three parameter
  files and repeat the same "filtered by the given one or more" sentence
  verbatim, while the three schemas are scalar strings with no `description`
  field at all — so the mismatch, the combination rule, the match mode and
  the absent-path case are unstated identically on both.
- **Ask:** define repeatability, combination, match mode, and
  absent-path matching; reconcile "one or more" with the scalar schemas.
  (Ours: scalar, AND-combined, exact, case-sensitive, one rule for both
  routes. The demographic route's unbounded SCOPE is reported separately —
  UPR-102.)

### UPR-59 — duplicate identity pairs in one tag PUT body

- **Component:** ITS-REST (UpdateItemTag array)
- **Register:** AMB-95 (fixed_handling)
- **Ask:** uniqueItems or a merge rule (ours: last-wins).

### UPR-60 — empty-string vs absent target_path splits the tag identity

- **Components:** RM (item_tag target_path 0..1), ITS-REST (six of the seven
  `ItemTagOf<T>` examples)
- **Register:** AMB-96 (fixed_handling, widened 2026-07-27)
- **Facts:** `target_path: ""` is the released MAJORITY, not a stray: the
  EHR_STATUS example and all five demographic examples (Person, Agent, Group,
  Organisation, Role) carry it, and only the COMPOSITION example uses a real
  path. No released example omits the attribute — so an example-following
  client emits `""` for an unpathed tag as a matter of course, while the RM
  types the attribute 0..1 with no non-empty invariant, making `""` and
  absent two distinct identities under the (key, target_path) rule.
- **Ask:** pick one representation (ours: "" normalizes to absent, on both
  families — the only reading under which such a client's tags stay
  addressable by the DELETE-by-key route).

### UPR-61 — the ITEM_TAG editorial defect bundle

- **Components:** RM (item_tag, is_justified), BASE (String), ITS-REST, ITS-XML
- **Register:** AMB-97 (editorial, widened 2026-07-27)
- **Facts/Asks:** define `is_justified` (or restate the invariant on the
  prose rule); fix the "(logically) deleted" wording; give BOTH space-wide
  lists a spanning schema — the EHR-wide list declares the
  COMPOSITION-specific one and `/demographic/tags` declares
  `200_PERSON_ItemTagList_retrieved` for a response spanning all five party
  kinds, so a conformant ROLE tag contradicts the declared schema; drop the
  "associated with any target within given EHR" sentence from
  `demographic_tags_get`, the one tag route with no EHR (a copy-paste from
  `ehr_tags_get`); settle the container-form wording inside the party tag
  family, whose get/delete say the HIER_OBJECT_ID is "taken from
  `VERSIONED_PARTY.uid.value`" while the update in the same family says
  "`VERSIONED_OBJECT.uid.value`" for the resource its next sentence calls
  "the target VERSIONED_PARTY container"; fix the copy-pasted `_updated`
  descriptions (all seven are byte-identical to their `_retrieved`
  sibling — "successfully retrieved" on a PUT); either
  add a canonical-XML ITEM_TAG type or drop XML from the tag enums; fix
  the HIER_OBJECT_ID-with-type-COMPOSITION example and add a
  VERSION-targeted example; align the DELETE descriptions' "tag_key" with
  the {key} parameter; give FOLDER tags a route (or scope the overview);
  define {key} encoding, list ordering/paging across all nine GETs, and the
  optional-feature refusal branch; state tag lifecycle vs target
  lifecycle; note the development-RM provenance of the whole class.
  Not a spec defect, noted for the same reader: the five party
  `<t>_update.yaml` omit the `openehr-item-tag` request parameter while
  their own prose says "a `openehr-item-tag` or `openehr-version-item-tag`
  request header can be set" (their `<t>_create.yaml` siblings declare
  both) — an API-definition omission against released prose, which the
  prose wins; we accept both headers there.

### UPR-62 — REST paging vs AQL LIMIT/OFFSET is unlegislated

- **Components:** ITS-REST (Request.md, the fetch bullet), QUERY (AQL §TOP/§LIMIT)
- **Register:** AMB-98 (fixed_handling)
- **Facts:** the exclusion clause targets the deprecated TOP; LIMIT (its
  replacement, same release) has no combination rule; no status is
  assigned even to the stated prohibition.
- **Ask:** update the clause for LIMIT and define the composition (ours:
  REST windows over the AQL-limited set).

### UPR-63 — POST parameter-source precedence is unassigned

- **Component:** ITS-REST (Request.md SHOULD-list vs the body-only POST declarations)
- **Register:** AMB-99 (fixed_handling)
- **Ask:** assign the precedence (ours: equal accepted, conflict 400).

### UPR-64 — protocol parameter names are not reserved from $parameter binding

- **Component:** ITS-REST (Request.md; the QueryParameters/StoredQuery examples collide)
- **Register:** AMB-100 (fixed_handling)
- **Ask:** reserve q/ehr_id/offset/fetch (ours) or rename the protocol
  parameters; fix the colliding examples.

### UPR-65 — ehr_id scoping semantics + the unknown-EHR branch

- **Components:** ITS-REST (Request.md §About the ehr_id parameter), SM (i_query_service)
- **Register:** AMB-101 (fixed_handling)
- **Facts:** routing-hint wording only; no filter/predicate/interaction
  rule; the SM's ehr_id_does_not_exist error has no wire realization.
- **Ask:** define the scoping effect and bind the unknown-EHR branch
  (ours: scope-constrained execution; unknown → 404 realizing the SM
  error).

### UPR-66 — malformed version/name selectors have no branch

- **Component:** ITS-REST (parameters/path/version.yaml + 404_Query_version)
- **Register:** AMB-102 (fixed_handling)
- **Ask:** assign the malformed-selector branch (ours: 404 — an
  unmatchable selector is a version that does not exist).

### UPR-67 — the Query editorial defect bundle

- **Components:** SM (i_query_service + the execute-spec/result-set classes, master08), ITS-REST, QUERY, ITS-XML, CNF
- **Register:** AMB-103 (editorial)
- **Facts/Asks:** reconcile the SM RESULT_SET requiredness inversion, the
  competing identifier grammar, exact-only version matching, the
  Hash<String,String> parameter typing (vs the AQL quoting NOTE), the
  unrealized ehr_ids list, the SM-only negative-value semantics, the
  wrapped row shape; surface the executed stored version on the wire
  (the flattened descriptor loses version/formalism/registration_time);
  fix "Rox data."/the empty Meaning/the left-in TODO; fix Request.md's
  FETCH-keyword example + malformed at-code; vendor the missing AQL
  grammar files; fix the .yaml-suffixed operationIds; give 400_Query an
  error body; reconcile JSON-only with the canonical-format MUST (the
  ITS-XML RESULT_SET is a draft stub); add the query terms to the
  glossary; fix the CNF querying schedule's demographic-service link.

### UPR-68 — no grammar for an ADL 1.4 `template_id`, yet partial resolution is promised

- **Components:** ITS-REST (`parameters/path/template_id.yaml`), AM (ADL 1.4 `master02-overview.adoc` §Templates)
- **Register:** AMB-104 (option_select)
- **Facts:** the path parameter promises "A partial `template_id` will resolve
  to 'latest' major version of that template" and shows legacy examples
  ("Vital Signs", "vital_signs.v1"), but no released component defines a
  legacy template-id grammar, so nothing says which part of such an id is the
  artefact and which the version. The ADL2 side has that grammar (AOM2
  ARCHETYPE_HRID + the `{version}` prefix rule); ADL 1.4 has none.
- **Ask:** define the ADL 1.4 `template_id` grammar, or scope the
  partial-resolution promise to identifiers whose version part is defined
  (ours: an ADL 1.4 id is always complete, so only exact matches resolve).

### UPR-69 — SM says replace, ITS-REST says 409, for the same ADL2 upload

- **Components:** SM (`i_definition_adl2.adoc` §upload_artefact), ITS-REST (`definition_template_adl2_upload.yaml`, `409_template_already_exists.yaml`)
- **Register:** AMB-105 (fixed_handling)
- **Facts:** "If an artefact with the same physical identifier and namespace
  exists, replace it" against "`409 Conflict` is returned when a template with
  same `template_id` already exists" — two released components, mutually
  exclusive outcomes for one request.
- **Ask:** reconcile in either direction (ours: the wire follows the ITS 409).

### UPR-70 — the SM's ADL 1.4 OPT key is unobtainable from the wire

- **Components:** SM (`i_definition_adl14.adoc`), ITS-REST (`parameters/path/template_id.yaml`, `schemas/definition/TemplateMetadata.yaml`)
- **Register:** AMB-106 (report_only)
- **Facts:** `has_opt` / `get_opt` / `delete_opt` take `an_opt_id: UUID` and
  `list_opts` returns `List<UUID>`, while the sibling `list_matching_opts`
  returns `List<ARCHETYPE_ID>` — the interface disagrees with itself. The wire
  keys everything by the `template_id` string and its list metadata carries no
  `uid` at all, so a client cannot obtain the SM key even in principle.
- **Ask:** settle one identifier for ADL 1.4 OPTs across the two components,
  or surface the UUID on the wire if the SM key is meant to be real.

### UPR-71 — regex in the SM, `*` wildcards in ITS-REST, for the same filter

- **Components:** SM (`i_definition_adl14.adoc`, `i_definition_adl2.adoc` — the `list_matching_*` operations), ITS-REST (`parameters/query/filter_template_id.yaml`, `concept.yaml`, both list operations)
- **Register:** AMB-107 (fixed_handling)
- **Facts:** the SM matches `id_pattern` as a regex and declares an
  `invalid_id_pattern` error; the ITS filters "support wildcards `*`" and the
  list operations declare a single 200 with no rejection branch. `*` is legal
  in both languages with different meanings, so one filter string selects
  different sets under the two readings, and the SM's error has no wire code.
- **Ask:** state one pattern language for both components, and either bind the
  invalid-pattern error to a status or drop it (ours: glob, no rejection).

### UPR-72 — Definition realization gaps in both directions

- **Components:** SM (`i_definition_adl14.adoc`), ITS-REST (the `/definition/template/*` route set, `/example` under SPECITS-58)
- **Register:** AMB-108 (report_only)
- **Facts:** SM→ITS — `valid_opt`, `has_opt`, `opts_count` and
  `list_matching_opts` have no ADL 1.4 wire. ITS→SM — the `/example`
  sub-resource added in Release-1.1.0 exists on both template routes and
  realizes no SM operation at all, so a released wire behaviour has no
  service-model anchor.
- **Ask:** realize or retire the four OPT operations, and give example
  generation an SM operation. (The neighbouring gaps are UPR-05 / UPR-14 /
  UPR-16.)

### UPR-73 — the TEMPLATE editorial defect bundle

- **Components:** ITS-REST (`definition_template_adl2_upload.yaml` example, `docs/simplified_formats/master04-basic_concepts.adoc`, `schemas/web_template/WebTemplate.yaml`), AM (`OPT2/master02-overview.adoc`, `OPT2/master03-opt_raw.adoc`)
- **Register:** AMB-109 (editorial)
- **Facts/Asks:** the ADL2 upload's request example carries a `specialize`
  clause in a body typed as an `OperationalTemplateV2`, which AM forbids ("no
  specialisation statement") and whose `adl_operational_template` ANTLR rule
  has no such production — fix the example. The Web Template metadata example
  carries a `semVer` member the released `WebTemplate` schema does not declare
  — add it to the schema or drop it from the example, and say how it relates
  to the declared `version`.

### UPR-74 — the template uploads' unassigned response branches

- **Components:** ITS-REST (both upload operations, `responses/400.yaml`, both `201_Template_*_upload.yaml`, `docs/overview/Requests_and_responses.md`)
- **Register:** AMB-110 (fixed_handling)
- **Facts:** the uploads declare 201/400/409 only, and the released 400 trigger
  is explicitly syntactic — so a body that parses but is not a valid
  operational template has no assigned status, even though the SM requires the
  upload to fail. Separately, the operations' own 201 text ("If the `Prefer`
  header is missing or set to `return=minimal`, the body is empty")
  contradicts the overview's "If no response body is returned, the service
  SHOULD use `204 No Content`".
- **Ask:** add the semantic-rejection status to both uploads (ours: 422, the
  overview's own well-formed-but-semantically-wrong code), and reconcile the
  201-empty text with the SHOULD-204 sentence (ours: the operation text wins).

### UPR-75 — template retrieval fidelity is unstated

- **Components:** ITS-REST (both template GET operations and their 200 responses), SM (`get_opt` / `get_artefact`)
- **Register:** AMB-111 (fixed_handling)
- **Facts:** the ADL 1.4 GET says it can return "the original (canonical) `XML`
  based OPT format" and the ADL2 GET says only that it retrieves the template;
  neither says whether the served artefact must be byte-identical to the
  uploaded one, semantically equivalent after a round trip, or merely of the
  same identity — a real difference, since re-serialization can reorder,
  normalize or expand content the client validated against.
- **Ask:** state the fidelity rule on both retrievals (ours: verbatim).

### UPR-76 — do ETag/Last-Modified apply to a template resource?

- **Components:** ITS-REST (`docs/overview/Requests_and_responses.md` §ETag and Last-Modified; the template 200/201 responses)
- **Register:** AMB-112 (fixed_handling)
- **Facts:** the SHOULD is scoped to "VERSION, VERSIONED_OBJECT, or other
  resources that have versioning or unique state identifiers"; a template is
  neither named kind, and the trailing clause is undefined. A template has a
  unique identity but no state that changes — the release offers no template
  update — so a `Last-Modified` would track nothing.
- **Ask:** say whether templates fall under the SHOULD, and if so what
  `Last-Modified` means for a resource that never changes (ours: a weak ETag
  naming the resolved identity, no `Last-Modified`).

### UPR-77 — the deprecated ADL2 version endpoint: no signalling, no pre-release SEMVER

- **Components:** ITS-REST (`definition_template_adl2_version_get.yaml`, `parameters/path/version.yaml`, `docs/overview/Requests_and_responses.md`), AM (`Identification/master04-versioning.adoc`)
- **Register:** AMB-113 (fixed_handling)
- **Facts:** the operation is `deprecated: true` (SPECITS-87) yet fully
  specified, and the release defines no `Deprecation`/`Sunset` header and no
  withdrawal contract — served and withdrawn are equally conformant. The
  `{version}` selector defines only exact three-part versions and `{major}` /
  `{major}.{minor}` prefixes, saying nothing about the SEMVER pre-release forms
  AM admits or how a prefix ranks them.
- **Ask:** define deprecation signalling and the withdrawal contract, and
  extend the version selector to the pre-release forms (ours: served
  unsignalled; pre-release selectors match exact strings only).

### UPR-78 — the template list filters are never specified as a query

- **Components:** ITS-REST (`parameters/query/{filter_template_id,concept,filter_version,offset,fetch}.yaml`, both list operations)
- **Register:** AMB-114 (fixed_handling)
- **Facts:** nothing says how multiple filters combine, what an `offset` at or
  beyond the end of the result set returns, or what the `version` filter means
  for an id whose version grammar is undefined (UPR-68). The version filter's
  "if missing, then only the latest version will be returned" default also
  sits oddly beside a list schema whose `version` member is deprecated and an
  example response that carries none.
- **Ask:** define filter combination (ours: conjunction), the past-the-end page
  (ours: an empty list), and the version filter's meaning for legacy ids.

### UPR-79 — the example routes' unsupported-value branch is unassigned

- **Components:** ITS-REST (`definition_template_adl1.4_example_get.yaml`, `definition_template_adl2_example_get.yaml`, `parameters/query/example_{type,detail_level}.yaml`)
- **Register:** AMB-115 (fixed_handling)
- **Facts:** "it will fall back to the closest supported level, or it may
  return an error (typically `400 Bad Request`)" offers two conformant
  behaviours with no rule for choosing, and the ADL2 twin — identical
  parameters, identical enums, identical 400 response — carries no such
  sentence at all, leaving it unclear whether the latitude applies there.
  Neither operation covers a value outside the declared enums.
- **Ask:** assign the unsupported-value branch (ours: 400, never a silent
  substitution) and give the ADL2 operation the same prose as its ADL 1.4
  twin.

### UPR-80 — a missing mandatory ADL2 section belongs to both AM phases

- **Component:** AM (`OPT2/master03-opt_raw.adoc` §Artefact Structure, `AOM2/master08-validation.adoc` §Phase 1 - Basic Integrity)
- **Register:** AMB-116 (fixed_handling)
- **Facts:** the `adl_operational_template` ANTLR rule makes `language`,
  `description`, `definition`, `terminology` and `component_terminologies`
  grammar productions, so a source lacking one fails to parse; the AOM2
  validation catalogue's first list ALSO owns the same defect ("any missing
  mandatory parts, e.g. `terminology` section (STCNT)"), so a validator is
  equally entitled to report it. AM assigns no phase, and the two phases carry
  different wire statuses downstream.
- **Ask:** state which phase reports a missing mandatory section, or make the
  grammar and validity catalogues disjoint (ours: whichever phase detects it —
  so the conformance catalogue pins only phase-unambiguous fixtures).

### UPR-81 — a stored query is overwritable and immutable, and the minted version is unstated

- **Components:** ITS-REST (`operations/definition_query_store.yaml`, `tags/StoredQuery_schema.md`, `responses/409_StoredQuery_version.yaml`, `docs/query/Qualified_query_name.md`, `headers/Location_Query.yaml`)
- **Register:** AMB-117 (fixed_handling)
- **Facts:** the version-less store "Stores a new query, or updates an existing
  query on the system" and declares no conflict branch, while the resource tag
  says stored queries are an "immutable way to identify a specific AQL
  statement" and the versioned store declares the 409 immutability implies.
  Separately, nothing says which `version` a version-less store writes: the
  operation takes none, and `Qualified_query_name.md`'s absent/partial-version
  rule is a rule for USING a stored query, not for creating one. The only
  illustration is the Location example's `…/1.0.1`.
- **Ask:** say that the version-less store is the overwriting form and the
  versioned store the immutable one (or assign the conflict), and state the
  version a version-less store mints (ours: a constant `1.0.0` default slot,
  updated in place, even when higher versions exist).

### UPR-82 — SM `store_query` returns a descriptor and permits a nameless store; the wire does neither

- **Components:** SM (`i_definition_query.adoc` §store_query, `query_descriptor.adoc`), ITS-REST (`parameters/path/qualified_query_name.yaml`, `responses/200_StoredQuery_stored.yaml`, `schemas/query/StoredQuery.yaml`)
- **Register:** AMB-118 (report_only)
- **Facts:** SM types the store as returning a `QUERY_DESCRIPTOR` and says "If
  no name is provided, one is created in the service. Return a Query descriptor
  containing the query name and unique identifier." The wire makes the name a
  required path segment (so the nameless form cannot be requested and a minted
  name could not be delivered) and answers a bodyless 200 (so no descriptor is
  returned). The promised "unique identifier" is in neither model:
  `QUERY_DESCRIPTOR` declares no identifier attribute, and the wire
  `StoredQuery` carries name/type/version/saved/q.
- **Ask:** reconcile the store's signature with its wire realization — either
  make the name optional and return the descriptor, or drop the nameless form
  and the identifier sentence from the SM.

### UPR-83 — the overview's two rules for `Location` on a write disagree about the status

- **Components:** ITS-REST (`docs/overview/Requests_and_responses.md` §Location + §"Prefer minimal, identifier or full representation response", both stored-query store operations)
- **Register:** AMB-119 (fixed_handling)
- **Facts:** §Location says "The `Location` header MUST ONLY be used for
  resource creation (e.g., `201 Created`) or redirect responses", while the
  §Prefer branches say the response "SHOULD include a `Location` header
  pointing to the newly created or updated resource" (minimal) and "MAY
  include a `Location` header" where "The HTTP status is typically `201
  Created` or `200 OK`" (identifier, representation). Both stored-query stores
  are assigned `200` and never `201`, so nothing says which passage governs
  them.
- **Ask:** say whether the §Location status rule or the §Prefer branch governs
  a creation answered with `200` (ours: the §Prefer default-minimal sentence —
  the stores declare no `Prefer` parameter — so a 200 carries the `Location`
  of the resource written).

### UPR-84 — two grammars for one stored-query identifier, and a casing example that contradicts the schema

- **Components:** SM (`master04-definition_package.adoc` §Registered Queries + §Query Formalism), ITS-REST (`docs/query/Qualified_query_name.md`, `parameters/query/query_type.yaml`, `schemas/query/{QueryType,StoredQuery}.yaml`, `schemas/definition/QueryList.yaml`)
- **Register:** AMB-120 (fixed_handling)
- **Facts:** SM admits `<namespace>::<query-name>` AND
  `<namespace>::<formalism>::<query-name>`, defaults a missing namespace to
  `"misc"`, and treats the formalism case-insensitively with an optional
  `::`-separated version defaulting to major "1". ITS-REST defines only the
  two-part form with an optional namespace and NO default, plus a flat
  `query_type` string. So the SM's own three-part example is not a legal ITS
  name, and `my_compositions` is one identifier under ITS but
  `misc::my_compositions` under SM. Independently, `query_type` and
  `QueryType` declare `default: "AQL"` while every released example spells the
  served member `"aql"`, with no rule saying whether the value is echoed or
  normalized.
- **Ask:** state one identifier grammar across the two components (including
  whether the `misc` default applies on the wire), and settle the formalism
  casing (ours: the ITS two-part wire grammar, the SM `misc` default and
  three-part decomposition as the storage key, and the declared `type` echoed
  as stored).

### UPR-85 — SM's regex + paged query listing has no wire

- **Components:** SM (`i_definition_query.adoc` §list_queries, §list_matching_queries), ITS-REST (`operations/definition_query_list.yaml`)
- **Register:** AMB-121 (report_only)
- **Facts:** SM lists by two PERL regexes — one on the query identifier, one on
  the archetype/template identifiers referenced inside the query text — with
  `item_offset`/`items_to_fetch` paging and an `invalid_id_pattern` error. The
  wire has one list operation keyed by a required name matched as a PREFIX,
  with no paging, no artefact-reference filter, and a single 200 branch, so the
  SM error has no status to wear.
- **Ask:** realize or retire `list_matching_queries`, and state whether the
  released list is meant to page (ours: prefix matching only, no paging, no
  rejection branch).

### UPR-86 — the versioned store's `{version}` grammar is described only for reads

- **Components:** ITS-REST (`parameters/path/version.yaml`, `operations/definition_query_version_store.yaml`, `responses/400.yaml`)
- **Register:** AMB-122 (fixed_handling)
- **Facts:** the one `{version}` description covers reads and writes alike and
  is written in resolution terms ("a pattern as partial prefix … the highest
  (latest) version matching the prefix will be considered"), which cannot apply
  to a store: a prefix selects among existing versions and a store creates one.
  No released sentence assigns an outcome to a prefix, a SEMVER pre-release, or
  a malformed token on the write; the 409 covers only a duplicate pair.
- **Ask:** define the write-side version grammar (ours: an exact numeric
  `major.minor.patch`, everything else `400` — a non-numeric or partial stored
  version would break the read-side prefix resolution and the SEMVER ordering
  the same surface requires).

### UPR-87 — store-time validity is required by the SM, unassigned by the ITS, and undefined by QUERY

- **Components:** SM (`i_definition_query.adoc` §store_query `Pre_valid_query`, §valid_query), ITS-REST (both store operations, `responses/400.yaml`, `docs/query/Qualified_query_name.md` §NOTE, `parameters/query/query_type.yaml`), QUERY 1.1.0
- **Register:** AMB-123 (fixed_handling)
- **Facts:** SM makes a valid query text a PRECONDITION of the store, but the
  precondition names `is_valid_query` while the interface declares
  `valid_query` — a naming defect that leaves it formally dangling. ITS-REST
  declares only the general syntactic 400 and never mentions validity, and
  QUERY 1.1.0 says nothing about stored queries at all, so no semantic validity
  contract exists for a store. Two neighbouring branches are equally
  unassigned: a store under the reserved query-name `aql` (a MUST-NOT with no
  status) and a store declaring an unsupported `query_type`.
- **Ask:** fix the `is_valid_query`/`valid_query` naming, and assign the three
  branches (ours: a syntactic parse failure, the reserved name, and an
  unsupported formalism are each `400`; no semantic status exists on this
  surface).

### UPR-88 — the stored-query list: no empty-result branch, no order, no paging, an unreachable wildcard

- **Components:** ITS-REST (`operations/definition_query_list.yaml`, `parameters/path/qualified_query_name.yaml`, `schemas/definition/QueryList.yaml`, `definition.openapi.yaml`), SM (`i_definition_query.adoc` §list_queries)
- **Register:** AMB-124 (fixed_handling)
- **Facts:** the operation declares a single 200 and never says what a pattern
  matching nothing returns, never orders the array (the example happens to be
  version-ascending), exposes none of the SM's paging, and carries a clause —
  "when is empty, it will be treated as "wildcard" in the search" — describing
  a request the release cannot express: the name parameter is `required: true`
  and no bare `/definition/query` path is declared.
- **Ask:** assign the empty-result branch, state the listing order, decide
  whether the operation pages, and either declare the wildcard route or drop
  the clause (ours: `200 []`, name + SEMVER-ascending, no paging; our bare
  listing route is a flagged extension exercised by no case).

### UPR-89 — do ETag/Last-Modified/If-Match apply to a stored query?

- **Components:** ITS-REST (`docs/overview/Requests_and_responses.md` §"ETag and Last-Modified" + §"If-Match and accidental overwrites"; the stored-query responses and operations)
- **Register:** AMB-125 (fixed_handling)
- **Facts:** the SHOULD covers "VERSION, VERSIONED_OBJECT, or other resources
  that have versioning or unique state identifiers", and a stored query is
  version-addressed and carries a `saved` timestamp — yet no stored-query
  response declares either header. The version-less store is an overwriting
  write with no `preceding_version_uid` in its path, the exact shape §If-Match
  addresses, and it declares no `If-Match` parameter.
- **Ask:** say whether stored queries fall under the conditional-header SHOULD,
  and whether the overwriting store takes `If-Match` (ours: none of the three
  is emitted or honoured — a stored query is not a change-controlled version
  tree, and no released stored-query response declares the headers).

### UPR-90 — stored-query retrieval fidelity is unstated

- **Components:** ITS-REST (`operations/definition_query_version_get.yaml`, `responses/200_StoredQuery_get.yaml`, `schemas/query/StoredQuery.yaml`)
- **Register:** AMB-126 (fixed_handling)
- **Facts:** the read "Retrieves the definition of a particular stored query
  (at specified version) and its associated metadata" and types `q` as the AQL
  text, but never says whether the served text is byte-identical to what was
  stored, a re-printed equivalent, or merely an equivalent query — a real
  difference, since a parse-and-print round trip re-cases, re-indents and
  collapses the client's own definition.
- **Ask:** state the fidelity rule on the stored-query read (ours: verbatim —
  a re-printed definition would change the artefact under an unchanged name
  and version).

### UPR-91 — `I_DEFINITION_QUERY` realization gaps in both directions

- **Components:** SM (`i_definition_query.adoc`), ITS-REST (the `/definition/query/**` route set)
- **Register:** AMB-127 (report_only)
- **Facts:** SM→ITS — of nine operations the wire realizes two (`store_query`
  in both forms, `list_queries`); `delete_query` (with pre/postconditions),
  `list_matching_queries`, `queries_count` and `store_query_set` have no
  endpoint, and `store_query_set`'s released Meaning is "Register a query set.
  TODO: determine details."; `has_query` and `valid_query` exist only
  indirectly (through the list read and the store). ITS→SM — the released
  single-definition read `definition_query_version_get` realizes NO SM
  operation, because the interface declares no get-a-query function.
- **Ask:** realize or retire the four unrealized operations, finish
  `store_query_set`, and give the single-definition read an SM operation. (The
  neighbouring Definition-side gaps are UPR-05 / UPR-14 / UPR-16 / UPR-72.)

### UPR-92 — the party routes offer Simplified Formats that have no PARTY shape and no template channel

- **Components:** ITS-REST (`docs/overview/Requests_and_responses.md` §openehr-template-id; `docs/overview/Resources.md` §Simplified Formats + §"Data representation"; `docs/simplified_formats/` master02 + master05; the party create/update/get operations and their `Accept`/`Content-Type` enums)
- **Register:** AMB-128 (fixed_handling)
- **Facts:** the party create/update/get operations negotiate
  `application/openehr.wt.flat+json` and `application/openehr.wt.structured+json`
  alongside the canonical types, but the Simplified Formats sub-specification is
  template-derived by construction ("Field identifiers are specific to each
  operational template"; "field identifiers are generated from OPT definitions")
  and its RM-mapping chapter's only top-level mapping is COMPOSITION — no
  demographic PARTY root is mapped anywhere. The header that names the template
  on a Simplified commit is scoped by a MUST to one class ("whenever committing
  COMPOSITION"), and none of the ten party create/update operations declares it,
  so a FLAT party commit has no way to identify the template its field
  identifiers would come from.
- **Ask:** either give the Simplified Formats a demographic shape and the party
  commits a template-naming channel, or remove the two Simplified MIME types
  from the party routes' negotiation. (Ours: the party routes are
  canonical-only — a Simplified `Content-Type` is 415 and a Simplified-only
  `Accept` is 406, both by the §Simplified Formats MUSTs.)

### UPR-93 — `PARTY.Uid_mandatory` is an invariant the create contract cannot satisfy

- **Components:** RM demographic (`org.openehr.rm.demographic.party.adoc` §Description NOTE + §Invariants; `master02-demographic_package.adoc` §Party Identification), RM common (`master06-change_control_package.adoc` §Version Identification), ITS-REST (`operations/person_create.yaml`)
- **Register:** AMB-129 (fixed_handling)
- **Facts:** the PARTY class table carries `Uid_mandatory: uid /= Void` as an
  invariant while the NOTE in the same table calls populating that attribute
  "strongly recommended"; §Party Identification then fixes the value as "the
  `_uid_` attribute (type `OBJECT_VERSION_ID`) of the containing `VERSION`",
  which a create request cannot know because the version does not exist yet. No
  sentence says whether the invariant is evaluated on the request or on the
  committed object. The NOTE's own example is internally inconsistent too: it
  prescribes the value "copied from the `_object_id()_`" — the leading Guid —
  and then copies the full `87284370-…::uk.nhs.ehr1::2`.
- **Ask:** reconcile the invariant with the NOTE's recommendation strength,
  state the evaluation point, and fix the object_id()-versus-example
  contradiction. (Ours: the server mints and injects the uid; the invariant
  holds post-assignment on the stored and served PARTY, in the full three-part
  form — the same form UPR-27/AMB-65 settles for top-level objects generally.)

### UPR-94 — `PARTY_RELATIONSHIP.Target_valid` is uncomputable, asymmetric, and contradicts PARTY

- **Components:** RM demographic (`org.openehr.rm.demographic.party_relationship.adoc` §Invariants; `org.openehr.rm.demographic.party.adoc` §Functions + §Invariants), BASE (`org.openehr.base.base_types.party_ref.adoc`, `…object_ref.adoc`)
- **Register:** AMB-130 (editorial)
- **Facts:** `Target_valid: target /= Void and then not
  target.reverse_relationships.has (self)` dereferences `reverse_relationships`
  on a `PARTY_REF`, which inherits `OBJECT_REF` and declares only
  `namespace`/`type`/`id`. Its sibling `Source_valid` has the same
  wrong-type dereference but asserts the POSITIVE membership, and PARTY's own
  `Reverse_relationships_validity` invariant plus the `reverse_relationships()`
  postcondition both assert the positive over exactly the set `Target_valid`
  negates. Even read charitably the expression is empty:
  `reverse_relationships` is `List<LOCATABLE_REF>`, so `has (self)` compares a
  `PARTY_RELATIONSHIP` to references and is never True.
- **Ask:** restate both invariants over features their declared types have
  (resolution through the demographic repository, as the `reverse_relationships`
  postcondition already does), fix the `Target_valid` polarity, and align the
  membership test with the `LOCATABLE_REF` element type. (Ours: neither
  invariant is asserted on the wire; the released structure they gesture at —
  relationships stored inline in the source party, `source`/`target` as
  container-scoped `PARTY_REF`s — is.)

### UPR-95 — the typed demographic routes assign no branch to a wrong subtype or a wrong-kind container

- **Components:** ITS-REST (the five `{kind}_create` and `{kind}_get` operations; `docs/overview/Requests_and_responses.md` §HTTP status codes), RM common (`master06-change_control_package.adoc` §Typing)
- **Register:** AMB-131 (fixed_handling)
- **Facts:** the demographic API is five parallel typed route families over one
  abstract class. A body whose `_type` is another concrete PARTY subtype than
  the route's (a `ROLE` posted to `/demographic/person`) is well-formed JSON and
  a well-formed RM object, and the create's 400/422/404 set assigns it nothing.
  A `uid_based_id` naming a container that exists but holds another kind (a
  PERSON's container addressed via `/demographic/organisation/{uid_based_id}`)
  is a well-formed identifier resolving to a real container, and the get's
  200/204/404 set assigns it nothing — although §Typing makes a version
  container single-typed, so the addressed resource genuinely does not exist.
- **Ask:** assign both branches explicitly on the typed demographic routes.
  (Ours: wrong subtype in the body is 422 — the request is well-formed and the
  failure is semantic; wrong kind behind the uid is 404 — the addressed
  resource does not exist.)

### UPR-96 — a party create's body `uid` has no rule, where the CONTRIBUTION create has one

- **Components:** ITS-REST (`operations/person_create.yaml` and its four siblings; `operations/contribution_create.yaml`; `operations/person_update.yaml`), RM demographic (`master02-demographic_package.adoc` §Party Identification)
- **Register:** AMB-132 (fixed_handling)
- **Facts:** the CONTRIBUTION create states the rule for a client-supplied
  envelope uid ("when provided, it will be accepted in case is not in-use,
  otherwise error will be returned") and the party update states a match rule
  against the URL; the party CREATE states neither, declares no conflict
  branch, and leaves open whether a body uid is honoured, ignored or refused.
  The value is also not one a client can legitimately choose: §Party
  Identification makes the PARTY uid the containing VERSION's
  OBJECT_VERSION_ID, whose object_id() part is the container the create is
  about to mint.
- **Ask:** say what a party create does with a body `uid`, as the CONTRIBUTION
  create already does. (Ours: the create succeeds and server-minted identity
  wins — the body value does not survive, because the identifier is derived
  from a version this very request creates.)

### UPR-97 — `resolve_refs` is described generically but cannot be requested on the reference-densest API

- **Components:** ITS-REST (`docs/overview/Requests_and_responses.md` §"Prefer resolving Object references"; the five party gets and the four versioned_party reads), RM demographic (`party.adoc`, `role.adoc`, `actor.adoc`)
- **Register:** AMB-133 (report_only)
- **Facts:** the preference is a bare client-side MAY with one worked context
  ("retrieving lists of COMPOSITION resources within an EHR"), assigning the
  service no obligation, no status and no failure mode. The demographic
  payloads are the most reference-dense the API has —
  `PARTY.relationships[i].source`/`.target` and `ROLE.performer` are
  `PARTY_REF`s, `ACTOR.roles` is a list of them, `PARTY.reverse_relationships`
  is a list of `LOCATABLE_REF`s — yet no demographic read declares a `Prefer`
  parameter at all, so the preference cannot be expressed on the resources it
  would help most.
- **Ask:** declare `Prefer` on the demographic reads and state what resolving a
  `PARTY_REF` yields, or say the preference does not apply there. (Ours: not
  implemented on any demographic read — an honest boundary carried in the
  wire-surface register; asserting either behaviour would over-read the MAY.)

### UPR-98 — the two CONTRIBUTION commit routes describe different formats, and neither says what an XML envelope is

- **Components:** ITS-REST (`operations/contribution_create.yaml`, `operations/demographic_contribution_create.yaml`, `docs/overview/Resources.md` §"Data representation" + §XML/JSON/Simplified Formats, `docs/overview/Amendment_record.md` SPECITS-84)
- **Register:** AMB-134 (fixed_handling)
- **Facts:** the EHR-side commit's own prose adds a Simplified-Formats arm and
  confines it — "the CONTRIBUTION envelope itself remains canonical JSON … Only
  the inner versioned payload … is serialized in the chosen FLAT or STRUCTURED
  form" — while the same operation negotiates `application/xml` for the whole
  exchange, a combination the envelope-stays-canonical-JSON rule cannot
  express. The demographic-side commit carries the same `operationId` and no
  such paragraph at all. Neither operation describes what an XML CONTRIBUTION
  envelope would be, and the two routes' documented format sets differ with no
  stated reason.
- **Ask:** state the envelope's format rule once, for both routes — including
  whether an XML envelope exists — and either extend the SPECITS-84 arm to the
  demographic commit or say why demographic payloads are excluded. (Ours: the
  envelope is canonical JSON on both routes; the EHR route accepts the
  Simplified types for `versions[i].data` exactly as SPECITS-84 defines, the
  demographic route does not, since the Simplified Formats have no PARTY shape
  — UPR-92. §"Data representation" requires "at least one of" the canonical
  formats and JSON is that one.)

### UPR-99 — the SM demographic editorial defect bundle

- **Components:** SM (`i_demographic_service.adoc`, `i_party.adoc`, `i_party_relationship.adoc`, `i_validity_checker.adoc`), RM common (`master06-change_control_package.adoc` §Contributions + §Logical Deletion + §Version Identification)
- **Register:** AMB-135 (editorial)
- **Facts:** (1) `i_party (a_versioned_party_id)` and `i_party_relationship
  (a_versioned_party_rel_id)` declare untyped parameters; (2)
  `get_party_at_version`'s `Pre_has_party_version` names a function the
  interface does not declare — the declared probe is `has_party_version_id`;
  (3) its `I_PARTY_RELATIONSHIP` twin `get_party_relationship_at_version`
  declares no precondition at all although `has_party_relationship` exists; (4)
  the create/update preconditions call `valid_content` while
  `I_VALIDITY_CHECKER` declares `content_valid` (the same defect UPR-39
  reports on `I_EHR_COMPOSITION`); (5) `create_party_relationship` lists the
  error `definition_unknown` with no `definitions_valid` precondition, where
  `create_party` carries both; (6) every identifier is typed bare `UUID`,
  including the ones naming a VERSION, whose identifier is the three-part
  OBJECT_VERSION_ID; (7) `delete_party`'s `Post_party_deleted: not has_party
  (…)` postulates a physical delete, which RM common forbids — "a versioned
  repository … is by definition indelible", and deletion is the four-step
  logical procedure of §Logical Deletion.
- **Ask:** fix (1)–(6) editorially and restate (7) as the RM's logical
  deletion. (Ours: the catalogue keeps the SM names as anchors and adjudicates
  behaviour from the ITS-REST docs text + RM; the party DELETE commits a
  `523|deleted|` VERSION and the container survives.)

### UPR-100 — demographic realization gaps in both directions

- **Components:** SM (`i_party.adoc`, `i_party_relationship.adoc`, `i_demographic_service.adoc`, `docs/openehr_platform/master06-demographic_service.adoc`), ITS-REST (the `/demographic/versioned_party/**` and `/demographic/contribution` routes)
- **Register:** AMB-136 (report_only)
- **Facts:** ITS→SM — six released routes realize no SM operation: the four
  `/demographic/versioned_party/**` reads (container, `revision_history`,
  version-at-time, version-by-id), because `I_PARTY` declares no container
  read, no revision-history operation and no operation returning a VERSION
  envelope; and the two `/demographic/contribution` routes, because the SM
  demographic chapter includes only `I_DEMOGRAPHIC_SERVICE`, `I_PARTY`,
  `I_PARTY_RELATIONSHIP` and their two UV classes — there is no demographic
  contribution interface anywhere in SM. SM→ITS — `has_party` and
  `has_party_version_id` have no wire (the API declares no existence probe), and
  `i_party`/`i_party_relationship` are interface factories with no wire meaning.
- **Ask:** give the versioned-party reads and the demographic contribution an
  SM home, and decide whether the existence probes deserve a wire. (Ours: the
  two versioned_party VERSION reads and the two contribution routes are
  anchored to their closest SM read/commit under an explicit variant — a
  catalogue naming convention, not an SM claim, the same device UPR-91/AMB-127
  uses for the stored-query read; the container get and `revision_history` stay
  a cited honest boundary; the existence probes are bound to the 200/404 of
  their corresponding reads.)

### UPR-101 — a demographic tag's mandatory `owner_id` has no owner to name

- **Components:** RM common (`UML/classes/org.openehr.rm.common.item_tag.adoc`, `master07-tags.adoc`), RM ehr (`UML/classes/org.openehr.rm.ehr.ehr.adoc` §Attributes), RM demographic (`docs/demographic/master02-demographic_package.adoc` + every `org.openehr.rm.demographic.*` class), ITS-REST (`schemas/demographic/ItemTagOf*.yaml`, `schemas/ehr/ItemTagOf*.yaml`)
- **Register:** AMB-137 (fixed_handling)
- **Facts:** `ITEM_TAG.owner_id` is mandatory (1..1 `OBJECT_REF`) and its whole
  definition is the gloss "Identifier of owner object, such as EHR" — an
  example, not a rule. On the EHR side the RM supplies the missing rule
  itself: `EHR.tags` declares the containment and confines the reach ("Tag
  `_target_` values can only be within the same EHR"), and both EHR-side
  examples set `owner_id.type: EHR`. The demographic side has no analogue at
  all — no class in the RM demographic package declares a `tags` containment
  (the chapter and all thirteen class files mention no tag attribute; the only
  "tag" hits are the substrings in "stage names"/"stagename") — so a mandatory
  attribute is left with no derivable value on five released route families.
  The single released signal is the five demographic `ItemTagOf<T>` examples,
  unanimous on `owner_id: {namespace: local, type: SYSTEM}` — neither the
  party nor an EHR nor any resource the demographic API addresses.
- **Ask:** state who owns a tag that lives outside an EHR — either declare a
  demographic `tags` containment (the `EHR.tags` analogue) or say in
  `item_tag.adoc` what `owner_id` names when there is no owning aggregate.
  (Ours: exactly the released examples' shape — an `OBJECT_REF` with
  `namespace: local`, `type: SYSTEM`, whose `id` carries the server's system
  identifier; naming the tagged party would duplicate `target` and contradict
  "owner", and naming an EHR would invent a containment the RM does not have.)

### UPR-102 — the space-wide demographic tag list has no scope, no page and no bound

- **Components:** ITS-REST (`operations/demographic_tags_get.yaml`, `operations/ehr_tags_get.yaml`, the five party tag gets, `docs/overview/Requests_and_responses.md` §"Authentication and authorization" + §"HTTP status codes")
- **Register:** AMB-138 (fixed_handling)
- **Facts:** `GET /demographic/tags` is the only tag route in the released
  surface with no scoping parameter of any kind. Its EHR twin is bounded by
  construction (`ehr_id` in the path, plus a 404 for an unknown one), and each
  typed party route is bounded by one `uid_based_id`; this one takes no path
  parameter, requires no filter ("In case no such parameter is provided then
  all ITEM_TAG resources will be retrieved"), declares no paging parameter and
  carries only a 200/400 response set. The released reading of an unfiltered
  call is therefore every ITEM_TAG on every party in the whole demographic
  space, with no maximum, no page, no continuation and no ordering — and no
  access-scope rule to fall back on, since the only released text on
  authorization is scheme-agnostic and resource-neutral, fixing which status
  codes an authorizing service returns without assigning scope to any
  resource.
- **Ask:** bound the space-wide read — a required filter, a paging pair, or an
  explicit statement that the whole space is the intended answer. (Ours: served
  whole, one unpaged list of what the caller is entitled to see, under the
  deployment's authorization layer, which is where the released text puts
  access control; the operation's only client-error branch is 400, so refusing
  an unfiltered call would refuse a request the text calls valid, and a
  silently truncated list would be a wrong answer rather than a loud one. What
  a supplied filter MEANS is UPR-58; this report is about the missing scope.)

### UPR-103 — `202 Accepted` is a declared admin success and is absent from the status-code table

- **Components:** ITS-REST (`operations/admin_ehr_delete.yaml`, `operations/admin_ehr_delete_all.yaml`, `responses/202.yaml`, `docs/overview/Requests_and_responses.md` §"HTTP status codes")
- **Register:** AMB-139 (editorial)
- **Facts:** both admin operations state the async branch in prose ("The server
  may execute this operation asynchronously (e.g. in batches), in which case
  returns status `202 Accepted`") and enumerate a `202` response, while the
  overview's status-code table — introduced as "The following subset is used in
  this specification" — lists sixteen codes and no 202. The table's own closing
  clause resolves it ("Additional status codes MAY be used as long as they do
  not conflict with the predefined codes") and 202 conflicts with none of the
  rows, so this is editorial, not semantic: the specification simply does not
  list a code two of its own operations return.
- **Ask:** add the 202 row to the status-code table (or say in the table's
  lead-in that operation-declared codes extend the subset). (Ours: synchronous,
  always `204`. Stated plainly, this branch is unauthorable in our instrument:
  the closed outcome vocabulary has no `accepted` kind, so 202 is absorbed as an
  `alt_status` of `ok_empty` on both admin bindings — a bodyless success either
  way — which means no case can require or distinguish the asynchronous branch.
  That is a recorded limit of the catalogue, not coverage.)

### UPR-104 — the bulk EHR delete has no partial-failure rule and its 404 has no trigger

- **Components:** ITS-REST (`operations/admin_ehr_delete_all.yaml`, `responses/404.yaml`, `parameters/query/ehr_id_Admin.yaml`, `operations/admin_ehr_delete.yaml`, `responses/404_unknown_ehr_id.yaml`, `docs/overview/Requests_and_responses.md` §"HTTP status codes"), SM (`i_admin_service.adoc` §`physical_ehr_delete`)
- **Register:** AMB-140 (fixed_handling)
- **Facts:** `admin_ehr_delete_all` accepts a subset of ids and declares
  202/204/404/405, but its 404 is the GENERIC one ("the server did not find a
  current representation of a target resource"), not the single route's
  `404_unknown_ehr_id`, and no sentence gives it a trigger. Three questions have
  no released answer: what a mixed list of known and unknown ids does; what an
  unfiltered call against an empty repository does; and whether the operation is
  atomic — which the released async branch makes unobservable from the client
  anyway, since a `202` caller cannot see where the batches stopped. The
  single-EHR route has none of this ambiguity (one id, `Pre_has_ehr`, a
  dedicated 404).
- **Ask:** state the subset rule — is an unknown id in the list an error or a
  no-op — give the declared 404 a trigger or remove it, and say whether the
  operation is atomic. (Ours: delete-what-exists → `204`, so the bulk route is
  idempotent where the single-EHR route deliberately is not; an unfiltered call
  against an empty repository is likewise `204`; and a malformed id is refused
  with `400` — the released generic-client-error row — BEFORE any deletion runs,
  so a typo in one id can never silently destroy the others in the same call.
  Atomicity is left unasserted, because no client-observable test can
  distinguish it while the async branch stands.)

### UPR-105 — the SM ADMIN editorial defect bundle

- **Components:** SM (`UML/classes/i_admin_service.adoc`, `i_admin_dump_load.adoc`, `encoding_format.adoc`, `export_format.adoc`, `compression_format.adoc`, `platform_service.adoc`, `export_spec.adoc`, `dump_load_fail_report.adoc`, `i_ehr_service.adoc`, `docs/openehr_platform/master15-admin_service.adoc`, `UML/class_index.adoc`)
- **Register:** AMB-141 (editorial)
- **Facts:** (1) the `ENCODING_FORMAT` enumeration is EMPTY — a Description row
  with no text and no Attributes section, so an enumeration two signatures
  reference has zero members; (2) four class files the admin chapter's
  cross-references point at are included by no chapter anywhere in the SM docs
  tree (`platform_service`, `export_format`, `compression_format`,
  `encoding_format` — grep-verified over every `include::` in
  `docs/openehr_platform`, `docs/serial_data_formats`, `docs/simplified_im_b`),
  so those `<<…>>` references resolve to anchors the rendered document does not
  contain; `PLATFORM_SERVICE` is the worst case, being the first parameter of
  four `I_ADMIN_SERVICE` operations; (3) `EXPORT_SPEC` and
  `DUMP_LOAD_FAIL_REPORT` are included by master15 and listed in the class index
  but referenced by no signature — `export_ehrs` flattens the three format
  enumerations into its parameter list instead of taking the `EXPORT_SPEC` that
  exists to carry them (and never sees its only mandatory attribute,
  `segment_split_size`), and nothing returns a `DUMP_LOAD_FAIL_REPORT` although
  dump/load are exactly the operations that would report per-entity failures;
  (4) `load_ehrs`, a READ from the file system, declares the error
  `file_not_writable` copy-pasted from `export_ehrs`, while the failures it can
  actually suffer — an unreadable archive, or the duplicate-id case its own
  Meaning names ("import EHRs with duplicate EHR ids will fail") — have no error
  at all; (5) `physical_ehr_delete` declares `Pre_has_ehr`, naming a function
  declared on `I_EHR_SERVICE` and not on `I_ADMIN_SERVICE`, while its twin
  `physical_party_delete` declares no precondition at all despite carrying the
  matching `party_id_does_not_exist` error — the same shape UPR-39 reports on
  `I_EHR_COMPOSITION` and UPR-99 on the demographic interfaces.
- **Ask:** fix (1)–(5) editorially — populate or remove `ENCODING_FORMAT`,
  include the four orphaned class files in the admin chapter, either use
  `EXPORT_SPEC`/`DUMP_LOAD_FAIL_REPORT` in the dump/load signatures or drop
  them, give `load_ehrs` its own errors, and make the two physical deletes'
  preconditions symmetric and interface-local. (Ours: the catalogue keeps the SM
  names as case anchors and adjudicates behaviour from the ITS-REST docs text +
  RM; none of the five reaches the wire, since `physical_ehr_delete` is the only
  realized admin operation — and CNF `master12` omits a `load_ehrs` section
  entirely, covering nine of the ten SM admin operations.)

### UPR-106 — the admin routes have no authorization contract at all

- **Components:** ITS-REST (`admin.openapi.yaml`, `operations/admin_ehr_delete.yaml`, `operations/admin_ehr_delete_all.yaml`, `responses/405.yaml`, `docs/overview/Requests_and_responses.md` §"Authentication and authorization" + §"HTTP status codes")
- **Register:** AMB-142 (fixed_handling)
- **Facts:** the most destructive operations in the released API declare
  `security: []` — identical to all six other API groups — and neither declares
  a 401 or a 403, so an authorizing service refusing an unprivileged caller
  answers with a status the operation does not enumerate. The only released text
  is scheme-agnostic and resource-neutral: "Services SHOULD implement and
  support an HTTP Authentication and Authorization framework, though this
  specification does not mandate a specific authentication scheme", plus the
  conditional MUST that fixes 401/403/407 as the codes an authorizing service
  returns. Nothing anywhere says that permanently destroying every EHR on a
  server requires more privilege than reading one composition. The nearest
  access-related sentence is a deployment hint on the bulk route only ("intended
  primarily for **development** or **testing** purposes and may be disabled in
  **production** environments, in which case server may respond with `405 Method
  Not Allowed`"), not an authorization rule.
- **Ask:** state the privilege level the admin operations require and declare
  their 401/403 branches — or say explicitly that authorization for them is a
  deployment concern the specification leaves open. (Ours, flagged as our own
  design and not as conformance: both released admin operations are classified
  `OperationClass::Admin` and require the configured admin role, while every
  other generated operation is `Clinical`; the refusal statuses we emit are the
  released 401/403, but the trigger is ours. No CNF case asserts an admin
  authorization branch — the catalogue drives these routes with the admin
  principal only, because a case refusing an unprivileged caller would gate a
  boundary the specification never draws.)

### UPR-107 — nothing says what a physically deleted EHR looks like afterwards, and the cascade list is exemplary

- **Components:** ITS-REST (`operations/admin_ehr_delete.yaml`, `operations/admin_ehr_delete_all.yaml`, `responses/204_deleted_hard.yaml`, `operations/ehr_create_with_id.yaml`, `responses/409_EHR.yaml`, `responses/404_unknown_ehr_id.yaml`), SM (`i_admin_service.adoc` §`physical_ehr_delete`), RM ehr (`UML/classes/org.openehr.rm.ehr.ehr.adoc`)
- **Register:** AMB-143 (fixed_handling)
- **Facts:** both operations describe the deletion and stop. No sentence states
  that a subsequent read of the deleted `ehr_id` is a 404 rather than a 410 or a
  tombstone; none states whether the same `ehr_id` may be created again
  (`ehr_create_with_id`'s only conflict branch is the subject/namespace
  duplicate, which says nothing about an id that used to exist); and
  `physical_ehr_delete` states a PREcondition and no postcondition, where SM's
  `delete_query` does state `Post_query_deleted`. The cascade itself is given by
  example — "All resources associated with or owned by the specified EHR (**such
  as** COMPOSITION, EHR_STATUS, ITEM_TAG, CONTRIBUTION, and their historical
  versions)" — so a client cannot tell whether anything outside those four
  classes (EHR_ACCESS, the directory FOLDER tree, attestations, audit records)
  survives.
- **Ask:** give `physical_ehr_delete` a postcondition, state the post-delete
  read behaviour and whether the `ehr_id` may be reused, and make the cascade
  exhaustive by referring to the EHR aggregate rather than listing examples.
  (Ours: subsequent reads are `404` — the branch `404_unknown_ehr_id` already
  assigns; re-creation under the same `ehr_id` is permitted, since no EHR
  remains to conflict with and refusing would require remembering an id we were
  told to destroy; and the cascade is total — every versioned object and all its
  versions and nodes, attestations, ITEM_TAGs, CONTRIBUTIONs, the audit records
  they referenced, archive markers, and any externalized blob no surviving node
  references. The storage graph that closure walks is our own design; no openEHR
  spec governs it.)

### UPR-108 — physical deletion requires no audit trail, and its cascade destroys the one that existed

- **Components:** ITS-REST (`docs/overview/Requests_and_responses.md` §"openehr-version and openehr-audit-details", `operations/admin_ehr_delete.yaml`, `operations/admin_ehr_delete_all.yaml`), SM (`docs/openehr_platform/master02-overview.adoc`, `UML/classes/i_system_log.adoc`), RM common (`master06-change_control_package.adoc` §Logical Deletion + §Contributions)
- **Register:** AMB-144 (report_only)
- **Facts:** the audit obligation is scoped away from these operations by
  construction — the `openehr-audit-details` MUST sits in a paragraph about
  "committing content … for all change-controlled resources", and a physical
  delete commits nothing and produces no VERSION. Neither admin operation
  declares an audit header, and neither description mentions a record, a log or
  an actor. SM offers nothing either: the platform overview promises an "IHE
  ATNA-compliant system log", but `I_SYSTEM_LOG` is an EMPTY class table — no
  description, no functions — included by no chapter and reachable only from the
  class index. Meanwhile the cascade named in the same admin descriptions
  removes "CONTRIBUTION, and their historical versions", i.e. exactly the
  structures holding each change's `AUDIT_DETAILS` — in a model whose logical
  deletion rule exists because "Medicolegal and traceability requirements mean
  that information cannot be literally removed".
- **Ask:** require an audit record for physical deletion — actor, time, ids
  destroyed — and say where it lives, given that the operation's own cascade
  removes the CONTRIBUTIONs and AUDIT_DETAILS carrying every other change's
  history; and either populate `I_SYSTEM_LOG` or drop the ATNA claim from the
  platform overview. (Ours: no case asserts an audit of a physical delete,
  because no released sentence requires one. This server emits the admin delete
  to its system log — the DICOM PS3.15 + FHIR `AuditEvent`/BALP renderings into
  the local Audit Record Repository — so the erasure leaves a trace outside the
  EHR it destroys; that is our own design/extension, not conformance, and it
  lives in a separate store precisely because the cascade empties the one the RM
  would have used.)
