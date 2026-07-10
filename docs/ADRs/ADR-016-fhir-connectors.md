# ADR-016: FHIR R4 connectors + read façade (not a FHIR server)

- **Status:** accepted (posture owner-confirmed 2026-07-10, roadmap §2.1)
- **Date:** 2026-07-11
- **Spec basis:** SM master10 (DATA_FRAME/HL7_FHIR_SAMPLE — FHIR as a named
  but "currently not standardised" frame source); RM ehr_extract (the
  sanctioned interop serialization; GENERIC_ENTRY bridges non-openEHR
  formats); TERM's official FHIR CodeSystem/ValueSet renderings; B4's FHIR
  terminology client. FHIR↔openEHR mapping is **spec-silent** — this ADR is
  the design record.

## Decision

1. **Connectors + façade only** (owner ruling): the openEHR CDR stays the
   system of record; FHIR is a boundary language. No FHIR persistence, no
   FHIR Search engine, no second product.
2. **Mapping-as-data.** Versioned mapping artefacts (JSON documents,
   uploaded/validated like templates, stored per baseline discipline) bind
   one openEHR template ↔ one FHIR resource profile: field paths (FHIRPath-
   lite on the FHIR side, simplified openEHR paths on the template side),
   code-system translations via the TerminologyService seam. Mappings are
   deployable data, not code — the "custom mappings" parity.
3. **Inbound:** config-gated `POST /fhir/r4/{resourceType}` accepts a FHIR
   R4 resource, resolves the mapping by profile/type, builds the
   COMPOSITION, and commits through the NORMAL validated path (never a
   bypass). Provenance recorded spec-correctly in `FEEDER_AUDIT`
   (originating system/id/version — RM common FEEDER_AUDIT_DETAILS).
4. **Outbound:** (a) event-driven — an E1 outbox consumer emits mapped FHIR
   resources for subscribed changes; (b) the **read façade** — config-gated
   `GET /fhir/r4/{resourceType}[?patient=...]` executes template-bound AQL
   and maps result sets to FHIR Bundles on the fly. Read-only, stateless.
5. **Starter scope:** Patient (↔ EHR_STATUS/demographics), Observation,
   Condition, DocumentReference. Everything else: typed
   `501 OperationOutcome` with PORT NOTEs. Search: only the façade's
   explicit query params — never generic FHIR Search.
6. **Errors as OperationOutcome**; validation failures surface the openEHR
   validator's message (the CDR's rules win — a FHIR resource that maps to
   an invalid COMPOSITION is rejected, not partially stored).

## Consequences

New `fhir` connector module in `ehrbase-rest` (feature/config-gated, off by
default) + mapping store migration + an outbox-consumer emitter; wiremock
fixtures keep CI network-free; ECC untouched (zero-drift gate).

## Alternatives considered

Full FHIR server (owner-rejected: second product); hardcoded per-resource
Rust mappings (kills the deployable-mapping parity); generic FHIR Search
(unbounded scope, rejected with the server).
