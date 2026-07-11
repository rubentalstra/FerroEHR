# ehrbase-rs Conformance Report (generated)

> Generated from a conformance run — never hand-asserted. Scoped and
> honest: the deviations section lists every skip with its reason.

## 1. SUT identity

- Product: ehrbase-rs 3.0.0
- SUT: `http://localhost:8080/ehrbase/rest/openehr/v1`
- Spec versions: RM 1.2.0 · ITS-REST development@e8a093e · AQL 1.1.0 · TERM 3.1.0
- Auth mode: basic
- Started: 2026-07-11T10:45:03.644171Z

**341 case×format executions · 315 passed · 0 failed · 0 not applicable.**

### Per-area matrix

| Area | Catalogue (active) | Passed | Failed | Errored | Skipped | N/A |
|---|--:|--:|--:|--:|--:|--:|
| EHR — EHR service | 13 | 13 | 0 | 0 | 0 | 0 |
| STA — EHR_STATUS | 10 | 10 | 0 | 0 | 0 | 0 |
| COM — COMPOSITION | 31 | 38 | 0 | 0 | 0 | 0 |
| CTB — CONTRIBUTION (change sets) | 31 | 26 | 0 | 0 | 5 | 0 |
| DIR — DIRECTORY (FOLDER) | 37 | 37 | 0 | 0 | 0 | 0 |
| TPL — Template / OPT provisioning | 16 | 12 | 0 | 0 | 4 | 0 |
| SQR — Stored-query provisioning | 7 | 5 | 0 | 0 | 2 | 0 |
| QRY — AQL execution | 13 | 13 | 0 | 0 | 0 | 0 |
| VAL — Content / archetype validation | 119 | 119 | 0 | 0 | 0 | 0 |
| DEM — Demographic service | 24 | 24 | 0 | 0 | 0 | 0 |
| ADM — Admin service | 6 | 6 | 0 | 0 | 0 | 0 |
| SEC — Security / authorization | 2 | 2 | 0 | 0 | 0 | 0 |
| SIG — Version signing | 5 | 5 | 0 | 0 | 1 | 0 |
| MSG — Messaging | 10 | 0 | 0 | 0 | 10 | 0 |
| TS — Terminology-server integration | 9 | 5 | 0 | 0 | 4 | 0 |

### Failures

_No failures in this run._

### Not applicable to this SUT (extensions / RM-version-sensitive)

_None — every catalogued case applies to this SUT._

## 2. Scope of test

| Field | Value |
|---|---|
| Profiles requested | all |
| Data formats | json, xml |
| Catalogue (active cases) | 333 |
| Executed | 341 |
| Passed | 315 |
| Failed | 0 |
| Not applicable | 0 |

## 3. Detailed test report

