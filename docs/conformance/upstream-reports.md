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

### UPR-01 — AM/ADL 1.4 defines no template versioning

- **Register entry:** AMB-4
- **Channel:** SPECPR
- **Status:** draft
- **Spec citation:** AM ADL 1.4 `master02-overview.adoc` §Templates — templates
  are defined at a local level in the dADL formalism and "do not introduce any
  new semantics to archetypes"; no template versioning or duplicate-`template_id`
  handling is defined (silence confirmed first-hand).
- **Problem:** With no formal versioning for ADL 1.4 templates, the handling of a
  second upload under an existing `template_id` (reject-as-conflict vs
  accept-as-new-version) is implementation-defined; conformance cannot require a
  single behaviour.
- **Ask:** Define template identification + versioning semantics for ADL 1.4, or
  state explicitly that duplicate-`template_id` handling is implementation-defined,
  so the suite can gate one behaviour rather than carry sibling option cases.

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

### UPR-04 — Physical deletion of a VERSIONED_OBJECT is unspecified

- **Register entry:** AMB-10
- **Channel:** SPECPR
- **Status:** draft
- **Spec citation:** RM common `master06-change_control_package.adoc` §Change
  Control (a versioned repository is "by definition indelible") + §Logical
  Deletion (information "can only ever be logically deleted") — silence confirmed
  first-hand: no operation for physically destroying a whole VERSIONED_OBJECT
  container is defined.
- **Problem:** The RM defines only logical deletion (a new Version with `data`
  Void and `lifecycle_state` deleted); physically deleting an entire
  VERSIONED_OBJECT container "needs further specification at the openEHR Service
  Model".
- **Ask:** Specify physical VERSIONED_OBJECT deletion at the SM/RM boundary (or
  confirm it is out of scope); until then only statement-declared behaviour is
  exercised.

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

### UPR-12 — SM ADMIN operations are largely unrealized in ITS-REST

- **Register entry:** AMB-33
- **Channel:** ITS-REST
- **Status:** draft
- **Spec citation:** SM `i_admin_service.adoc` / `i_admin_dump_load.adoc` /
  `i_admin_archive.adoc` (`list_contributions`, `contribution_count`,
  `versioned_composition_count`, `composition_version_count`,
  `physical_party_delete`, `export_ehrs`, `archive_ehrs`, `archive_parties`) vs
  released ITS-REST 1.1.0 (only physical EHR deletion realized).
- **Problem:** The SM ADMIN component defines counts, listing, party deletion,
  dump/load and archive operations, but the released ITS-REST API realizes only
  physical EHR deletion.
- **Ask:** Add the missing ADMIN operations to the ITS-REST API (SM↔ITS
  alignment).

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
