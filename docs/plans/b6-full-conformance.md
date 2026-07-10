# B6 — P19: full conformance (the final burn-down)

- Status: done (2026-07-10)
- Started: 2026-07-10   Owner: Ruben
- Governing plan: blueprint §3 B6; the 12 real failures from the B5-close run
  (docs/conformance/results.json) + the map rows 13/14/15 wire tails
- Baseline: ECC 341 executed · 303 passed · 12 failed · zero-drift gate

## Burn-down clusters (from the live failure list)

- [x] 1. Stored-query validation (ECC-SQR-006/007): PUT stored query must
      parse/validate the AQL at store time → 400/422 for non-AQL/malformed.
- [x] 2. AQL `e/ehr_status` on EHR (ECC-QRY-006/010, A/106 golden; blueprint
      row 11 RM-model special case): the engine resolves e/ehr_status
      (+ e/time_created family per master03) instead of rejecting.
      Done: `PathTarget::EhrStatus` joins the EHR's current EHR_STATUS VO
      (`vo_version.ehr_id=ehr.id`, kind EHR_STATUS, latest) and reassembles the
      whole object / extracts inline leaves under it; the ehr_id/time_created/
      system_id fields already resolved. `service_aql.rs` tests cover the A/106
      SELECT list, whole ehr_status, leaf extraction, and the empty-DB shape.
- [x] 3. Demographic delete family (ECC-DEM-005/006/011/014/017/020): party
      DELETE 400s and get-after-delete 200s — fix the delete wire/service
      path (likely version-id handling) so delete → 204 and subsequent
      get → 404.
- [x] 4. XML-format failures + the MUST-level ITS-REST protocol tail (map
      row 13). (A) VERSION-family canonical XML (F-05-06): the reads route
      through `negotiate::respond_rm`/`read_rm` with the concrete
      `OriginalVersion<Composition>`/`<EhrStatus>`/`VersionedObjectData`/
      `RevisionHistory` (the generated `ToXml` already existed — the 406 was a
      REST-edge policy, so a runtime fix, no emitter change), closing
      ECC-COM-022 + ECC-SIG-001 on `Accept: application/xml`. (B) committal
      headers `openEHR-VERSION.*`/`openEHR-AUDIT_DETAILS.*` parse+merge
      (`ehrbase-rest::committal`, R4 MUST); `Last-Modified` emission
      (`negotiate::set_resource_headers`, R9); If-Match hardening — malformed →
      400 (`require_if_match`) + full-OVID compare (`ensure_if_match`,
      F-01-09/F-02-08); `OPTIONS /` (`status::system_options` above the CORS
      layer, R32). Wire tests: `tests/protocol_tail.rs` + `negotiate`/`committal`
      unit tests (ehrbase-rest 249/249 green).
- [x] 5. Remaining honest AqlBasic/QueryProvisioning/protocol edges surfaced
      by re-runs; status-code fixes (F-02-10, F-03-09/13/14, F-01-11); query
      wire tail (RESULT_SET ETag, query-level 408, query_type).
- [x] 6. Spec-audit backlog: F-01-09/F-02-08/F-05-06 + the 07-family closed
      through B2–B6 waves (finding files updated per wave); remaining ledger
      entries are PERF/documentation residue tracked in SPEC_AUDIT.md for the
      P20 tail — no conformance-bearing finding remains open.

## Exit criteria

- [x] Full ECC green: 2026-07-10 run — **341 executed · 315 passed ·
      0 failed** (26 documented skip-with-reason adjudications); machine
      verdicts **CORE PASS · STANDARD PASS · OPTIONS OBTAINED** in
      CONFORMANCE_STATEMENT.md / CONFORMANCE_CERTIFICATE.md; zero
      regressions vs every prior baseline.

## Handoff

Opened from develop @ B5 merge (PR #40). Process rules: ONE cargo runner;
agents isolate CARGO_TARGET_DIR; foreground verification; ECC centrally only.
