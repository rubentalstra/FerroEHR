# Conformance Certificate

## System Under Test

| Field | Value |
| --- | --- |
| Solution | ehrbase-rs 3.11.0 |
| Vendor | Ruben Talstra |
| Runner | cnf-runner 3.11.0 |
| Infrastructure | ixit.json#/environment |

## Scope of Test

| Dimension | Value |
| --- | --- |
| Functional | CORE, STANDARD, OPTIONS, SEC-BASIC |
| Sec & Priv | SEC-BASIC PASS |
| Performance | class POC (earned) |
| Ext Data Fmt | canonical-json, canonical-xml, wt-flat, wt-structured |

## Profile Report

Result column: ITS its-rest (canonical-json, canonical-xml, wt-flat, wt-structured)

The Realization column says what the row's cases were verified against: `released-wire` = released ITS-REST operations; `extension` = routes this product serves of its own design, which no openEHR specification governs and which therefore never gate an openEHR profile tier (those rows are always OPT).

| Family | Capability | Required in profile | Realization | Result |
| --- | --- | --- | --- | --- |
| Platform | Adl14ArchetypeProvisioning | Y | released-wire | excused (unrealized on this technology profile) |
| Platform | Adl14OptProvisioning | Y | released-wire | pass |
| Platform | Adl2ArchetypeProvisioning | OPT | released-wire | excused (unrealized on this technology profile) |
| Platform | Adl2OptProvisioning | OPT | released-wire | pass |
| Platform | TemplateExamples | OPT | released-wire | pass |
| Platform | QueryProvisioning | Y | released-wire | pass |
| Platform | EhrOperations | Y | released-wire | pass |
| Platform | EhrStatus | Y | released-wire | pass |
| Platform | CompositionOps | Y | released-wire | pass |
| Platform | DirectoryOps | Y | released-wire | pass |
| Platform | ChangeSets | Y | released-wire | pass |
| Platform | Versioning | Y | released-wire | pass |
| Platform | ArchetypeValidation | Y | released-wire | pass |
| Platform | PartyOperations | OPT | released-wire | pass |
| Platform | PartyRelationshipOperations | OPT | extension | pass |
| Platform | DemographicArchetypeValidation | OPT | released-wire | pass |
| Platform | AqlBasic | Y | released-wire | pass |
| Platform | AqlAdvanced | OPT | released-wire | pass |
| Platform | AqlTerminology | OPT | released-wire | pass |
| Platform | ActivityReport | OPT | released-wire | excused (unrealized on this technology profile) |
| Platform | PhysicalDeletion | OPT | released-wire | pass |
| Platform | EhrDumpLoad | OPT | released-wire | excused (unrealized on this technology profile) |
| Platform | BulkEhrLoad | OPT | released-wire | pass |
| Platform | EhrArchive | OPT | released-wire | excused (unrealized on this technology profile) |
| Platform | DemographicArchive | OPT | released-wire | excused (unrealized on this technology profile) |
| Platform | EhrExtract | OPT | released-wire | excused (unrealized on this technology profile) |
| Platform | Tds | OPT | released-wire | excused (unrealized on this technology profile) |
| Platform | DefinitionApi | Y | released-wire | pass |
| Platform | EhrApi | Y | released-wire | pass |
| Platform | DemographicApi | OPT | released-wire | pass |
| Platform | QueryApi | Y | released-wire | pass |
| Platform | AdminApi | OPT | released-wire | pass |
| Platform | MessageApi | OPT | released-wire | excused (unrealized on this technology profile) |
| Platform | SystemApi | OPT | released-wire | pass |
| Platform | ItemTags | OPT | released-wire | pass |
| Platform | Signing | OPT | released-wire | pass |
| Platform | SimplifiedFormats | OPT | released-wire | pass |
| Platform | SmartAppLaunch | OPT | released-wire | pass |
| Security | EhrDemographicSeparation | Y | released-wire | pass |
| Security | AuthenticatedAccess | Y | released-wire | pass |
| Security | AuthorizationSeparation | Y | released-wire | pass |
| Security | AuditAccountability | Y | released-wire | pass |
| Security | AnonymousEhrs | Y | released-wire | pass |

