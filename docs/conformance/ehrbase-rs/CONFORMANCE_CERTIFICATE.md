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
| Platform | DemographicArchetypeValidation | OPT | released-wire | no cases |
| Platform | AqlBasic | Y | released-wire | pass |
| Platform | AqlAdvanced | OPT | released-wire | pass |
| Platform | AqlTerminology | OPT | released-wire | pass |
| Platform | ActivityReport | OPT | released-wire | excused (unrealized on this technology profile) |
| Platform | PhysicalDeletion | OPT | released-wire | pass |
| Platform | EhrDumpLoad | OPT | released-wire | excused (unrealized on this technology profile) |
| Platform | BulkEhrLoad | OPT | released-wire | no cases |
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
| Adl14ArchetypeProvisioning | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| Adl14OptProvisioning | yes | yes |
| Adl2ArchetypeProvisioning | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| Adl2OptProvisioning | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| TemplateExamples | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| QueryProvisioning | yes | yes |
| EhrOperations | yes | yes |
| EhrStatus | yes | yes |
| CompositionOps | yes | yes |
| DirectoryOps | yes | yes |
| ChangeSets | yes | yes |
| Versioning | yes | yes |
| ArchetypeValidation | yes | yes |
| PartyOperations | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| PartyRelationshipOperations | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| AqlBasic | yes | yes |
| AqlAdvanced | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| AqlTerminology | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| ActivityReport | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| PhysicalDeletion | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| EhrDumpLoad | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| EhrArchive | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| DemographicArchive | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| EhrExtract | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| Tds | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| DefinitionApi | yes | yes |
| EhrApi | yes | yes |
| DemographicApi | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| QueryApi | yes | yes |
| AdminApi | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| MessageApi | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| SystemApi | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| ItemTags | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| Signing | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| SimplifiedFormats | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| SmartAppLaunch | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| EhrDemographicSeparation | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| AuthenticatedAccess | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| AuthorizationSeparation | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| AuditAccountability | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |
| AnonymousEhrs | yes | no — adjudicated exclusion (AMB-170): not yet exercised by the hospital simulation - pending journey-catalogue extension (#625) |

Claimed capabilities excluded from the measured workload by adjudication (28): Adl14ArchetypeProvisioning, Adl2ArchetypeProvisioning, Adl2OptProvisioning, TemplateExamples, PartyOperations, PartyRelationshipOperations, AqlAdvanced, AqlTerminology, ActivityReport, PhysicalDeletion, EhrDumpLoad, EhrArchive, DemographicArchive, EhrExtract, Tds, DemographicApi, AdminApi, MessageApi, SystemApi, ItemTags, Signing, SimplifiedFormats, SmartAppLaunch, EhrDemographicSeparation, AuthenticatedAccess, AuthorizationSeparation, AuditAccountability, AnonymousEhrs. Each row above names its register entry; the exclusion bounds the LOAD instrument only — the functional catalogue still owes every one of them verdict-bearing cases at its `min_cases` floor.

Every claimed capability is exercised by the simulation or carries an adjudicated exclusion — no undecided rows.

## Performance Rating

Classes are EARNED by measurement (never declared); every earned class is bound to the measured environment recorded in the results.

| Class | Case | Claimed | Verdict |
| --- | --- | --- | --- |
| POC | PERF-hospital_sim-class_POC | yes | EARNED |

Environment (PERF-hospital_sim-class_POC): consumer-laptop · 8 cores · 16 GB · nvme · single-node docker compose (8-CPU/8GB Docker VM) on Apple M2

