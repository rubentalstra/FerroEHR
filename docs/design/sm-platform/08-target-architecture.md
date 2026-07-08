# SM design — target architecture: the SM-aligned platform

Part of the SM-platform design set (`docs/design/sm-platform/README.md`).
This is the greenfield design that makes ehrbase-rs an explicit realization
of the openEHR Platform Service Model (digests 1–6), with full component
coverage (owner ruling 2026-07-08: nothing deferred; EHR_EXTRACT in scope).

## 0. Design stance

1. **SM governs the internal decomposition and call semantics; ITS-REST 1.0.3
   + CNF govern the wire.** SM is TRIAL/DEVELOPMENT; where SM and ITS-REST/
   CNF disagree (naming, paging params, RESULT_SET fields), the wire spec
   wins at the boundary and the SM name is kept internally. Every such
   resolution carries a `// PORT NOTE:` + citation.
2. **Formal equivalence, not literal transcription** (sanctioned by
   `master02-overview.adoc` §Interface Calls): our calls may differ in shape
   from the SM IDL as long as pre/post-conditions match and multi-call
   realizations are transactionally protected. The SM's stateful
   `last_call_status()` maps to our stateless typed-error model — explicitly
   permitted by §Functional Style.
3. **The application stays modern idiomatic Rust of our own design**
   (ADR-006/008). SM service types are hand-written in the app layer (the SM
   component publishes no BMM; its UML is MagicDraw-only), unlike the
   generated `openehr-*` spec crates.

## 1. The assumed architecture, realized

SM's picture — a nominal **native API** behind **protocol adapters** — maps
onto the workspace as:

```
                        ┌───────────────────────────────┐
   protocol adapters    │        native API             │      component
┌──────────────────┐    │  ehrbase-sm (trait layer)     │  ┌────────────────┐
│ ehrbase-rest     │───▶│   EhrService      Definitions │◀─│ ehrbase        │
│  (ITS-REST 1.0.3)│    │   Demographic     Query       │  │  EhrbaseService│
│ ehrbase-compat   │───▶│   EhrIndex        Terminology │  │  (PG18 storage,│
│  (EhrScape/FLAT) │    │   Message         SubjectProxy│  │   vobject, AQL)│
│ mgmt/admin HTTP  │───▶│   Admin           SystemLog   │  └────────────────┘
└──────────────────┘    │   ValidityChecker             │
                        └───────────────────────────────┘
```

**New crate: `ehrbase-sm`** — the native API. Holds the service traits, the
shared service types (update-version envelope, paging, error enums, summary
DTOs), and the SM↔HTTP error table. `ehrbase-rest` (adapter) and `ehrbase`
(component) both depend on it; the current `ehrbase-rest::backend` trait
family migrates there. This fixes today's inversion (the native API living
inside the REST adapter crate) and gives every future adapter (EhrScape,
gRPC, message queue) the same seam. Dependencies stay downward:
`ehrbase-rest → ehrbase-sm → openehr-*`; `ehrbase → ehrbase-sm`.

## 2. Trait layer (one trait per SM interface)

Idiomatic Rust names; each trait's doc-comment cites its SM interface and
per-method the SM call + pre/post-conditions. `#[async_trait]`, methods take
`&self`; every mutating method is one transaction (SM §Interface Calls).

| SM interface | Rust trait (ehrbase-sm) | Notes |
|---|---|---|
| `I_EHR_SERVICE` | `EhrService` | today's seam, + `ehr_summary` gains `contribution_count`/`composition_count` |
| `I_EHR` (accessor) | — flattened | stateless: methods carry `ehr_id`; formal equivalence documented |
| `I_EHR_STATUS` | `EhrStatusService` | split out of today's mega-trait |
| `I_EHR_DIRECTORY` | `EhrDirectoryService` | |
| `I_EHR_COMPOSITION` | `EhrCompositionService` | |
| `I_EHR_CONTRIBUTION` | `EhrContributionService` | + `list_contributions(time_range, page)`, `contribution_count` |
| `I_DEFINITION_ADL14` | `DefinitionAdl14Service` | + archetype store, regex listing, counts, delete |
| `I_DEFINITION_ADL2` | `DefinitionAdl2Service` | replaces the 501s |
| `I_DEFINITION_QUERY` | `DefinitionQueryService` | + `valid_query`, `delete_query`, `queries_count` |
| `I_VALIDITY_CHECKER` | `ValidityChecker` | names today's validation choke points |
| `I_DEMOGRAPHIC_SERVICE`/`I_PARTY` | `DemographicService` | existing |
| `I_PARTY_RELATIONSHIP` | `PartyRelationshipService` | new |
| `I_EHR_INDEX` | `EhrIndexService` | new component |
| `I_QUERY_SERVICE` | `QueryService` | existing |
| `I_TERMINOLOGY_SERVICE` | `TerminologyService` | new surface over `openehr-term` |
| `I_MESSAGE_SERVICE` | `MessageService` | umbrella (spec stub → filled by design) |
| `I_EHR_EXTRACT_SERVICE` | `EhrExtractService` | new; RM `ehr_extract` generated from BMM |
| `I_TDD_SERVICE` | `TddService` | new |
| `I_SUBJECT_PROXY_SERVICE` | `SubjectProxyService` | new component |
| `I_DATA_BINDING` | `DataBinding` | internal trait; openEHR frame impl first |
| `I_ADMIN_SERVICE`/`_ARCHIVE`/`_DUMP_LOAD` | `AdminService`, `AdminArchive`, `AdminDumpLoad` | extend existing |
| `I_SYSTEM_LOG` | `SystemLog` | facade naming over `ehrbase-audit` |
| `I_STATUS` | — | realized by the typed-error model (§5) |