| ECC id | Capability | Format | Data sets | Result |
|---|---|---|--:|---|
| ECC-EHR-001 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-002 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-003 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-004 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-005 | EhrOperations | json | 16/16 | PASS |
| ECC-EHR-006 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-007 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-008 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-009 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-010 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-011 | EhrOperations | json | 1/1 | PASS |
| ECC-STA-001 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-002 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-003 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-004 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-005 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-006 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-007 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-008 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-009 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-010 | EhrStatus | json | 1/1 | PASS |
| ECC-EHR-012 | EhrOperations | json | 11/11 | PASS |
| ECC-EHR-013 | AnonymousEhrs | json | 1/1 | PASS |
| ECC-COM-001 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-001 | CompositionOps | xml | 1/1 | PASS |
| ECC-COM-002 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-002 | CompositionOps | xml | 1/1 | PASS |
| ECC-COM-003 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-004 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-005 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-006 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-007 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-008 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-008 | CompositionOps | xml | 1/1 | PASS |
| ECC-COM-009 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-010 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-011 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-012 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-013 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-013 | CompositionOps | xml | 1/1 | PASS |
| ECC-COM-014 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-014 | CompositionOps | xml | 1/1 | PASS |
| ECC-COM-015 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-016 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-017 | CompositionOps | json | 3/3 | PASS |
| ECC-COM-018 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-018 | CompositionOps | xml | 1/1 | PASS |
| ECC-COM-019 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-020 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-021 | CompositionOps | json | 2/2 | PASS |
| ECC-COM-022 | Versioning | json | 1/1 | PASS |
| ECC-COM-022 | Versioning | xml | 1/1 | PASS |
| ECC-COM-023 | Versioning | json | 1/1 | PASS |
| ECC-COM-024 | Versioning | json | 1/1 | PASS |
| ECC-COM-025 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-026 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-027 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-028 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-029 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-030 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-031 | CompositionOps | json | 1/1 | PASS |
| ECC-CTB-001 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-002 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-003 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-004 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-005 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-006 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-007 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-008 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-009 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-010 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-011 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-012 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-013 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-014 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-015 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-016 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-017 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-018 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-019 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-020 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-021 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-022 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-023 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-024 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-025 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-026 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-027 | ChangeSets | json | 0/0 | skipped |
| ECC-CTB-028 | ChangeSets | json | 0/0 | skipped |
| ECC-CTB-029 | ChangeSets | json | 0/0 | skipped |
| ECC-CTB-030 | ChangeSets | json | 0/0 | skipped |
| ECC-CTB-031 | ChangeSets | json | 0/0 | skipped |
| ECC-DIR-001 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-002 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-003 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-004 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-005 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-006 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-007 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-008 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-009 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-010 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-011 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-012 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-013 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-014 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-015 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-016 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-017 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-018 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-019 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-020 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-021 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-022 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-023 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-024 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-025 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-026 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-027 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-028 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-029 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-030 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-031 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-032 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-033 | Versioning | json | 1/1 | PASS |
| ECC-DIR-034 | Versioning | json | 1/1 | PASS |
| ECC-DIR-035 | Versioning | json | 1/1 | PASS |
| ECC-DIR-036 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-037 | DirectoryOps | json | 1/1 | PASS |
| ECC-TPL-001 | Adl14ArchetypeProvisioning | json | 1/1 | PASS |
| ECC-TPL-002 | Adl14OptProvisioning | json | 18/18 | PASS |
| ECC-TPL-003 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-004 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-005 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-006 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-007 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-008 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-009 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-010 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-011 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-012 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-013 | Adl14OptProvisioning | json | 0/0 | skipped |
| ECC-TPL-014 | Adl14OptProvisioning | json | 0/0 | skipped |
| ECC-TPL-015 | Adl14OptProvisioning | json | 0/0 | skipped |
| ECC-TPL-016 | Adl14OptProvisioning | json | 0/0 | skipped |
| ECC-SQR-001 | QueryProvisioning | json | 1/1 | PASS |
| ECC-SQR-002 | QueryProvisioning | json | 1/1 | PASS |
| ECC-SQR-003 | QueryProvisioning | json | 1/1 | PASS |
| ECC-SQR-004 | QueryProvisioning | json | 0/0 | skipped |
| ECC-SQR-005 | QueryProvisioning | json | 0/0 | skipped |
| ECC-SQR-006 | QueryProvisioning | json | 1/1 | PASS |
| ECC-SQR-007 | QueryProvisioning | json | 1/1 | PASS |
| ECC-QRY-001 | AqlBasic | json | 1/1 | PASS |
| ECC-QRY-002 | AqlBasic | json | 1/1 | PASS |
| ECC-QRY-003 | AqlBasic | json | 1/1 | PASS |
| ECC-QRY-004 | AqlBasic | json | 1/1 | PASS |
| ECC-QRY-005 | AqlBasic | json | 2/2 | PASS |
| ECC-QRY-006 | AqlBasic | json | 25/25 | PASS |
| ECC-QRY-007 | AqlBasic | json | 18/18 | PASS |
| ECC-QRY-008 | AqlBasic | json | 11/11 | PASS |
| ECC-QRY-009 | AqlBasic | json | 16/16 | PASS |
| ECC-QRY-010 | AqlBasic | json | 21/21 | PASS |
| ECC-QRY-011 | AqlBasic | json | 15/15 | PASS |
| ECC-QRY-012 | AqlBasic | json | 1/1 | PASS |
| ECC-QRY-013 | AqlBasic | json | 7/7 | PASS |
| ECC-ADM-001 | AdminApi | json | 1/1 | PASS |
| ECC-ADM-002 | AdminApi | json | 1/1 | PASS |
| ECC-ADM-003 | AdminApi | json | 1/1 | PASS |
| ECC-ADM-004 | AdminApi | json | 1/1 | PASS |
| ECC-ADM-005 | AdminApi | json | 1/1 | PASS |
| ECC-ADM-006 | AdminApi | json | 1/1 | PASS |
| ECC-DEM-001 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-002 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-003 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-004 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-005 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-006 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-007 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-008 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-009 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-010 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-011 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-012 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-013 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-014 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-015 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-016 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-017 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-018 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-019 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-020 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-021 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-022 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-023 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-024 | DemographicApi | json | 1/1 | PASS |
| ECC-VAL-001 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-002 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-003 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-004 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-005 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-006 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-007 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-008 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-009 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-010 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-011 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-012 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-013 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-014 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-015 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-016 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-017 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-018 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-019 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-020 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-021 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-022 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-023 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-024 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-025 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-026 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-027 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-028 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-029 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-030 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-031 | ArchetypeValidation | json | 1/1 | PASS |
| ECC-VAL-032 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-033 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-034 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-035 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-036 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-037 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-038 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-039 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-040 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-041 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-042 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-043 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-044 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-045 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-046 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-047 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-048 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-049 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-050 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-051 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-052 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-053 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-054 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-055 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-056 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-057 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-058 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-059 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-060 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-061 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-062 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-063 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-064 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-065 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-066 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-067 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-068 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-069 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-070 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-071 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-072 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-073 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-074 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-075 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-076 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-077 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-078 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-079 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-080 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-081 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-082 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-083 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-084 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-085 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-086 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-087 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-088 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-089 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-090 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-091 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-092 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-093 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-094 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-095 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-096 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-097 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-098 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-099 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-100 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-101 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-102 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-103 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-104 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-105 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-119 | ArchetypeValidation | json | 1/1 | PASS |
| ECC-VAL-106 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-107 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-108 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-109 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-110 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-111 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-112 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-113 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-114 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-115 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-116 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-117 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-118 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-SIG-001 | Signing | json | 1/1 | PASS |
| ECC-SIG-001 | Signing | xml | 1/1 | PASS |
| ECC-SIG-002 | Signing | json | 1/1 | PASS |
| ECC-SIG-003 | Signing | json | 4/4 | PASS |
| ECC-SIG-004 | Signing | json | 1/1 | PASS |
| ECC-SIG-005 | Signing | json | 0/0 | skipped |
| ECC-MSG-001 | Messaging | json | 0/0 | skipped |
| ECC-MSG-002 | Messaging | json | 0/0 | skipped |
| ECC-MSG-003 | Messaging | json | 0/0 | skipped |
| ECC-MSG-004 | Messaging | json | 0/0 | skipped |
| ECC-MSG-005 | Messaging | json | 0/0 | skipped |
| ECC-MSG-006 | Messaging | json | 0/0 | skipped |
| ECC-MSG-007 | Messaging | json | 0/0 | skipped |
| ECC-MSG-008 | Messaging | json | 0/0 | skipped |
| ECC-MSG-009 | Messaging | json | 0/0 | skipped |
| ECC-MSG-010 | Messaging | json | 0/0 | skipped |
| ECC-TS-001 | Terminology | json | 1/1 | PASS |
| ECC-TS-002 | Terminology | json | 2/2 | PASS |
| ECC-TS-003 | Terminology | json | 1/1 | PASS |
| ECC-TS-004 | Terminology | json | 1/1 | PASS |
| ECC-TS-005 | Terminology | json | 1/1 | PASS |
| ECC-TS-006 | Terminology | json | 0/0 | skipped |
| ECC-TS-007 | Terminology | json | 0/0 | skipped |
| ECC-TS-008 | Terminology | json | 0/0 | skipped |
| ECC-TS-009 | Terminology | json | 0/0 | skipped |
| ECC-SEC-001 | Authentication | json | 1/1 | PASS |
| ECC-SEC-002 | Authentication | json | 1/1 | PASS |

