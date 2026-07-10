# B3 — SM-5/SM-6: the designed-but-unbuilt services

- Status: in-progress
- Started: 2026-07-10   Owner: Ruben
- Governing plan: `docs/blueprint/00-THE-BLUEPRINT.md` §3 B3; designs in
  `docs/design/sm-platform/` (esp. `10-message-integration.md`)
- Oracle: `docs/specs/openehr/SM/docs/openehr_platform/` (master09 Message,
  master12), RM `ehr_extract`, CNF schedule; ECC baseline 293/319 (zero-drift
  gate per phase step, `scripts/conformance.sh`)

## Tasks (blueprint §3 B3)

- [x] 1. SM-4 wave 3 — Admin dump/load: `export_ehrs`/`load_ehrs`,
      `EXPORT_SPEC`, segmenting, `DUMP_LOAD_FAIL_REPORT`; round-trip test +
      duplicate-id failure. *Done 2026-07-10: AdminDumpLoad trait (no default
      bodies, ADR-011) + ExportSpec/DumpLoadFailReport catalog (spec-cited,
      i_admin_dump_load.adoc); canonical-JSON archive with manifest +
      greedy segmenting; lossless verbatim re-insert through the storage
      codec; round-trip + duplicate-id tests green (ehrbase 226/226,
      ehrbase-sm 9/9, rest 218/218). PORT NOTEs: native-API-only (no REST
      wire), JSON-uncompressed-only this wave.*
- [ ] 2. SM-5 — Message service: `I_EHR_EXTRACT_SERVICE` (export whole-EHR +
      spec-driven; import into fixed/existing EHR) over `vobject` + generated
      `ehr_extract` types; import lands IMPORTED_VERSION storage, clone-EHR
      with reused ehr_id, versioning Cases 2/3 (ch 1 reqs 13/31/35/50–53);
      `I_TDD_SERVICE.import_tdd` (TDD → COMPOSITION over OPT/WebTemplate).
      Decide version branching or keep the typed rejection PORT-NOTEd.
      *Progress: export side done (db693b120); import side done 2026-07-10 —
      IMPORTED_VERSION replay through commit_import (master06 Cases 2/3,
      preserved 3-part identity + commit_audit, local import CONTRIBUTION
      249, synthetic local sys_period chain, trunk-only branching rejection
      F-06-09); 4 import integration tests. TDD seam landed (b96c0a3be):
      TddService trait + envelope layer (namespace/template_id/EHR/OPT
      resolution, 6 corpus-fixture tests, typed rejections). Remaining
      sub-item (explicit, not an open deferral): the OPT-guided TDD body →
      COMPOSITION converter (archie TemplateDataDocument equivalent).*
- [ ] 3. SM-6 — Subject Proxy: subject/variable/data-set/binding stores,
      `I_DATA_BINDING` with the openEHR frame = AQL over our Query service;
      FHIR/HL7v2 frame seams stubbed.
- [ ] 4. MSG ECC cases (`Area::Msg`, zero cases today) land with SM-5.

## Exit criteria

- [ ] Blueprint map rows 6 + 16 fully DONE; MSG area evidenced.
- [ ] Workspace suites green; full ECC ≥293/319, zero drift.

## Handoff for next session

Phase opened from develop @ B2 merge (PR #37). Working discipline learned in
B2: ONE cargo runner at a time (background parallelism caused every "stuck"
lock wait); agents get isolated CARGO_TARGET_DIRs; ECC runs only via
scripts/conformance.sh. Next action: task 1 — read
docs/design/sm-platform/ (admin dump/load section) + SM master12, then
implement export_ehrs/load_ehrs.