`Backend` remains the composition alias (`EhrService + EhrStatusService +
… + AdminService + …`) so adapters keep one state type; splitting the
mega-trait is a mechanical migration of today's methods.

## 3. Shared service types (`ehrbase-sm::types`)

- **`UpdateVersion<T>`** (`UPDATE_VERSION<T>`): `preceding_version_uid:
  Option<ObjectVersionId>`, `lifecycle_state: TerminologyCode`,
  `attestations: Option<Vec<Attestation>>`, `data: T`, `audit: UpdateAudit`.
  Today's `vobject::Change` + per-call args become constructors of this; the
  ITS-REST adapter builds it from body + headers (`If-Match`, `Prefer`,
  committer) — the wire *is* the update-version envelope, now named.
- **`UpdateAudit`** (`UPDATE_AUDIT`): `change_type`, `description?`,
  `committer: PartyProxy` (today's `AuditInput`, invariant
  `Change_type_valid` enforced via `openehr-term`).
- **`EhrSummary`** (`EHR_SUMMARY`): + `contribution_count`,
  `composition_count`.
- **`Page`** (`item_offset`, `items_to_fetch`; 0 ⇒ all) — the SM cursor
  convention, used by every unbounded list call.
- Query: `StoredQueryExecuteSpec`, `AdhocQueryExecuteSpec` (formalism
  default `"aql"`), `QueryDescriptor`/`ResultQueryDescriptor`.
- EHR Index: `ResourceStatus` (`instance_type`, `start_valid_time:
  Option<Timestamp>`, `end_valid_time: Option<Timestamp>` — `@@` in spec,
  fixed as ISO date-time by `// PORT NOTE:`), `ResourceInstanceType`
  {Primary, Duplicate, Supplementary}, `LocationDesc` (spec stub — designed:
  `system_id: String`, `uri: Option<String>`, `description: Option<String>`,
  `// PORT NOTE:` on the filled contract).
- Message: `ExportSpec`, `ExportFormat` {CanonicalXml, CanonicalJson},
  `CompressionFormat` {Zip, SevenZ}, `EncodingFormat` (spec enum empty —
  designed: {Utf8}, extensible), `DumpLoadFailReport`.
- Terminology: `TerminologyDescription`, `TerminologyExtract`, `TermCode`,
  `DefinedTerm`, `TermRelationship`, `TerminologyRelation` (xor invariant
  enforced in constructor).
- Subject Proxy: `SubjectProxy`, `SubjectVariable`, `SubjectDataSet`,
  `DataSetResult`, `Sample<T>`, `DataFrameSample`, `OpenehrSample`
  (`result: ResultSet`), `VariableSample`, `VariableValue`
  {Single, List, TimeSeries}, `EnvBinding`, `DataFrame` (method fields typed
  as our own `RetrieveCall` enum {Api{system_id, call_name, parameters},
  Query{system_id, query_text}} — the PROC `SYSTEM_CALL` is not vendored;
  `// PORT NOTE:`).

## 4. Component designs (new pieces)

### 4.1 EHR Index (`EhrIndexService`)

New table `ehr_index` (`ehr_id uuid`, subject `OBJECT_REF` columns
(`subject_id`, `namespace`, `type`), `instance_type`, `valid_period
tstzrange`, `notes`, `location jsonb`), N:M by design, PK
`(ehr_id, subject_id, namespace)`. The five SM calls map 1:1. The existing
`ehr.subject_id` unique promotion remains the Primary-instance fast path for
`ehr_get_by_subject`; index rows beyond Primary capture the
duplicate/supplementary states the SM wants surfaced. Writes emit audit
events (System Log), not contributions (index entries are not versioned
objects — SM defines no versioning here; `// PORT NOTE:`).

### 4.2 Terminology surface (`TerminologyService`)

Trait over two providers behind one dispatch: (a) `openehr-term` bundle
(terminology ids `openehr`, code sets; `subsumes` = false except identity —
the openEHR vocabulary is flat; `value_set_validate` = group membership);
(b) an external-terminology provider seam (FHIR `tx` adapter later, same
trait). `get_term` returns a `TerminologyExtract` with `Defined_term` rows
from the bundle rubrics. `at_date` honoured by (a) as "current bundle"
(single pinned version, reported in `available_versions`).

### 4.3 Message service (`MessageService` = `EhrExtractService` + `TddService`)

