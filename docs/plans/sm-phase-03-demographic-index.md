# Phase SM-3 — Demographic completion (PARTY_RELATIONSHIP) + EHR Index

- Status: in-progress
- Started: 2026-07-09
- Consumes: ADR-010; design `docs/design/sm-platform/` (03 digest §1–2, 07
  §1.3–1.4, 08 §4.1/§4.6, 09 §SM-3)
- Compile required: yes

## Spec oracle (read per task — hard rule)

- `docs/specs/openehr/SM/docs/UML/classes/i_party_relationship.adoc` — the 6
  calls (has/get/at-time/update [pre definitions_valid + has; new
  ORIGINAL_VERSION + CONTRIBUTION]/delete [post not-has]/at-version); errors
  `versioned_object_does_not_exist`/`object_version_does_not_exist`/
  `definition_unknown`/`content_invalid`
- `i_demographic_service.adoc` — `create_party_relationship(UV_PARTY_RELATIONSHIP): UUID`
  (pre `valid_content`; server-side VERSIONED_OBJECT + ORIGINAL_VERSION +
  CONTRIBUTION), `i_party_relationship` factory
- `i_ehr_index.adoc` — the 5 calls (add/update-status/update-loc-desc/
  remove-ehr-subject/remove-subject); N:M semantics + duplicate-management
  purpose per `master07-ehr_index_service.adoc`
- `resource_status.adoc` (`start/end_valid_time` typed `@@` — placeholder;
  decided: ISO date-time, PORT NOTE), `resource_instance_type.adoc`
  (Primary/Duplicate/Supplementary), `location_desc.adoc` (**empty stub** —
  designed contract `{system_id, uri?, description?}`, PORT NOTE; design 08 §3)
- Wire: demographic is our own design (no ITS-REST contract; CNF master10
  TBD); EHR Index has no wire contract — service-level this phase, extension
  routes with the extension-OAS work (design 08 §7). Zero ECC drift gate.

## Tasks

- [x] (done 2026-07-09; verified: migration 0007 extends the kind CHECK, codec `is_versioned_root_type` keeps nested relationships inline, source/target checked in validation) `Kind::PartyRelationship` in `vobject` (+ migration extending the
      `vo_version.kind` CHECK — mirror how 0003 added party kinds);
      structural validation = deserialize `openehr_rm` `PartyRelationship`
      (+ source/target refs present)
- [x] (done 2026-07-09; verified: SM citations on every call, spec asymmetries normalized + PORT NOTEd) `PartyRelationshipService` trait (`ehrbase-sm`): create (on
      `DemographicService` per `i_demographic_service.adoc`) + the 6
      `I_PARTY_RELATIONSHIP` calls; SM citations per method; Backend/
      StubBackend updated
- [x] (done 2026-07-09; service_sm3 e2e: lifecycle/versioning/at-time/at-version/revision history/If-Match) Impl on `EhrbaseService` over the shared `vobject` machinery
      (versioning, contributions, revision history, at-time/at-version) —
      same paths as PARTY
- [x] (done 2026-07-09; 3 wire tests in demographic_http; PORT NOTE: our own design) Wire: `/demographic/party_relationship/*` extension routes in the
      demographic dispatcher (same envelope/header rules as the party
      routes; our own design, PORT NOTE)
- [x] (done 2026-07-09; @@-placeholder + LOCATION_DESC-stub + read-silence decisions all PORT NOTEd) `EhrIndexService` trait + `ResourceStatus`/`ResourceInstanceType`/
      `LocationDesc` types in `ehrbase-sm::types` (spec-cited; the two
      placeholder/stub decisions PORT-NOTEd); the 5 SM calls + design-filled
      read calls `ehr_subjects(ehr_id)` / `subject_ehrs(subject)` (the spec
      defines no getters — silence filled by design, PORT NOTE)
- [x] (done 2026-07-09; migration 0007; subject promotion untouched; N:M + duplicate cases e2e-tested) `ehr_index` table (N:M; PK (ehr_id, subject_id, subject_namespace);
      instance_type CHECK; valid period; notes; location jsonb) + impl;
      reconcile with the `ehr.subject_id` unique promotion (that fast path
      stays the Primary-instance lookup; index rows model the N:M +
      duplicate states — design 08 §4.1)
- [x] (done 2026-07-09; service_sm3 7 tests + 3 wire tests, all green) e2e tests (testcontainers): relationship CRUD/versioning/at-time +
      error cases; index add/update/remove/N:M/duplicate listing; SM
      pre/post-conditions as assertions
- [ ] ECC zero-drift run (baseline 211/318) + workspace gates

## Exit criteria

- [ ] Workspace green (build, nextest, clippy-neutral, fmt)
- [ ] ECC ≥ 211/318, zero regressions
- [ ] New trait methods doc-cite their SM calls
- [ ] Checkboxes ticked; PROGRESS updated at close

## Handoff

SM-2 merged (PR #32, develop 9ed6a3e29). Branch
`claude/sm-phase-03-demographic-index` created. Specs for both interfaces
read 2026-07-09; design decisions fixed above.

## Storage-semantics audit wave (added 2026-07-09 — owner request "spec-1:1 storage")

Full audit of the persistence semantics vs RM common master06 + UML classes
ran 2026-07-09: **no blockers; core change-control engine verified faithful**
(indelibility, logical delete, contribution atomicity, EHR creation,
change_type preservation, attestations, signatures, revision history — all
confirmed with citations). Fixes for the findings, in this phase:

- [x] (done 2026-07-09; resolve_lifecycle validates the group -> 422; 553/800/801 stored+served, five-state e2e test; direct paths keep 532/523 so the wire is unchanged; CNF findings PORT NOTEd — 800/801 un-tested by CNF, lifecycle header + relaxed incomplete-validation are recorded follow-ups) **M1** Honor the client-supplied `lifecycle_state` on create/modify
      (all five normative codes: 532/553/523/800/801; force only the
      delete path to 523) — `master06` §Version Lifecycle; check CNF for
      incomplete/inactive cases; e2e tests per state
- [x] (done 2026-07-09; migration 0008; uid + signature stability across a config change proven by test) **M2** Persist `creating_system_id` per version (migration on
      `vo_version`) and reconstruct `OBJECT_VERSION_ID`s + signatures from
      the stored value, never live config — `master06` §Distributed
      Versioning
- [x] (done 2026-07-09; defaults copied from the contribution audit, explicit values preserved + PORT NOTE; test) **m4** Contribution→version audit copy rule ("should be copied"):
      default per-version committer/system_id from the contribution audit;
      PORT NOTE the divergence when a client supplies distinct values
- [x] (done 2026-07-09; full corpus through jsonb, one container) **m5** Extend the through-`jsonb` round-trip test to the full corpus
- [x] (done 2026-07-09; migration 0008) **m6** `CHECK (system_id <> '')` on `audit` (AUDIT_DETAILS
      System_id_valid)
- [x] (done 2026-07-09; vobject.rs merge PORT NOTE; indelibility note in migration header + admin doc) **m3/m7** PORT NOTEs: version merging out of scope (`master06`
      §Version Merging); admin-cascade vs indelibility design note
