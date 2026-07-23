# Conformance Certificate

## System Under Test

| Field | Value |
| --- | --- |
| Solution | ehrbase-rs 3.7.0 |
| Vendor | Ruben Talstra |
| Runner | cnf-runner 3.7.0 |
| Infrastructure | ixit.json#/environment |

## Scope of Test

| Dimension | Value |
| --- | --- |
| Functional | CORE, STANDARD, OPTIONS, SEC-BASIC |
| Sec & Priv | SEC-BASIC PASS |
| Performance | — |
| Ext Data Fmt | canonical-json |

## Profile Report

Result column: ITS its-rest (canonical-json)

| Family | Capability | Required in profile | Result |
| --- | --- | --- | --- |
| Platform | Adl14ArchetypeProvisioning | Y | excused (unrealized on this technology profile) |
| Platform | Adl14OptProvisioning | Y | pass |
| Platform | Adl2ArchetypeProvisioning | OPT | excused (unrealized on this technology profile) |
| Platform | Adl2OptProvisioning | OPT | pass |
| Platform | QueryProvisioning | Y | pass |
| Platform | EhrOperations | Y | pass |
| Platform | EhrStatus | Y | pass |
| Platform | CompositionOps | Y | pass |
| Platform | DirectoryOps | Y | pass |
| Platform | ChangeSets | Y | pass |
| Platform | Versioning | Y | pass |
| Platform | ArchetypeValidation | Y | pass |
| Platform | PartyOperations | OPT | pass |
| Platform | PartyRelationshipOperations | OPT | excused (unrealized on this technology profile) |
| Platform | DemographicArchetypeValidation | OPT | no cases |
| Platform | AqlBasic | Y | pass |
| Platform | AqlAdvanced | OPT | pass |
| Platform | AqlTerminology | OPT | pass |
| Platform | ActivityReport | OPT | excused (unrealized on this technology profile) |
| Platform | PhysicalDeletion | OPT | pass |
| Platform | EhrDumpLoad | OPT | excused (unrealized on this technology profile) |
| Platform | BulkEhrLoad | OPT | no cases |
| Platform | EhrArchive | OPT | excused (unrealized on this technology profile) |
| Platform | DemographicArchive | OPT | excused (unrealized on this technology profile) |
| Platform | EhrExtract | OPT | excused (unrealized on this technology profile) |
| Platform | Tds | OPT | excused (unrealized on this technology profile) |
| Platform | DefinitionApi | Y | pass |
| Platform | EhrApi | Y | pass |
| Platform | DemographicApi | OPT | pass |
| Platform | QueryApi | Y | pass |
| Platform | AdminApi | OPT | pass |
| Platform | MessageApi | OPT | excused (unrealized on this technology profile) |
| Platform | Signing | OPT | pass |
| Platform | SimplifiedFormats | OPT | pass |
| Security | EhrDemographicSeparation | Y | pass |
| Security | AuthenticatedAccess | Y | pass |
| Security | AuthorizationSeparation | Y | pass |
| Security | AuditAccountability | Y | pass |
| Security | AnonymousEhrs | Y | pass |

## Workload Coverage

The exercised-capability set of the measured hospital-simulation workload against the claimed matrix — a claimed capability the simulation never touches is a gap in the journey catalogue, listed explicitly.

| Capability | Claimed | Exercised by workload |
| --- | --- | --- |
| Adl14ArchetypeProvisioning | yes | NO — catalogue gap |
| Adl14OptProvisioning | yes | yes |
| Adl2ArchetypeProvisioning | yes | NO — catalogue gap |
| Adl2OptProvisioning | yes | NO — catalogue gap |
| QueryProvisioning | yes | yes |
| EhrOperations | yes | yes |
| EhrStatus | yes | yes |
| CompositionOps | yes | yes |
| DirectoryOps | yes | yes |
| ChangeSets | yes | yes |
| Versioning | yes | yes |
| ArchetypeValidation | yes | yes |
| PartyOperations | yes | NO — catalogue gap |
| PartyRelationshipOperations | yes | NO — catalogue gap |
| AqlBasic | yes | yes |
| AqlAdvanced | yes | NO — catalogue gap |
| AqlTerminology | yes | NO — catalogue gap |
| ActivityReport | yes | NO — catalogue gap |
| PhysicalDeletion | yes | NO — catalogue gap |
| EhrDumpLoad | yes | NO — catalogue gap |
| EhrArchive | yes | NO — catalogue gap |
| DemographicArchive | yes | NO — catalogue gap |
| EhrExtract | yes | NO — catalogue gap |
| Tds | yes | NO — catalogue gap |
| DefinitionApi | yes | yes |
| EhrApi | yes | yes |
| DemographicApi | yes | NO — catalogue gap |
| QueryApi | yes | yes |
| AdminApi | yes | NO — catalogue gap |
| MessageApi | yes | NO — catalogue gap |
| Signing | yes | NO — catalogue gap |
| SimplifiedFormats | yes | NO — catalogue gap |
| EhrDemographicSeparation | yes | NO — catalogue gap |
| AuthenticatedAccess | yes | NO — catalogue gap |
| AuthorizationSeparation | yes | NO — catalogue gap |
| AuditAccountability | yes | NO — catalogue gap |
| AnonymousEhrs | yes | NO — catalogue gap |

Claimed capabilities the simulation never touches (24): Adl14ArchetypeProvisioning, Adl2ArchetypeProvisioning, Adl2OptProvisioning, PartyOperations, PartyRelationshipOperations, AqlAdvanced, AqlTerminology, ActivityReport, PhysicalDeletion, EhrDumpLoad, EhrArchive, DemographicArchive, EhrExtract, Tds, DemographicApi, AdminApi, MessageApi, Signing, SimplifiedFormats, EhrDemographicSeparation, AuthenticatedAccess, AuthorizationSeparation, AuditAccountability, AnonymousEhrs. Each is either a journey-catalogue gap to close or a capability outside the measured-load surface (admin, demographics, messaging, security posture — exercised by the functional schedule, not the load instrument).

## Performance Rating

Classes are EARNED by measurement (never declared); every earned class is bound to the measured environment recorded in the results.

| Class | Case | Claimed | Verdict |
| --- | --- | --- | --- |
| POC | PERF-hospital_sim-class_POC | yes | not earned |

Environment (PERF-hospital_sim-class_POC): consumer-laptop · 8 cores · 16 GB · nvme · single-node docker compose (8-CPU/8GB Docker VM) on Apple M2

