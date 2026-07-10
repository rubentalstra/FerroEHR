# B4 — Terminology-server integration (+ its test harness)

- Status: in-progress
- Started: 2026-07-10   Owner: Ruben
- Governing plan: blueprint §3 B4; designs `docs/terminology-validation.md`
  (client) + `docs/design/terminology-server-integration.md` (Docker TS)
- Oracle: QUERY master03 (AQL TERMINOLOGY family), SM master12; ECC baseline
  293 passed / 329 executed, zero-drift gate

## Tasks (blueprint §3 B4)

- [x] 1. External tx-server provider (FHIR R4 TS via reqwest) behind the
      existing `TerminologyService` trait — real subsumes /
      value_set_validate / get_value_set against a remote server; the
      openEHR-bundle provider stays the local default. *Done 2026-07-10:
      FhirTerminologyProvider (validate-code/expand/subsumes-strict/lookup,
      figment config, bundle-default/fhir-opt-in, typed 404/exception
      mapping); 12 wiremock tests incl. fault injection; OAuth2/mTLS +
      walker hookup PORT-NOTEd for later tasks.*
- [ ] 2. AQL terminology family (Q-15/16/23): `TERMINOLOGY('expand'|'validate'
      |…, service_api, params_uri)` + `matches {uri}` + mixed lists, expansion
      merged into matches at semantic analysis (master03 lines 756–759);
      staged expand → validate-as-boolean → URI operand; typed rejects until
      each lands.
- [ ] 3. Terminology wire exposure (extension OAS, design doc 08 §7) for
      I_TERMINOLOGY_SERVICE (+ EHR Index/Admin wire while in the area).
- [ ] 4. Test harness in tools/conformance: `TS` case area — wiremock-backed
      FHIR-tx fixture server spun up by the runner (expand/validate/lookup/
      subsumes + fault injection) and optional real-server mode
      (`--tx-server-url`, skip-with-reason when unset).

## Exit criteria

- [ ] Mission item demonstrable in CI without network + against a real Docker
      TS on demand; workspace suites green; full ECC zero drift.

## Handoff

Opened from develop @ B3 merge (PR #38). Process rules: ONE cargo runner at a
time; agents use isolated CARGO_TARGET_DIR; verification inline foreground;
ECC only via scripts/conformance.sh, run centrally at phase close.
