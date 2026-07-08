# SM digest 1/6 — Platform Service Model: foundations, common + definitions packages

Part of the SM-platform design set (`docs/design/sm-platform/README.md`).
Sources: `docs/specs/openehr/SM/docs/openehr_platform/master01..04*.adoc`,
`docs/specs/openehr/SM/docs/UML/classes/*.adoc`, `manifest.json`,
`PROVENANCE.md` (all read in full).

## 1. Spec identity

- Component **SM** ("openEHR Service Model"), component status **DEVELOPMENT**
  (`manifest.json`). This specification, **"openEHR Platform Service Model"**,
  is `spec_status: TRIAL` — SM Release 1.0.0 **unreleased**. Pinned commit
  `23ffc4711c10bae2ae43724b1948fe3b24a0964e` of
  `github.com/openEHR/specifications-SM` (`PROVENANCE.md`).
- Amendment history highlights (`master00-amendment_record.adoc`): 0.9.0
  initial (2017) → 0.9.3 adds call conventions + EHR Index → 0.9.5 renames
  `row_offset`/`rows_to_fetch` → `item_offset`/`items_to_fetch`, adds
  `*_count()` + `valid_*()` calls, fixes `I_EHR_COMPOSITION.get_composition`
  → `get_composition_latest` → 0.9.7 adds Subject Proxy Service (2021-04) →
  0.9.8 renames `I_DEFINITION_QUERY.register_*` → `store_*`, `formalism`
  param → `a_type` (2021-12).
- **Maturity consequence:** TRIAL/DEVELOPMENT, not STABLE. It is a naming +
  semantics reference, not a conformance oracle of RM/ITS grade. ITS-REST
  1.0.3 (STABLE) remains the wire-conformance target; SM governs internal
  service decomposition and call semantics.

## 2. The platform model

Abstract architecture (`master02-overview.adoc` §General Assumptions): a
nominal **native API** reached through *protocol adapters* (REST, SOAP,
protobuf, Kafka, …). The spec standardises component **naming** and interface
**semantics** — a "formal equivalent" of any real architecture, not a product
architecture. Packages: `common`, `definition` (the service components),
`interface` (the interfaces attached to each component).

The ten named services (§openEHR Platform Model, verbatim table):

| Service | Description |
|---|---|
| Definitions | Upload and querying of definition artefacts: archetypes, templates, queries. |
| EHR | Versioned persistence service for EHRs. |
| Demographic | Versioned persistence service for demographic data. |
| EHR Index | EHR id / demographic subject cross-reference service. |
| Query | AQL query retrieval for EHR, demographics and other content services. |
| Terminology | Access to terminology, including intentional value sets. |
| Message | Message import/export, incl. EHR Extracts and documents. |
| System Log | IHE ATNA-compliant system log. |
| Subject Proxy | Registration of subject-focussed data-sets giving a 'proxy' picture of the subject over time. |
| Admin | Administrative facilities on all services (e.g. back-up). |

`PLATFORM_SERVICE` enumeration (`platform_service.adoc`): `Admin`,
`Definitions`, `Ehr`, `Ehr_index`, `Demographic`, `Message`, `Query`,
`System_log`. **Spec defect:** omits `Terminology` and `Subject_proxy`
despite the prose table.

## 3. Global conventions (normative for our service-trait design)

`master02-overview.adoc` §Interface Calls, §Anatomy, §Global Conventions:

- **Command/query separation.** Every call is either a query (returns, no
  state change) or a command (changes state, returns nothing) — side-effect
  functions avoided. (The `create_*` calls returning the new id are the
  sanctioned exception pattern.)
- **Formal equivalence + transactionality.** An implementation may realise
  one spec call as several of its own, *iff* pre/post conditions match and
  the group is transactionally protected. "Any single call constitutes a
  self-standing transaction … that will leave [the service] in a consistent
  state."
