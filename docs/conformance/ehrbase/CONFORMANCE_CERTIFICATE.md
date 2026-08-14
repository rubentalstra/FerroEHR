# Conformance Certificate

## System Under Test

| Field | Value |
| --- | --- |
| Solution | ehrbase 2.34.0 |
| Vendor | EHRbase (vitagroup / upstream open-source project) |
| Runner | cnf-runner 3.17.5 |
| Infrastructure | — |

## Scope of Test

| Dimension | Value |
| --- | --- |
| Functional | CORE, STANDARD |
| Sec & Priv | — |
| Performance | — |
| Ext Data Fmt | canonical-json, canonical-xml |

## Profile Report

Result column: ITS its-rest (canonical-json, canonical-xml)

The Realization column says what the row's cases were verified against: `released-wire` = released ITS-REST operations; `extension` = routes this product serves of its own design, which no openEHR specification governs and which therefore never gate an openEHR profile tier (those rows are always OPT).

| Family | Capability | Required in profile | Realization | Result |
| --- | --- | --- | --- | --- |
| Platform | Adl14ArchetypeProvisioning | OPT | extension | FAIL |
| Platform | Adl14OptProvisioning | Y | released-wire | FAIL |
| Platform | Adl2ArchetypeProvisioning | OPT | released-wire | not claimed |
| Platform | Adl2OptProvisioning | OPT | released-wire | not claimed |
| Platform | TemplateExamples | OPT | released-wire | not evidenced |
| Platform | QueryProvisioning | Y | released-wire | FAIL |
| Platform | EhrOperations | Y | released-wire | FAIL |
| Platform | EhrStatus | Y | released-wire | FAIL |
| Platform | CompositionOps | Y | released-wire | INCONCLUSIVE (errored rows — never green by absorption) |
| Platform | DirectoryOps | Y | released-wire | FAIL |
| Platform | ChangeSets | Y | released-wire | FAIL |
| Platform | Versioning | Y | released-wire | FAIL |
| Platform | ArchetypeValidation | Y | released-wire | FAIL |
| Platform | PartyOperations | OPT | released-wire | not claimed |
| Platform | PartyRelationshipOperations | OPT | extension | not claimed |
| Platform | DemographicArchetypeValidation | OPT | released-wire | not claimed |
| Platform | AqlBasic | Y | released-wire | FAIL |
| Platform | AqlAdvanced | OPT | released-wire | INCONCLUSIVE (errored rows — never green by absorption) |
| Platform | AqlTerminology | OPT | released-wire | not claimed |
| Platform | ActivityReport | OPT | extension | not claimed |
| Platform | PhysicalDeletion | OPT | released-wire | not evidenced |
| Platform | EhrDumpLoad | OPT | extension | not claimed |
| Platform | BulkEhrLoad | OPT | released-wire | not claimed |
| Platform | EhrArchive | OPT | extension | not claimed |
| Platform | DemographicArchive | OPT | extension | not claimed |
| Platform | EhrExtract | OPT | extension | not claimed |
| Platform | Tds | OPT | extension | not claimed |
| Platform | DefinitionApi | Y | released-wire | FAIL |
| Platform | EhrApi | Y | released-wire | FAIL |
| Platform | DemographicApi | OPT | released-wire | not claimed |
| Platform | QueryApi | Y | released-wire | FAIL |
| Platform | AdminApi | OPT | released-wire | not evidenced |
| Platform | MessageApi | OPT | extension | not claimed |
| Platform | SystemApi | OPT | released-wire | not claimed |
| Platform | ItemTags | OPT | released-wire | not claimed |
| Platform | Signing | OPT | released-wire | not claimed |
| Platform | SimplifiedFormats | OPT | released-wire | not claimed |
| Platform | SmartAppLaunch | OPT | released-wire | not claimed |
| Security | EhrDemographicSeparation | Y | released-wire | pass |
| Security | AuthenticatedAccess | Y | released-wire | pass |
| Security | AuthorizationSeparation | Y | released-wire | not evidenced |
| Security | AuditAccountability | Y | released-wire | not claimed |
| Security | AnonymousEhrs | Y | released-wire | not claimed |