## Workload Coverage

The exercised-capability set of the measured hospital-simulation workload against the claimed matrix. A claimed capability the simulation never touches is either an ADJUDICATED exclusion — the capability-matrix row names the register entry that decided it and the reason is printed in the row — or an undecided catalogue gap, which the `workload-coverage` validate gate fails on, so no published certificate reaches this section carrying one.

| Capability | Claimed | Exercised by workload |
| --- | --- | --- |
| Adl14ArchetypeProvisioning | yes | no — adjudicated exclusion (AMB-170): no request exists for the load instrument to send: ITS-REST 1.1.0 defines no wire for this capability and this product serves no extension route for it either, so the hospital simulation has nothing to drive - the exclusion lapses the day the routes exist |
| Adl14OptProvisioning | yes | yes |
| Adl2ArchetypeProvisioning | yes | no — adjudicated exclusion (AMB-170): no request exists for the load instrument to send: ITS-REST 1.1.0 defines no wire for this capability and this product serves no extension route for it either, so the hospital simulation has nothing to drive - the exclusion lapses the day the routes exist |
| Adl2OptProvisioning | yes | NO — catalogue gap (UNADJUDICATED) |
| TemplateExamples | yes | NO — catalogue gap (UNADJUDICATED) |
| QueryProvisioning | yes | yes |
| EhrOperations | yes | yes |
| EhrStatus | yes | yes |
| CompositionOps | yes | yes |
| DirectoryOps | yes | yes |
| ChangeSets | yes | yes |
| Versioning | yes | yes |
| ArchetypeValidation | yes | yes |
| PartyOperations | yes | NO — catalogue gap (UNADJUDICATED) |
| PartyRelationshipOperations | yes | NO — catalogue gap (UNADJUDICATED) |
| DemographicArchetypeValidation | yes | NO — catalogue gap (UNADJUDICATED) |
| AqlBasic | yes | yes |
| AqlAdvanced | yes | NO — catalogue gap (UNADJUDICATED) |
| AqlTerminology | yes | NO — catalogue gap (UNADJUDICATED) |
| ActivityReport | yes | no — adjudicated exclusion (AMB-170): no request exists for the load instrument to send: ITS-REST 1.1.0 defines no wire for this capability and this product serves no extension route for it either, so the hospital simulation has nothing to drive - the exclusion lapses the day the routes exist |
| PhysicalDeletion | yes | no — adjudicated exclusion (AMB-170): destructive mid-measurement: the released admin operations erase an EHR and its whole version history outright, so every arrival would shrink the population-anchored corpus the measured window is bound to and the earned class would no longer describe the declared volumetric class |
| EhrDumpLoad | yes | no — adjudicated exclusion (AMB-170): no request exists for the load instrument to send: ITS-REST 1.1.0 defines no wire for this capability and this product serves no extension route for it either, so the hospital simulation has nothing to drive - the exclusion lapses the day the routes exist; a whole-server export/import is also a one-shot operation, never a sustainable arrival |
| BulkEhrLoad | yes | no — adjudicated exclusion (AMB-170): the bulk load IS every measured run's own seeding phase: the scale corpus is loaded strictly through the public API before the window opens, so the capability is exercised per-run by the seeder - as a sustained ARRIVAL it would be a second population grower distorting the population-anchored envelope, the same one-shot family as a whole-server dump/load |
| EhrArchive | yes | no — adjudicated exclusion (AMB-170): no request exists for the load instrument to send: ITS-REST 1.1.0 defines no wire for this capability and this product serves no extension route for it either, so the hospital simulation has nothing to drive - the exclusion lapses the day the routes exist |
| DemographicArchive | yes | no — adjudicated exclusion (AMB-170): no request exists for the load instrument to send: ITS-REST 1.1.0 defines no wire for this capability and this product serves no extension route for it either, so the hospital simulation has nothing to drive - the exclusion lapses the day the routes exist |
| EhrExtract | yes | no — adjudicated exclusion (AMB-170): no request exists for the load instrument to send: ITS-REST 1.1.0 defines no wire for this capability and this product serves no extension route for it either, so the hospital simulation has nothing to drive - the exclusion lapses the day the routes exist |
| Tds | yes | no — adjudicated exclusion (AMB-170): no request exists for the load instrument to send: ITS-REST 1.1.0 defines no wire for this capability and this product serves no extension route for it either, so the hospital simulation has nothing to drive - the exclusion lapses the day the routes exist |
| DefinitionApi | yes | yes |
| EhrApi | yes | yes |
| DemographicApi | yes | NO — catalogue gap (UNADJUDICATED) |
| QueryApi | yes | yes |
| AdminApi | yes | no — adjudicated exclusion (AMB-170): destructive mid-measurement: the released ADMIN API of ITS-REST 1.1.0 is exactly admin_ehr_delete and admin_ehr_delete_all, so the only wire this claim covers is the same corpus-erasing pair PhysicalDeletion excludes (the further /admin routes this server mounts are extensions no claim rests on) |
| MessageApi | yes | no — adjudicated exclusion (AMB-170): no request exists for the load instrument to send: ITS-REST 1.1.0 defines no wire for this capability and this product serves no extension route for it either, so the hospital simulation has nothing to drive - the exclusion lapses the day the routes exist |
| SystemApi | yes | NO — catalogue gap (UNADJUDICATED) |
| ItemTags | yes | yes |
| Signing | yes | NO — catalogue gap (UNADJUDICATED) |
| SimplifiedFormats | yes | NO — catalogue gap (UNADJUDICATED) |
| SmartAppLaunch | yes | NO — catalogue gap (UNADJUDICATED) |
| EhrDemographicSeparation | yes | yes |
| AuthenticatedAccess | yes | NO — catalogue gap (UNADJUDICATED) |
| AuthorizationSeparation | yes | NO — catalogue gap (UNADJUDICATED) |
| AuditAccountability | yes | yes |
| AnonymousEhrs | yes | yes |