- **Error model.** "Nearly stateless": results return directly; failures are
  interrogated via `I_STATUS.last_call_failed(): Boolean` +
  `last_call_status(): CALL_STATUS`. Statuses cover auth errors,
  precondition violations (argument validity), and server exceptions
  (post-condition/invariant violations). Mapping to a stateless style (status
  in the response envelope) is explicitly permitted — that is what our typed
  `Result<T, E>` + HTTP mapping realises.
- **Auth is out of band.** "Assumed to have been dealt with before any
  particular call" via standard technologies (OAuth, RFC 7235) + RBAC;
  0.9.2 removed `auth_tok` args from all calls.
- **List handling.** Cursor params `item_offset` (0-based; 0 = from first)
  and `items_to_fetch` (0 = all) on all unbounded-list calls.
- **Naming conventions.** `ehr_id` (= `EHR.ehr_id.value`, usually UUID),
  `versioned_object_uid` (= `VERSIONED_OBJECT.uid.value`), `version_uid`
  (= `VERSION.uid.value`, `uuid::system::n`), `preceding_version_uid`,
  `object_id` (either of the above), `time` (ISO 8601).
- **Call spec anatomy.** Call + Arguments + named Pre-conditions + named
  Post-conditions + Exceptions (e.g. `create_ehr_with_id`: pre
  `Valid_id: not has_ehr(an_id)`, post `Ehr_created: has_ehr(an_id)`,
  exceptions `Ehr_already_exists`, `Auth_error`).

## 4. Common package (`master03-common_package.adoc`)

### 4.1 `I_STATUS` (interface)

"Interface to obtain status of previous calls; use by inheritance."
`last_call_failed(): Boolean` (any result other than `CALL_STATUSES.success`),
`last_call_status(): CALL_STATUS`.

### 4.2 `CALL_STATUS` (class)

`code: CALL_STATUS_TYPE [1]`, `call_name: String [1]`,
`call_string: String [1]` (stringified full call), `meaning: String [1]`,
`message: String [1]`.

### 4.3 `CALL_STATUS_TYPE` (enumeration; services extend by inheritance)

| Value | Meaning |
|---|---|
| `success` | Call succeeded. |
| `auth_failure` | Authorisation failure. |
| `precondition_violation` | Precondition violation occurred. |
| `object_version_does_not_exist` | Referenced Object version of a Versioned Object does not exist. |
| `versioned_object_does_not_exist` | No Versioned Object with referenced identifier found. |
| `exception` | Exception other than precondition violation. |
| `ehr_id_does_not_exist` | EHR with provided id not found. |
| `party_id_does_not_exist` | Party with provided id not found. |
| `file_not_writable` | File system locator cannot be written to. |
| `version_mismatch` | (meaning blank in source) |

### 4.4 Version update semantics (§Version Update Semantics — load-bearing)

Calls that create/update a versioned top-level object "implicitly require the
creation of a new `CONTRIBUTION` on the server side, as well as one or more
new `ORIGINAL_VERSION` objects, and in creation cases, new
`VERSIONED_OBJECTS`." The client supplies an `UPDATE_VERSION<T>`; the server
constructs the full `VERSION<T>`/`AUDIT_DETAILS` (it generates
`time_committed` + `system_id`, hence the partial `UPDATE_AUDIT`).
`preceding_version_uid` mandatory except for first versions;
`lifecycle_state` mandatory always (e.g. `532|complete|`, `553|incomplete|`,
`523|deleted|`). Per top-level type, concrete subtypes bind `T`:
`UV_COMPOSITION`, `UV_FOLDER`, `UV_PARTY`, `UV_PARTY_RELATIONSHIP`.

`UPDATE_VERSION<T>` (abstract): `preceding_version_uid: OBJECT_VERSION_ID
[0..1]`, `lifecycle_state: Terminology_code [1]`, `attestations:
List<ATTESTATION> [0..1]`, `data: T [1]`, `audit: UPDATE_AUDIT [1]`.

`UPDATE_AUDIT`: `change_type: Terminology_code [1]` (openEHR *audit change
type* group), `description: String [0..1]`, `committer: PARTY_PROXY [1]`.
Invariant `Change_type_valid: terminology(Terminology_id_openehr).
has_code_for_group_id(Group_id_audit_change_type, change_type.defining_code)`.

