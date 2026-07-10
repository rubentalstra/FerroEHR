# B3 — SM-5/SM-6: the designed-but-unbuilt services

- Status: in-progress
- Started: 2026-07-10   Owner: Ruben
- Governing plan: `docs/blueprint/00-THE-BLUEPRINT.md` §3 B3; designs in
  `docs/design/sm-platform/` (esp. `10-message-integration.md`)
- Oracle: `docs/specs/openehr/SM/docs/openehr_platform/` (master09 Message,
  master12), RM `ehr_extract`, CNF schedule; ECC baseline 293/319 (zero-drift
  gate per phase step, `scripts/conformance.sh`)

## Tasks (blueprint §3 B3)

- [ ] 1. SM-4 wave 3 — Admin dump/load: `export_ehrs`/`load_ehrs`,
      `EXPORT_SPEC`, segmenting, `DUMP_LOAD_FAIL_REPORT`; round-trip test +
      duplicate-id failure.
- [ ] 2. SM-5 — Message service: `I_EHR_EXTRACT_SERVICE` (export whole-EHR +
      spec-driven; import into fixed/existing EHR) over `vobject` + generated
      `ehr_extract` types; import lands IMPORTED_VERSION storage, clone-EHR
      with reused ehr_id, versioning Cases 2/3 (ch 1 reqs 13/31/35/50–53);
      `I_TDD_SERVICE.import_tdd` (TDD → COMPOSITION over OPT/WebTemplate).
      Decide version branching or keep the typed rejection PORT-NOTEd.
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
