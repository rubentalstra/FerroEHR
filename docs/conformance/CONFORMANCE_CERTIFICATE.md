# ehrbase-rs Conformance Certificate (generated, self-assessed)

> A self-assessed certificate produced by the ehrbase-rs Conformance
> Catalogue (ECC) from a conformance run. Its structure follows the CNF
> `certificate/master03-certificate.adoc` template; every verdict is
> machine-computed from `results.json`.

## System Under Test (SUT)

| | |
|---|---|
| Solution | ehrbase-rs @ `http://localhost:8080/ehrbase/rest/openehr/v1` |
| Vendor | ehrbase-rs (self-assessed) |
| Assessor | ehrbase-rs Conformance Catalogue (ECC) — self-assessment |
| Infrastructure | reference corpus openEHR/specifications-CNF@33251d2a; SUT auth mode basic |
| Date | 2026-07-10T11:53:58.183187Z |

## Scope of Test

| | |
|---|---|
| Functional | Core (PASS), Standard (PASS), Options (OBTAINED) |
| Sec & Priv | Signing pass, Anonymous EHRs pass |
| Ext Data Fmt | json, xml |

## Profile Report

### Core — PASS

| Capability | Required in profile | Result |
|---|:--:|---|
| Adl14ArchetypeProvisioning | Y | pass |
| Adl14OptProvisioning | Y | pass |
| EhrOperations | Y | pass |
| EhrStatus | Y | pass |
| CompositionOps | Y | pass |
| ChangeSets | Y | pass |
| Versioning | Y | pass |
| ArchetypeValidation | Y | pass |
| AnonymousEhrs | Y | pass |

### Standard — PASS

| Capability | Required in profile | Result |
|---|:--:|---|
| Adl14ArchetypeProvisioning | Y | pass |
| Adl14OptProvisioning | Y | pass |
| EhrOperations | Y | pass |
| EhrStatus | Y | pass |
| CompositionOps | Y | pass |
| ChangeSets | Y | pass |
| Versioning | Y | pass |
| ArchetypeValidation | Y | pass |
| AnonymousEhrs | Y | pass |
| DirectoryOps | Y | pass |
| QueryProvisioning | Y | pass |
| AqlBasic | Y | pass |
| Signing | Y | pass |

### Options — OBTAINED

| Capability | Required in profile | Result |
|---|:--:|---|
| Adl2Provisioning | OPT | not evidenced |
| DemographicApi | OPT | pass |
| AqlAdvanced | OPT | not evidenced |
| Terminology | OPT | pass |
| AdminApi | OPT | pass |
| AdminActivityReport | OPT | not evidenced |
| AdminPhysicalDeletion | OPT | not evidenced |
| AdminEhrDumpLoad | OPT | not evidenced |
| AdminBulkEhrLoad | OPT | not evidenced |
| AdminEhrArchive | OPT | not evidenced |
| AdminDemographicArchive | OPT | not evidenced |
| Messaging | OPT | not evidenced |

## Detailed Test Report