### 4.5 `I_VALIDITY_CHECKER` (interface)

"Utility functions for checking validity of use of definitions within data"
(referenced by every create/update precondition):
- `definitions_valid(a_content: LOCATABLE): Boolean` — archetype/template
  ids known in the local definitions service.
- `content_valid(a_content: LOCATABLE): Boolean` — content is a valid RM
  instance. (Preconditions elsewhere say `valid_content(...)` — naming
  inconsistency in the source.)

## 5. Definitions service (`master04-definition_package.adoc`)

Conventions: query qualified names `<namespace>::<query-name>` or
`<namespace>::<formalism>::<query-name>`; missing namespace ⇒ `"misc"`.
Formalism (`a_type`) is case-insensitive, optional `::`-separated version;
no version ⇒ major `"1"` assumed (`"AQL"` ≡ `"aql"` ≡ `"AQL::1"`).

### 5.1 `I_DEFINITION_ADL14`

"Interface to ADL 1.4 definitions (archetypes and OPTs)."

| Call | Signature | Pre/Post | Errors |
|---|---|---|---|
| `has_archetype` | `(an_id: ARCHETYPE_ID): Boolean` | — | — |
| `valid_archetype` | `(an_arch: ARCHETYPE): Boolean` | — | — |
| `upload_archetype` | `(an_arch: ARCHETYPE)` | post `has_archetype(an_arch.identifier)`; replace-if-exists; must be valid | `invalid_archetype` |
| `get_archetype` | `(an_id: ARCHETYPE_ID): ARCHETYPE` | — | `artefact_does_not_exist` |
| `list_archetypes` | `(item_offset, items_to_fetch): List<ARCHETYPE_ID>` | — | — |
| `list_matching_archetypes` | `(id_pattern: String, item_offset, items_to_fetch): List<ARCHETYPE_ID>` | regex match | `invalid_id_pattern` |
| `delete_archetype` | `(an_id: ARCHETYPE_ID)` | pre `has_artefact(an_id)`; post `not has_archetype(an_id)` | `invalid_archetype` |
| `has_opt` | `(an_opt_id: UUID): Boolean` | — | — |
| `valid_opt` | `(an_opt: ARCHETYPE): Boolean` | — | — |
| `upload_opt` | `(an_opt: ARCHETYPE)` | pre `valid_opt(an_opt)` | `invalid_template` |
| `get_opt` | `(an_opt_id: UUID): ARCHETYPE` | — | `artefact_does_not_exist` |
| `list_opts` | `(item_offset, items_to_fetch): List<UUID>` | — | — |
| `list_matching_opts` | `(id_pattern, item_offset, items_to_fetch): List<ARCHETYPE_ID>` (source inconsistency: element type vs `list_opts`) | — | `invalid_id_pattern` |
| `delete_opt` | `(an_id: UUID)` | pre `has_opt`; post `not has_opt` | `invalid_template` |
| `archetypes_count` | `(): Integer` | — | — |
| `opts_count` | `(): Integer` | — | — |

### 5.2 `I_DEFINITION_ADL2`

ADL2 artefacts (archetype/template/OPT all `AUTHORED_ARCHETYPE`, id'd by
`ARCHETYPE_HRID`): `has_artefact`, `valid_artefact`, `upload_artefact` (pre
valid, post has; replace by physical id + namespace), `get_artefact` (pre
exists; error `artefact_does_not_exist`), `list_artefacts` /
`list_archetypes` / `list_templates` / `list_opts` (by concrete type, all
paged), `list_matching_artefacts(id_pattern, …)` (error
`invalid_id_pattern`), `delete_artefact` (error `artefact_does_not_exist`),
`artefacts_count`, `archetypes_count`, `templates_count`, `opts_count`.

### 5.3 `I_DEFINITION_QUERY`

