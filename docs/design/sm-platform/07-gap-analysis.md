# SM design — gap analysis: ehrbase-rs vs the Platform Service Model

Part of the SM-platform design set (`docs/design/sm-platform/README.md`).
Left side: the SM component/interface model (digests 1–6). Right side: the
current workspace (P09–P16 merged + the P16 branch), surveyed 2026-07-08.
Scope ruling (owner, 2026-07-08): **every SM service is in scope — nothing
is deferred.** Rows marked *gap* are build items in doc 09, not exclusions.

Verdict up front: the application already *is* an SM-shaped platform for the
EHR core — `Backend` = the interface bundle, `EhrbaseService` = the
component, `vobject` = the UPDATE_VERSION/CONTRIBUTION engine, `ehrbase-rest`
= the REST protocol adapter. What is missing is (a) the explicit,
spec-named service-trait layer, and (b) six whole components
(EHR Index, Terminology surface, Message/Extract/TDD, Subject Proxy,
Admin-archive/dump-load, Demographic PARTY_RELATIONSHIP).

## 1. Component-by-component

### 1.1 Definitions service (`I_DEFINITION_ADL14` / `ADL2` / `QUERY`, `I_VALIDITY_CHECKER`)

| SM call | Current state | Gap |
|---|---|---|
| `has/upload/get/list/delete_archetype` (ADL 1.4) | **gap** — we store OPTs, not source archetypes | archetype store + list/delete |
| `has/valid/upload/get/list_opts/delete_opt` | `service/template.rs`: `store_template` (409 duplicate, structural validation), `get_template_meta/xml`, `list_templates` | `delete_opt` missing (admin/template delete); `valid_opt` exists implicitly (parse+validate) — expose as call |
| `list_matching_*` (regex), `*_count` | **gap** | pattern listing + counts |
| ADL2 artefact family | DEFINITION `adl2` = 501 | full ADL2 ingest (openehr-am `am24` types exist; service missing) |
| `has/valid/store/list/list_matching/delete_query`, `queries_count` | `service/stored_query.rs`: store (immutability + semver), get (exact/prefix/latest), list (pattern) | `valid_query` as a standalone call; `delete_query`; `queries_count`; `store_query_set` (spec itself TODO — fill contract by design) |
| `I_VALIDITY_CHECKER.definitions_valid/content_valid` | exists as internal choke points: `validate_for_commit`, `validate_composition_for_commit` (RM + terminology + WebTemplate) | expose as a named trait so preconditions cite it |

### 1.2 EHR service (`I_EHR_SERVICE`, `I_EHR`, `I_EHR_STATUS`, `I_EHR_DIRECTORY`, `I_EHR_COMPOSITION`, `I_EHR_CONTRIBUTION`)

The strongest area — near-complete:

| SM call family | Current state |
|---|---|
| `has_ehr`, `create_ehr(_with_id)`, default EHR_STATUS (`is_modifiable`/`is_queryable` True, PARTY_SELF subject) | `service/ehr.rs::create_ehr` + `default_ehr_status()` — matches the SM default rule |
| `has/get_ehrs_for_subject`, `create_ehr_for_subject(_with_id)` | `ehr_by_subject` + subject promotion (`ehr_subject_uq` → 409 = `ehr_for_subject_already_exists`) |
| `get_ehr: EHR_SUMMARY` | `ehr_summary` (EHR root + status); SM's `contribution_count`/`composition_count` fields not included — add |
| EHR_STATUS get/at-time/at-version/versioned + flag mutations | `status_*` family; flags mutate via full status update (SM's dedicated set/clear calls map onto it) |
| Directory CRUD + at-time/at-version/versioned + `has_path` | `service/directory.rs` (incl. sub-folder path navigation, optimistic lock) |
| Composition CRUD + at-time/at-version/versioned + logical delete `523\|deleted\|` | `service/composition.rs` — delete semantics match verbatim |
| `commit_contribution(versions, audit)` | `service/contribution.rs::create_contribution`/`commit_version_set` — atomic multi-version, change-type classification against the full `audit_change_type` group |
| `list_contributions`/`contribution_count` (time-range + paging) | `get_contribution` exists; **list + count with time-range/paging are gaps** |
| UPDATE_VERSION semantics (preceding uid except first, lifecycle mandatory, server-side time/system_id, UPDATE_AUDIT partial) | `vobject::Change` + `AuditInput` + If-Match enforcement — semantics match; the ITS-REST wire (headers/body) is the concrete UPDATE_VERSION realization |
| Attestations (`UPDATE_VERSION.attestations`) | contribution path **rejects** `666\|attestation\|` — gap to build |
| Branch version ids | trunk-only (F-06-09) — SM silent on branches; keep, note |

### 1.3 Demographic service

| SM call | Current state | Gap |
|---|---|---|
| `create/get/update/delete_party` + at-time/at-version + versioned | `service/demographic.rs` full PARTY CRUD (5 kinds), revision history, demographic contributions, tags | — |
| `I_PARTY_RELATIONSHIP` family | **gap** — no PARTY_RELATIONSHIP support | model + CRUD + versioning (RM class exists in `openehr-rm`) |
| `UV_PARTY` semantics | via shared `vobject` | — |

### 1.4 EHR Index service (`I_EHR_INDEX`)

**Whole component gap.** Today: `ehr.subject_id/namespace` promotion gives a
1:1 subject↔EHR lookup with uniqueness. SM requires an N:M association store
with `RESOURCE_STATUS` metadata (Primary/Duplicate/Supplementary),
`LOCATION_DESC`, and the five maintenance calls — explicitly to *manage*
duplicate-EHR / multi-subject error conditions. Build item: `ehr_index`
table + service; our current unique-subject behaviour becomes the
Primary-instance fast path.

### 1.5 Query service (`I_QUERY_SERVICE`)

| SM aspect | Current state |
|---|---|
| `execute_ad_hoc_query` / `execute_stored_query` (+ params, paging) | P16 `QueryService` + AQL engine + all six `/query/*` endpoints |
| RESULT_SET (columns/rows/meta) | ITS-REST 1.0.3 RESULT_SET (the concrete realization; SM's `id`/`creation_time`/`RESULT_QUERY_DESCRIPTOR` fields — audit against wire shape) |
| population scope = `is_queryable = True` | enforced in AQL exec scope — verify + test explicitly (CNF cross-check) |
| `formalism` default `"aql"` | AQL-only; other formalisms rejected typed — matches "matching one of: aql; any other string" |

### 1.6 Terminology service (`I_TERMINOLOGY_SERVICE`)

**Surface gap.** `openehr-term::bundle` implements the openEHR vocabulary
internally (validators, code sets, units) but exposes no service interface:
no `get_terminology_ids`, `get_term`, `subsumes`, `value_set_validate`,
`Terminology_extract`. Build item: a `terminology` service trait over the
bundle (openEHR terminology first, external terminologies via the
FHIR-terminology adapter later — same trait).

### 1.7 Message service (`I_MESSAGE_SERVICE`, `I_EHR_EXTRACT_SERVICE`, `I_TDD_SERVICE`)

**Whole component gap — in scope (owner decision).**
- `I_EHR_EXTRACT_SERVICE`: the RM `ehr_extract` types (`EXTRACT`,
  `EXTRACT_SPEC`, `X_VERSIONED_*`, …) are **already generated** in
  `openehr-rm` with canonical JSON + XML (verified 2026-07-09, review F1) —
  the gap is only the service: export (whole-EHR, spec-driven extract) and
  import (whole-EHR with fixed id, extract-into-existing-EHR) over the
  existing `vobject` machinery.
- `I_TDD_SERVICE`: TDD import = template-data-document → COMPOSITION commit;
  builds on OPT + WebTemplate assets.
- `I_MESSAGE_SERVICE` is a spec stub — the design fills it as the umbrella
  trait over extract + TDD.

### 1.8 System Log service (`I_SYSTEM_LOG`)

**Done.** `ehrbase-audit` (ATNA/DICOM audit over syslog, total-coverage op
classification, fail-open/closed) realizes the "IHE ATNA-compliant system
log" component. Remaining: name it in the SM facade so the component map is
explicit.

### 1.9 Subject Proxy service (`I_SUBJECT_PROXY_SERVICE`, `I_DATA_BINDING`)

**Whole component gap — in scope.** Nothing exists. Needs: subject/variable/
data-set/binding stores, the sample/value hierarchy, an `I_DATA_BINDING`
executor with an openEHR data-frame implementation (AQL over our own Query
service = `OPENEHR_SAMPLE` with `result: RESULT_SET`) and adapter seams for
FHIR/HL7v2 frames. The `DATA_FRAME.primary_method: SYSTEM_CALL` type comes
from the PROC specs (not vendored) — fill the contract by design with
`// PORT NOTE:`.

### 1.10 Admin service (`I_ADMIN_SERVICE`, `I_ADMIN_ARCHIVE`, `I_ADMIN_DUMP_LOAD`)

| SM call | Current state | Gap |
|---|---|---|
| `physical_ehr_delete` | `service/admin.rs` (FK-cascade + orphan audit cleanup) | — |
| `physical_party_delete` | **gap** | party + relationships physical delete |
| `list_contributions`/`*_count` per `PLATFORM_SERVICE` | **gap** | admin statistics queries |
| `archive_ehrs`/`archive_parties` | **gap** | archival storage tier |
| `export_ehrs`/`load_ehrs` (+ `EXPORT_SPEC`, canonical XML/JSON, zip/7z, segmenting, `DUMP_LOAD_FAIL_REPORT`) | **gap** | dump/load subsystem |

### 1.11 Cross-cutting: `I_STATUS` / `CALL_STATUS` error model

SM's stateful `last_call_failed()` is explicitly mappable to a stateless
style; our typed-error `Result<_, ServiceError>` → `ApiError` → HTTP is that
mapping. Gap: a documented, single table SM `CALL_STATUS_TYPE`(+ descendants)
↔ `ServiceError` ↔ HTTP status (doc 08 §5) so every abstract error name has
one concrete home. Conflicts always resolve to ITS-REST/CNF at the wire.

## 2. Simplified IM + SDF (digests 5–6) vs `openehr-flat`

- FLAT/STRUCTURED converters + `ctx/` defaults exist (P14, Better parity).
- Build items: audit converters against SIM-B transformation rules +
  APP_CONTEXT vocabulary; accept SDF-normative leaf encodings (interval
  strings, ordinal/scale/proportion forms) alongside Better forms; document
  the three-way quantity divergence (SIM-B space / SDF comma / Better
  `|magnitude`).

## 3. Summary table

| SM component | Status | Doc 09 phase |
|---|---|---|
| Definitions | partial (OPT + queries; no archetypes/ADL2/regex/counts/delete) | SM-2 |
| EHR | near-complete (contribution list/count, attestations, EHR_SUMMARY counts) | SM-1 |
| Demographic | partial (PARTY done; PARTY_RELATIONSHIP missing) | SM-3 |
| EHR Index | missing | SM-3 |
| Query | complete (verify `is_queryable` gate + RESULT_SET meta) | SM-1 |
| Terminology | internal only — service surface missing | SM-4 |
| Message (Extract + TDD) | missing — in scope | SM-5 |
| System Log | **done** (`ehrbase-audit`) | SM-1 (facade naming) |
| Subject Proxy | missing — in scope | SM-6 |
| Admin | partial (physical EHR delete only) | SM-4 |
| SIM-B / SDF alignment | partial (Better parity; SDF/SIM audit pending) | P17 |
