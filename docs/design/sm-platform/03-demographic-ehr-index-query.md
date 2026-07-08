# SM digest 3/6 — Platform Service Model: Demographic, EHR Index, Query services

Part of the SM-platform design set (`docs/design/sm-platform/README.md`).
Sources: `docs/specs/openehr/SM/docs/openehr_platform/master06/07/08-*.adoc` +
included UML class files (all read in full).

## 1. Demographic service (`platform.interface.demographic`)

Mirrors the EHR-service pattern: root interface hands out per-object
interfaces; writes go through `UV_*` update-version objects that the back-end
expands into VERSION/VERSIONED_OBJECT/CONTRIBUTION.

### 1.1 `I_DEMOGRAPHIC_SERVICE`

| Call | Signature | Pre | Errors |
|---|---|---|---|
| `create_party` | `(a_version: UV_PARTY): UUID` | `definitions_valid(a_version)` + `valid_content(a_version)` | `definition_unknown`, `content_invalid` |
| `create_party_relationship` | `(a_version: UV_PARTY_RELATIONSHIP): UUID` | `valid_content(a_version)` (no `definitions_valid` — asymmetry, verbatim) | `definition_unknown`, `content_invalid` |
| `i_party` | `(a_versioned_party_id): I_PARTY` (param type omitted in source) | — | `versioned_object_does_not_exist` |
| `i_party_relationship` | `(a_versioned_party_rel_id): I_PARTY_RELATIONSHIP` (param type omitted) | — | `versioned_object_does_not_exist` |

Both `create_*` "cause server-side creation of a new `VERSIONED_OBJECT`,
`ORIGINAL_VERSION` and new `CONTRIBUTION`."

### 1.2 `I_PARTY`

| Call | Signature | Pre / Post | Errors |
|---|---|---|---|
| `has_party` | `(a_versioned_party_id: UUID): Boolean` | — | — |
| `has_party_version_id` | `(a_party_version_id: UUID): Boolean` | — | — (name clash: precondition elsewhere says `has_party_version`) |
| `get_party` | `(a_versioned_party_id: UUID): PARTY` | pre `has_party` | `versioned_object_does_not_exist` |
| `get_party_at_time` | `(a_versioned_party_id: UUID, a_time: Iso8601_date_time): PARTY` | — | `versioned_object_does_not_exist` |
| `update_party` | `(a_versioned_party_id: UUID, a_version: UV_PARTY): UUID` | pre `definitions_valid` + `has_party` | `versioned_object_does_not_exist`, `object_version_does_not_exist`, `definition_unknown`, `content_invalid` |
| `delete_party` | `(a_versioned_party_id: UUID)` | pre `has_party`; post `not has_party` | `versioned_object_does_not_exist` |
| `get_party_at_version` | `(a_party_version_id: UUID): PARTY` | pre `has_party_version` | `object_version_does_not_exist` |

### 1.3 `I_PARTY_RELATIONSHIP`

Same shape for `PARTY_RELATIONSHIP`: `has_party_relationship`,
`get_party_relationship` (no `has_` precondition — asymmetry vs `get_party`),
`get_party_relationship_at_time`, `update_party_relationship` (pre
`definitions_valid` + `has_party_relationship`; full error set),
`delete_party_relationship` (post not-has), `get_party_relationship_at_version`
(error `object_version_does_not_exist`).

### 1.4 `UV_PARTY` / `UV_PARTY_RELATIONSHIP`

`UPDATE_VERSION<PARTY>` / `UPDATE_VERSION<PARTY_RELATIONSHIP>`; no own
attributes (see digest 1 §4.4 for the base class + `UPDATE_AUDIT`).

## 2. EHR Index service (`platform.interface.ehr_index`)

Purpose (verbatim core): "enable the recording of associations of subject
identifiers … with EHR identifiers. In a privacy-supporting environment, this
enables EHRs to be persisted with only an EHR id; the EHR Index has to be used
to obtain the subject identifier," keying into a demographic/MPI service.

N:M reality, both directions are *error conditions to manage*:
- multiple subject ids for one EHR id — "dangerous error condition, needs to
  be detected and rectified";
- multiple EHR ids per subject — duplicate-EHR error, "less dangerous".

Association metadata = `RESOURCE_STATUS`; optional dynamic EHR location =
`LOCATION_DESC`.

### 2.1 `I_EHR_INDEX` (all procedures, no pre/post)

| Call | Signature | Errors |
|---|---|---|
| `add_ehr_subject` | `(an_ehr_id: UUID, a_subject_id: OBJECT_REF, a_status: RESOURCE_STATUS [0..1], a_loc_desc: LOCATION_DESC [0..1])` | — |
| `update_ehr_subject_status` | `(an_ehr_id, a_subject_id, a_status: RESOURCE_STATUS [1])` | `subject_id_does_not_exist`, `ehr_id_does_not_exist` |
| `update_ehr_subject_loc_desc` | `(an_ehr_id, a_subject_id, a_loc_desc [0..1])` (optional ⇒ clearing allowed) | same |
| `remove_ehr_subject` | `(an_ehr_id, a_subject_id)` — subject may remain associated with other EHRs | same |
| `remove_subject` | `(a_subject_id)` — remove all entries for a subject | `subject_id_does_not_exist` |

### 2.2 Supporting classes

- `RESOURCE_STATUS`: `instance_type: RESOURCE_INSTANCE_TYPE [1]`,
  `start_valid_time`/`end_valid_time` [0..1] typed **`@@`** (unresolved
  placeholder in spec), `notes: String [0..1]`.
