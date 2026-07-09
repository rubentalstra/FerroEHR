# Self-hostable terminology server (Docker) — which one to run, and how to wire it

- **Status:** design — the server selection + run/wire recipe. Built at **B4**
  (blueprint `00-THE-BLUEPRINT.md` §3, map rows 12/21).
- **Date:** 2026-07-09
- **Pairs with:** `docs/terminology-validation.md` (the FHIR-R4 terminology
  **client** we build inside the CDR) and `docs/design/container-images.md`
  (the quickstart compose this extends). This doc is the *server* half.

## 1. Why this exists

Two mission items (blueprint rows 12 and 21) need a real terminology server to
develop and test against:

- **External terminology binding validation (P15 / B4).** Templates can bind a
  coded element to an external value set; the composition validator asks a FHIR
  R4 terminology server (TS) whether a `DV_CODED_TEXT`/`CODE_PHRASE` is a member
  (`$validate-code`, `$expand`) — see `docs/terminology-validation.md`.
- **The AQL terminology family (Q-15/16/23) and its conformance harness (B4).**
  `TERMINOLOGY('expand'|'validate'|…)` and `matches {uri}` resolve against a TS;
  the conformance runner (`tools/conformance`) must exercise these against a
  reachable server.

**Decision: we do not build a terminology server.** We run an existing,
spec-conformant, open-source FHIR R4 TS in Docker and point the CDR and the
conformance runner at it **by URL** (`…/fhir`). Our code is only ever a *client*
of it (the `oauth2`/`reqwest` client in `docs/terminology-validation.md`). This
keeps the CDR's scope to being an openEHR CDR, and lets us test against the same
class of server real deployments use.

## 2. Options considered (FHIR R4, self-hostable, Docker)

| Server | Image | License / content | Fit |
|---|---|---|---|
| **HAPI FHIR JPA starter** | `hapiproject/hapi` (Docker Hub) | Apache-2.0; **bring-your-own** CodeSystem/ValueSet (upload custom, plus LOINC/optional SNOMED delta loads) | **Default.** Single container (embedded H2) or + Postgres; implements `$validate-code`, `$expand`, `$lookup`, `$subsumes`, `$translate`; load test resources over plain REST. Light enough for CI. |
| **Snowstorm** (SNOMED International) | `snomedinternational/snowstorm` (Docker Hub) | Apache-2.0 code; SNOMED CT content needs a member/affiliate licence | **Opt-in, for real SNOMED CT.** FHIR R4 API (built on the HAPI-FHIR library); requires **Elasticsearch** and ~8 GB (16 GB recommended to load the International Edition). Heavy — not the CI default. |
| **Ontoserver** (CSIRO) | `quay.io/aehrc/ontoserver` | **Commercial licence** (free for some jurisdictions) | Production-grade; excluded as the *default* because it is not freely self-hostable without an agreement. Supported as an external URL if a deployment has a licence. |
| **tx.fhir.org** (FHIR reference TS) | public server (not self-hosted) | public test service | Useful as an ad-hoc external URL for manual checks; **not** used as a test dependency (network + shared state make it non-hermetic). |
| LinuxForHealth / Microsoft FHIR | various | Apache-2.0 / MIT | Full FHIR servers; terminology support is thinner/cloud-oriented than HAPI. Not selected. |

**Verdict:** **HAPI FHIR** is the default local + CI terminology server (open,
one container, arbitrary custom ValueSets — exactly what our binding tests
need). **Snowstorm** is the opt-in profile for genuine SNOMED CT / LOINC
subsumption testing. Any TS is ultimately just a URL to the client, so a
deployment may point at Ontoserver or a managed TS instead.

## 3. Run it (compose service, extends the §4 quickstart)

Add an optional `terminology` service to the repo-root `docker-compose.yml`
(the one in `docs/design/container-images.md`), behind a compose **profile** so
it is off by default and started only when testing terminology:

