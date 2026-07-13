# CKM template pack — provenance

Vendored from the official openEHR CKM (`https://ckm.openehr.org/ckm/rest/v1`) by
`scripts/vendor-ckm-templates.sh` on 2026-07-13T16:25:46Z.
Each file is CKM's own OPT export for the cited template, verbatim.

| cid | slug | display name | status | modified | workload role |
|---|---|---|---|---|---|
| 1013.26.380 | vital-signs | Vital signs | DRAFT | 2021-03-08T13:30:44+01:00 | E2 shift observations (small event composition) |
| 1013.26.408 | generic-lab-test-result | Generic lab test result example simple | DRAFT | 2021-10-18T11:28:46+02:00 | E4 lab results (contribution batches) |
| 1013.26.80 | eprescription-fhir | ePrescription (FHIR) | DRAFT | 2016-05-23T23:01:02+02:00 | E3 medication rounds (ePrescription, COMPOSITION-rooted) |
| 1013.26.2 | ereferral | eReferral | DRAFT | 2010-03-25T07:26:15+01:00 | E1 admission / E9 discharge (large clinical summary, COMPOSITION-rooted) |
| 1013.26.376 | international-patient-summary | International Patient Summary | DRAFT | 2020-08-18T04:28:14+02:00 | vendored, NOT wired: the server example/validator mismatch on ACTION.medication description is W-12 |
| 1013.26.191 | gp-data-set | GP data set | INITIAL | 2018-10-15T02:11:04+02:00 | E7 documentation corrections (GP encounter data set, COMPOSITION-rooted) |

## Example skeletons (`*.example.json`)

Generated 2026-07-13 from the vendored OPTs by the ehrbase-rs server's own
example generator (`GET /definition/template/adl1.4/{template_id}/example`,
composed local stack) and committed so every benchmarked SUT receives
**byte-identical** request payloads (fairness: the skeleton is a committed
artefact, never fetched per-SUT at run time). Each WIRED skeleton was
verified to commit (`POST …/composition` → 201) against the composed
server. Regenerate by re-running the upload + example fetch against a
composed stack after re-vendoring the OPTs.

| slug | template_id | wired | commit-verified |
|---|---|---|---|
| vital-signs | Vital signs | yes | 201 |
| generic-lab-test-result | Generic lab test result example simple | yes | 201 |
| eprescription-fhir | ePrescription (FHIR) | yes | 201 |
| ereferral | eReferral | yes | 201 |
| gp-data-set | GP data set | yes | 201 |
| international-patient-summary | International Patient Summary | **no — W-12** | 422: the server's own example is rejected by its own validator ("unexpected node 'at0001' under 'description'" on ACTION.medication inside the Medication Summary section) — a real generator-or-validator defect to be triaged spec-first (AOM/OPT constraint semantics), never papered over here |
