# E3 — FHIR connectors + read façade

- Status: in-progress
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
- [ ] 4. Outbound: subscription-driven emission (E1 outbox consumer) +
      read façade GETs mapping AQL result sets to FHIR bundles.
- [ ] 5. Tests: mapping round-trip fixtures; inbound commit + validation
      rejection; façade reads against committed data; wiremock outbound.

## Exit criteria

- [ ] ADR-016 accepted; starter resource set works end-to-end both
      directions; ECC zero drift; scorecard rows flipped.