- `RESOURCE_INSTANCE_TYPE`: `Primary`, `Duplicate`, `Supplementary` (last has
  no meaning text).
- `LOCATION_DESC`: **empty stub — no attributes defined**. Content model is
  a design decision point.

## 3. Query service (`platform.interface.query`)

Model (verbatim core): execute either **stored** queries (held in the
DEFINITION service; language not assumed to be AQL) or **ad hoc** queries;
parameters must be provided for open parameters; a successful execution
returns a `RESULT_SET`; paging via offset/fetch params.

Stored-query identifier: `reverse-domain-name '::' semantic-id ['/' version]`
e.g. `org.example.departmentx.test::diabetes-patient-overview/1.0.2`.

**Spec inconsistency:** overview prose says `item_offset`/`items_to_fetch`;
the interface signatures say `row_offset`/`rows_to_fetch`.

### 3.1 `I_QUERY_SERVICE`

```
execute_stored_query (exec_spec: STORED_QUERY_EXECUTE_SPEC[1],
                      row_offset: Integer[0..1], rows_to_fetch: Integer[0..1],
                      ehr_ids: List<UUID>[0..1]): RESULT_SET

execute_ad_hoc_query (exec_spec: ADHOC_QUERY_EXECUTE_SPEC[1],
                      row_offset: Integer[0..1], rows_to_fetch: Integer[0..1],
                      ehr_ids: List<UUID>[0..1]): RESULT_SET
```

Load-bearing semantics:
- `row_offset` ≤ 0 ⇒ offset 0; `rows_to_fetch` ≤ 0 ⇒ all.
- **Empty `ehr_ids` ⇒ full population query over all EHRs whose
  `EHR_STATUS.is_queryable = True`** (the `is_queryable` gate is normative
  here).
- Error: `ehr_id_does_not_exist`.

### 3.2 Execute-spec + result classes

- `STORED_QUERY_EXECUTE_SPEC`: `qualified_query_name: String [1]`
  (`reverse_domain::name`), `version: String [0..1]` (semver; absent ⇒
  latest), `query_parameters: Hash<String,String> [1]` (tags must match query
  parameter names).
- `ADHOC_QUERY_EXECUTE_SPEC`: `source: String [1]` (AQL text),
  `formalism: String [0..1]` (**default `"aql"`**),
  `query_parameters: Hash<String,String> [1]`.
- `RESULT_SET` (inherits `Any`): `columns: List<RESULT_SET_COLUMN> [1]`,
  `id: String [1]`, `creation_time: Iso8601_date_time [1]`,
  `query: RESULT_QUERY_DESCRIPTOR [0..1]`, `rows: List<RESULT_SET_ROW>
  [0..1]` ("Rox data." — spec typo). "Ideally the Result set has sufficient
  meta-data to be processible independently of the original query."
- `RESULT_SET_COLUMN`: `name: String [1]`, `archetype_id: String [0..1]`
  (meaning is an unresolved authoring TODO: "check on whether needed or
  inside the path"), `path: String [0..1]`.
- `RESULT_SET_ROW`: `values: List<Any> [0..1]`, positionally aligned to
  `columns`.
- `RESULT_QUERY_DESCRIPTOR` extends `QUERY_DESCRIPTOR` (digest 1 §5.7) with
  `executed: String [0..1]` (parameter-substituted executed text).

## 4. Abstract error vocabulary (union, these three services)

Demographic: `definition_unknown`, `content_invalid`,
`versioned_object_does_not_exist`, `object_version_does_not_exist`.
EHR Index: `subject_id_does_not_exist`, `ehr_id_does_not_exist`.
Query: `ehr_id_does_not_exist`.
These are abstract names, not HTTP codes; the SM is silent on the REST
mapping — that mapping lives in ITS-REST + CNF, which win at the wire.

## 5. Spec defects & silences (design decision points)

1. `RESOURCE_STATUS.start/end_valid_time` typed `@@` (unspecified — almost
   certainly ISO 8601 date-time; decide with `// PORT NOTE:`).
2. `LOCATION_DESC` empty stub.
3. `RESOURCE_INSTANCE_TYPE.Supplementary` meaning blank.
4. `RESULT_SET_COLUMN.archetype_id` semantics unresolved upstream.
5. `ADHOC_QUERY_EXECUTE_SPEC.formalism` + `RESULT_SET.query` meaning blank.
6. Paging-name inconsistency (`item_*` vs `row_*`).
7. `has_party_version_id` vs `has_party_version` name clash.
8. `i_party`/`i_party_relationship` param types omitted.
9. `create_party` requires `definitions_valid`; `create_party_relationship`
   does not; `get_party` has a `has_party` precondition, `get_party_relationship`
   has none — asymmetries to reconcile deliberately.

## 6. Mapping note (current code)

- Demographic: `ehrbase::service::demographic` + the bespoke
  `DemographicService` seam already cover `I_PARTY` CRUD + versioning +
  demographic contributions; `PARTY_RELATIONSHIP` is **not** implemented.
- EHR Index: no dedicated service; `ehr.subject_id/namespace` promotion +
  `ehr_get_by_subject` partially covers the lookup direction only.
- Query: `QueryService` seam + P16 AQL engine cover both calls; the
  `is_queryable` population gate and RESULT_SET are implemented at the
  ITS-REST shape (which supersedes the SM naming divergences at the wire).