## Workload Coverage

The exercised-capability set of the measured hospital-simulation workload against the claimed matrix. A claimed capability the simulation never touches is either an ADJUDICATED exclusion — the capability-matrix row names the register entry that decided it and the reason is printed in the row — or an undecided catalogue gap, which the `workload-coverage` validate gate fails on, so no published certificate reaches this section carrying one.

| Capability | Claimed | Exercised by workload |
| --- | --- | --- |
| Adl14ArchetypeProvisioning | yes | no — adjudicated exclusion (AMB-170): definition administration, not a sustainable per-patient arrival: archetype provisioning is a design-time operation a hospital simulation would not repeatedly drive (the same family reason the ADL2/OPT provisioning rows carry journeys for is satisfied by the definition-poll journey; this row's own upload/delete churn would grow the definition store unboundedly through a measured hold) |
| Adl14OptProvisioning | yes | yes |
| TemplateExamples | yes | NO — catalogue gap (UNADJUDICATED) |
| QueryProvisioning | yes | yes |
| EhrOperations | yes | yes |
| EhrStatus | yes | yes |
| CompositionOps | yes | yes |
| DirectoryOps | yes | yes |
| ChangeSets | yes | yes |
| Versioning | yes | yes |
| ArchetypeValidation | yes | yes |
| AqlBasic | yes | yes |
| AqlAdvanced | yes | NO — catalogue gap (UNADJUDICATED) |
| PhysicalDeletion | yes | no — adjudicated exclusion (AMB-170): destructive mid-measurement: the released admin operations erase an EHR and its whole version history outright, so every arrival would shrink the population-anchored corpus the measured window is bound to and the earned class would no longer describe the declared volumetric class |
| DefinitionApi | yes | yes |
| EhrApi | yes | yes |
| QueryApi | yes | yes |
| AdminApi | yes | no — adjudicated exclusion (AMB-170): destructive mid-measurement: the released ADMIN API of ITS-REST 1.1.0 is exactly admin_ehr_delete and admin_ehr_delete_all, so the only wire this claim covers is the same corpus-erasing pair PhysicalDeletion excludes (the further /admin routes this server mounts are extensions no claim rests on) |
| EhrDemographicSeparation | yes | yes |
| AuthenticatedAccess | yes | NO — catalogue gap (UNADJUDICATED) |
| AuthorizationSeparation | yes | NO — catalogue gap (UNADJUDICATED) |

Claimed capabilities excluded from the measured workload by adjudication (3): Adl14ArchetypeProvisioning, PhysicalDeletion, AdminApi. Each row above names its register entry; the exclusion bounds the LOAD instrument only — the functional catalogue still owes every one of them verdict-bearing cases at its `min_cases` floor.

UNADJUDICATED gaps (4): TemplateExamples, AqlAdvanced, AuthenticatedAccess, AuthorizationSeparation. These rows are a defect in this submission, not a property of the product: the `workload-coverage` validate gate fails on each of them, so this certificate was rendered from an artifact tree that does not pass its own gates.

## Performance Rating

Classes are EARNED by measurement (never declared); every earned class is bound to the measured environment recorded in the results.

| Class | Case | Claimed | Verdict |
| --- | --- | --- | --- |
| POC | PERF-hospital_sim-class_POC | no | not earned |

Environment (PERF-hospital_sim-class_POC): ci-runner · 4 cores · 16 GB · ssd · single-node docker compose (docker/sut-ehrbase-java.yml, ehrbase/ehrbase:2.34.0 + ehrbase-v2-postgres:16.2; no readonly principal — EHRbase Basic auth carries one clinical user and one admin user)

