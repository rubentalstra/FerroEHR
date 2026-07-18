# Conformance Report (generated)

> Generated from a conformance run — never hand-asserted. Every claim is a
> pure function of the recorded outcomes; every coverage bound is printed.

## 1. System under test

| Field | Value |
|---|---|
| Product | ehrbase-rs ehrbase-rs 3.1.1 |
| SUT class | ours (ehrbase-rs) |
| Base URL | `http://localhost:8080/ehrbase/rest/openehr/v1` |
| Auth mode | basic |
| Edition policy | pinned (development) |
| Spec versions | RM 1.2.0 · ITS-REST development@e8a093e · AQL 1.1.0 · TERM 3.1.0 |
| Reference corpus | openEHR/specifications-CNF@33251d2a |
| Run started | 2026-07-18T14:19:21.017017Z |

**386 case×format executions · 0 passed · 0 failed · 352 errored · 34 skipped · 0 not applicable.**

## 2. Per-area matrix

| Area | Catalogue (active) | Passed | Failed | Errored | Skipped | N/A |
|---|--:|--:|--:|--:|--:|--:|
| EHR — EHR service | 13 | 0 | 0 | 13 | 0 | 0 |
| STA — EHR_STATUS | 10 | 0 | 0 | 10 | 0 | 0 |
| COM — COMPOSITION | 32 | 0 | 0 | 39 | 0 | 0 |
| CTB — CONTRIBUTION (change sets) | 31 | 0 | 0 | 26 | 5 | 0 |
| DIR — DIRECTORY (FOLDER) | 37 | 0 | 0 | 37 | 0 | 0 |
| TPL — Template / OPT provisioning | 17 | 0 | 0 | 13 | 4 | 0 |
| SQR — Stored-query provisioning | 7 | 0 | 0 | 5 | 2 | 0 |
| QRY — AQL execution | 25 | 0 | 0 | 24 | 1 | 0 |
| VAL — Content / archetype validation | 119 | 0 | 0 | 119 | 0 | 0 |
| DEM — Demographic service | 31 | 0 | 0 | 31 | 0 | 0 |
| ADM — Admin service | 14 | 0 | 0 | 6 | 8 | 0 |
| SEC — Security / authorization | 2 | 0 | 0 | 2 | 0 | 0 |
| SIG — Version signing | 5 | 0 | 0 | 5 | 1 | 0 |
| MSG — Messaging | 10 | 0 | 0 | 0 | 10 | 0 |
| TS — Terminology-server integration | 9 | 0 | 0 | 6 | 3 | 0 |
| SF — Simplified Formats (FLAT / STRUCTURED / Web Template) | 16 | 0 | 0 | 16 | 0 | 0 |

## 3. Capability matrix

Cases grouped by capability; the evidence classification folds a transport error into `failed` (an errored capability is never claimed as passed).

| Capability | Passed | Failed | Errored | Skipped | N/A | Evidence |
|---|--:|--:|--:|--:|--:|---|
| Adl14ArchetypeProvisioning | 0 | 0 | 1 | 0 | 0 | **FAIL** |
| Adl14OptProvisioning | 0 | 0 | 12 | 4 | 0 | **FAIL** |
| EhrOperations | 0 | 0 | 12 | 0 | 0 | **FAIL** |
| EhrStatus | 0 | 0 | 10 | 0 | 0 | **FAIL** |
| CompositionOps | 0 | 0 | 35 | 0 | 0 | **FAIL** |
| ChangeSets | 0 | 0 | 26 | 5 | 0 | **FAIL** |
| Versioning | 0 | 0 | 7 | 0 | 0 | **FAIL** |
| ArchetypeValidation | 0 | 0 | 119 | 0 | 0 | **FAIL** |
| DirectoryOps | 0 | 0 | 34 | 0 | 0 | **FAIL** |
| QueryProvisioning | 0 | 0 | 5 | 2 | 0 | **FAIL** |
| AqlBasic | 0 | 0 | 23 | 1 | 0 | **FAIL** |
| AqlAdvanced | 0 | 0 | 1 | 0 | 0 | **FAIL** |
| PartyOperations | 0 | 0 | 25 | 0 | 0 | **FAIL** |
| PartyRelationshipOperations | 0 | 0 | 6 | 0 | 0 | **FAIL** |
| AdminActivityReport | 0 | 0 | 0 | 4 | 0 | not evidenced |
| AdminPhysicalDeletion | 0 | 0 | 6 | 1 | 0 | **FAIL** |
| AdminEhrDumpLoad | 0 | 0 | 0 | 1 | 0 | not evidenced |
| AdminEhrArchive | 0 | 0 | 0 | 1 | 0 | not evidenced |
| AdminDemographicArchive | 0 | 0 | 0 | 1 | 0 | not evidenced |
| MessagingEhrExtract | 0 | 0 | 0 | 7 | 0 | not evidenced |
| MessagingTds | 0 | 0 | 0 | 3 | 0 | not evidenced |
| Signing | 0 | 0 | 5 | 1 | 0 | **FAIL** |
| AnonymousEhrs | 0 | 0 | 1 | 0 | 0 | **FAIL** |
| Authentication | 0 | 0 | 2 | 0 | 0 | **FAIL** |
| Terminology | 0 | 0 | 6 | 3 | 0 | **FAIL** |
| SimplifiedFormats | 0 | 0 | 16 | 0 | 0 | **FAIL** |

## 4. Profile verdict (machine-computed)

CORE/STANDARD are all-of (every listed capability must be `pass`); OPTIONS is any-of (obtained if any optional capability passes) — `master03-profiles.adoc`. An unevidenced required capability fails the claim.

### Core — not claimable

| Capability | Passed | Failed | Skipped | N/A | Evidence |
|---|--:|--:|--:|--:|---|
| Adl14ArchetypeProvisioning | 0 | 1 | 0 | 0 | **FAIL** |
| Adl14OptProvisioning | 0 | 12 | 4 | 0 | **FAIL** |
| EhrOperations | 0 | 12 | 0 | 0 | **FAIL** |
| EhrStatus | 0 | 10 | 0 | 0 | **FAIL** |
| CompositionOps | 0 | 35 | 0 | 0 | **FAIL** |
| ChangeSets | 0 | 26 | 5 | 0 | **FAIL** |
| Versioning | 0 | 7 | 0 | 0 | **FAIL** |
| ArchetypeValidation | 0 | 119 | 0 | 0 | **FAIL** |
| AnonymousEhrs | 0 | 1 | 0 | 0 | **FAIL** |

### Standard — not claimable

| Capability | Passed | Failed | Skipped | N/A | Evidence |
|---|--:|--:|--:|--:|---|
| Adl14ArchetypeProvisioning | 0 | 1 | 0 | 0 | **FAIL** |
| Adl14OptProvisioning | 0 | 12 | 4 | 0 | **FAIL** |
| EhrOperations | 0 | 12 | 0 | 0 | **FAIL** |
| EhrStatus | 0 | 10 | 0 | 0 | **FAIL** |
| CompositionOps | 0 | 35 | 0 | 0 | **FAIL** |
| ChangeSets | 0 | 26 | 5 | 0 | **FAIL** |
| Versioning | 0 | 7 | 0 | 0 | **FAIL** |
| ArchetypeValidation | 0 | 119 | 0 | 0 | **FAIL** |
| AnonymousEhrs | 0 | 1 | 0 | 0 | **FAIL** |
| QueryProvisioning | 0 | 5 | 2 | 0 | **FAIL** |
| DirectoryOps | 0 | 34 | 0 | 0 | **FAIL** |
| AqlBasic | 0 | 23 | 1 | 0 | **FAIL** |
| Signing | 0 | 5 | 1 | 0 | **FAIL** |

### Options — not obtained

| Capability | Passed | Failed | Skipped | N/A | Evidence |
|---|--:|--:|--:|--:|---|
| Adl2Provisioning | 0 | 0 | 0 | 0 | no cases |
| PartyOperations | 0 | 25 | 0 | 0 | **FAIL** |
| PartyRelationshipOperations | 0 | 6 | 0 | 0 | **FAIL** |
| AqlAdvanced | 0 | 1 | 0 | 0 | **FAIL** |
| AqlTerminology | 0 | 0 | 0 | 0 | no cases |
| AdminActivityReport | 0 | 0 | 4 | 0 | not evidenced |
| AdminPhysicalDeletion | 0 | 6 | 1 | 0 | **FAIL** |
| AdminEhrDumpLoad | 0 | 0 | 1 | 0 | not evidenced |
| AdminBulkEhrLoad | 0 | 0 | 0 | 0 | no cases |
| AdminEhrArchive | 0 | 0 | 1 | 0 | not evidenced |
| AdminDemographicArchive | 0 | 0 | 1 | 0 | not evidenced |
| MessagingEhrExtract | 0 | 0 | 7 | 0 | not evidenced |
| MessagingTds | 0 | 0 | 3 | 0 | not evidenced |
| SimplifiedFormats | 0 | 16 | 0 | 0 | **FAIL** |

