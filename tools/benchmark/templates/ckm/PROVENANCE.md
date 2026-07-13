# CKM template pack — provenance

Vendored from the official openEHR CKM (`https://ckm.openehr.org/ckm/rest/v1`) by
`scripts/vendor-ckm-templates.sh` on 2026-07-13T15:23:58Z.
Each file is CKM's own OPT export for the cited template, verbatim.

| cid | slug | display name | status | modified | workload role |
|---|---|---|---|---|---|
| 1013.26.380 | vital-signs | Vital signs | DRAFT | 2021-03-08T13:30:44+01:00 | E2 shift observations (small event composition) |
| 1013.26.408 | generic-lab-test-result | Generic lab test result example simple | DRAFT | 2021-10-18T11:28:46+02:00 | E4 lab results (contribution batches) |
| 1013.26.341 | medication-order | Medication order item R1 | INITIAL | 2020-07-13T09:19:26+02:00 | E3 medication rounds |
| 1013.26.376 | international-patient-summary | International Patient Summary | DRAFT | 2020-08-18T04:28:14+02:00 | E1 admission / E9 discharge (large, deep) |
| 1013.26.356 | clinical-synopsis | Clinical synopsis item R1 | INITIAL | 2020-07-22T09:12:24+02:00 | E7 documentation corrections (notes) |

## Example skeletons (`*.example.json`)

Generated 2026-07-13 from the vendored OPTs by the ehrbase-rs server's own
example generator (`GET /definition/template/adl1.4/{template_id}/example`,
composed local stack) and committed so every benchmarked SUT receives
**byte-identical** request payloads (fairness: the skeleton is a committed
artefact, never fetched per-SUT at run time). Each skeleton was verified to
commit (`POST …/composition` → 201) against the composed server. Regenerate
by re-running the upload + example fetch against a composed stack after
re-vendoring the OPTs.

| slug | template_id |
|---|---|
| vital-signs | Vital signs |
| generic-lab-test-result | Generic lab test result example simple |
| medication-order | Medication order item R1 |
| international-patient-summary | International Patient Summary |
| clinical-synopsis | Clinical synopsis item R1 |
