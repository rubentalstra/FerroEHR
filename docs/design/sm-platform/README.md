# SM Platform Service Model — per-chapter compliance design register

Owner directive 2026-07-12: the whole vendored SM `openehr_platform`
specification (not only the Subject Proxy chapter) audited chapter-by-chapter
against the implementation, one design/gap document per chapter, because the
service layer must follow the official specs properly and completely.

**Oracle:** `docs/specs/openehr/SM/docs/openehr_platform/master*.adoc` — each
chapter's normative content is mostly in the `UML/classes/*.adoc` files it
`include::`s; every document below cites the chapter **and** the class files
it read. Method and document template follow
[`10-subject-proxy.md`](10-subject-proxy.md) (the W-3c pilot): verified
current state (file:line), a cited gap register (`G-n` rows), target design,
and the honest PORT-NOTE residue.

| Chapter | Document | Implementation seam |
|---|---|---|
| master02 — Platform Overview | [02-overview.md](02-overview.md) | crate/component map (`ehrbase-sm` / `ehrbase` / `ehrbase-rest`) |
| master03 — Common Package | [03-common.md](03-common.md) | `ehrbase-sm` error/status model (`CALL_STATUS`, …) |
| master04 — Definition Package | [04-definition.md](04-definition.md) | `DefinitionAdl14Service` / `DefinitionAdl2Service` / `DefinitionQueryService` |
| master05 — EHR Service | [05-ehr.md](05-ehr.md) | `EhrService` + status/composition/directory/contribution traits |
| master06 — Demographic Service | [06-demographic.md](06-demographic.md) | `DemographicService` |
| master07 — EHR Index Service | [07-ehr-index.md](07-ehr-index.md) | `EhrIndexService` |
| master08 — Query Service | [08-query.md](08-query.md) | `QueryService` + AQL engine seam |
| master09 — Message Service | [09-message.md](09-message.md) | `MessageService` / `EhrExtractService` / `TddService` |
| master10 — Subject Proxy Service | [10-subject-proxy.md](10-subject-proxy.md) | `SubjectProxyService` / `DataBinding` (**W-3c**) |
| master12 — Terminology Service | [12-terminology.md](12-terminology.md) | `TerminologyService` (+ B4 FHIR provider) |
| master15 — Admin Service | [15-admin.md](15-admin.md) | `AdminService` (+ dump/load) |

Not registered (no requirements): master00 amendment record, master01
preface. Chapter numbers 11, 13, 14 do not exist in the vendored spec
(development-edition numbering gaps — a spec-side observation, not ours).

Open gaps found by these documents are executed through
`docs/plans/WORKLIST.md` (W-3c for chapter 10; new rows as the registers
produce them) — a gap lives in code or a re-verified cited PORT NOTE, never
only in this folder.
