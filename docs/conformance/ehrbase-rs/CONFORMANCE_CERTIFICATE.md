# Conformance Certificate

## System Under Test

| Field | Value |
| --- | --- |
| Solution | ehrbase-rs 3.6.0 |
| Vendor | Ruben Talstra |
| Runner | cnf-runner 3.6.0 |
| Infrastructure | — |

## Scope of Test

| Dimension | Value |
| --- | --- |
| Functional | CORE, STANDARD, OPTIONS, SEC-BASIC |
| Sec & Priv | SEC-BASIC PASS |
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
| Platform | ArchetypeValidation | Y | FAIL |
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
| Platform | DemographicApi | OPT | no cases |
| Platform | QueryApi | Y | pass |
| Platform | AdminApi | OPT | no cases |
| Platform | MessageApi | OPT | no cases |
| Platform | Signing | OPT | pass |
| Platform | SimplifiedFormats | OPT | FAIL |
| Security | EhrDemographicSeparation | Y | pass |
| Security | AuthenticatedAccess | Y | pass |
| Security | AuthorizationSeparation | Y | pass |
| Security | AuditAccountability | Y | pass |
| Security | AnonymousEhrs | Y | pass |