Claimed capabilities excluded from the measured workload by adjudication (12): Adl14ArchetypeProvisioning, Adl2ArchetypeProvisioning, ActivityReport, PhysicalDeletion, EhrDumpLoad, BulkEhrLoad, EhrArchive, DemographicArchive, EhrExtract, Tds, AdminApi, MessageApi. Each row above names its register entry; the exclusion bounds the LOAD instrument only — the functional catalogue still owes every one of them verdict-bearing cases at its `min_cases` floor.

UNADJUDICATED gaps (14): Adl2OptProvisioning, TemplateExamples, PartyOperations, PartyRelationshipOperations, DemographicArchetypeValidation, AqlAdvanced, AqlTerminology, DemographicApi, SystemApi, Signing, SimplifiedFormats, SmartAppLaunch, AuthenticatedAccess, AuthorizationSeparation. These rows are a defect in this submission, not a property of the product: the `workload-coverage` validate gate fails on each of them, so this certificate was rendered from an artifact tree that does not pass its own gates.

## Performance Rating

Classes are EARNED by measurement (never declared); every earned class is bound to the measured environment recorded in the results.

| Class | Case | Claimed | Verdict |
| --- | --- | --- | --- |
| POC | PERF-hospital_sim-class_POC | yes | EARNED |

Environment (PERF-hospital_sim-class_POC): consumer-laptop · 8 cores · 16 GB · nvme · single-node docker compose (8-CPU/8GB Docker VM) on Apple M2

