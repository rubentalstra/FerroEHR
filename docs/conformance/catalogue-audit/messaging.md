# Catalogue audit — MESSAGING chapter

Issue #231 · audited 2026-07-24 · 14 cases · verdicts: 14 ok / 0 defects / 0 ambiguities
(10 cases carry the systemic dangling `REQUIREMENTS.md` comment pointer — swept chapter-wide at the end of the audit, not a per-case defect)

Chapter context: CNF `master13-func_tc_messaging.adoc` is placeholder-only
(verified: every table is "Test Case aaaa/bbbb"; the chapter also anchors the
non-existent interface `I_EHR_EXTRACT` with `export_ehr()` listed twice). The
whole chapter is unrealized on ITS-REST 1.1.0 — there is no MESSAGE/Extract/TDD
API — and every case carries `ambiguities: [AMB-34]`, whose register entry
covers BOTH facts (the unwired surface and the naming defect, resolution oracle
= the vendored SM). All 14 cases are therefore N/A-with-citation on the its-rest
profile; their flows are SM-derived drafts for a future MESSAGE API release.

| case | verdict | evidence | resolution |
|---|---|---|---|
| export_ehr_extracts-by_spec | ok | SM `i_ehr_extract_service.adoc`: `export_ehr_extracts(extract_spec: EXTRACT_SPEC[1]): List<EXTRACT>` — flow matches the signature; fixture `cnf.messaging.extract_spec.v1` in MANIFEST | none |
| export_ehr_extracts-empty_result | ok | Same op; empty-system fulfilment with no content (`ok_empty`) is the SM-consistent draft | none |
| export_ehrs-export_existing | ok | `export_ehrs(an_ehr_id: UUID[1]): List<EXTRACT>` — "Export whole EHR" | none |
| export_ehrs-export_unknown | ok | Non-existent target → not_found, the standard outcome-kind mapping | none |
| import_ehr-new | ok | `import_ehr(an_ehr_id: UUID[0..1], an_extract: EXTRACT[1])` — id optional, so no-id import creating a new EHR matches the signature | none |
| import_ehr-with_id | ok | Same op — "optionally providing a fixed EHR identifier … to match the identifier of EHR(s) for the same patient in other EHR services" (quoted in the case) | none |
| import_ehr-duplicate | ok | Duplicate fixed id → already_exists (the uniqueness outcome kind); consistent with the id-matching purpose of the parameter | none |
| import_ehr_extract-into_existing | ok | `import_ehr_extract(an_ehr_id: UUID[1], an_extract: EXTRACT[1])` — "Import an EHR Extract into an existing EHR" → updated | none |
| import_ehr_extract-unknown_ehr | ok | an_ehr_id mandatory + "into an existing EHR" → not_found for a non-existent EHR | none |
| import_ehr_extract-invalid | ok | Semantically invalid extract → validation_failed; fixture `cnf.messaging.ehr_extract.invalid` exists | none |
| import_tdd-valid | ok | SM `i_tdd_service.adoc`: `import_tdd(an_ehr_id: UUID[1], tdd: String[1])`; fixture exists | none |
| import_tdd-invalid | ok | Malformed TDD → validation_failed | none |
| import_tdds-bulk_valid | ok | `import_tdds` — "Bulk import numerous TDDs" | none |
| import_tdds-bulk_invalid | ok | Bulk set containing a malformed TDD → validation_failed draft | none |

Checks common to the chapter:
- **Ground (dim 1):** master13 placeholder status and its I_EHR_EXTRACT naming defect verified in the vendored text; AMB-34 read — it records both, names the six real SM operations, and prescribes the SM as the resolution oracle. Every case anchors a real SM operation (all six verified in `i_ehr_extract_service.adoc` / `i_tdd_service.adoc`, all 0..1 optional operations).
- **Expectations (dim 2):** unrealized draft flows; each expect maps to the outcome-kind vocabulary consistently with the SM signatures and prose. Nothing gates until a MESSAGE API release realizes the bindings.
- **Fixtures (dim 4):** all five `cnf.messaging.*` keys present in `corpus/MANIFEST.yaml`.
- **Captures (dim 5):** no cross-step captures; single-step flows.
- **Ambiguity tags (dim 6):** AMB-34 on all 14 — covers each case's exact status.