One row per ECC case (formats collapsed to a combined REST verdict). *Conformance point* is the CNF-schedule `<SERVICE>.<operation>` id where the case traces to one, else `—`. (There is no protobuf technology under test — the CNF template's protobuf column is omitted.)

| openEHR Component | Capability | Conformance point | Test Case | REST |
|---|---|---|---|---|
| EHR service | EhrOperations | — | ECC-EHR-001 — EHR existence check — existing EHR id | pass |
| EHR service | EhrOperations | — | ECC-EHR-002 — EHR existence check — existing subject id | pass |
| EHR service | EhrOperations | — | ECC-EHR-003 — EHR existence check — non existing EHR id | pass |
| EHR service | EhrOperations | — | ECC-EHR-004 — EHR existence check — non existing subject id | pass |
| EHR service | EhrOperations | — | ECC-EHR-005 — Create EHR — main | pass |
| EHR service | EhrOperations | — | ECC-EHR-006 — Create EHR — same EHR twice | pass |
| EHR service | EhrOperations | — | ECC-EHR-007 — Create EHR — two EHRs same patient | pass |
| EHR service | EhrOperations | — | ECC-EHR-008 — Get EHR — existing EHR by EHR id | pass |
| EHR service | EhrOperations | — | ECC-EHR-009 — Get EHR — existing EHR by subject id | pass |
| EHR service | EhrOperations | — | ECC-EHR-010 — Get EHR — get EHR by invalid EHR id | pass |
| EHR service | EhrOperations | — | ECC-EHR-011 — Get EHR — get EHR by invalid subject id | pass |
| EHR_STATUS | EhrStatus | — | ECC-STA-001 — Get EHR_STATUS — get by EHR id | pass |
| EHR_STATUS | EhrStatus | — | ECC-STA-002 — Get EHR_STATUS — bad EHR | pass |
| EHR_STATUS | EhrStatus | — | ECC-STA-003 — Set EHR_STATUS is_queryable — existing EHR | pass |
| EHR_STATUS | EhrStatus | — | ECC-STA-004 — Set EHR_STATUS is_queryable — bad EHR | pass |
| EHR_STATUS | EhrStatus | — | ECC-STA-005 — Set EHR_STATUS is_modifiable — existing EHR | pass |
| EHR_STATUS | EhrStatus | — | ECC-STA-006 — Set EHR_STATUS is_modifiable — bad EHR | pass |
| EHR_STATUS | EhrStatus | — | ECC-STA-007 — Clear EHR_STATUS is_queryable — existing EHR | pass |
| EHR_STATUS | EhrStatus | — | ECC-STA-008 — Clear EHR_STATUS is_queryable — bad EHR | pass |
| EHR_STATUS | EhrStatus | — | ECC-STA-009 — Clear EHR_STATUS is_modifiable — existing EHR | pass |
| EHR_STATUS | EhrStatus | — | ECC-STA-010 — Clear EHR_STATUS is_modifiable — bad EHR | pass |
| EHR service | EhrOperations | — | ECC-EHR-012 — Create EHR — reject invalid EHR_STATUS data sets | pass |
| EHR service | AnonymousEhrs | — | ECC-EHR-013 — Create anonymous (subject-less) EHR | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-001 — Create composition — event | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-002 — Create composition — persistent | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-003 — Create composition — same OPT twice | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-004 — Create composition — invalid event | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-005 — Create composition — invalid persistent | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-006 — Create composition — event bad OPT | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-007 — Create composition — event bad EHR | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-008 — Get latest composition | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-009 — Get latest composition — bad composition | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-010 — Get latest composition — bad EHR | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-011 — Composition existence check — bad composition | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-012 — Composition existence check — bad EHR | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-013 — Get composition at time | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-014 — Get composition at time — no time arg | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-015 — Get composition at time — bad composition | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-016 — Get composition at time — bad EHR | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-017 — Get composition at multiple times | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-018 — Get composition version | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-019 — Get composition version — bad version | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-020 — Get composition version — bad EHR | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-021 — Get composition versions | pass |
| COMPOSITION | Versioning | — | ECC-COM-022 — Get versioned composition | pass |
| COMPOSITION | Versioning | — | ECC-COM-023 — Get versioned composition — non existent | pass |
| COMPOSITION | Versioning | — | ECC-COM-024 — Get versioned composition — bad EHR | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-025 — Update composition — event | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-026 — Update composition — persistent | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-027 — Update composition — non existent | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-028 — Update composition — wrong template | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-029 — Delete composition — event | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-030 — Delete composition — persistent | pass |
| COMPOSITION | CompositionOps | — | ECC-COM-031 — Delete composition — non existent | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-001 — Commit contribution — valid composition | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-002 — Commit contribution — invalid composition | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-003 — Commit contribution — empty | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-004 — Commit contribution — valid invalid compositions | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-005 — Commit contribution — non exiting OPT | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-006 — Commit contribution — event composition | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-007 — Commit contribution — persistent composition | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-008 — Commit contribution — delete | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-009 — Commit contribution — two commits second invalid | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-010 — Commit contribution — two commits second creation | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-011 — Commit contribution — minimal EHR status | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-012 — Commit contribution — full EHR status | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-013 — Commit contribution — EHR status invalid change type | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-014 — Commit contribution — invalid EHR status | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-015 — Commit contribution — valid directory | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-016 — Commit contribution — fail create existing directory | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-017 — Commit contribution — fail modify non existing directory | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-018 — Commit contribution — update existing directory | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-019 — Get contribution — existing | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-020 — Get contribution — empty EHR | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-021 — Get contribution — bad EHR | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-022 — Get contribution — bad contribution | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-023 — Contribution existence check — existing | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-024 — Contribution existence check — bad contribution | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-025 — Contribution existence check — bad EHR | pass |
| CONTRIBUTION (change sets) | ChangeSets | — | ECC-CTB-026 — Contribution existence check — empty EHR | pass |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.list_contributions (CNF master08:595) | ECC-CTB-027 — List contributions — empty | skipped |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.list_contributions (CNF master08:595) | ECC-CTB-028 — List contributions — non existing EHR | skipped |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.list_contributions (CNF master08:595) | ECC-CTB-029 — List contributions — post commit | skipped |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.list_contributions (CNF master08:595) | ECC-CTB-030 — List contributions — EHR containing directory | skipped |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.list_contributions (CNF master08:595) | ECC-CTB-031 — List contributions — EHR containing EHR status | skipped |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-001 — Create directory — empty EHR | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-002 — Create directory — EHR with directory | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-003 — Create directory — bad EHR | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-004 — Get directory — EHR root directory | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-005 — Get directory — bad EHR | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-006 — Get directory at time — EHR with directory | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-007 — Get directory at time — bad EHR | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-008 — Update directory — EHR with directory | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-009 — Update directory — bad EHR | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-010 — Delete directory — EHR with directory | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-011 — Delete directory — bad EHR | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-012 — Directory existence check — empty EHR | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-013 — Directory existence check — EHR with directory | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-014 — Directory existence check — bad EHR | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-015 — Directory path existence check — EHR root directory | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-016 — Directory path existence check — folder structure | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-017 — Directory path existence check — empty EHR | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-018 — Directory path existence check — bad EHR | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-019 — Directory version existence check — empty EHR | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-020 — Directory version existence check — directory with two versions | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-021 — Directory version existence check — bad EHR | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-022 — Get directory — empty EHR | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-023 — Get directory — directory with structure | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-024 — Get directory at time — EHR with directory empty time | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-025 — Get directory at time — EHR with directory versions | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-026 — Get directory at time — EHR with directory versions empty time | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-027 — Get directory at time — empty EHR | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-028 — Get directory at time — empty EHR empty time | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-029 — Get directory at time — multiple versions first | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-030 — Get directory at version — bad EHR | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-031 — Get directory at version — directory with two versions | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-032 — Get directory at version — empty EHR | pass |
| DIRECTORY (FOLDER) | Versioning | I_EHR_DIRECTORY.get_versioned_directory (CNF master09:670) | ECC-DIR-033 — Get versioned directory — empty EHR | pass |
| DIRECTORY (FOLDER) | Versioning | I_EHR_DIRECTORY.get_versioned_directory (CNF master09:670) | ECC-DIR-034 — Get versioned directory — directory with two versions | pass |
| DIRECTORY (FOLDER) | Versioning | I_EHR_DIRECTORY.get_versioned_directory (CNF master09:670) | ECC-DIR-035 — Get versioned directory — bad EHR | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-036 — Update directory — empty EHR | pass |
| DIRECTORY (FOLDER) | DirectoryOps | — | ECC-DIR-037 — Delete directory — empty EHR | pass |
| Template / OPT provisioning | Adl14ArchetypeProvisioning | — | ECC-TPL-001 — Upload OPT — valid OPT (provisions ADL 1.4 archetypes) | pass |
| Template / OPT provisioning | Adl14OptProvisioning | — | ECC-TPL-002 — Upload OPT — invalid OPT | pass |
| Template / OPT provisioning | Adl14OptProvisioning | — | ECC-TPL-003 — List OPTs — retrieve all no OPTs | pass |
| Template / OPT provisioning | Adl14OptProvisioning | — | ECC-TPL-004 — Upload OPT — valid OPT twice conflict | pass |
| Template / OPT provisioning | Adl14OptProvisioning | — | ECC-TPL-005 — Upload OPT — valid OPT twice no conflict | pass |
| Template / OPT provisioning | Adl14OptProvisioning | — | ECC-TPL-006 — Get OPT — retrieve single | pass |
| Template / OPT provisioning | Adl14OptProvisioning | — | ECC-TPL-007 — Get OPT — retrieve latest version | pass |
| Template / OPT provisioning | Adl14OptProvisioning | — | ECC-TPL-008 — Get OPT — retrieve specific version | pass |
| Template / OPT provisioning | Adl14OptProvisioning | — | ECC-TPL-009 — Get OPT — retrieve fail | pass |
| Template / OPT provisioning | Adl14OptProvisioning | — | ECC-TPL-010 — List OPTs — retrieve all | pass |
| Template / OPT provisioning | Adl14OptProvisioning | — | ECC-TPL-011 — Validate OPT — valid OPT | pass |
| Template / OPT provisioning | Adl14OptProvisioning | — | ECC-TPL-012 — Validate OPT — invalid OPT | pass |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.delete_opt (CNF master04:319) | ECC-TPL-013 — Delete OPT — delete non existing | skipped |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.delete_opt (CNF master04:319) | ECC-TPL-014 — Delete OPT — delete existing | skipped |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.delete_opt (CNF master04:319) | ECC-TPL-015 — Delete OPT — delete latest version | skipped |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.delete_opt (CNF master04:319) | ECC-TPL-016 — Delete OPT — delete specific version | skipped |
| Stored-query provisioning | QueryProvisioning | — | ECC-SQR-001 — Store stored query — valid | pass |
| Stored-query provisioning | QueryProvisioning | — | ECC-SQR-002 — List stored queries — non empty | pass |
| Stored-query provisioning | QueryProvisioning | — | ECC-SQR-003 — Stored query existence check — xxx | pass |
| Stored-query provisioning | QueryProvisioning | I_DEFINITION_QUERY.list_queries (CNF master05:93) | ECC-SQR-004 — List stored queries — empty | skipped |
| Stored-query provisioning | QueryProvisioning | I_DEFINITION_QUERY.list_queries (CNF master05:93) | ECC-SQR-005 — List stored queries — select items | skipped |
| Stored-query provisioning | QueryProvisioning | — | ECC-SQR-006 — Store stored query — bad formalism | pass |
| Stored-query provisioning | QueryProvisioning | — | ECC-SQR-007 — Store stored query — invalid | pass |
| AQL execution | AqlBasic | — | ECC-QRY-001 — Query service smoke test | pass |
| AQL execution | AqlBasic | — | ECC-QRY-002 — Execute ad-hoc AQL query — empty db | pass |
| AQL execution | AqlBasic | — | ECC-QRY-003 — Execute stored AQL query — empty db | pass |
| AQL execution | AqlBasic | — | ECC-QRY-004 — Execute ad-hoc AQL query — loaded db | pass |
| AQL execution | AqlBasic | — | ECC-QRY-005 — AQL corpus — invalid queries rejected | pass |
| AQL execution | AqlBasic | — | ECC-QRY-006 — AQL corpus — A empty db | pass |
| AQL execution | AqlBasic | — | ECC-QRY-007 — AQL corpus — B empty db | pass |
| AQL execution | AqlBasic | — | ECC-QRY-008 — AQL corpus — C empty db | pass |
| AQL execution | AqlBasic | — | ECC-QRY-009 — AQL corpus — D empty db | pass |
| AQL execution | AqlBasic | — | ECC-QRY-010 — AQL corpus — A loaded db | pass |
| AQL execution | AqlBasic | — | ECC-QRY-011 — AQL corpus — B loaded db | pass |
| AQL execution | AqlBasic | — | ECC-QRY-012 — AQL corpus — C loaded db | pass |
| AQL execution | AqlBasic | — | ECC-QRY-013 — AQL corpus — D loaded db | pass |
| Admin service | AdminApi | — | ECC-ADM-001 — Admin EHR delete | pass |
| Admin service | AdminApi | — | ECC-ADM-002 — Admin EHR delete absent | pass |
| Admin service | AdminApi | — | ECC-ADM-003 — Admin EHR delete idempotent | pass |
| Admin service | AdminApi | — | ECC-ADM-004 — Admin EHR delete all | pass |
| Admin service | AdminApi | — | ECC-ADM-005 — Admin EHR delete all partial | pass |
| Admin service | AdminApi | — | ECC-ADM-006 — Admin EHR delete all empty | pass |
| Demographic service | DemographicApi | — | ECC-DEM-001 — Demographic person create | pass |
| Demographic service | DemographicApi | — | ECC-DEM-002 — Demographic person get | pass |
| Demographic service | DemographicApi | — | ECC-DEM-003 — Demographic person get by version | pass |
| Demographic service | DemographicApi | — | ECC-DEM-004 — Demographic person update | pass |
| Demographic service | DemographicApi | — | ECC-DEM-005 — Demographic person delete | pass |
| Demographic service | DemographicApi | — | ECC-DEM-006 — Demographic person get deleted | pass |
| Demographic service | DemographicApi | — | ECC-DEM-007 — Demographic person get absent | pass |
| Demographic service | DemographicApi | — | ECC-DEM-008 — Demographic person update bad if match | pass |
| Demographic service | DemographicApi | — | ECC-DEM-009 — Demographic agent create | pass |
| Demographic service | DemographicApi | — | ECC-DEM-010 — Demographic agent get | pass |
| Demographic service | DemographicApi | — | ECC-DEM-011 — Demographic agent delete | pass |
| Demographic service | DemographicApi | — | ECC-DEM-012 — Demographic group create | pass |
| Demographic service | DemographicApi | — | ECC-DEM-013 — Demographic group get | pass |
| Demographic service | DemographicApi | — | ECC-DEM-014 — Demographic group delete | pass |
| Demographic service | DemographicApi | — | ECC-DEM-015 — Demographic organisation create | pass |
| Demographic service | DemographicApi | — | ECC-DEM-016 — Demographic organisation get | pass |
| Demographic service | DemographicApi | — | ECC-DEM-017 — Demographic organisation delete | pass |
| Demographic service | DemographicApi | — | ECC-DEM-018 — Demographic role create | pass |
| Demographic service | DemographicApi | — | ECC-DEM-019 — Demographic role get | pass |
| Demographic service | DemographicApi | — | ECC-DEM-020 — Demographic role delete | pass |
| Demographic service | DemographicApi | — | ECC-DEM-021 — Demographic create bad body | pass |
| Demographic service | DemographicApi | — | ECC-DEM-022 — Demographic versioned party get | pass |
| Demographic service | DemographicApi | — | ECC-DEM-023 — Demographic versioned party revision history | pass |
| Demographic service | DemographicApi | — | ECC-DEM-024 — Demographic person tags | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-001 — Validate COMPOSITION — content card any context any | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-002 — Validate COMPOSITION — content card 1plus context any | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-003 — Validate COMPOSITION — content card 3plus context any | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-004 — Validate COMPOSITION — content card OPT context any | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-005 — Validate COMPOSITION — content card mand context any | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-006 — Validate COMPOSITION — content card 3to5 context any | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-007 — Validate COMPOSITION — content card any context mand | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-008 — Validate COMPOSITION — content card 1plus context mand | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-009 — Validate COMPOSITION — content card 3plus context mand | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-010 — Validate COMPOSITION — content card OPT context mand | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-011 — Validate COMPOSITION — content card mand context mand | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-012 — Validate COMPOSITION — content card 3to5 context mand | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-013 — Validate OBSERVATION — state ex OPT protocol ex OPT | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-014 — Validate OBSERVATION — state ex OPT protocol ex mand | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-015 — Validate OBSERVATION — state ex mand protocol ex OPT | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-016 — Validate OBSERVATION — state ex mand protocol ex mand | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-017 — Validate HISTORY — events card any summary ex OPT | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-018 — Validate HISTORY — events card 1plus summary ex OPT | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-019 — Validate HISTORY — events card 3plus summary ex OPT | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-020 — Validate HISTORY — events card OPT summary ex OPT | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-021 — Validate HISTORY — events card mand summary ex OPT | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-022 — Validate HISTORY — events card 3to5 summary ex OPT | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-023 — Validate HISTORY — events card any summary ex mand | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-024 — Validate HISTORY — events card 1plus summary ex mand | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-025 — Validate HISTORY — events card 3plus summary ex mand | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-026 — Validate HISTORY — events card OPT summary ex mand | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-027 — Validate HISTORY — events card mand summary ex mand | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-028 — Validate HISTORY — events card 3to5 summary ex mand | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-029 — Validate EVENT — state ex OPT | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-030 — Validate EVENT — state ex mand | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-031 — Validate EVENT — type any | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-032 — Validate EVENT — type point event | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-033 — Validate EVENT — type interval event | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-034 — Validate ITEM_STRUCTURE — type any | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-035 — Validate ITEM_STRUCTURE — type item tree | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-036 — Validate ITEM_STRUCTURE — type item list | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-037 — Validate ITEM_STRUCTURE — type item table | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-038 — Validate ITEM_STRUCTURE — type item single | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-039 — Validate DV_BOOLEAN — anything allowed | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-040 — Validate DV_BOOLEAN — only true allowed | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-041 — Validate DV_BOOLEAN — only false allowed | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-042 — Validate DV_IDENTIFIER — all pattern | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-043 — Validate DV_IDENTIFIER — all list | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-044 — Validate DV_TEXT — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-045 — Validate DV_TEXT — list | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-046 — Validate DV_CODED_TEXT — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-047 — Validate DV_CODED_TEXT — local codes | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-048 — Validate DV_CODED_TEXT — ext term | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-049 — Validate DV_ORDINAL — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-050 — Validate DV_ORDINAL — constraint | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-051 — Validate DV_SCALE — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-052 — Validate DV_SCALE — constraint | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-053 — Validate DV_COUNT — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-054 — Validate DV_COUNT — range | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-055 — Validate DV_COUNT — list | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-056 — Validate DV_QUANTITY — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-057 — Validate DV_QUANTITY — property | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-058 — Validate DV_QUANTITY — property units | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-059 — Validate DV_QUANTITY — property units mag | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-060 — Validate DV_PROPORTION — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-061 — Validate DV_PROPORTION — ratio | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-062 — Validate DV_PROPORTION — unitary | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-063 — Validate DV_PROPORTION — percent | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-064 — Validate DV_PROPORTION — fraction | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-065 — Validate DV_PROPORTION — integer fraction | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-066 — Validate DV_PROPORTION — any fraction | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-067 — Validate DV_PROPORTION — ratio range | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-068 — Validate DV_INTERVAL<DV_COUNT> — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-069 — Validate DV_INTERVAL<DV_COUNT> — lower upper | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-070 — Validate DV_INTERVAL<DV_COUNT> — lower upper list | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-071 — Validate DV_INTERVAL<DV_QUANTITY> — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-072 — Validate DV_INTERVAL<DV_QUANTITY> — upper lower | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-073 — Validate DV_INTERVAL<DV_DATE_TIME> — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-074 — Validate DV_INTERVAL<DV_DATE_TIME> — lower upper constraint | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-075 — Validate DV_INTERVAL<DV_DATE_TIME> — lower upper range | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-076 — Validate DV_INTERVAL<DV_DATE> — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-077 — Validate DV_INTERVAL<DV_DATE> — lower upper constraint | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-078 — Validate DV_INTERVAL<DV_DATE> — lower upper range | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-079 — Validate DV_INTERVAL<DV_TIME> — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-080 — Validate DV_INTERVAL<DV_TIME> — lower upper constraint | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-081 — Validate DV_INTERVAL<DV_TIME> — lower upper range | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-082 — Validate DV_INTERVAL<DV_DURATION> — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-083 — Validate DV_INTERVAL<DV_DURATION> — constraint | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-084 — Validate DV_INTERVAL<DV_DURATION> — range | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-085 — Validate DV_INTERVAL<DV_ORDINAL> — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-086 — Validate DV_INTERVAL<DV_ORDINAL> — constraint | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-087 — Validate DV_INTERVAL<DV_SCALE> — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-088 — Validate DV_INTERVAL<DV_SCALE> — constraint | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-089 — Validate DV_INTERVAL<DV_PROPORTION> — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-090 — Validate DV_INTERVAL<DV_PROPORTION> — ratio | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-091 — Validate DV_INTERVAL<DV_PROPORTION> — unitary | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-092 — Validate DV_INTERVAL<DV_PROPORTION> — percentage | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-093 — Validate DV_INTERVAL<DV_PROPORTION> — fraction | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-094 — Validate DV_INTERVAL<DV_PROPORTION> — integer fraction | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-095 — Validate DV_INTERVAL<DV_PROPORTION> — ratio range | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-096 — Validate DV_DURATION — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-097 — Validate DV_DURATION — fields | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-098 — Validate DV_DURATION — range | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-099 — Validate DV_DURATION — fields range | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-100 — Validate DV_TIME — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-101 — Validate DV_TIME — constraint | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-102 — Validate DV_TIME — range | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-103 — Validate DV_DATE — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-104 — Validate DV_DATE — constraint | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-105 — Validate DV_DATE — range | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-119 — Validate DV_DATE — day disallowed by C_DATE pattern (defective vendored fixture rejected) | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-106 — Validate DV_DATE_TIME — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-107 — Validate DV_DATE_TIME — constraint | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-108 — Validate DV_DATE_TIME — range | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-109 — Validate DV_PARSABLE — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-110 — Validate DV_PARSABLE — value formalism | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-111 — Validate DV_MULTIMEDIA — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-112 — Validate DV_MULTIMEDIA — media type | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-113 — Validate DV_URI — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-114 — Validate DV_URI — pattern | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-115 — Validate DV_URI — list | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-116 — Validate DV_EHR_URI — open | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-117 — Validate DV_EHR_URI — pattern | pass |
| Content / archetype validation | ArchetypeValidation | — | ECC-VAL-118 — Validate DV_EHR_URI — list | pass |
| Version signing | Signing | — | ECC-SIG-001 — Version signing — digest present | pass |
| Version signing | Signing | — | ECC-SIG-002 — Version signing — digest recomputes | pass |
| Version signing | Signing | — | ECC-SIG-003 — Version signing — all kinds | pass |
| Version signing | Signing | — | ECC-SIG-004 — Version signing — client verbatim | pass |
| Version signing | Signing | — | ECC-SIG-005 — Version signing — pgp verifies | skipped |
| Messaging | Messaging | I_EHR_EXTRACT_SERVICE.export_ehrs (CNF master13, TBD) | ECC-MSG-001 — EHR Extract — export whole EHR (export_ehrs) | skipped |
| Messaging | Messaging | I_EHR_EXTRACT_SERVICE.export_ehr_extracts (CNF master13, TBD) | ECC-MSG-002 — EHR Extract — spec-driven export (export_ehr_extracts) | skipped |
| Messaging | Messaging | I_EHR_EXTRACT_SERVICE.export_ehrs (CNF master13, TBD) | ECC-MSG-003 — EHR Extract — export of unknown EHR fails | skipped |
| Messaging | Messaging | I_EHR_EXTRACT_SERVICE.import_ehr (CNF master13, TBD) | ECC-MSG-004 — EHR Extract — import whole-EHR clone reusing source id (import_ehr) | skipped |
| Messaging | Messaging | I_EHR_EXTRACT_SERVICE.import_ehr (CNF master13, TBD) | ECC-MSG-005 — EHR Extract — import whole EHR into a caller-fixed id (import_ehr) | skipped |
| Messaging | Messaging | I_EHR_EXTRACT_SERVICE.import_ehr (CNF master13, TBD) | ECC-MSG-006 — EHR Extract — import into a duplicate target id fails | skipped |
| Messaging | Messaging | I_EHR_EXTRACT_SERVICE.import_ehr_extract (CNF master13, TBD) | ECC-MSG-007 — EHR Extract — import extract into an existing EHR (import_ehr_extract) | skipped |
| Messaging | Messaging | I_TDD_SERVICE.import_tdd (CNF master13, TBD) | ECC-MSG-008 — TDD — import a TDD as a committed COMPOSITION (import_tdd) | skipped |
| Messaging | Messaging | I_TDD_SERVICE.import_tdd (CNF master13, TBD) | ECC-MSG-009 — TDD — import rejects malformed / non-TDD / unknown EHR / unknown template | skipped |
| Messaging | Messaging | I_TDD_SERVICE.import_tdds (CNF master13, TBD) | ECC-MSG-010 — TDD — batch import commits all, fail-fast on error (import_tdds) | skipped |
| Terminology-server integration | Terminology | — | ECC-TS-001 — TERMINOLOGY expand (bundle) — accepted, well-formed RESULT_SET | pass |
| Terminology-server integration | Terminology | — | ECC-TS-002 — TERMINOLOGY expand (bundle) — expansion constrains matches to the value set's codes | pass |
| Terminology-server integration | Terminology | — | ECC-TS-003 — TERMINOLOGY expand (bundle) — explicit code merged with the expansion (matches list) | pass |
| Terminology-server integration | Terminology | — | ECC-TS-004 — TERMINOLOGY expand — unknown value set rejected (400) | pass |
| Terminology-server integration | Terminology | — | ECC-TS-005 — TERMINOLOGY expand — unknown service_api rejected (400) | pass |
| Terminology-server integration | Terminology | — | ECC-TS-006 — TERMINOLOGY expand (FHIR service_api) — accepted when a provider is configured | skipped |
| Terminology-server integration | Terminology | — | ECC-TS-007 — TERMINOLOGY expand (FHIR) — terminology-server timeout is a server fault (500) | skipped |
| Terminology-server integration | Terminology | — | ECC-TS-008 — TERMINOLOGY expand (FHIR) — terminology-server 5xx is a server fault (500) | skipped |
| Terminology-server integration | Terminology | — | ECC-TS-009 — TERMINOLOGY expand (FHIR) — malformed terminology response is a server fault (500) | skipped |
| Security / authorization | Authentication | SECURITY_TESTS/I_OAuth2_Keycloak §06 API endpoints are secured | ECC-SEC-001 — Unauthenticated request to a protected route is refused (401) | pass |
| Security / authorization | Authentication | — | ECC-SEC-002 — Regular credential on an ADMIN-only route is forbidden (403) | pass |
