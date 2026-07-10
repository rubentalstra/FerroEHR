# B6 — P19: full conformance (the final burn-down)

- Status: in-progress
- Started: 2026-07-10   Owner: Ruben
- Governing plan: blueprint §3 B6; the 12 real failures from the B5-close run
  (docs/conformance/results.json) + the map rows 13/14/15 wire tails
- Baseline: ECC 341 executed · 303 passed · 12 failed · zero-drift gate

## Burn-down clusters (from the live failure list)

- [ ] 1. Stored-query validation (ECC-SQR-006/007): PUT stored query must
      parse/validate the AQL at store time → 400/422 for non-AQL/malformed.
- [ ] 2. AQL `e/ehr_status` on EHR (ECC-QRY-006/010, A/106 golden; blueprint
      row 11 RM-model special case): the engine resolves e/ehr_status
      (+ e/time_created family per master03) instead of rejecting.
- [ ] 3. Demographic delete family (ECC-DEM-005/006/011/014/017/020): party
      DELETE 400s and get-after-delete 200s — fix the delete wire/service
      path (likely version-id handling) so delete → 204 and subsequent
      get → 404.
- [ ] 4. XML-format failures (list from results.json non-json rows) + the
      ITS-REST protocol tail from map row 13 that is MUST-level:
      openEHR-VERSION.*/openEHR-AUDIT_DETAILS.* committal-header parse+merge,
      Last-Modified emission, If-Match hardening (F-01-09/F-02-08),
      OPTIONS / conformance endpoint (R32).
- [ ] 5. Remaining honest AqlBasic/QueryProvisioning/protocol edges surfaced
      by re-runs; status-code fixes (F-02-10, F-03-09/13/14, F-01-11); query
      wire tail (RESULT_SET ETag, query-level 408, query_type).
- [ ] 6. Close/PORT-NOTE the spec-audit backlog residue (blueprint §3 B6).

## Exit criteria

- [ ] Full ECC green minus documented skip-with-reason adjudications; CORE +
      STANDARD claimed in the Statement/Certificate artefacts; blueprint §1/§2
      updated to the "first fully spec-compliant openEHR CDR" claim state.

## Handoff

Opened from develop @ B5 merge (PR #40). Process rules: ONE cargo runner;
agents isolate CARGO_TARGET_DIR; foreground verification; ECC centrally only.
