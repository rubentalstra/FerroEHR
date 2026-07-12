# W-3c — Subject Proxy Service: complete spec-true rewrite

Design (the authority for every step): `docs/design/sm-platform/10-subject-proxy.md`
(13-row gap register G-1..G-13 + target architecture). Oracle:
`docs/specs/openehr/SM/docs/openehr_platform/master10-subject_proxy_service.adoc`
+ its UML class files. Branch: `claude/w3c-subject-proxy`. WORKLIST row: W-3c.

**Execution order (owner rulings 2026-07-12, big-bang):** (1) the complete
`ehrbase-sm` crate rewrite FIRST — every SM chapter as a subfolder, big files
broken down, chapter-register trait/type gaps implemented (05 G-1 discrete
`I_EHR_STATUS` mutators, 05 G-2 `VERSIONED_FOLDER` reads, 08 G-1 multi-EHR
query scoping, 03 G-1 dead `CALL_STATUS` struct removed, extensions
quarantined in their own folder) — intermediate steps NEED NOT COMPILE;
(2) then the `ehrbase` + `ehrbase-rest` fixes in one pass; (3) then tests.
The steps below are the content checklist, not the commit order.

- [ ] 1. Model split + corrections (`ehrbase-sm`): module directory;
  `SUBJECT_VARIABLE.history`/`last_frame` read-model fields (G-1 model);
  `SYSTEM_CALL`-faithful `API_CALL`/`QUERY_CALL` methods (G-6);
  master10 YAML/JSON example round-trip test.
- [ ] 2. Storage: `sp_sample` store + `using_app_ids` maintenance in
  `0001_baseline.sql`; persistence expected-tables test updated (G-1, G-10).
- [ ] 3. Engine: executor seam; primary→fallback pipeline (G-3); openEHR
  executor port; subject-id resolution via EHR Index (G-8); every attempt
  recorded as a sample.
- [ ] 4. Freshness: currency evaluation + serve-from-store (G-2); data-set
  currency tightening on `register_application_data_set`.
- [ ] 5. FHIR executor (`reqwest`, config-gated allowlist) + wiremock tests
  (G-4).
- [ ] 6. Extraction v2: `type_name` coercion, time-series selector, FHIR
  JSON pointer, data-set alias resolution on reads (G-7, G-9).
- [ ] 7. Wire: `/rest/subject_proxy` extension routes, JSON+YAML ingestion,
  `notify_variable_sample` (G-11), ATNA audit, extension OAS, website book
  page, changelog (G-5).
- [ ] 8. ECC `SP` area + closure sweep: every G-row in code or re-verified
  cited PORT NOTE (G-12, G-13 recorded); scrub dangling
  `docs/design/sm-platform/0{4,8}-*` references; full gates + ECC zero-drift.
- [ ] 9. **Service-layer restructure (owner directive 2026-07-12):** break the
  whole `app/ehrbase/src/service/` layer into per-SM-chapter subfolders
  mirroring `docs/design/sm-platform/` (ehr/, definition/, demographic/,
  ehr_index/, query/, message/, subject_proxy/ ✓, terminology/, admin/),
  splitting the oversized files (`vobject.rs` 2140, `contribution.rs` 1345,
  `message.rs` 1185, `ehr.rs` 1131, `definition.rs` 864 lines …) into
  focused modules — pure moves + splits, no behaviour change; workspace
  suites prove it.