- **Codegen prerequisite:** emit the RM `ehr_extract` package from the
  vendored RM BMM (`openehr-codegen -- emit`; the package is in the BMM —
  enable it like every other RM package; drift gate extends to it).
- `export_ehrs(ehr_id)`: assemble `EHR_EXTRACT` from the node store —
  reuse `vobject` reads + `versioned.rs` ORIGINAL_VERSION builders; full
  version history included per EXTRACT_SPEC.
- `export_ehr_extracts(spec)`: EXTRACT_SPEC-driven subset (RM types govern).
- `import_ehr(an_ehr_id?, extract)`: create EHR (optionally with fixed id —
  the SM's cross-service same-patient-id case) then replay versions through
  `commit_contribution` preserving audits as IMPORTED_VERSION semantics.
- `import_ehr_extract(ehr_id, extract)`: replay into existing EHR.
- `import_tdd(ehr_id, tdd)`: TDD (XML instance of a template) → typed OPT
  content model → COMPOSITION → normal validated commit. `import_tdds` =
  batched loop, per-item fail report (contract filled by design, spec has no
  signature; `// PORT NOTE:`).

### 4.4 Subject Proxy (`SubjectProxyService` + `DataBinding`)

Config store (new tables `sp_subject`, `sp_variable`, `sp_data_set`,
`sp_binding`; config persists until `reset()` per spec §Persistence). Sample
histories in a `sp_sample` table (JSONB payload). `DataBinding` executors:
`OpenehrFrame` (executes AQL through `QueryService`, yields `OpenehrSample`
with `result: RESULT_SET`); adapter seams for FHIR/HL7v2 frames (reqwest
client behind the same trait). Freshness = `effective_time` vs
`SUBJECT_VARIABLE.currency`; `get_variable` re-executes the bound frame when
stale, else serves `last_frame` extraction via `frame_path`.

### 4.5 Admin extensions

- `physical_party_delete` (+ relationships) mirroring the EHR delete tx.
- Statistics: `list_contributions`/counts per `PLATFORM_SERVICE` — SQL over
  `contribution`/`vo_version` keyed by `Kind`.
- Archive: `archived boolean`/`archived_at` tier on `vo_version` + movement
  calls (archival storage = same DB, partition-ready; `// PORT NOTE:` — SM
  does not define the storage form).
- Dump/load: streaming export of canonical JSON/XML per EHR (reuse
  `openehr-its` encoders + `EXPORT_SPEC` segmenting/compression), load with
  duplicate-EHR-id failure per spec, `DumpLoadFailReport` per entity.

### 4.6 Demographic completion

`Kind::PartyRelationship` in `vobject` (the machinery is already generic),
CRUD + at-time/at-version + versioned via the same paths as PARTY; RM
`PARTY_RELATIONSHIP` type exists in `openehr-rm`. Asymmetries in the SM
(missing preconditions) are normalized to the PARTY pattern
(`// PORT NOTE:`).

## 5. Error model: one table

`ehrbase-sm::error` defines `CallStatus` mirroring `CALL_STATUS_TYPE` +
service descendants, and the single mapping used by every adapter:

| SM `CALL_STATUS_TYPE`(+desc.) | `ServiceError` today | HTTP (ITS-REST/CNF wins) |
|---|---|---|
| `success` | `Ok` | 2xx |
| `auth_failure` | rest auth layer | 401/403 |
| `precondition_violation` | `BadRequest` | 400 |
| `versioned_object_does_not_exist`, `*_does_not_exist` | `NotFound` | 404 |
| `object_version_does_not_exist` | `NotFound` | 404 |
| `version_mismatch` (blank in spec) | `VersionConflict` | 412 |
| `ehr_create_fail_duplicate_id`, `*_already_exists` | `Conflict` | 409 |
| `definition_unknown`, `content_invalid`, `invalid_*` | `Unprocessable`/`ValidationFailed` | 422 |
| `file_not_writable` | new `AdminIo` | 500 |
| `exception` | `Storage`/`Database`/`Signing` | 500 |

Prose-only error names in the SM (`definition_unknown` etc. appearing in no
enum) get explicit variants here — the table *is* the reconciliation.

## 6. What does NOT change

ADR-008 storage (node + `vo_version`), the generated `openehr-*` foundation,
the AQL engine, validation, auth, ATNA audit, observability, and the
ITS-REST wire behaviour proven by the ECC conformance runs. This design
renames and completes the service layer; it does not re-litigate storage or
wire decisions. All existing tests remain green throughout (the trait split
is behaviour-preserving).

## 7. Wire exposure for the new components

ITS-REST 1.0.3 defines no contract for EHR Index, Terminology, Message,
Subject Proxy, or dump/load. These are exposed under the server's extension
namespace (as the admin API already is): `/rest/ehr_index`,
`/rest/terminology`, `/rest/message`, `/rest/subject_proxy`, `/rest/admin/*`
extensions — spec-first from the SM call semantics, documented in our own
OAS (utoipa) and excluded from the ITS-REST drift check. If/when openEHR
publishes ITS-REST contracts for them, `emit-rest` takes over (ADR-005
pattern) and the extension routes migrate.