```yaml
services:
  terminology:                     # off by default: `--profile terminology`
    image: hapiproject/hapi:latest
    profiles: ["terminology"]
    environment:
      # serve at http://terminology:8080/fhir ; R4
      hapi.fhir.fhir_version: R4
      hapi.fhir.allow_external_references: "true"
      hapi.fhir.enforce_referential_integrity_on_write: "false"
    ports: ["8090:8080"]           # host 8090 → avoid clashing with ehrbase 8080
    healthcheck:
      test: ["CMD", "curl", "-fsS", "http://localhost:8080/fhir/metadata"]
      interval: 10s
      timeout: 5s
      retries: 30

  ehrbase:
    environment:
      # point the CDR's external-terminology client at the TS by URL
      EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_ENABLED: "true"
      EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_PROVIDERS__DEFAULT__TYPE: "fhir"
      EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_PROVIDERS__DEFAULT__URL: "http://terminology:8090/fhir"
    depends_on:
      terminology:
        condition: service_healthy
```

The env keys are exactly the `figment` shape in `docs/terminology-validation.md`
§4 (`__` = nesting). For SNOMED, a second `terminology-snowstorm` service +
`elasticsearch` sidecar behind a `snomed` profile; expect ~8–16 GB and a longer
first-start while the RF2 import runs (`snomedinternational/snowstorm` docs).

## 4. Seed it with test content

HAPI starts empty; we seed the fixtures the client + AQL tests reference
(`docs/terminology-validation.md` uses `ValueSet/surface`, code `B`=Buccal):

- **Load via plain REST** (no CLI needed):
  `PUT http://localhost:8090/fhir/CodeSystem/<id>` then
  `PUT …/ValueSet/<id>` with the test CodeSystem/ValueSet JSON. ValueSets are
  background-expanded on upload; `$expand`/`$validate-code` then answer from the
  pre-computed expansion.
- Fixtures live under `tools/conformance/` (the `TS` case area, blueprint B4) so
  the runner can `POST`/`PUT` them at start-up and tear them down after.
- Larger external systems (LOINC, SNOMED delta) use
  `hapi-fhir-cli upload-terminology` when a test genuinely needs them; the
  default suite stays on small custom ValueSets so CI is fast and offline-ish.

## 5. Test wiring (the B4 harness)

Blueprint B4 defines a `TS` conformance case area with two modes; this server is
the "real server" mode:

1. **Hermetic CI (default): `wiremock`.** The runner spins a wiremock FHIR-tx
   fixture (canned `$expand`/`$validate-code`/`$lookup`/`$subsumes` responses +
   fault injection: timeouts, 5xx, malformed) — no container, no network. This
   is what gates every CI run.
2. **Real-server mode: `--tx-server-url http://localhost:8090/fhir`.** The same
   cases run against the HAPI (or Snowstorm/Ontoserver) container from §3;
   skip-with-reason when the flag is unset. The wiremock exchange / real HTTP
   exchange is recorded in the conformance report.

The CDR's own client is validated the same way: unit tests parse the
`terminology://…$expand?url=…` binding and decide membership from a mocked
`$validate-code`; `wiremock` integration tests cover accept/reject, fail-open vs
fail-closed, OAuth2 token attach, and cache-hit (see
`docs/terminology-validation.md` §5).

## 6. Notes / constraints

- **Content licences are the operator's responsibility.** LOINC is free with
  registration; SNOMED CT requires a member/affiliate licence; our test
  fixtures are small, licence-free custom CodeSystems/ValueSets so the default
  path needs no licence.
- **Ports:** TS on host `8090` to leave `8080` for the CDR.
- **Not a runtime dependency of the CDR.** With external terminology disabled
  (`EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_ENABLED=false`, the default),
  validation uses only the in-process `openehr-term` bundle; the TS is needed
  only for external-value-set bindings and the AQL terminology family.
- **Auth:** HAPI runs open by default (dev). When a TS requires OAuth2
  client-credentials or mTLS, that is configured on the *client* side per
  `docs/terminology-validation.md` §4 (per-provider `oauth2_client` / `mtls`).

## Sources

- [HAPI FHIR — Terminology (JPA server)](https://hapifhir.io/hapi-fhir/docs/server_jpa/terminology.html)
- [HAPI FHIR — Validation Support Modules](https://hapifhir.io/hapi-fhir/docs/validation/validation_support_modules.html)
- [IHTSDO/snowstorm — Scalable SNOMED CT Terminology Server (using Docker)](https://github.com/IHTSDO/snowstorm/blob/master/docs/using-docker.md)
- [snomedinternational/snowstorm — Docker Hub](https://hub.docker.com/r/snomedinternational/snowstorm)
- [HL7 Confluence — Open Source FHIR Implementations](https://confluence.hl7.org/display/FHIR/Open+Source+Implementations)
