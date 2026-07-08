# SM digest 4/6 — Platform Service Model: Message, Subject Proxy, Terminology, Admin, System Log

Part of the SM-platform design set (`docs/design/sm-platform/README.md`).
Sources: `docs/specs/openehr/SM/docs/openehr_platform/master09/10/12/15-*.adoc`
+ included UML class files (all read in full).

## 1. Message service (`platform.interface.message`)

Three interfaces: `I_MESSAGE_SERVICE`, `I_EHR_EXTRACT_SERVICE`,
`I_TDD_SERVICE`.

- **`I_MESSAGE_SERVICE`** — "Generic message service." **Header-only stub:
  no functions, no attributes** in the vendored spec.
- **`I_EHR_EXTRACT_SERVICE`** — imports/exports `EHR_EXTRACT`s per the
  openEHR EHR Extract spec (all calls availability `0..1`, no pre/post/errors
  defined):
  - `export_ehrs(an_ehr_id: UUID): List<EXTRACT>` — whole EHR(s).
  - `export_ehr_extracts(extract_spec: EXTRACT_SPEC): List<EXTRACT>`.
  - `import_ehr(an_ehr_id: UUID [0..1], an_extract: EXTRACT)` — optional
    fixed EHR id (to match the same patient's EHR id in other services).
  - `import_ehr_extract(an_ehr_id: UUID, an_extract: EXTRACT)`.
- **`I_TDD_SERVICE`** — Template Data Documents: `import_tdd(an_ehr_id:
  UUID, tdd: String)`; `import_tdds` (bulk — **no signature in source**).

RM `EHR_EXTRACT` is **in scope (owner decision, 2026-07-08)**: the RM
`ehr_extract` package is generated from the BMM like every other RM package,
and `I_EHR_EXTRACT_SERVICE` is built as part of the SM-aligned service layer
(see doc 09 for the build order).

## 2. Subject Proxy service (`sm.platform.interface.subject_proxy`) — added SM 0.9.7 (2021)

### 2.1 Concept

Exposes **symbolic variables** describing the real-world state of a *subject*
(usually a patient; possibly devices, sites). Variables are a standard means
of obtaining information that concretely resides in different back-end
systems/devices or may be requested from live users; the SPS shields callers
from each source's model/query-language/API. **Consumers: openEHR Task Plans
and Decision Logic Modules** (Process/Planning stack).

Naming: canonical name = optional `namespace` + `name`
(`cha2ds2vasc::has_heart_failure` vs global `date_of_birth`); unique in the
service; no whitespace/unprintables; data sets may carry local aliases
("dob").

Five operation kinds: register subject, add subject variable, register
application data set, register binding (per execution 'environment'), add
binding data frame. Persistence: SPS *configuration* persists for the life of
the system; `reset()` returns to virgin state.

### 2.2 `I_SUBJECT_PROXY_SERVICE`

| Call | Signature | Pre | Notes |
|---|---|---|---|
| `register_subject` | `(subject_id: String, subject_category: String [0..1])` | `not has_subject` | |
| `add_subject_variable` | `(subject_id: String, var: SUBJECT_VARIABLE)` | `has_subject` | |
| `register_application_data_set` | `(definition: SUBJECT_DATA_SET)` | `has_subject(subject_id)` (subject_id is inside `definition` — spec imprecision) | may create variables or tighten currency |
| `remove_application_data_set` | `(subject_id, application_id)` | `has_subject` + `has_application` | |
| `remove_subject` | `(subject_id)` | `has_subject` | |
| `remove_application` | `(application_id)` | `has_application` | across all subjects |
| `get_variable` | `(subject_id, var_name): VARIABLE_VALUE` | `has_subject` | |
| `get_data_set` | `(subject_id, data_set_id): DATA_SET_RESULT` | `has_subject` | |
| `has_subject` / `has_application` / `has_binding` | `(id): Boolean` | — | |
| `get_variable_defs` | `(subject_id): List<String>` | — | entries `"name: Type"` |
| `register_binding` | `(binding: ENV_BINDING)` | `not has_binding(binding.env_id)` | spec TODO: SPS may be 1:1 with an environment |
| `add_binding_frame` | `(env_id: String, frame)` — **frame type unspecified in source** | `has_binding` | |
| `reset` | `()` | — | drop all subjects/variables/bindings |

### 2.3 Data classes

- `SUBJECT_PROXY`: `subject_id: String [1]`, `variables:
  Hash<String, SUBJECT_VARIABLE> [0..1]`, `create_time: Iso8601_date_time
  [1]`, `subject_category: String [1]` (TODO: uncontrolled), `data_sets:
  Hash<String, SUBJECT_DATA_SET> [0..1]`.
- `SUBJECT_VARIABLE`: `namespace [0..1]`, `name [1]`, `type_name [1]`,
  `currency: Iso8601_duration [0..1]` (unset ⇒ most recent valid),
  `ask_user: Boolean [0..1]`, `is_manual: Boolean [1]`, `history:
  List<VARIABLE_SAMPLE> [0..1]`, `frame_id: String [1]`, `last_frame:
  DATA_FRAME_SAMPLE [0..1]`, `frame_path: String [1]`. Functions:
  `canonical_name(): String`, `value(): VARIABLE_SAMPLE`, `is_global():
  Boolean` (doc text contradicts post-condition `Result = (namespace = Void)`
  — post-condition is the consistent reading).
- `SUBJECT_DATA_SET`: `id [1]`, `subject_id [1]`, `creating_app_id [0..1]`,
  `using_app_ids: List<String> [0..1]`, `variables: Hash<String,
  SUBJECT_VARIABLE> [1]` (keyed by *local* name), `last_result:
  DATA_SET_RESULT [0..1]`; fn `value(): DATA_SET_RESULT`.
- `DATA_SET_RESULT`: `name [1]`, `subject_id [1]`, `variables:
  List<VARIABLE_SAMPLE> [0..1]`.
- Sample hierarchy: `SAMPLE<T>` (abstract; `retrieve_time [1]`,
  `effective_time [0..1]` — compared to `currency` for freshness,
  `is_unavailable: Boolean [1]`, `unavailable_reason [0..1]`, `result: T
  [0..1]`) → `DATA_FRAME_SAMPLE<T>` (abstract) → `OPENEHR_SAMPLE`
  (`result: RESULT_SET`), `HL7v2_SAMPLE`, `HL7_FHIR_SAMPLE`; and
  `VARIABLE_SAMPLE` (direct SAMPLE child; `result: VARIABLE_VALUE`).
- Value hierarchy: `VARIABLE_VALUE` (abstract) → `VARIABLE_VALUE_SINGLE`
  (`value: Any`), `VARIABLE_VALUE_LIST` (`value: List<Any>`),
  `VARIABLE_VALUE_TIME_SERIES` (`value: Hash<Iso8601_date_time, Any>`).
- Bindings: `I_DATA_BINDING` (digest 1 §6); `ENV_BINDING` (`env_id [1]`,
  `description [0..1]`, `data_frames: List<DATA_FRAME> [0..1]`);
  `DATA_FRAME` (`primary_method`/`fallback_method: SYSTEM_CALL [0..1]` from
  the PROC/task-planning spec, `id [1]`, `model_type [1]` e.g. `"openehr"`,
  `"hl7v2"`, `"hl7-fhir"` — "currently not standardised"; fn `execute():
  DATA_FRAME_SAMPLE`, "partially specified").
- Orphans: `SP_VARIABLE_DEF` + `SP_VARIABLE_CATEGORY` (`state`, `problem_dx`,
  `vital_signs`, `medication`, `past_procedure`) exist as files but are NOT
  included by the chapter — legacy/unwired.

Data-set and binding payloads are exchanged as text (JSON/YAML), e.g.
`!!SUBJECT_DATA_SET {name, creating_app_id, variables[{name, type_name,
currency}]}` and `!!ENV_BINDING {data_frames[{frame_id, model_type,
primary_method: !!API_CALL|!!QUERY_CALL}]}`.

## 3. Terminology service (`platform.interface.terminology`)

Includes a model for **terminology extracts**: terms (bare code or defined)
plus relationships.

### 3.1 `I_TERMINOLOGY_SERVICE`

| Call | Signature | Pre |
|---|---|---|
| `get_terminology_ids` | `(): List<String>` (may be URIs) | — |
| `has_terminology` | `(terminology_id: String): Boolean` | — |
| `get_terminology_description` | `(terminology_id): Terminology_description` | `has_terminology` |
| `has_term` | `(terminology_id, code: String, at_date: Iso8601_date [0..1]): Boolean` | `has_terminology` |
| `get_term` | `(terminology_id, code, attributes: Hash<String,String> [0..1], at_date [0..1]): Terminology_extract` | `has_terminology` + `has_term` |
| `subsumes` | `(terminology_id, ref_code, candidate_child_code): Boolean` — strict subsumption | `has_terminology` |
| `value_set_validate` | `(terminology_id, value_set_id, candidate_code, at_date [0..1]): Boolean` | `has_terminology` |
| `has_value_set` | `(terminology_id, value_set_code): Boolean` | — |
| `get_value_set` | `(terminology_id, value_set_code): Terminology_extract` | `has_terminology` + `has_value_set` |

`at_date` gives temporal terminology semantics (definition/membership as of a
date). No exceptions defined for any call.

### 3.2 Extract classes

- `Terminology_description`: `publisher [1]`, `available_versions:
  List<String> [0..1]`, `attributes: List<String> [0..1]`, `uri [1]`.
- `Terminology_extract`: `terminology_id [1]` (e.g. `"snomed_ct"`),
  `terminology_version [0..1]`, `terms: Hash<String, Term_code> [0..1]`,
  `relationships: List<Term_relationship> [0..1]`, `relations: Hash<String,
  Terminology_relation> [0..1]`; fn `create_terminology_code(code):
  Terminology_code`. May represent a flat value set, a ref-set, or a
  subsumption hierarchy.
- `Terminology_relation`: `name [1]`, `local_code [0..1]`, `external_code:
  Terminology_code [0..1]`. **Invariant:** `local_code /= Void xor
  external_code /= Void`.
- `Term_relationship`: `origin_code [1]`, `relation_name [1]` (must key into
  `Terminology_extract.relations`), `target_codes: List<String> [0..1]`.
- `Term_code`: `code [1]` (single term, value set, or post-coordinated
  expression). `Defined_term` extends it: `text [1]`, `language [0..1]`,
  `is_preferred_term [0..1]`.

## 4. Admin service (`platform.interface.admin`)

### 4.1 `I_ADMIN_SERVICE`

| Call | Signature | Pre | Errors |
|---|---|---|---|
| `list_contributions` | `(a_service: PLATFORM_SERVICE, time_interval: Interval<Iso8601_date_time> [0..1]): List<UUID>` | — | — |
| `contribution_count` | `(a_service, time_interval [0..1]): Integer` | — | — |
| `versioned_composition_count` | `(a_service, time_interval [0..1]): Integer` | — | — |
| `composition_version_count` | `(a_service, time_interval [0..1]): Integer` | — | — |
| `physical_ehr_delete` | `(an_ehr_id: UUID)` | `has_ehr` | `ehr_id_does_not_exist` |
| `physical_party_delete` | `(a_party_id: UUID)` — also deletes related party relationships | — | `party_id_does_not_exist` |

### 4.2 `I_ADMIN_ARCHIVE`

`archive_ehrs(ehr_ids: List<UUID> [0..1])` (error `ehr_id_does_not_exist`);
`archive_parties(party_ids: List<UUID> [0..1])` (error
`party_id_does_not_exist`) — move to archival storage.

### 4.3 `I_ADMIN_DUMP_LOAD`

- `export_ehrs(file_sys_loc: String, logical_fmt: EXPORT_FORMAT [0..1],
  comp_fmt: COMPRESSION_FORMAT [0..1], enc_format: ENCODING_FORMAT [0..1])`
  — error `file_not_writable`.
- `load_ehrs(file_sys_loc: String)` — repository need not be empty; imports
  with duplicate EHR ids fail. Error `file_not_writable`.
- `DUMP_LOAD_FAIL_REPORT`: `entity_type [1]`, `entity_id [1]`, `dump_status:
  Boolean [1]`, `error [0..1]`.
- `EXPORT_SPEC`: `logical_format: EXPORT_FORMAT [0..1]`,
  `compression_format: COMPRESSION_FORMAT [0..1]`, `encoding:
  ENCODING_FORMAT [0..1]`, `segment_split_size: Integer [1]` (kb).
- `EXPORT_FORMAT`: `openehr_canonical_xml`, `openehr_canonical_json`.
  `COMPRESSION_FORMAT`: `zip`, `7z`. `ENCODING_FORMAT`: **empty enumeration
  (no values in source)**.

## 5. System Log service

**No chapter exists** (master13/14 missing); `I_SYSTEM_LOG` is an **empty
stub interface**. The only normative statement is the overview table:
"**IHE ATNA-compliant system log**." The contract must be sourced from IHE
ATNA itself. Our `ehrbase-audit` crate (DICOM audit messages over syslog,
`docs/enterprise/atna-audit.md`) is exactly this component realized —
already ahead of the spec text.

## 6. Spec defects & silences

`I_MESSAGE_SERVICE` + `I_SYSTEM_LOG` empty stubs; `ENCODING_FORMAT` empty
enum; no System Log chapter; `add_binding_frame` frame type unspecified;
`register_application_data_set` precondition references non-argument;
`SUBJECT_VARIABLE.is_global` doc-vs-post contradiction; `I_DATA_BINDING.
get_frame.frame_id` doc says "name of the variable"; `SP_VARIABLE_DEF`/
`SP_VARIABLE_CATEGORY` unwired; multiple in-source TODOs (SPS-per-environment,
`subject_category` uncontrolled, `ask_user` mechanics, subject-id resolution).

## 7. Mapping note (current code)

- Admin: `AdminService` seam + `service/admin.rs` implement
  `physical_ehr_delete` (+ bulk); counts/archive/dump-load are gaps to build
  (doc 07/09).
- Terminology: `openehr-term::bundle` covers the openEHR vocabulary access
  internally; the external `I_TERMINOLOGY_SERVICE` surface (terminology ids,
  term lookup, subsumption, value sets) is a gap to build (doc 07/09).
- System Log: `ehrbase-audit` (ATNA) = the component, done.
- Message (incl. `I_EHR_EXTRACT_SERVICE` + `I_TDD_SERVICE`) and Subject
  Proxy: not yet implemented — **both in scope**; where the SM text is a
  stub, the design fills the contract explicitly (doc 08) and records each
  filled gap with a `// PORT NOTE:` + citation.
