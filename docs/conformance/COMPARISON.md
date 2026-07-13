# openEHR CDR conformance comparison (generated)

> **Measured, not asserted.** Every cell below is derived from a committed `results.json`; nothing here is hand-entered.
>
> - Foreign SUTs are triaged through a committed fairness register before publication: an ehrbase-rs *extension* route (a capability the SUT does not implement) reads `not-applicable`, never `fail`; a genuine spec gap reads `fail`.
>
> - This published comparison makes **no certification claim on behalf of any other vendor**: each cell is a capability result computed from that SUT's own run, never a certificate reference. (Each run does produce its own self-assessment Certificate, which is that operator's to publish, not ours.)
>
> - Where a comparison SUT out-performs ehrbase-rs on a capability, its cell reads `pass` while ours reads `fail`/`not-evidenced` — stated plainly, not hidden.

## Systems under test

| # | Product | Class | Base URL | Run date | Edition level |
|--:|---|---|---|---|---|
| 1 | ehrbase-rs ehrbase-rs 3.0.0 | ours | `http://localhost:8080/ehrbase/rest/openehr/v1` | 2026-07-13T15:05:02.514176Z | pinned (development) |
| 2 | ehrbase-java EHRbase upstream | foreign | `http://localhost:8091/ehrbase/rest/openehr/v1` | 2026-07-13T15:05:42.186992Z | findings: release-1.0.3 |

## Capability comparison

| Capability | ehrbase-rs | ehrbase-java |
|---|---|---|
| Adl14ArchetypeProvisioning | pass | pass |
| Adl14OptProvisioning | pass | **fail** |
| EhrOperations | pass | **fail** |
| EhrStatus | pass | **fail** |
| CompositionOps | pass | **fail** |
| ChangeSets | pass | **fail** |
| Versioning | pass | pass |
| ArchetypeValidation | pass | **fail** |
| DirectoryOps | pass | pass |
| QueryProvisioning | pass | pass |
| AqlBasic | pass | **fail** |
| AqlAdvanced | pass | pass |
| PartyOperations | pass | not-applicable |
| PartyRelationshipOperations | pass | not-applicable |
| AdminActivityReport | not-evidenced | not-evidenced |
| AdminPhysicalDeletion | pass | **fail** |
| AdminEhrDumpLoad | not-evidenced | not-evidenced |
| AdminEhrArchive | not-evidenced | not-evidenced |
| AdminDemographicArchive | not-evidenced | not-evidenced |
| MessagingEhrExtract | not-evidenced | not-evidenced |
| MessagingTds | not-evidenced | not-evidenced |
| Signing | pass | not-applicable |
| AnonymousEhrs | pass | pass |
| Authentication | pass | pass |
| Terminology | pass | **fail** |

_Cells: `pass` (evidenced), `**fail**` (a conformance finding or transport error), `not-applicable` (adjudicated extension / RM-version-sensitive, fairness register), `not-evidenced` (only skipped cases), `—` (no case exercises it for that SUT)._