## 5. Failures

_No failures in this run._

## 5b. Runner/SUT errors (transport)

Transport-level errors — not conformance findings, but the affected capabilities cannot be claimed as passed.

- **ECC-EHR-001** EHR existence check — existing EHR id (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/ff190a1e-19b0-4951-9fa8-f3689c806e70)
- **ECC-EHR-002** EHR existence check — existing subject id (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-EHR-003** EHR existence check — non existing EHR id (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/7e041593-3ea3-492c-a442-eab2529895c9)
- **ECC-EHR-004** EHR existence check — non existing subject id (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr?subject_id=nobody-c759d919-915f-455c-90eb-1d8b89403ae6&subject_namespace=conformance)
- **ECC-EHR-005** Create EHR — main (valid data-set matrix) (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-EHR-006** Create EHR — same EHR twice (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/572c270e-43a4-485d-a524-e7920b6e50a2)
- **ECC-EHR-007** Create EHR — two EHRs same patient (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-EHR-008** Get EHR — existing EHR by EHR id (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-EHR-009** Get EHR — existing EHR by subject id (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-EHR-010** Get EHR — get EHR by invalid EHR id (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/d6aa142c-7707-44a9-92af-2ab93d0d224b)
- **ECC-EHR-011** Get EHR — get EHR by invalid subject id (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr?subject_id=nobody-13a93f2d-1eb3-4df9-b4ee-108c4bdf85b3&subject_namespace=conformance)
- **ECC-STA-001** Get EHR_STATUS — get by EHR id (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-STA-002** Get EHR_STATUS — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/1a684d81-89e1-4519-8df2-4cb68941421c/ehr_status)
- **ECC-STA-003** Set EHR_STATUS is_queryable — existing EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-STA-004** Set EHR_STATUS is_queryable — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-STA-005** Set EHR_STATUS is_modifiable — existing EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-STA-006** Set EHR_STATUS is_modifiable — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-STA-007** Clear EHR_STATUS is_queryable — existing EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-STA-008** Clear EHR_STATUS is_queryable — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-STA-009** Clear EHR_STATUS is_modifiable — existing EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-STA-010** Clear EHR_STATUS is_modifiable — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-EHR-012** Create EHR — reject invalid EHR_STATUS data sets (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-EHR-013** Create anonymous (subject-less) EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-001** Create composition — event (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-001** Create composition — event (xml): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-002** Create composition — persistent (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-002** Create composition — persistent (xml): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-003** Create composition — same OPT twice (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-004** Create composition — invalid event (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-005** Create composition — invalid persistent (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-006** Create composition — event bad OPT (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-007** Create composition — event bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-COM-032** Composition existence check — existing composition (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-011** Composition existence check — bad composition (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-012** Composition existence check — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/6be6321e-1878-41d9-aa89-6fc47c9fcf4c/composition/eb95cf4a-2861-4be7-ae74-1eb77ecb4872)
- **ECC-COM-008** Get latest composition (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-008** Get latest composition (xml): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-009** Get latest composition — bad composition (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-010** Get latest composition — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/c311e356-89ea-4cfe-ab59-cc0fc5e709de/composition/94161ba9-2b22-4b7d-87f2-ef13ca707104)
- **ECC-COM-013** Get composition at time (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-013** Get composition at time (xml): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-014** Get composition at time — no time arg (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-014** Get composition at time — no time arg (xml): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-015** Get composition at time — bad composition (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-016** Get composition at time — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/7b52254a-bc36-42e4-ba17-79ebb8b0a07f/composition/8e0e97e4-ed57-4c34-92aa-8c7788e857c2?version_at_time=2030-01-01T00:00:00Z)
- **ECC-COM-017** Get composition at multiple times (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-018** Get composition version (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-018** Get composition version (xml): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-019** Get composition version — bad version (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-020** Get composition version — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-021** Get composition versions (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-022** Get versioned composition (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-022** Get versioned composition (xml): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-023** Get versioned composition — non existent (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-024** Get versioned composition — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/56d61814-383c-4508-bb2e-2968f31ca200/versioned_composition/071ab331-1e75-470a-a4f3-3e6ac6249ae3)
- **ECC-COM-025** Update composition — event (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-026** Update composition — persistent (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-027** Update composition — non existent (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-028** Update composition — wrong template (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-029** Delete composition — event (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-030** Delete composition — persistent (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-COM-031** Delete composition — non existent (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-CTB-001** Commit contribution — valid composition (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-CTB-002** Commit contribution — invalid composition (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-CTB-003** Commit contribution — empty (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-CTB-004** Commit contribution — valid invalid compositions (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-CTB-005** Commit contribution — non exiting OPT (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-CTB-006** Commit contribution — event composition (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-CTB-007** Commit contribution — persistent composition (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-CTB-008** Commit contribution — delete (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-CTB-009** Commit contribution — two commits second invalid (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-CTB-010** Commit contribution — two commits second creation (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-CTB-011** Commit contribution — minimal EHR status (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-CTB-012** Commit contribution — full EHR status (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-CTB-013** Commit contribution — EHR status invalid change type (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-CTB-014** Commit contribution — invalid EHR status (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-CTB-015** Commit contribution — valid directory (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-CTB-016** Commit contribution — fail create existing directory (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-CTB-017** Commit contribution — fail modify non existing directory (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-CTB-018** Commit contribution — update existing directory (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-CTB-019** Get contribution — existing (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-CTB-020** Get contribution — empty EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-CTB-021** Get contribution — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/72a6ffd7-4f39-4c46-bf9d-37c2ad5a2b64/contribution/85109e60-28c3-4c91-8e97-2d67963599b2)
- **ECC-CTB-022** Get contribution — bad contribution (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-CTB-023** Contribution existence check — existing (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-CTB-024** Contribution existence check — bad contribution (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-CTB-025** Contribution existence check — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/a5904e4e-3a12-45ae-99eb-f636f369417a/contribution/1318b39b-4309-48f3-8a82-0a573fc6ae2b)
- **ECC-CTB-026** Contribution existence check — empty EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-012** Directory existence check — empty EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-013** Directory existence check — EHR with directory (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-014** Directory existence check — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/0e1a38ae-4d42-4a7d-a190-bf6d70ea3aae/directory)
- **ECC-DIR-015** Directory path existence check — EHR root directory (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-016** Directory path existence check — folder structure (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-017** Directory path existence check — empty EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-018** Directory path existence check — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/9b3ff836-1e1c-48dc-8bd0-b57520e61181/directory?path=/)
- **ECC-DIR-001** Create directory — empty EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-002** Create directory — EHR with directory (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-003** Create directory — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/58ef2900-017b-4f7b-9545-0cf4146f883f/directory)
- **ECC-DIR-022** Get directory — empty EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-004** Get directory — EHR root directory (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-023** Get directory — directory with structure (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-005** Get directory — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/78db12fd-909f-44a8-8f5a-67650432489e/directory)
- **ECC-DIR-006** Get directory at time — EHR with directory (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-007** Get directory at time — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/f9e59d30-0b18-4691-bff4-344215660c6e/directory?version_at_time=2026-07-18T14:19:15.385458Z)
- **ECC-DIR-008** Update directory — EHR with directory (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-009** Update directory — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-010** Delete directory — EHR with directory (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-011** Delete directory — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-019** Directory version existence check — empty EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-020** Directory version existence check — directory with two versions (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-021** Directory version existence check — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-024** Get directory at time — EHR with directory empty time (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-025** Get directory at time — EHR with directory versions (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-026** Get directory at time — EHR with directory versions empty time (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-027** Get directory at time — empty EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-028** Get directory at time — empty EHR empty time (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-029** Get directory at time — multiple versions first (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-030** Get directory at version — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-031** Get directory at version — directory with two versions (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-032** Get directory at version — empty EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-033** Get versioned directory — empty EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-034** Get versioned directory — directory with two versions (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-035** Get versioned directory — bad EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-036** Update directory — empty EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-DIR-037** Delete directory — empty EHR (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-TPL-011** Validate OPT — valid OPT (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-TPL-012** Validate OPT — invalid OPT (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-TPL-001** Upload OPT — valid OPT (provisions ADL 1.4 archetypes) (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-TPL-002** Upload OPT — invalid OPT (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-TPL-004** Upload OPT — valid OPT twice conflict (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-TPL-005** Upload OPT — valid OPT twice no conflict (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-TPL-006** Get OPT — retrieve single (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-TPL-009** Get OPT — retrieve fail (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4/does.not.exist.032b4d15ebf64e3b8a02a6fec54178ca.v1)
- **ECC-TPL-007** Get OPT — retrieve latest version (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-TPL-008** Get OPT — retrieve specific version (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-TPL-010** List OPTs — retrieve all (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-TPL-003** List OPTs — retrieve all no OPTs (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-TPL-017** Example COMPOSITION round-trips (ADL 1.4 example → commit) (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-SQR-001** Store stored query — valid (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/query/org.conformance::qb7f793c2f80e43ec810d96559ddfc654/1.0.0)
- **ECC-SQR-007** Store stored query — invalid (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/query/org.conformance::q6c517c2e4d5f4127b61727bd8809ebc8/1.0.0)
- **ECC-SQR-006** Store stored query — bad formalism (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/query/org.conformance::q82e474b12d5645e0bda71eda2f1caacb/1.0.0)
- **ECC-SQR-008** Stored query existence check — existing (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/query/org.conformance::q2965354aea334f9ea50a00e84ab51649/1.0.0)
- **ECC-SQR-002** List stored queries — non empty (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/query/org.conformance::qb13c08440a084cb1998261fcc9f2aaae/1.0.0)
- **ECC-QRY-001** Query service smoke test (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-002** Execute ad-hoc AQL query — empty db (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-003** Execute stored AQL query — empty db (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/query/org.conformance::stored_b82c1e273fd142ee8a55b36b65cddf57/1.0.0)
- **ECC-QRY-004** Execute ad-hoc AQL query — loaded db (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-QRY-025** AQL uid projection — c/uid/value returns the version id (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-QRY-005** AQL corpus — invalid queries rejected (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-014** AQL advanced — ORDER BY + LIMIT/OFFSET (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-006** AQL corpus — A empty db (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-007** AQL corpus — B empty db (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-008** AQL corpus — C empty db (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-009** AQL corpus — D empty db (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-010** AQL corpus — A loaded db (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-011** AQL corpus — B loaded db (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-013** AQL corpus — D loaded db (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-015** AQL corpus — dialect-adjudicated query rejected (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-016** AQL corpus — dialect-adjudicated query rejected (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-017** AQL corpus — dialect-adjudicated query rejected (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-018** AQL corpus — dialect-adjudicated query rejected (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-019** AQL corpus — dialect-adjudicated query rejected (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-020** AQL corpus — dialect-adjudicated query rejected (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-021** AQL corpus — dialect-adjudicated query rejected (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-022** AQL corpus — dialect-adjudicated query rejected (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-023** AQL corpus — dialect-adjudicated query rejected (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-QRY-024** AQL corpus — dialect-adjudicated query rejected (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-VAL-001** Validate COMPOSITION — content card any context any (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-002** Validate COMPOSITION — content card 1plus context any (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-003** Validate COMPOSITION — content card 3plus context any (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-004** Validate COMPOSITION — content card OPT context any (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-005** Validate COMPOSITION — content card mand context any (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-006** Validate COMPOSITION — content card 3to5 context any (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-007** Validate COMPOSITION — content card any context mand (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-008** Validate COMPOSITION — content card 1plus context mand (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-009** Validate COMPOSITION — content card 3plus context mand (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-010** Validate COMPOSITION — content card OPT context mand (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-011** Validate COMPOSITION — content card mand context mand (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-012** Validate COMPOSITION — content card 3to5 context mand (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-013** Validate OBSERVATION — state ex OPT protocol ex OPT (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-014** Validate OBSERVATION — state ex OPT protocol ex mand (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-015** Validate OBSERVATION — state ex mand protocol ex OPT (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-016** Validate OBSERVATION — state ex mand protocol ex mand (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-017** Validate HISTORY — events card any summary ex OPT (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-018** Validate HISTORY — events card 1plus summary ex OPT (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-019** Validate HISTORY — events card 3plus summary ex OPT (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-020** Validate HISTORY — events card OPT summary ex OPT (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-021** Validate HISTORY — events card mand summary ex OPT (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-022** Validate HISTORY — events card 3to5 summary ex OPT (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-023** Validate HISTORY — events card any summary ex mand (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-024** Validate HISTORY — events card 1plus summary ex mand (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-025** Validate HISTORY — events card 3plus summary ex mand (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-026** Validate HISTORY — events card OPT summary ex mand (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-027** Validate HISTORY — events card mand summary ex mand (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-028** Validate HISTORY — events card 3to5 summary ex mand (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-029** Validate EVENT — state ex OPT (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-030** Validate EVENT — state ex mand (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-031** Validate EVENT — type any (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-032** Validate EVENT — type point event (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-033** Validate EVENT — type interval event (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-034** Validate ITEM_STRUCTURE — type any (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-035** Validate ITEM_STRUCTURE — type item tree (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-036** Validate ITEM_STRUCTURE — type item list (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-037** Validate ITEM_STRUCTURE — type item table (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-038** Validate ITEM_STRUCTURE — type item single (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-039** Validate DV_BOOLEAN — anything allowed (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-040** Validate DV_BOOLEAN — only true allowed (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-041** Validate DV_BOOLEAN — only false allowed (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-042** Validate DV_IDENTIFIER — all pattern (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-043** Validate DV_IDENTIFIER — all list (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-044** Validate DV_TEXT — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-045** Validate DV_TEXT — list (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-046** Validate DV_CODED_TEXT — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-047** Validate DV_CODED_TEXT — local codes (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-048** Validate DV_CODED_TEXT — ext term (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-049** Validate DV_ORDINAL — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-050** Validate DV_ORDINAL — constraint (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-051** Validate DV_SCALE — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-052** Validate DV_SCALE — constraint (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-053** Validate DV_COUNT — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-054** Validate DV_COUNT — range (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-055** Validate DV_COUNT — list (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-056** Validate DV_QUANTITY — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-057** Validate DV_QUANTITY — property (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-058** Validate DV_QUANTITY — property units (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-059** Validate DV_QUANTITY — property units mag (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-060** Validate DV_PROPORTION — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-061** Validate DV_PROPORTION — ratio (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-062** Validate DV_PROPORTION — unitary (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-063** Validate DV_PROPORTION — percent (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-064** Validate DV_PROPORTION — fraction (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-065** Validate DV_PROPORTION — integer fraction (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-066** Validate DV_PROPORTION — any fraction (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-067** Validate DV_PROPORTION — ratio range (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-068** Validate DV_INTERVAL<DV_COUNT> — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-069** Validate DV_INTERVAL<DV_COUNT> — lower upper (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-070** Validate DV_INTERVAL<DV_COUNT> — lower upper list (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-071** Validate DV_INTERVAL<DV_QUANTITY> — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-072** Validate DV_INTERVAL<DV_QUANTITY> — upper lower (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-073** Validate DV_INTERVAL<DV_DATE_TIME> — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-074** Validate DV_INTERVAL<DV_DATE_TIME> — lower upper constraint (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-075** Validate DV_INTERVAL<DV_DATE_TIME> — lower upper range (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-076** Validate DV_INTERVAL<DV_DATE> — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-077** Validate DV_INTERVAL<DV_DATE> — lower upper constraint (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-078** Validate DV_INTERVAL<DV_DATE> — lower upper range (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-079** Validate DV_INTERVAL<DV_TIME> — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-080** Validate DV_INTERVAL<DV_TIME> — lower upper constraint (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-081** Validate DV_INTERVAL<DV_TIME> — lower upper range (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-082** Validate DV_INTERVAL<DV_DURATION> — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-083** Validate DV_INTERVAL<DV_DURATION> — constraint (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-084** Validate DV_INTERVAL<DV_DURATION> — range (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-085** Validate DV_INTERVAL<DV_ORDINAL> — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-086** Validate DV_INTERVAL<DV_ORDINAL> — constraint (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-087** Validate DV_INTERVAL<DV_SCALE> — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-088** Validate DV_INTERVAL<DV_SCALE> — constraint (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-089** Validate DV_INTERVAL<DV_PROPORTION> — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-090** Validate DV_INTERVAL<DV_PROPORTION> — ratio (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-091** Validate DV_INTERVAL<DV_PROPORTION> — unitary (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-092** Validate DV_INTERVAL<DV_PROPORTION> — percentage (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-093** Validate DV_INTERVAL<DV_PROPORTION> — fraction (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-094** Validate DV_INTERVAL<DV_PROPORTION> — integer fraction (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-095** Validate DV_INTERVAL<DV_PROPORTION> — ratio range (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-096** Validate DV_DURATION — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-097** Validate DV_DURATION — fields (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-098** Validate DV_DURATION — range (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-099** Validate DV_DURATION — fields range (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-100** Validate DV_TIME — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-101** Validate DV_TIME — constraint (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-102** Validate DV_TIME — range (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-103** Validate DV_DATE — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-104** Validate DV_DATE — constraint (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-105** Validate DV_DATE — range (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-119** Validate DV_DATE — day disallowed by C_DATE pattern (defective vendored fixture rejected) (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-106** Validate DV_DATE_TIME — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-107** Validate DV_DATE_TIME — constraint (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-108** Validate DV_DATE_TIME — range (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-109** Validate DV_PARSABLE — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-110** Validate DV_PARSABLE — value formalism (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-111** Validate DV_MULTIMEDIA — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-112** Validate DV_MULTIMEDIA — media type (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-113** Validate DV_URI — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-114** Validate DV_URI — pattern (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-115** Validate DV_URI — list (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-116** Validate DV_EHR_URI — open (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-117** Validate DV_EHR_URI — pattern (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-VAL-118** Validate DV_EHR_URI — list (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-DEM-001** Demographic person create (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person)
- **ECC-DEM-021** Demographic create bad body (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person)
- **ECC-DEM-002** Demographic person get (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person)
- **ECC-DEM-007** Demographic person get absent (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person/e4bdf2b1-4634-486d-84b4-76618fb156ee)
- **ECC-DEM-006** Demographic person get deleted (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person)
- **ECC-DEM-003** Demographic person get by version (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person)
- **ECC-DEM-025** Demographic person get at time (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person)
- **ECC-DEM-004** Demographic person update (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person)
- **ECC-DEM-008** Demographic person update bad if match (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person)
- **ECC-DEM-005** Demographic person delete (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person)
- **ECC-DEM-009** Demographic agent create (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/agent)
- **ECC-DEM-010** Demographic agent get (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/agent)
- **ECC-DEM-011** Demographic agent delete (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/agent)
- **ECC-DEM-012** Demographic group create (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/group)
- **ECC-DEM-013** Demographic group get (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/group)
- **ECC-DEM-014** Demographic group delete (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/group)
- **ECC-DEM-015** Demographic organisation create (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/organisation)
- **ECC-DEM-016** Demographic organisation get (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/organisation)
- **ECC-DEM-017** Demographic organisation delete (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/organisation)
- **ECC-DEM-018** Demographic role create (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/role)
- **ECC-DEM-019** Demographic role get (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/role)
- **ECC-DEM-020** Demographic role delete (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/role)
- **ECC-DEM-022** Demographic versioned party get (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person)
- **ECC-DEM-023** Demographic versioned party revision history (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person)
- **ECC-DEM-024** Demographic person tags (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person)
- **ECC-DEM-026** Demographic relationship create (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person)
- **ECC-DEM-027** Demographic relationship get (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person)
- **ECC-DEM-028** Demographic relationship get at time (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person)
- **ECC-DEM-029** Demographic relationship update (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person)
- **ECC-DEM-030** Demographic relationship delete (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person)
- **ECC-DEM-031** Demographic relationship get by version (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person)
- **ECC-ADM-001** Admin EHR delete (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-ADM-002** Admin EHR delete absent (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/admin/ehr/78dea3ac-728c-4507-ba72-214cc518bea6)
- **ECC-ADM-003** Admin EHR delete idempotent (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-ADM-004** Admin EHR delete all (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-ADM-005** Admin EHR delete all partial (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-ADM-006** Admin EHR delete all (empty selector) (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/admin/ehr/all)
- **ECC-SEC-001** Unauthenticated request to a protected route is refused (401) (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/46cc2676-9055-420f-a56f-6b8ceb59a73b)
- **ECC-SEC-002** Regular credential on an ADMIN-only route is forbidden (403) (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/admin/ehr/458388e4-4bb5-464b-aab9-80ad5579f2f7)
- **ECC-SIG-001** Version signing — digest present (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-SIG-001** Version signing — digest present (xml): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-SIG-002** Version signing — digest recomputes (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-SIG-003** Version signing — all kinds (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-SIG-004** Version signing — client verbatim (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-TS-001** TERMINOLOGY expand (bundle) — accepted, well-formed RESULT_SET (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-TS-002** TERMINOLOGY expand (bundle) — expansion constrains matches to the value set's codes (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-TS-003** TERMINOLOGY expand (bundle) — explicit code merged with the expansion (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-TS-004** TERMINOLOGY expand — unknown value set rejected (400) (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-TS-005** TERMINOLOGY expand — unknown service_api rejected (400) (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-TS-006** TERMINOLOGY expand (FHIR service_api) — accepted when a provider is configured (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/query/aql)
- **ECC-SF-001** FLAT commit then read-back as FLAT, canonical JSON, and STRUCTURED (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-SF-002** STRUCTURED commit then read-back as STRUCTURED, canonical JSON, and FLAT (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-SF-003** Accept q-values select the highest-weight simplified format; every non-204 carries Content-Type (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-SF-004** Deprecated + legacy simplified media types are rejected on Accept (406) (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-SF-005** Deprecated + legacy simplified media types are rejected on write Content-Type (415) (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr)
- **ECC-SF-006** FLAT commit without openehr-template-id (and no payload template id) → 422 (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-SF-007** FLAT commit with an unknown field identifier → 422 (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-SF-008** FLAT commit with |other combined with |code on one coded leaf → 422 (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-SF-009** GET a template as a Web Template document (Accept application/openehr.wt+json) (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-SF-010** GET a template example in each of the four Accept forms (json, xml, flat, structured) (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-SF-011** GET a template example with an unsupported Accept → 406 (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-SF-012** CONTRIBUTION with a FLAT COMPOSITION inner payload: canonical envelope in, simplified read-back (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)
- **ECC-SF-013** EHR_STATUS has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415 (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/501ad0e4-038d-40a4-a171-a7afff229c06/ehr_status)
- **ECC-SF-014** DIRECTORY (FOLDER) has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415 (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/ehr/6ba86876-5788-42b4-b5ef-6bfcc2cb6908/directory)
- **ECC-SF-015** Demographic PARTY has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415 (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/demographic/person/67afe7a8-6b34-43f5-a43d-3cad9ce59c30)
- **ECC-SF-016** FLAT ctx/time sets EVENT_CONTEXT.start_time; ctx/setting defaults to openehr::238 (json): transport: error sending request for url (http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4)

## 6. Skipped, by reason

| Reason | Cases |
|---|--:|
| NativeApiOnly: I_ADMIN_ARCHIVE.archive_ehrs is exercised by app/ehrbase/tests/service_admin.rs::archive_marks_vos_idempotently_and_reads_stay_unchanged — no ITS-REST admin route reaches it | 1 |
| NativeApiOnly: I_ADMIN_DUMP_LOAD.export_ehrs/load_ehrs is exercised by app/ehrbase/tests/service_dump_load.rs::export_then_load_into_fresh_db_round_trips_byte_equal — no ITS-REST admin route reaches it | 1 |
| NativeApiOnly: I_ADMIN_SERVICE.composition_version_count is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it | 1 |
| NativeApiOnly: I_ADMIN_SERVICE.contribution_count is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it | 1 |
| NativeApiOnly: I_ADMIN_SERVICE.list_contributions is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it | 1 |
| NativeApiOnly: I_ADMIN_SERVICE.versioned_composition_count is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehr_extracts is exercised by app/ehrbase/tests/service_extract.rs::export_ehr_extracts_honours_item_list_and_all_versions — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehrs (unknown EHR) is exercised by app/ehrbase/tests/service_extract.rs::export_ehrs_unknown_ehr_is_ehr_id_does_not_exist — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehrs is exercised by app/ehrbase/tests/service_extract.rs::export_ehrs_carries_every_versioned_object_latest_only — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr (duplicate id) is exercised by app/ehrbase/tests/service_import.rs::import_ehr_duplicate_target_is_rejected — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr (fixed id) is exercised by app/ehrbase/tests/service_import.rs::import_ehr_into_fixed_fresh_id — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr is exercised by app/ehrbase/tests/service_import.rs::import_ehr_clone_into_fresh_target_reuses_source_id — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr_extract is exercised by app/ehrbase/tests/service_import.rs::import_ehr_extract_adds_a_versioned_object_and_rejects_re_import — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_TDD_SERVICE.import_tdd (typed rejections) is exercised by app/ehrbase/tests/service_tdd.rs::{tdd_import_rejects_malformed_payload, tdd_import_rejects_non_tdd_xml, tdd_import_rejects_unknown_ehr, tdd_import_rejects_unknown_template} — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_TDD_SERVICE.import_tdd is exercised by app/ehrbase/tests/service_tdd.rs::tdd_import_commits_composition — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_TDD_SERVICE.import_tdds is exercised by app/ehrbase/tests/service_tdd.rs::{tdd_import_tdds_batch_commits_all, tdd_import_tdds_batch_fail_fast} — Messaging has no ITS-REST binding | 1 |
| NoRestBinding: I_ADMIN_ARCHIVE.archive_parties has no ITS-REST route and acts on the demographic extension; the archive path is proven natively by app/ehrbase/tests/service_admin.rs::archive_marks_vos_idempotently_and_reads_stay_unchanged | 1 |
| NoRestBinding: I_ADMIN_SERVICE.physical_party_delete has no ITS-REST route and acts on the demographic extension; exercised natively by app/ehrbase/tests/service_admin.rs::physical_party_delete_cascades_relationships_and_spares_partner | 1 |
| SutConfig: server not in `pgp` mode (needs a configured OpenPGP key); a pgp-keyed compose profile is a follow-up — the digest cases prove the Signing capability | 1 |
| SutConfig: the 5xx fault requires a fault-injecting terminology server wired to the SUT (--tx-server-url + an SUT FHIR provider pointed at it); the HTTP-only ECC cannot reconfigure an external SUT's provider per case. Harness tx server: http://127.0.0.1:55326 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_server_error_is_5xx + app/ehrbase/tests/terminology_fhir.rs::server_5xx_is_an_exception. | 1 |
| SutConfig: the malformed fault requires a fault-injecting terminology server wired to the SUT (--tx-server-url + an SUT FHIR provider pointed at it); the HTTP-only ECC cannot reconfigure an external SUT's provider per case. Harness tx server: http://127.0.0.1:55326 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_malformed_is_not_json + app/ehrbase/tests/terminology_fhir.rs::malformed_body_is_an_exception. | 1 |
| SutConfig: the timeout fault requires a fault-injecting terminology server wired to the SUT (--tx-server-url + an SUT FHIR provider pointed at it); the HTTP-only ECC cannot reconfigure an external SUT's provider per case. Harness tx server: http://127.0.0.1:55326 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_timeout_exceeds_a_short_client_deadline + app/ehrbase/tests/terminology_fhir.rs::timeout_is_an_exception. | 1 |
| all 11 C/loaded_db goldens are dialect-routed or require id-substitution/binds | 1 |
| master04 §delete_opt: SM I_DEFINITION_ADL14.delete_opt() has no ITS-REST ADL 1.4 binding — deletion lives in the ADMIN API only; a 405 here would be a schedule-vs-ITS-REST gap, not a server defect (register 01 G-5 / D2). The ADMIN template-deletion path is evidenced in the Admin area. | 4 |
| master05 §list_queries: SM I_DEFINITION_QUERY.list_queries() (bare collection) has no ITS-REST binding — Release-1.0.3 and development@e8a093e expose GET /definition/query/{qualified_query_name}, not a bare GET /definition/query. An edition exposing a bare-list resource would make this case live (register 02 G-2 edition probe). | 2 |
| master08 §list_contributions: the SM operation I_EHR_CONTRIBUTION.list_contributions() has no ITS-REST binding — /ehr/{ehr_id}/contribution is POST-only (no GET collection resource) in the tested development@e8a093e OAS and in Release-1.0.3; the list is a native-API concern, not wire-exercisable | 5 |

## 7. Not applicable to this SUT (extensions / RM-version-sensitive)

_None — every catalogued case applies to this SUT._

## 8. Edition findings (the SUT's discovered edition profile)

A case satisfied its normative core at a rung below the newest edition — recorded, never a silent pass (`master03-overview.adoc` §API Conformance; the aggregated findings feed the Conformance Statement's supported-versions field).

_None — every laddered assertion matched the newest edition form._

## 9. Coverage bounds (driven vs schedule data-set rows)

Cases whose driven data-set count is below the governing schedule table's row count — a bound is logged, never silent (honesty invariant 3; register 13 G-2). Widening the driven set is data, not a new case.

_No coverage bounds — every case drives its full schedule data set._

## 10. ECC-original cases (no direct schedule backing)

Stub-derived / extension cases — labelled here and **never presented as schedule-conformant** (register 08 G-1). Their result stands, but the claim is against our own derivation, not an abstract schedule test case.

- **ECC-EHR-012** Create EHR — reject invalid EHR_STATUS data sets — data-set class 2 (master06 §Test Data Sets, invalid EHR_STATUS shapes); no single master06 test case enumerates class 2
- **ECC-EHR-013** Create anonymous (subject-less) EHR — extension: Anonymous EHRs non-functional capability (master03-profiles §Non-Functional); doubles as class 1.b default-EHR_STATUS coverage; no master06 functional test case
- **ECC-TPL-017** Example COMPOSITION round-trips (ADL 1.4 example → commit) — CNF master04/master15 define no example-generation/commit case; the ITS-REST example operation is non-normative. ECC-derived: asserts the operation's own committable-`required` contract end-to-end (upload OPT → GET example → commit 201).
- **ECC-SQR-001** Store stored query — valid — schedule stub (master05 is TBD); derived from ITS-REST 1.0.3 DEFINITION QUERY + AQL 1.1 — I_DEFINITION_QUERY.valid_query-valid (master05:54, A.3.a)
- **ECC-SQR-007** Store stored query — invalid — schedule stub (master05 is TBD); derived from ITS-REST 1.0.3 DEFINITION QUERY + AQL 1.1 — I_DEFINITION_QUERY.valid_query-invalid (master05:67, A.3.b)
- **ECC-SQR-006** Store stored query — bad formalism — schedule stub (master05 is TBD); derived from ITS-REST 1.0.3 DEFINITION QUERY + AQL 1.1 — I_DEFINITION_QUERY.valid_query-bad_formalism (master05:80, A.3.c)
- **ECC-SQR-008** Stored query existence check — existing — schedule stub (master05 is TBD); derived from ITS-REST 1.0.3 DEFINITION QUERY + AQL 1.1 — I_DEFINITION_QUERY.has_query-xxx (master05:37, placeholder id; slug descriptivised per G-3)
- **ECC-SQR-002** List stored queries — non empty — schedule stub (master05 is TBD); derived from ITS-REST 1.0.3 DEFINITION QUERY (named list resource, D2 rebind) + AQL 1.1 — I_DEFINITION_QUERY.list_queries-non_empty (master05:110)
- **ECC-SQR-004** List stored queries — empty — schedule stub (master05 is TBD); derived from ITS-REST 1.0.3 DEFINITION QUERY + AQL 1.1 — I_DEFINITION_QUERY.list_queries-empty (master05:97)
- **ECC-SQR-005** List stored queries — select items — schedule stub (master05 is TBD); derived from ITS-REST 1.0.3 DEFINITION QUERY + AQL 1.1 — I_DEFINITION_QUERY.list_queries-select_items (master05:123)
- **ECC-QRY-001** Query service smoke test — I_QUERY_SERVICE.smoke_test (master11:48, stub xx flow)
- **ECC-QRY-002** Execute ad-hoc AQL query — empty db — I_QUERY_SERVICE.execute_ad_hoc_query-empty_db (master11:83, A.1.z, stub xx flow)
- **ECC-QRY-003** Execute stored AQL query — empty db — I_QUERY_SERVICE.execute_stored_query-empty_db (master11:61, stub xx flow)
- **ECC-QRY-004** Execute ad-hoc AQL query — loaded db — I_QUERY_SERVICE.execute_ad_hoc_query-loaded_db (master11:96, A.1.a, stub xx flow)
- **ECC-QRY-025** AQL uid projection — c/uid/value returns the version id — schedule stub (master11 is TBD); the loaded-db case asserts only the projected column path — this case asserts the projected CELL equals the committed OBJECT_VERSION_ID (a null cell was a real, otherwise-invisible engine defect)
- **ECC-QRY-005** AQL corpus — invalid queries rejected — schedule stub (master11 is TBD — no invalid-query case); AQL 1.1 negative-rejection evidence
- **ECC-QRY-014** AQL advanced — ORDER BY + LIMIT/OFFSET — schedule stub (master11 is TBD); AQL-advanced ORDER BY + LIMIT/OFFSET, profiles §AQL advanced OPTIONS
- **ECC-QRY-006** AQL corpus — A empty db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-007** AQL corpus — B empty db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-008** AQL corpus — C empty db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-009** AQL corpus — D empty db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-010** AQL corpus — A loaded db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-011** AQL corpus — B loaded db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-012** AQL corpus — C loaded db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-013** AQL corpus — D loaded db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-015** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); TIMEWINDOW golden, spec-supersedes-corpus (adjudications/ecc-own.toml)
- **ECC-QRY-016** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); TIMEWINDOW golden, spec-supersedes-corpus (adjudications/ecc-own.toml)
- **ECC-QRY-017** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); TIMEWINDOW golden, spec-supersedes-corpus (adjudications/ecc-own.toml)
- **ECC-QRY-018** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-QRY-019** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-QRY-020** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-QRY-021** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-QRY-022** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-QRY-023** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-QRY-024** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-VAL-119** Validate DV_DATE — day disallowed by C_DATE pattern (defective vendored fixture rejected) — no schedule case — ECC-authored negative guard for the corrected all_types fixture (§3, testdata/fixtures/REGISTER.md); the vendored all_types.composition.json carries a day-bearing DV_DATE at a leaf whose OPT C_DATE pattern disallows the day; a spec-correct validator must 422 it (archie is lenient)
- **ECC-DEM-001** Demographic person create — schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM
- **ECC-DEM-021** Demographic create bad body — schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM
- **ECC-DEM-002** Demographic person get — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-007** Demographic person get absent — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-006** Demographic person get deleted — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-003** Demographic person get by version — schedule stub (master10 §get_party_at_version TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party_at_version + RM common Versioning
- **ECC-DEM-025** Demographic person get at time — schedule stub (master10 §get_party_at_time TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party_at_time + RM common version_at_time
- **ECC-DEM-004** Demographic person update — schedule stub (master10 §update_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.update_party + RM Demographic IM
- **ECC-DEM-008** Demographic person update bad if match — schedule stub (master10 §update_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.update_party + RM Demographic IM
- **ECC-DEM-005** Demographic person delete — schedule stub (master10 §delete_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.delete_party + RM Demographic IM
- **ECC-DEM-009** Demographic agent create — schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM
- **ECC-DEM-010** Demographic agent get — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-011** Demographic agent delete — schedule stub (master10 §delete_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.delete_party + RM Demographic IM
- **ECC-DEM-012** Demographic group create — schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM
- **ECC-DEM-013** Demographic group get — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-014** Demographic group delete — schedule stub (master10 §delete_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.delete_party + RM Demographic IM
- **ECC-DEM-015** Demographic organisation create — schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM
- **ECC-DEM-016** Demographic organisation get — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-017** Demographic organisation delete — schedule stub (master10 §delete_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.delete_party + RM Demographic IM
- **ECC-DEM-018** Demographic role create — schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM
- **ECC-DEM-019** Demographic role get — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-020** Demographic role delete — schedule stub (master10 §delete_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.delete_party + RM Demographic IM
- **ECC-DEM-022** Demographic versioned party get — extension: VERSIONED_PARTY read (RM common Versioning); no master10 SM operation
- **ECC-DEM-023** Demographic versioned party revision history — extension: REVISION_HISTORY read (RM common Versioning); no master10 SM operation
- **ECC-DEM-024** Demographic person tags — extension: item tags — no openEHR spec governs item tags
- **ECC-DEM-026** Demographic relationship create — schedule stub (master10 §create_party_relationship TBD); derived from SM I_PARTY_RELATIONSHIP + RM PARTY_RELATIONSHIP — ehrbase-rs extension wire
- **ECC-DEM-027** Demographic relationship get — schedule stub (master10 §get_party_relationship TBD); derived from SM I_PARTY_RELATIONSHIP + RM PARTY_RELATIONSHIP — ehrbase-rs extension wire
- **ECC-DEM-028** Demographic relationship get at time — schedule stub (master10 §get_party_relationship_at_time TBD); derived from SM I_PARTY_RELATIONSHIP + RM common version_at_time — ehrbase-rs extension wire
- **ECC-DEM-029** Demographic relationship update — schedule stub (master10 §update_party_relationship TBD); derived from SM I_PARTY_RELATIONSHIP — ehrbase-rs extension wire
- **ECC-DEM-030** Demographic relationship delete — schedule stub (master10 §delete_party_relationship TBD); derived from SM I_PARTY_RELATIONSHIP — ehrbase-rs extension wire
- **ECC-DEM-031** Demographic relationship get by version — schedule stub (master10 §get_party_relationship_at_version TBD); derived from SM I_PARTY_RELATIONSHIP + RM common OBJECT_VERSION_ID — ehrbase-rs extension wire
- **ECC-ADM-001** Admin EHR delete — schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml
- **ECC-ADM-002** Admin EHR delete absent — schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml
- **ECC-ADM-003** Admin EHR delete idempotent — schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml
- **ECC-ADM-004** Admin EHR delete all — schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml
- **ECC-ADM-005** Admin EHR delete all partial — schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml
- **ECC-ADM-006** Admin EHR delete all (empty selector) — schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml
- **ECC-ADM-007** Admin list contributions — schedule stub (master12 §list_contributions TBD); derived from SM I_ADMIN_SERVICE.list_contributions — native-API-only
- **ECC-ADM-008** Admin contribution count — schedule stub (master12 §contribution_count TBD); derived from SM I_ADMIN_SERVICE.contribution_count — native-API-only
- **ECC-ADM-009** Admin versioned composition count — schedule stub (master12 §versioned_composition_count TBD); derived from SM I_ADMIN_SERVICE.versioned_composition_count — native-API-only
- **ECC-ADM-010** Admin composition version count — schedule stub (master12 §composition_version_count TBD); derived from SM I_ADMIN_SERVICE.composition_version_count — native-API-only
- **ECC-ADM-011** Admin export EHRs (dump/load) — schedule stub (master12 §export_ehrs TBD); derived from SM I_ADMIN_DUMP_LOAD.export_ehrs — native-API-only
- **ECC-ADM-012** Admin archive EHRs — schedule stub (master12 §archive_ehrs TBD); derived from SM I_ADMIN_ARCHIVE.archive_ehrs — native-API-only
- **ECC-ADM-013** Admin physical party delete — schedule stub (master12 §physical_party_delete TBD); derived from SM I_ADMIN_SERVICE.physical_party_delete — demographic-dependent, no ITS-REST binding
- **ECC-ADM-014** Admin archive parties — schedule stub (master12 §archive_parties TBD); derived from SM I_ADMIN_ARCHIVE.archive_parties — demographic-dependent, no ITS-REST binding
- **ECC-MSG-001** EHR Extract — export whole EHR (export_ehrs) — schedule stub (master13 §I_EHR_EXTRACT.export_ehr TBD, listed twice — authoring duplicate); derived from SM I_EHR_EXTRACT_SERVICE.export_ehrs
- **ECC-MSG-002** EHR Extract — spec-driven export (export_ehr_extracts) — schedule stub (master13 §I_EHR_EXTRACT.export_ehr_extracts TBD); derived from SM I_EHR_EXTRACT_SERVICE.export_ehr_extracts
- **ECC-MSG-003** EHR Extract — export of unknown EHR fails — schedule stub (master13 §I_EHR_EXTRACT.export_ehr TBD); derived from SM I_EHR_EXTRACT_SERVICE.export_ehrs (unknown EHR)
- **ECC-MSG-004** EHR Extract — import whole-EHR clone reusing source id — schedule silent on import (master13 lists export + TDD only); RM common master06 §Copying Case 1; derived from SM I_EHR_EXTRACT_SERVICE.import_ehr
- **ECC-MSG-005** EHR Extract — import whole EHR into a caller-fixed id — schedule silent on import (master13 lists export + TDD only); RM common master06 §Copying; derived from SM I_EHR_EXTRACT_SERVICE.import_ehr
- **ECC-MSG-006** EHR Extract — import into a duplicate target id fails — schedule silent on import (master13 lists export + TDD only); RM common master06 §Copying; derived from SM I_EHR_EXTRACT_SERVICE.import_ehr (duplicate target)
- **ECC-MSG-007** EHR Extract — import extract into an existing EHR — schedule silent on import (master13 lists export + TDD only); RM common master06 §Copying Case 2; derived from SM I_EHR_EXTRACT_SERVICE.import_ehr_extract
- **ECC-MSG-008** TDD — import a TDD as a committed COMPOSITION — schedule stub (master13 §I_TDD.import_tdd TBD); derived from SM I_TDD_SERVICE.import_tdd
- **ECC-MSG-009** TDD — import rejects malformed / non-TDD / unknown EHR / unknown template — schedule stub (master13 §I_TDD.import_tdd TBD); derived from SM I_TDD_SERVICE.import_tdd (typed rejections)
- **ECC-MSG-010** TDD — batch import commits all, fail-fast on error — schedule stub (master13 §I_TDD.import_tdds TBD); derived from SM I_TDD_SERVICE.import_tdds
- **ECC-SEC-001** Unauthenticated request to a protected route is refused (401) — no CNF schedule chapter for authentication (out of band per SM master02); Robot SECURITY_TESTS/I_OAuth2_Keycloak §06 'API endpoints are secured' intent, reproduced over Basic auth
- **ECC-SEC-002** Regular credential on an ADMIN-only route is forbidden (403) — no CNF schedule chapter for authorization (out of band per SM master02); Robot SECURITY_TESTS/I_OAuth2_Keycloak role-distinction intent, reproduced over Basic auth
- **ECC-SIG-001** Version signing — digest present — extension: VERSION.signature is an ehrbase-rs feature (no openEHR spec governs the digest algorithm); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot
- **ECC-SIG-002** Version signing — digest recomputes — extension: sha256: digest recompute is an ehrbase-rs feature (RFC 8785 canonical form); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot
- **ECC-SIG-003** Version signing — all kinds — extension: version signing rides every versioned-object write (ehrbase-rs); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot
- **ECC-SIG-004** Version signing — client verbatim — extension: client-supplied signatures win (ehrbase-rs); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot
- **ECC-SIG-005** Version signing — pgp verifies — extension: pgp signing mode is an ehrbase-rs feature (RFC 4880); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot
- **ECC-TS-001** TERMINOLOGY expand (bundle) — accepted, well-formed RESULT_SET — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the 'openehr' bundle flavour is an ehrbase-rs AQL engine extension
- **ECC-TS-002** TERMINOLOGY expand (bundle) — expansion constrains matches to the value set's codes — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the 'openehr' bundle flavour is an ehrbase-rs AQL engine extension
- **ECC-TS-003** TERMINOLOGY expand (bundle) — explicit code merged with the expansion — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the 'openehr' bundle flavour is an ehrbase-rs AQL engine extension
- **ECC-TS-004** TERMINOLOGY expand — unknown value set rejected (400) — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the 'openehr' bundle flavour is an ehrbase-rs AQL engine extension
- **ECC-TS-005** TERMINOLOGY expand — unknown service_api rejected (400) — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the 'openehr' bundle flavour is an ehrbase-rs AQL engine extension
- **ECC-TS-006** TERMINOLOGY expand (FHIR service_api) — accepted when a provider is configured — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the FHIR service_api path realizes the spec mechanism (generic, not an extension)
- **ECC-TS-007** TERMINOLOGY expand (FHIR) — terminology-server timeout is a server fault (500) — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS); a fault-injecting tx cannot be wired into an external SUT over the HTTP-only ECC — fault→500 proven off-wire (MSG precedent)
- **ECC-TS-008** TERMINOLOGY expand (FHIR) — terminology-server 5xx is a server fault (500) — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS); a fault-injecting tx cannot be wired into an external SUT over the HTTP-only ECC — fault→500 proven off-wire (MSG precedent)
- **ECC-TS-009** TERMINOLOGY expand (FHIR) — malformed terminology response is a server fault (500) — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS); a fault-injecting tx cannot be wired into an external SUT over the HTTP-only ECC — fault→500 proven off-wire (MSG precedent)
- **ECC-SF-001** FLAT commit then read-back as FLAT, canonical JSON, and STRUCTURED — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-002** STRUCTURED commit then read-back as STRUCTURED, canonical JSON, and FLAT — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-003** Accept q-values select the highest-weight simplified format; every non-204 carries Content-Type — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-004** Deprecated + legacy simplified media types are rejected on Accept (406) — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-005** Deprecated + legacy simplified media types are rejected on write Content-Type (415) — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-006** FLAT commit without openehr-template-id (and no payload template id) → 422 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-007** FLAT commit with an unknown field identifier → 422 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-008** FLAT commit with |other combined with |code on one coded leaf → 422 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-009** GET a template as a Web Template document (Accept application/openehr.wt+json) — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-010** GET a template example in each of the four Accept forms (json, xml, flat, structured) — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-011** GET a template example with an unsupported Accept → 406 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-012** CONTRIBUTION with a FLAT COMPOSITION inner payload: canonical envelope in, simplified read-back — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-013** EHR_STATUS has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-014** DIRECTORY (FOLDER) has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-015** Demographic PARTY has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-016** FLAT ctx/time sets EVENT_CONTEXT.start_time; ctx/setting defaults to openehr::238 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)

## 11. Detailed test report

| ECC id | Capability | Format | Data sets | Rung | Result |
|---|---|---|--:|---|---|
| ECC-EHR-001 | EhrOperations | json | 0/0 | — | ERROR |
| ECC-EHR-002 | EhrOperations | json | 0/0 | — | ERROR |
| ECC-EHR-003 | EhrOperations | json | 0/0 | — | ERROR |
| ECC-EHR-004 | EhrOperations | json | 0/0 | — | ERROR |
| ECC-EHR-005 | EhrOperations | json | 0/0 | — | ERROR |
| ECC-EHR-006 | EhrOperations | json | 0/0 | — | ERROR |
| ECC-EHR-007 | EhrOperations | json | 0/0 | — | ERROR |
| ECC-EHR-008 | EhrOperations | json | 0/0 | — | ERROR |
| ECC-EHR-009 | EhrOperations | json | 0/0 | — | ERROR |
| ECC-EHR-010 | EhrOperations | json | 0/0 | — | ERROR |
| ECC-EHR-011 | EhrOperations | json | 0/0 | — | ERROR |
| ECC-STA-001 | EhrStatus | json | 0/0 | — | ERROR |
| ECC-STA-002 | EhrStatus | json | 0/0 | — | ERROR |
| ECC-STA-003 | EhrStatus | json | 0/0 | — | ERROR |
| ECC-STA-004 | EhrStatus | json | 0/0 | — | ERROR |
| ECC-STA-005 | EhrStatus | json | 0/0 | — | ERROR |
| ECC-STA-006 | EhrStatus | json | 0/0 | — | ERROR |
| ECC-STA-007 | EhrStatus | json | 0/0 | — | ERROR |
| ECC-STA-008 | EhrStatus | json | 0/0 | — | ERROR |
| ECC-STA-009 | EhrStatus | json | 0/0 | — | ERROR |
| ECC-STA-010 | EhrStatus | json | 0/0 | — | ERROR |
| ECC-EHR-012 | EhrOperations | json | 0/0 | — | ERROR |
| ECC-EHR-013 | AnonymousEhrs | json | 0/0 | — | ERROR |
| ECC-COM-001 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-001 | CompositionOps | xml | 0/0 | — | ERROR |
| ECC-COM-002 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-002 | CompositionOps | xml | 0/0 | — | ERROR |
| ECC-COM-003 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-004 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-005 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-006 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-007 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-032 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-011 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-012 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-008 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-008 | CompositionOps | xml | 0/0 | — | ERROR |
| ECC-COM-009 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-010 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-013 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-013 | CompositionOps | xml | 0/0 | — | ERROR |
| ECC-COM-014 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-014 | CompositionOps | xml | 0/0 | — | ERROR |
| ECC-COM-015 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-016 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-017 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-018 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-018 | CompositionOps | xml | 0/0 | — | ERROR |
| ECC-COM-019 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-020 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-021 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-022 | Versioning | json | 0/0 | — | ERROR |
| ECC-COM-022 | Versioning | xml | 0/0 | — | ERROR |
| ECC-COM-023 | Versioning | json | 0/0 | — | ERROR |
| ECC-COM-024 | Versioning | json | 0/0 | — | ERROR |
| ECC-COM-025 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-026 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-027 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-028 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-029 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-030 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-COM-031 | CompositionOps | json | 0/0 | — | ERROR |
| ECC-CTB-001 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-002 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-003 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-004 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-005 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-006 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-007 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-008 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-009 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-010 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-011 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-012 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-013 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-014 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-015 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-016 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-017 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-018 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-019 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-020 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-021 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-022 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-023 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-024 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-025 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-026 | ChangeSets | json | 0/0 | — | ERROR |
| ECC-CTB-027 | ChangeSets | json | 0/0 | — | skipped |
| ECC-CTB-028 | ChangeSets | json | 0/0 | — | skipped |
| ECC-CTB-029 | ChangeSets | json | 0/0 | — | skipped |
| ECC-CTB-030 | ChangeSets | json | 0/0 | — | skipped |
| ECC-CTB-031 | ChangeSets | json | 0/0 | — | skipped |
| ECC-DIR-012 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-013 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-014 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-015 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-016 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-017 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-018 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-001 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-002 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-003 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-022 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-004 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-023 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-005 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-006 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-007 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-008 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-009 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-010 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-011 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-019 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-020 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-021 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-024 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-025 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-026 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-027 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-028 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-029 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-030 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-031 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-032 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-033 | Versioning | json | 0/0 | — | ERROR |
| ECC-DIR-034 | Versioning | json | 0/0 | — | ERROR |
| ECC-DIR-035 | Versioning | json | 0/0 | — | ERROR |
| ECC-DIR-036 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-DIR-037 | DirectoryOps | json | 0/0 | — | ERROR |
| ECC-TPL-011 | Adl14OptProvisioning | json | 0/0 | — | ERROR |
| ECC-TPL-012 | Adl14OptProvisioning | json | 0/0 | — | ERROR |
| ECC-TPL-001 | Adl14ArchetypeProvisioning | json | 0/0 | — | ERROR |
| ECC-TPL-002 | Adl14OptProvisioning | json | 0/0 | — | ERROR |
| ECC-TPL-004 | Adl14OptProvisioning | json | 0/0 | — | ERROR |
| ECC-TPL-005 | Adl14OptProvisioning | json | 0/0 | — | ERROR |
| ECC-TPL-006 | Adl14OptProvisioning | json | 0/0 | — | ERROR |
| ECC-TPL-009 | Adl14OptProvisioning | json | 0/0 | — | ERROR |
| ECC-TPL-007 | Adl14OptProvisioning | json | 0/0 | — | ERROR |
| ECC-TPL-008 | Adl14OptProvisioning | json | 0/0 | — | ERROR |
| ECC-TPL-010 | Adl14OptProvisioning | json | 0/0 | — | ERROR |
| ECC-TPL-003 | Adl14OptProvisioning | json | 0/0 | — | ERROR |
| ECC-TPL-017 | Adl14OptProvisioning | json | 0/0 | — | ERROR |
| ECC-TPL-014 | Adl14OptProvisioning | json | 0/0 | — | skipped |
| ECC-TPL-015 | Adl14OptProvisioning | json | 0/0 | — | skipped |
| ECC-TPL-016 | Adl14OptProvisioning | json | 0/0 | — | skipped |
| ECC-TPL-013 | Adl14OptProvisioning | json | 0/0 | — | skipped |
| ECC-SQR-001 | QueryProvisioning | json | 0/0 | — | ERROR |
| ECC-SQR-007 | QueryProvisioning | json | 0/0 | — | ERROR |
| ECC-SQR-006 | QueryProvisioning | json | 0/0 | — | ERROR |
| ECC-SQR-008 | QueryProvisioning | json | 0/0 | — | ERROR |
| ECC-SQR-002 | QueryProvisioning | json | 0/0 | — | ERROR |
| ECC-SQR-004 | QueryProvisioning | json | 0/0 | — | skipped |
| ECC-SQR-005 | QueryProvisioning | json | 0/0 | — | skipped |
| ECC-QRY-001 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-002 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-003 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-004 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-025 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-005 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-014 | AqlAdvanced | json | 0/0 | — | ERROR |
| ECC-QRY-006 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-007 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-008 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-009 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-010 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-011 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-012 | AqlBasic | json | 0/0 | — | skipped |
| ECC-QRY-013 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-015 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-016 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-017 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-018 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-019 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-020 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-021 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-022 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-023 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-QRY-024 | AqlBasic | json | 0/0 | — | ERROR |
| ECC-VAL-001 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-002 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-003 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-004 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-005 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-006 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-007 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-008 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-009 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-010 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-011 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-012 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-013 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-014 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-015 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-016 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-017 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-018 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-019 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-020 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-021 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-022 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-023 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-024 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-025 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-026 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-027 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-028 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-029 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-030 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-031 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-032 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-033 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-034 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-035 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-036 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-037 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-038 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-039 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-040 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-041 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-042 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-043 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-044 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-045 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-046 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-047 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-048 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-049 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-050 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-051 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-052 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-053 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-054 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-055 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-056 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-057 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-058 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-059 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-060 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-061 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-062 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-063 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-064 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-065 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-066 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-067 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-068 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-069 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-070 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-071 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-072 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-073 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-074 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-075 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-076 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-077 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-078 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-079 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-080 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-081 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-082 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-083 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-084 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-085 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-086 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-087 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-088 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-089 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-090 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-091 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-092 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-093 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-094 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-095 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-096 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-097 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-098 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-099 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-100 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-101 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-102 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-103 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-104 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-105 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-119 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-106 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-107 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-108 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-109 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-110 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-111 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-112 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-113 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-114 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-115 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-116 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-117 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-VAL-118 | ArchetypeValidation | json | 0/0 | — | ERROR |
| ECC-DEM-001 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-021 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-002 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-007 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-006 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-003 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-025 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-004 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-008 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-005 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-009 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-010 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-011 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-012 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-013 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-014 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-015 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-016 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-017 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-018 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-019 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-020 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-022 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-023 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-024 | PartyOperations | json | 0/0 | — | ERROR |
| ECC-DEM-026 | PartyRelationshipOperations | json | 0/0 | — | ERROR |
| ECC-DEM-027 | PartyRelationshipOperations | json | 0/0 | — | ERROR |
| ECC-DEM-028 | PartyRelationshipOperations | json | 0/0 | — | ERROR |
| ECC-DEM-029 | PartyRelationshipOperations | json | 0/0 | — | ERROR |
| ECC-DEM-030 | PartyRelationshipOperations | json | 0/0 | — | ERROR |
| ECC-DEM-031 | PartyRelationshipOperations | json | 0/0 | — | ERROR |
| ECC-ADM-001 | AdminPhysicalDeletion | json | 0/0 | — | ERROR |
| ECC-ADM-002 | AdminPhysicalDeletion | json | 0/0 | — | ERROR |
| ECC-ADM-003 | AdminPhysicalDeletion | json | 0/0 | — | ERROR |
| ECC-ADM-004 | AdminPhysicalDeletion | json | 0/0 | — | ERROR |
| ECC-ADM-005 | AdminPhysicalDeletion | json | 0/0 | — | ERROR |
| ECC-ADM-006 | AdminPhysicalDeletion | json | 0/0 | — | ERROR |
| ECC-ADM-007 | AdminActivityReport | json | 0/0 | — | skipped |
| ECC-ADM-008 | AdminActivityReport | json | 0/0 | — | skipped |
| ECC-ADM-009 | AdminActivityReport | json | 0/0 | — | skipped |
| ECC-ADM-010 | AdminActivityReport | json | 0/0 | — | skipped |
| ECC-ADM-011 | AdminEhrDumpLoad | json | 0/0 | — | skipped |
| ECC-ADM-012 | AdminEhrArchive | json | 0/0 | — | skipped |
| ECC-ADM-013 | AdminPhysicalDeletion | json | 0/0 | — | skipped |
| ECC-ADM-014 | AdminDemographicArchive | json | 0/0 | — | skipped |
| ECC-MSG-001 | MessagingEhrExtract | json | 0/0 | — | skipped |
| ECC-MSG-002 | MessagingEhrExtract | json | 0/0 | — | skipped |
| ECC-MSG-003 | MessagingEhrExtract | json | 0/0 | — | skipped |
| ECC-MSG-004 | MessagingEhrExtract | json | 0/0 | — | skipped |
| ECC-MSG-005 | MessagingEhrExtract | json | 0/0 | — | skipped |
| ECC-MSG-006 | MessagingEhrExtract | json | 0/0 | — | skipped |
| ECC-MSG-007 | MessagingEhrExtract | json | 0/0 | — | skipped |
| ECC-MSG-008 | MessagingTds | json | 0/0 | — | skipped |
| ECC-MSG-009 | MessagingTds | json | 0/0 | — | skipped |
| ECC-MSG-010 | MessagingTds | json | 0/0 | — | skipped |
| ECC-SEC-001 | Authentication | json | 0/0 | — | ERROR |
| ECC-SEC-002 | Authentication | json | 0/0 | — | ERROR |
| ECC-SIG-001 | Signing | json | 0/0 | — | ERROR |
| ECC-SIG-001 | Signing | xml | 0/0 | — | ERROR |
| ECC-SIG-002 | Signing | json | 0/0 | — | ERROR |
| ECC-SIG-003 | Signing | json | 0/0 | — | ERROR |
| ECC-SIG-004 | Signing | json | 0/0 | — | ERROR |
| ECC-SIG-005 | Signing | json | 0/0 | — | skipped |
| ECC-TS-001 | Terminology | json | 0/0 | — | ERROR |
| ECC-TS-002 | Terminology | json | 0/0 | — | ERROR |
| ECC-TS-003 | Terminology | json | 0/0 | — | ERROR |
| ECC-TS-004 | Terminology | json | 0/0 | — | ERROR |
| ECC-TS-005 | Terminology | json | 0/0 | — | ERROR |
| ECC-TS-006 | Terminology | json | 0/0 | — | ERROR |
| ECC-TS-007 | Terminology | json | 0/0 | — | skipped |
| ECC-TS-008 | Terminology | json | 0/0 | — | skipped |
| ECC-TS-009 | Terminology | json | 0/0 | — | skipped |
| ECC-SF-001 | SimplifiedFormats | json | 0/0 | — | ERROR |
| ECC-SF-002 | SimplifiedFormats | json | 0/0 | — | ERROR |
| ECC-SF-003 | SimplifiedFormats | json | 0/0 | — | ERROR |
| ECC-SF-004 | SimplifiedFormats | json | 0/0 | — | ERROR |
| ECC-SF-005 | SimplifiedFormats | json | 0/0 | — | ERROR |
| ECC-SF-006 | SimplifiedFormats | json | 0/0 | — | ERROR |
| ECC-SF-007 | SimplifiedFormats | json | 0/0 | — | ERROR |
| ECC-SF-008 | SimplifiedFormats | json | 0/0 | — | ERROR |
| ECC-SF-009 | SimplifiedFormats | json | 0/0 | — | ERROR |
| ECC-SF-010 | SimplifiedFormats | json | 0/0 | — | ERROR |
| ECC-SF-011 | SimplifiedFormats | json | 0/0 | — | ERROR |
| ECC-SF-012 | SimplifiedFormats | json | 0/0 | — | ERROR |
| ECC-SF-013 | SimplifiedFormats | json | 0/0 | — | ERROR |
| ECC-SF-014 | SimplifiedFormats | json | 0/0 | — | ERROR |
| ECC-SF-015 | SimplifiedFormats | json | 0/0 | — | ERROR |
| ECC-SF-016 | SimplifiedFormats | json | 0/0 | — | ERROR |

## 12. Terminology server (TS area)

- Server: `http://127.0.0.1:55326`
- Mode: fixture

Recorded FHIR-tx exchange (4 request(s)):

| # | Method | Path | Query |
|--:|---|---|---|
| 1 | GET | `/ValueSet/$expand` | url=http%3A%2F%2Fhl7.org%2Ffhir%2FValueSet%2Fsurface |
| 2 | GET | `/ValueSet/$validate-code` | url=http%3A%2F%2Fhl7.org%2Ffhir%2FValueSet%2Fsurface&code=B |
| 3 | GET | `/CodeSystem/$lookup` | code=B |
| 4 | GET | `/CodeSystem/$subsumes` | codeA=L&codeB=O |
