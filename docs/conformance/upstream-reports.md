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

- **Component:** ITS-REST (the typed tag routes' 404 file)
- **Register:** AMB-91 (fixed_handling)
- **Ask:** state whether a kind mismatch is the "does not exist" 404 (our
  reading) or a 400.

### UPR-56 — return=identifier on a tag PUT cannot satisfy the identifier contract

- **Components:** ITS-REST (Prefer enum + §Prefer only identifier), RM (item_tag)
- **Register:** AMB-92 (fixed_handling)
- **Ask:** assign the branch or exclude the token for collection
  resources without a uid.

### UPR-57 — RM-invariant violations on the tag PUT have no status

- **Components:** RM (item_tag invariants), ITS-REST (tag update responses)
- **Register:** AMB-93 (fixed_handling)
- **Ask:** assign 422 (our handling) or 400 explicitly.

### UPR-58 — the ehr_tags_get filter grammar is undefined

- **Component:** ITS-REST (ehr_tags_get + the three query params)
- **Register:** AMB-94 (fixed_handling)
- **Ask:** define repeatability, combination, match mode, and
  absent-path matching; reconcile "one or more" with the scalar schemas.

### UPR-59 — duplicate identity pairs in one tag PUT body

- **Component:** ITS-REST (UpdateItemTag array)
- **Register:** AMB-95 (fixed_handling)
- **Ask:** uniqueItems or a merge rule (ours: last-wins).

### UPR-60 — empty-string vs absent target_path splits the tag identity

- **Components:** RM (item_tag target_path 0..1), ITS-REST (the "" example)
- **Register:** AMB-96 (fixed_handling)
- **Ask:** pick one representation (ours: "" normalizes to absent).

### UPR-61 — the ITEM_TAG editorial defect bundle

- **Components:** RM (item_tag, is_justified), BASE (String), ITS-REST, ITS-XML
- **Register:** AMB-97 (editorial)
- **Facts/Asks:** define `is_justified` (or restate the invariant on the
  prose rule); fix the "(logically) deleted" wording; add an EHR-wide
  ItemTagList schema; fix the copy-pasted _updated descriptions; either
  add a canonical-XML ITEM_TAG type or drop XML from the tag enums; fix
  the HIER_OBJECT_ID-with-type-COMPOSITION example and add a
  VERSION-targeted example; align the DELETE descriptions' "tag_key" with
  the {key} parameter; give FOLDER tags a route (or scope the overview);
  define {key} encoding, list ordering/paging, and the
  optional-feature refusal branch; state tag lifecycle vs target
  lifecycle; note the development-RM provenance of the whole class.