## 4. Profile verdict (machine-computed)

CORE/STANDARD are all-or-nothing (every capability must pass); OPTIONS is any-passes (obtained if ≥1 optional capability passes) — `master03-profiles.adoc`.

### Core — **PASS**

| Capability | Passed | Failed | Errored | Skipped | N/A | Verdict |
|---|--:|--:|--:|--:|--:|---|
| Adl14ArchetypeProvisioning | 1 | 0 | 0 | 0 | 0 | pass |
| Adl14OptProvisioning | 11 | 0 | 0 | 4 | 0 | pass |
| EhrOperations | 12 | 0 | 0 | 0 | 0 | pass |
| EhrStatus | 10 | 0 | 0 | 0 | 0 | pass |
| CompositionOps | 34 | 0 | 0 | 0 | 0 | pass |
| ChangeSets | 26 | 0 | 0 | 5 | 0 | pass |
| Versioning | 7 | 0 | 0 | 0 | 0 | pass |
| ArchetypeValidation | 119 | 0 | 0 | 0 | 0 | pass |
| AnonymousEhrs | 1 | 0 | 0 | 0 | 0 | pass |

### Standard — **PASS**

| Capability | Passed | Failed | Errored | Skipped | N/A | Verdict |
|---|--:|--:|--:|--:|--:|---|
| Adl14ArchetypeProvisioning | 1 | 0 | 0 | 0 | 0 | pass |
| Adl14OptProvisioning | 11 | 0 | 0 | 4 | 0 | pass |
| EhrOperations | 12 | 0 | 0 | 0 | 0 | pass |
| EhrStatus | 10 | 0 | 0 | 0 | 0 | pass |
| CompositionOps | 34 | 0 | 0 | 0 | 0 | pass |
| ChangeSets | 26 | 0 | 0 | 5 | 0 | pass |
| Versioning | 7 | 0 | 0 | 0 | 0 | pass |
| ArchetypeValidation | 119 | 0 | 0 | 0 | 0 | pass |
| AnonymousEhrs | 1 | 0 | 0 | 0 | 0 | pass |
| DirectoryOps | 34 | 0 | 0 | 0 | 0 | pass |
| QueryProvisioning | 5 | 0 | 0 | 2 | 0 | pass |
| AqlBasic | 13 | 0 | 0 | 0 | 0 | pass |
| Signing | 5 | 0 | 0 | 1 | 0 | pass |

