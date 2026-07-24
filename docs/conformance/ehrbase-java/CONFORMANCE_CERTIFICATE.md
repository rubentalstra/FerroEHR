# Conformance Certificate

## System Under Test

| Field | Value |
| --- | --- |
| Solution | ehrbase-java 2.34.0 |
| Vendor | EHRbase (vitagroup / upstream open-source project) |
| Runner | cnf-runner 3.8.0 |
| Infrastructure | — |

## Scope of Test

| Dimension | Value |
| --- | --- |
| Functional | CORE, STANDARD |
| Sec & Priv | — |
| Performance | — |
| Ext Data Fmt | canonical-json |

## Profile Report

Result column: ITS its-rest (canonical-json)

| Family | Capability | Required in profile | Result |
| --- | --- | --- | --- |
| Platform | Adl14ArchetypeProvisioning | Y | excused (unrealized on this technology profile) |
| Platform | Adl14OptProvisioning | Y | FAIL |
| Platform | Adl2ArchetypeProvisioning | OPT | not evidenced |
| Platform | Adl2OptProvisioning | OPT | not evidenced |
| Platform | QueryProvisioning | Y | pass |
| Platform | EhrOperations | Y | FAIL |
| Platform | EhrStatus | Y | FAIL |
| Platform | CompositionOps | Y | FAIL |
| Platform | DirectoryOps | Y | FAIL |
| Platform | ChangeSets | Y | FAIL |
| Platform | Versioning | Y | FAIL |
| Platform | ArchetypeValidation | Y | FAIL |
| Platform | PartyOperations | OPT | not evidenced |
| Platform | PartyRelationshipOperations | OPT | not evidenced |
| Platform | DemographicArchetypeValidation | OPT | no cases |
| Platform | AqlBasic | Y | FAIL |
| Platform | AqlAdvanced | OPT | not evidenced |
| Platform | AqlTerminology | OPT | not evidenced |
| Platform | ActivityReport | OPT | not evidenced |
| Platform | PhysicalDeletion | OPT | FAIL |
| Platform | EhrDumpLoad | OPT | not evidenced |
| Platform | BulkEhrLoad | OPT | no cases |
| Platform | EhrArchive | OPT | not evidenced |
| Platform | DemographicArchive | OPT | not evidenced |
| Platform | EhrExtract | OPT | not evidenced |
| Platform | Tds | OPT | not evidenced |
| Platform | DefinitionApi | Y | not evidenced |
| Platform | EhrApi | Y | pass |
| Platform | DemographicApi | OPT | not evidenced |
| Platform | QueryApi | Y | pass |
| Platform | AdminApi | OPT | FAIL |
| Platform | MessageApi | OPT | not evidenced |
| Platform | Signing | OPT | not evidenced |
| Platform | SimplifiedFormats | OPT | not evidenced |
| Security | EhrDemographicSeparation | Y | pass |
| Security | AuthenticatedAccess | Y | pass |
| Security | AuthorizationSeparation | Y | not evidenced |
| Security | AuditAccountability | Y | not evidenced |
| Security | AnonymousEhrs | Y | not evidenced |

## Workload Coverage

The exercised-capability set of the measured hospital-simulation workload against the claimed matrix — a claimed capability the simulation never touches is a gap in the journey catalogue, listed explicitly.

| Capability | Claimed | Exercised by workload |
| --- | --- | --- |
| Adl14ArchetypeProvisioning | yes | NO — catalogue gap |
| Adl14OptProvisioning | yes | yes |
| QueryProvisioning | yes | yes |
| EhrOperations | yes | yes |
| EhrStatus | yes | yes |
| CompositionOps | yes | yes |
| DirectoryOps | yes | yes |
| ChangeSets | yes | yes |
| Versioning | yes | yes |
| ArchetypeValidation | yes | yes |
| AqlBasic | yes | yes |
| AqlAdvanced | yes | NO — catalogue gap |
| PhysicalDeletion | yes | NO — catalogue gap |
| DefinitionApi | yes | yes |
| EhrApi | yes | yes |
| QueryApi | yes | yes |
| AdminApi | yes | NO — catalogue gap |
| EhrDemographicSeparation | yes | NO — catalogue gap |
| AuthenticatedAccess | yes | NO — catalogue gap |
| AuthorizationSeparation | yes | NO — catalogue gap |

Claimed capabilities the simulation never touches (7): Adl14ArchetypeProvisioning, AqlAdvanced, PhysicalDeletion, AdminApi, EhrDemographicSeparation, AuthenticatedAccess, AuthorizationSeparation. Each is either a journey-catalogue gap to close or a capability outside the measured-load surface (admin, demographics, messaging, security posture — exercised by the functional schedule, not the load instrument).

## Performance Rating

Classes are EARNED by measurement (never declared); every earned class is bound to the measured environment recorded in the results.

| Class | Case | Claimed | Verdict |
| --- | --- | --- | --- |
| POC | PERF-hospital_sim-class_POC | no | not earned |

Environment (PERF-hospital_sim-class_POC): ci-runner · 4 cores · 16 GB · ssd · single-node docker compose (docker/sut-ehrbase-java.yml, ehrbase/ehrbase:2.34.0 + ehrbase-v2-postgres:16.2; no readonly principal — EHRbase Basic auth carries one clinical user and one admin user)

