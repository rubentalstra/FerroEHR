# E3 — FHIR connectors + read façade

- Status: done (2026-07-11)
- Started: 2026-07-11   Owner: Ruben
- Governing design: docs/enterprise/product-roadmap.md §2.1 (owner-confirmed:
  connectors + façade, NOT a full FHIR server) → ADR-016; spec basis SM
  master10 DATA_FRAME/HL7_FHIR_SAMPLE + RM ehr_extract (GENERIC_ENTRY
  bridging) + TERM FHIR renderings; FHIR mapping itself is spec-silent.
- Gates: workspace suites green; full ECC zero drift (341/315/0); connector
  integration tests (wiremock FHIR fixtures; no network in CI).

## Tasks

- [x] 1. ADR-016 — connector design record: mapping-as-data model (versioned
      mapping artefacts template↔FHIR-profile), inbound pipeline (FHIR
      resource → mapped COMPOSITION → validated commit), outbound (outbox
      event → FHIR resource emission), read façade (config-gated
      /fhir/r4/{Patient,Observation,...} GETs over AQL), scope = the
      starter resource set (Patient, Observation, Condition,
      DocumentReference), everything else typed 501 with PORT NOTEs.
- [x] 2. Mapping engine + artefact store (migration; baseline discipline):
      mapping definitions (JSON) validated on upload; template-bound.
- [x] 3. Inbound connector: POST /fhir/r4/{resource} (config-gated) →
      mapping → COMPOSITION → validated commit path; provenance via
      FEEDER_AUDIT (spec-correct import trail).
- [x] 4. Outbound: subscription-driven emission (E1 outbox consumer) +
      read façade GETs mapping AQL result sets to FHIR bundles. Reverse
      mapping (`service::fhir::mapping::to_fhir`) inverts `build_flat` via
      `openehr-flat::to_flat`; read façade `GET /fhir/r4/{type}?patient&_count`
      queries `v/uid/value` through the QueryService seam then loads via the
      versioned read seam; outbound emitter (`ehrbase::fhir_outbound`) drains
      the outbox on its own cursor (migration 0006) → separate PHI exchange.
- [x] 5. Tests: mapping round-trip units; façade HTTP (Mock) + testcontainers
      integration (commit → GET Bundle); outbound testcontainers RabbitMQ
      (commit → consume FHIR resource). All green.

## Exit criteria

- [x] ADR-016 accepted; end-to-end both directions (inbound validated
      commits + FEEDER_AUDIT; reverse mapping; AQL-backed searchset façade;
      PHI-gated outbound on its own ehrbase.fhir exchange); ECC gate at
      close; scorecard flipped.