### Options — **OBTAINED** (any-passes)

| Capability | Passed | Failed | Errored | Skipped | N/A | Verdict |
|---|--:|--:|--:|--:|--:|---|
| Adl2Provisioning | 0 | 0 | 0 | 0 | 0 | not evidenced |
| DemographicApi | 24 | 0 | 0 | 0 | 0 | pass |
| AqlAdvanced | 0 | 0 | 0 | 0 | 0 | not evidenced |
| Terminology | 5 | 0 | 0 | 4 | 0 | pass |
| AdminApi | 6 | 0 | 0 | 0 | 0 | pass |
| AdminActivityReport | 0 | 0 | 0 | 0 | 0 | not evidenced |
| AdminPhysicalDeletion | 0 | 0 | 0 | 0 | 0 | not evidenced |
| AdminEhrDumpLoad | 0 | 0 | 0 | 0 | 0 | not evidenced |
| AdminBulkEhrLoad | 0 | 0 | 0 | 0 | 0 | not evidenced |
| AdminEhrArchive | 0 | 0 | 0 | 0 | 0 | not evidenced |
| AdminDemographicArchive | 0 | 0 | 0 | 0 | 0 | not evidenced |
| Messaging | 0 | 0 | 0 | 10 | 0 | not evidenced |

## 5. Deviations (skips), by reason

