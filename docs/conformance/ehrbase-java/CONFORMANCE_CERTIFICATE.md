# Conformance Certificate

## System Under Test

| Field | Value |
| --- | --- |
| Solution | ehrbase-java 2.34.0 |
| Vendor | EHRbase (vitagroup / upstream open-source project) |
| Runner | cnf-runner 3.7.0 |
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

