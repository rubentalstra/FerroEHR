# Phase 15 — Service layer

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): all prior phases (03-14)
- Compile required: no (Phase A)

## Objectives

Port EHRbase's `service` module: orchestration and transaction wrapping over
persistence (Phase 07), rm-db-format (Phase 14), and validation (Phase 11),
implementing versioning, contribution recording, and audit-detail insertion
on every write, wiring the REST skeleton (Phase 06) handlers to real behavior.

## Preconditions

- [ ] Phase 06 done: REST skeleton with stub handlers exists
- [ ] Phase 07, 11, 14 done: persistence, validation, rm-db-format available

## Scope

In: EHR/EHR_STATUS/COMPOSITION/FOLDER/CONTRIBUTION service implementations,
transaction wrapping, versioning orchestration (current + `_history` pairs),
audit_details + contribution insertion on every write, wiring REST handlers
to these services.
Out: AQL query execution wiring (Phase 13 is the engine; this phase wires the
`/aql` REST handler to it, a smaller task than the engine itself),
FLAT/STRUCTURED input translation (Phase 16 sits in front of this layer).

## Tasks

- [ ] Port the EHR service: create/get/update EHR, EHR_STATUS versioned-object semantics
- [ ] Port the COMPOSITION service: create/get/update/delete with Phase 11 validation invoked before persistence
- [ ] Port the FOLDER/DIRECTORY service and CONTRIBUTION service
- [ ] Implement transaction wrapping so every write is atomic across `comp_data`, `comp_version`, `audit_details`, and `contribution` inserts
- [ ] Implement versioning orchestration: current + `_history` table pair updates on every modifying write
- [ ] Implement audit_details insertion and contribution recording on every write, matching EHRbase's audit trail semantics
- [ ] Wire Phase 06's stub REST handlers for EHR/EHR_STATUS/COMPOSITION/FOLDER/CONTRIBUTION to these service implementations
- [ ] Wire the `/aql` REST handler to the Phase 13 AQL engine
- [ ] Write integration tests exercising create -> update -> get -> version-history-get for a composition end-to-end
- [ ] Add PORT STATUS trailers referencing EHRbase's `service` module Java classes as source

## Exit criteria

- [ ] A composition can be created, updated, and its version history retrieved end-to-end through the REST API
- [ ] Every write produces the expected audit_details and contribution rows
- [ ] The `/aql` endpoint executes a query end-to-end through the wired engine

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. This is the phase where the previously-independent pieces
(REST, persistence, validation, rm-db-format, AQL) get wired together for
the first time; expect to surface integration gaps in earlier phases and
loop back to fix them rather than papering over mismatches here.
