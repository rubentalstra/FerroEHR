# Catalogue audit — EHR chapter (I_EHR_SERVICE + I_EHR_STATUS)

Issue #231 · audited 2026-07-24 · 22 cases · verdicts: 22 ok / 0 defects / 0 ambiguities

Chapter context: CNF `master06-func_tc_ehr.adoc` carries 21 real official case
tables — all present in the catalogue with verbatim ids — plus the catalogue's
one addition (`create_ehr-invalid_status`, realizing the official INVALID
data-set class as its own negative case). Register coverage: AMB-12 (the
master06 caption mislabel on the provided-status table), AMB-14 (subject-keyed
cases anchored under has_ehr/get_ehr headings resolve to the SM
has_ehr_for_subject/get_ehrs_for_subject operations), AMB-15 (SM field-setters
have no dedicated endpoint — realized as read-modify-write over
GET + PUT ehr_status), AMB-3 (preceding_version_uid as If-Match), AMB-2
(create-side validation code unenumerated → 400).

| case | verdict | evidence | resolution |
|---|---|---|---|
| create_ehr-main | ok | The 18-row matrix = the official 16-row provided-status table (verified cell-by-cell: is_queryable/is_modifiable × other_details × ehr_id) + the two no-status rows (class 1.a, AMB-12); defaults assertion matches the official Notes (true/true/PARTY_SELF, server-assigned ehr_id); single_pass + aggregate uniqueness realizes "ehr_id … should be unique" | none |
| create_ehr-same_ehr_twice | ok | Official flow (create, re-create same id → negative, exactly one EHR remains); both id-source rows (server-assigned/data-set) per the official "read from the response / from the test data sets" | none |
| create_ehr-two_ehrs_same_patient | ok | Official ground (one EHR per subject); step-3 verification by subject via the AMB-14-resolved operation | none |
| create_ehr-invalid_status | ok | Catalogue addition realizing the official INVALID class; the four fixtures map onto the official illustrative list (missing flags, missing subject, invalid other_details); AMB-2 tagged for the wire code | none — see coverage note |
| get_ehr-existing_ehr_by_ehr_id / -by_subject_id | ok | Official grounds; subject case AMB-14-tagged, provisions its own subject on an empty server | none |
| get_ehr-get_ehr_by_invalid_ehr_id / _subject_id | ok | Random id / absent subject → not_found with the official error exemplars | none |
| has_ehr-existing_ehr_id / -existing_subject_id / -non_existing_ehr_id / -non_existing_subject_id | ok | Boolean ops realized by outcome kind; subject variants AMB-14-tagged; grounds match the official cells | none |
| get_ehr_status-get_by_ehr_id / -bad_ehr | ok | Official grounds; field-presence assertions on the status body | none |
| set_ehr_queryable-existing_ehr / set_ehr_modifiable-existing_ehr / clear_ehr_queryable-existing_ehr / clear_ehr_modifiable-existing_ehr | ok | Official flow (invoke setter → verify flag) realized as AMB-15 read-modify-write with the captured status version as If-Match (AMB-3); step-3 in-case verification asserts the exact official post-condition value (verified against the §set_ehr_queryable-existing_ehr cell: "should be `true`") | none |
| set_ehr_queryable-bad_ehr / set_ehr_modifiable-bad_ehr / clear_ehr_queryable-bad_ehr / clear_ehr_modifiable-bad_ehr | ok | Official ground (random ehr_id → negative "EHR with <ehr_id> doesn't exist"); the wire-required synthetic If-Match is sound under the RFC 9110 §13.2.2 precedence recorded in AMB-26 (the unconditioned response is the 404) | none |

Checks common to the chapter:
- **Ground (dim 1):** all 21 official ids verbatim; the one addition carries its authoring adjudication.
- **Expectations (dim 2):** recomputed from the official tables + SM `i_ehr_service.adoc`/`i_ehr_status.adoc`; the create-main matrix diffed against the official 16-row table exactly.
- **Fixtures (dim 4):** all `cnf.ehr_status.*` keys verified in `corpus/MANIFEST.yaml` (provided, with_subject.b/.c/.d, the four invalid variants); subject ids are per-case on empty servers — no shared-SUT collisions.
- **Captures (dim 5):** the setter cases' `status_version_uid` capture feeds the If-Match parameter — the AMB-15 realization verified; create-main's `new_ehr_id` feeds the aggregate uniqueness assertion.
- **Ambiguity tags (dim 6):** AMB-2/3/12/14/15 all read; each covers its tagged case.

Coverage note (no action mandated): the official invalid-EHR_STATUS list is
explicitly illustrative ("Any other data set is invalid, for instance …") and
also names empty-flag and invalid-`subject_id` variants the catalogue does not
realize; the four realized fixtures satisfy the ground. Adding the two extra
variants would deepen the INVALID class — candidate for a future catalogue wave,
not a defect.