| Reason | Cases |
|---|--:|
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
| SM I_DEFINITION_ADL14.delete_opt() (CNF master04:319) has no ITS-REST ADL 1.4 binding — ITS-REST development@e8a093e (and Release-1.0.3) define no DELETE verb on /definition/template/adl1.4/{id}; OPT deletion lives in the ADMIN API only | 4 |
| SM I_DEFINITION_QUERY.list_queries() (CNF master05:93) has no ITS-REST binding — ITS-REST development@e8a093e (and Release-1.0.3) expose GET /definition/query/{qualified_query_name}, not a bare GET /definition/query collection | 2 |
| SM I_EHR_CONTRIBUTION.list_contributions() (CNF master08:595) has no ITS-REST binding — ITS-REST development@e8a093e (and Release-1.0.3) define POST only on /ehr/{ehr_id}/contribution, with no GET collection resource; the list is a native-API concern, not wire-exercisable | 5 |
| SutConfig: no FHIR terminology provider configured on the SUT (EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_* unset) — a `hl7.org/fhir/4.0` expand is rejected as `UnknownTerminologyService`. harness terminology server: http://127.0.0.1:64681 (fixture). The bundle (`openehr`) expand cases prove the TERMINOLOGY family; wire this by pointing the SUT at a FHIR server (host.docker.internal for a runner-host fixture, docs/design/terminology-server-integration.md §5). | 1 |
| SutConfig: server not in `pgp` mode (needs a configured OpenPGP key); a pgp-keyed compose profile is a follow-up — digest cases prove the capability | 1 |
| SutConfig: the 5xx fault requires a fault-injecting terminology server wired to the SUT (--tx-server-url + an SUT FHIR provider pointed at it); the HTTP-only ECC cannot reconfigure an external SUT's provider per case. Harness tx server: http://127.0.0.1:64681 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_server_error_is_5xx + app/ehrbase/tests/terminology_fhir.rs::server_5xx_is_an_exception. | 1 |
| SutConfig: the malformed fault requires a fault-injecting terminology server wired to the SUT (--tx-server-url + an SUT FHIR provider pointed at it); the HTTP-only ECC cannot reconfigure an external SUT's provider per case. Harness tx server: http://127.0.0.1:64681 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_malformed_is_not_json + app/ehrbase/tests/terminology_fhir.rs::malformed_body_is_an_exception. | 1 |
| SutConfig: the timeout fault requires a fault-injecting terminology server wired to the SUT (--tx-server-url + an SUT FHIR provider pointed at it); the HTTP-only ECC cannot reconfigure an external SUT's provider per case. Harness tx server: http://127.0.0.1:64681 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_timeout_exceeds_a_short_client_deadline + app/ehrbase/tests/terminology_fhir.rs::timeout_is_an_exception. | 1 |

## 6. Terminology server (TS area)

- Server: `http://127.0.0.1:64681`
- Mode: fixture

Recorded FHIR-tx exchange (4 request(s)):

| # | Method | Path | Query |
|--:|---|---|---|
| 1 | GET | `/ValueSet/$expand` | url=http%3A%2F%2Fhl7.org%2Ffhir%2FValueSet%2Fsurface |
| 2 | GET | `/ValueSet/$validate-code` | url=http%3A%2F%2Fhl7.org%2Ffhir%2FValueSet%2Fsurface&code=B |
| 3 | GET | `/CodeSystem/$lookup` | code=B |
| 4 | GET | `/CodeSystem/$subsumes` | codeA=L&codeB=O |