| Call | Signature | Notes |
|---|---|---|
| `has_query` | `(a_query_name: String): Boolean` | qualified name |
| `valid_query` | `(a_query_text: String, a_type: String): Boolean` | `a_type` = formalism name + optional version |
| `store_query` | `(a_query_text: String, a_type: String, a_query_name: String [0..1]): QUERY_DESCRIPTOR` | pre `is_valid_query(a_query_text)` (source name/arity inconsistency vs `valid_query`); name auto-generated if absent |
| `store_query_set` | `(a_query_set_name: String [0..1]): UUID` | "TODO: determine details" — spec-incomplete |
| `list_queries` | `(item_offset, items_to_fetch): List<QUERY_DESCRIPTOR>` | |
| `list_matching_queries` | `(id_pattern: String, artefact_id_pattern: String, item_offset, items_to_fetch): List<QUERY_DESCRIPTOR>` | PERL regexes on query id and referenced artefact ids; error `invalid_id_pattern` |
| `delete_query` | `(a_query_name: String)` | pre `has_query`; post `not has_query`; error `invalid_query` |
| `queries_count` | `(): Integer` | |

`QUERY_DESCRIPTOR`: `qualified_query_name: String [1]`,
`version: String [0..1]` (semver), `registration_time: Iso8601_date_time [1]`,
`formalism: String [1]` ("aql" or other), `source: String [0..1]`.
`RESULT_QUERY_DESCRIPTOR` extends it with `executed: String [0..1]`
(parameter-substituted executed text).

`DEFINITION_CALL_STATUS_TYPE` (extends CALL_STATUS_TYPE):
`invalid_archetype`, `invalid_template`, `invalid_artefact` (meaning blank),
`invalid_query`, `invalid_id_pattern`, `artefact_does_not_exist`,
`template_does_not_exist`.

## 6. Cross-cutting interfaces vendored standalone

- **`I_DATA_BINDING`** — "Internal interface via which Variable bindings are
  invoked to obtain data" (Subject Proxy machinery). Attribute
  `bindings: List<ENV_BINDING> [0..1]`; call
  `get_frame(subject_id: String, frame_id: String): DATA_FRAME_SAMPLE`.
  `ENV_BINDING`: `env_id: String [1]`, `description: String [0..1]`,
  `data_frames: List<DATA_FRAME> [0..1]` (meaning blank).
- **`I_TDD_SERVICE`** — "Template Data Document (TDD) service":
  `import_tdd(an_ehr_id: UUID, tdd: String)`; `import_tdds` (bulk — signature
  absent in source).
- **`Defined_term`** (inherits `Term_code`): `text: String [1]`,
  `language: Terminology_code [0..1]` (ISO 639 / RFC 5646),
  `is_preferred_term: Boolean [0..1]`.
- **`RESOURCE_STATUS` / `RESOURCE_INSTANCE_TYPE` / `LOCATION_DESC`** — see
  digest 3 (EHR Index).

## 7. Spec defects & silences carried into the design

1. TRIAL/DEVELOPMENT status; release 1.0.0 unreleased.
2. `last_call_failed()` vs `last_call_error()` naming clash (prose vs code
   sample); `I_STATUS` defines `last_call_failed()`.
3. `PLATFORM_SERVICE` enum omits `Terminology`, `Subject_proxy`.
4. Full component↔interface matrix lives in excluded UML SVGs; the vendored
   text does not enumerate it — reconstructed across digests 1–4.
5. Unresolved placeholders: `RESOURCE_STATUS.start/end_valid_time` typed
   `@@`; `LOCATION_DESC` attribute-less; `T` class file empty;
   `I_TDD_SERVICE.import_tdds` unsigned; `store_query_set` TODO.
6. `list_matching_opts` returns `List<ARCHETYPE_ID>` while OPTs are
   UUID-identified.
7. `store_query` precondition references `is_valid_query(text)` (wrong name,
   wrong arity).
8. Blank enum meanings: `version_mismatch`, `invalid_artefact`,
   `Supplementary`, all `PLATFORM_SERVICE` values.
9. Call-multiplicity column (`1..1` vs `0..1`) is nowhere explained; read as
   mandatory-vs-optional implementation (inference, flagged).
