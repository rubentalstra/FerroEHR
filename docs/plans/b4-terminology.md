# B4 — Terminology-server integration (+ its test harness)

- Status: done (2026-07-10)
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
- [x] 2. AQL terminology family (Q-15/16/23): `TERMINOLOGY('expand'|'validate'
      |…, service_api, params_uri)` + `matches {uri}` + mixed lists, expansion
      merged into matches at semantic analysis (master03 lines 756–759);
      staged expand → validate-as-boolean → URI operand; typed rejects until
      each lands.
- [x] 3. Terminology wire exposure (extension OAS, design doc 08 §7) for
      I_TERMINOLOGY_SERVICE (+ EHR Index/Admin wire while in the area).
      *Done 2026-07-10: config-gated (`RestConfig::terminology`, OFF by
      default) `/terminology` extension routes in `ehrbase-rest`
      (`dispatch/terminology.rs`), dispatching to the `TerminologyService`
      seam — GET `/terminology` (ids), `/terminology/{tid}` (description),
      `/terminology/{tid}/term/{code}` (get_term/lookup),
      `/terminology/{tid}/subsumes` (subsumes),
      `/terminology/{tid}/value_set/{vs}` (get_value_set/expand),
      `/terminology/{tid}/value_set/{vs}/validate` (value_set_validate).
      Typed error mapping via the existing `sm_api_error` (bundle's
      `versioned_object_does_not_exist` → 404). 12 HTTP tests via the shared
      Mock (new terminology hooks in tests/common). Gates:
      nextest -p ehrbase-rest 231/231, -p ehrbase-sm 9/9, -p ehrbase 273
      (1 skip), clippy no new warnings, fmt clean.
      PORT NOTE (deferrals): (a) the boolean `has_terminology`/`has_term`/
      `has_value_set` calls are folded into the 200-vs-404 of their get_
      counterparts (idiomatic REST; not separate endpoints); (b) get_term's
      `attributes` allow-list is not surfaced on the wire (ambiguous
      map-vs-list shape, bundle ignores it) — passed as None; (c) EHR
      Index/Admin wire NOT added: §7 lists their namespaces but not their
      call/route shapes (unlike terminology's `TerminologyService` seam,
      EhrIndex/Admin dump-load are still SM-3/SM-4 service work — not
      equally mechanical), deferred to their own SM phases; §7's
      `/rest/terminology` namespace is realized as `{base_path}/terminology`
      (extension groups nest inside the API router, mirroring ADMIN — PORT
      NOTE in the dispatcher).*
- [x] 4. Test harness in tools/conformance: `TS` case area — wiremock-backed
      FHIR-tx fixture server spun up by the runner (expand/validate/lookup/
      subsumes + fault injection) and optional real-server mode
      (`--tx-server-url`, skip-with-reason when unset). *Done 2026-07-10:
      new `Area::Ts` + `Capability::Terminology`; `ts::fixture::FhirTxFixture`
      (canned + fault modes, self-check, exchange recording; 6 nextest tests);
      `--tx-server-url` CLI + runner spins up the hermetic fixture by default
      and records the FHIR-tx exchange in the report (`RunResults.terminology`).
      9 ECC-TS cases: bundle expand real passes (TS-001..005), FHIR-provider
      PASS/SKIP(SutConfig) (TS-006), fault-injection SKIP(SutConfig) citing the
      fixture + app wiremock evidence (TS-007..009). `nextest -p conformance`
      41/41 green; catalog regenerated (guard green).*

## Exit criteria

- [x] Mission item demonstrable in CI without network (wiremock fixture +
      bundle-provider wire passes) + against a real Docker TS on demand
      (--tx-server-url); workspace suites green (rest 231/231, ehrbase
      273/273, sm 9/9, conformance 41/41); phase-close ECC 2026-07-10:
      338 executed · 298 passed · zero drift (5 new live TS passes).

## Handoff

Opened from develop @ B3 merge (PR #38). Process rules: ONE cargo runner at a
time; agents use isolated CARGO_TARGET_DIR; verification inline foreground;
ECC only via scripts/conformance.sh, run centrally at phase close.
