# Platform rewrite register 05 — Definition + Query service seam

**Phase W-3f, spec-first.** Read-only audit of the `ehrbase` platform crate's
**Definition** and **Query** *service seam* against the vendored openEHR
specification. Method (owner ruling): the register skeleton is built **from the
spec** — the SM Definition component (`I_DEFINITION_ADL14` / `I_DEFINITION_ADL2`
/ `I_DEFINITION_QUERY`), the SM Query component (`I_QUERY_SERVICE`), the
QUERY-spec service-level semantics, and the BASE `ARCHETYPE_ID`/`TEMPLATE_ID`
lexical law — enumerated operation-by-operation with citations; existing code is
then **mapped onto** each item, never the reverse.

**Scope of this register — the SERVICE SEAM only.** It owns: stored-query
identity/versioning/registration, execution orchestration (parse→plan→execute→
assemble), the `RESULT_SET` assembly, definition/archetype/template
*provisioning* service logic, and the SM→service error contract.

**Out of scope (named seams, not re-planned here):**
- The **AQL engine** (`app/ehrbase/src/aql/`: parser use, IR, SQL lowering,
  terminology family, path analysis) → **register 08**.
- **Template ingestion / WebTemplate / OPT parsing / validation** machinery
  (`service/template.rs`, `opt_validation*`, `adl2_validation*`) → **register 10**.
  This register names the seam and its produced/consumed facts only.
- The **ITS-REST wire** (dispatch, status mapping, content negotiation in
  `ehrbase-rest`) → the ITS/REST register. The `DefinitionAdapter` wire-shaped
  extension is noted where it drives service behaviour but its route wiring is
  not owned here.

**FIXED seam (the trait surface the rewrite implements — not up for redesign):**
`app/ehrbase-sm/src/services/definition/{adl14,adl2,query}.rs` +
`app/ehrbase-sm/src/services/query/{service,request}.rs`. These traits
(`DefinitionAdl14Service` 16 ops, `DefinitionAdl2Service` 14 ops,
`DefinitionQueryService` 8 ops, `QueryService` 2 ops) + `QueryDescriptor` +
`AqlQueryRequest`/`QueryOutcome` are the contract; this register redesigns only
the `ehrbase`-crate *implementation* behind them.

**Oracles read:**
`docs/specs/openehr/SM/docs/openehr_platform/master04-definition_package.adoc`,
`master08-query_service.adoc`; the UML classes under
`docs/specs/openehr/SM/docs/UML/classes/` (`i_definition_adl14`,
`i_definition_adl2`, `i_definition_query`, `definition_call_status_type`,
`query_descriptor`, `i_query_service`, `stored_query_execute_spec`,
`adhoc_query_execute_spec`, `result_set`, `result_set_column`,
`result_set_row`);
`docs/specs/openehr/BASE/docs/base_types/master05-identification_package.adoc`
(§Archetype Identifiers, §Composite Identifiers and Case);
`docs/specs/openehr/QUERY/docs/AQL/master03-syntax.adoc` (§Parameters, §Parameter
Resolution). Absorbed impl-side audits: `docs/design/sm-platform/04-definition.md`,
`08-query.md`.

**Current code inventory (verified):**
`service/definition.rs` (864), `service/stored_query.rs` (247),
`service/aql_query.rs` (369 — dir listing shows 15 KB; execution glue),
`service/api/definition.rs` (434 — trait impls + `DefinitionAdapter`),
`service/api/query.rs` (46 — `QueryService` impl), `service/adl2_validation.rs`
(906 — register 10), `service/template.rs` (register 10).

---

## 1. Spec skeleton → code map

Column key: **Verdict** = conformant / divergent / missing / spec-silent.
Cite = spec file + section. File:line is the current realization.

### 1.1 `I_DEFINITION_ADL14` (`i_definition_adl14.adoc`; master04 §Archetypes and Templates)

Archetypes keyed by `ARCHETYPE_ID`; OPTs keyed by `UUID`. 16 operations.

| # | Spec op | Cite | Code (file:line) | Verdict |
|---|---------|------|------------------|---------|
| A14-1 | `has_archetype(ARCHETYPE_ID): Boolean` | i_definition_adl14 | `definition.rs:36` `archetype_exists` | conformant |
| A14-2 | `valid_archetype(ARCHETYPE): Boolean` | ″ | `definition.rs:620` `valid_archetype_source` | **divergent** — lexical only, no AOM parse (G-05-01) |
| A14-3 | `upload_archetype(ARCHETYPE)`, Post `has_archetype` | ″ | `definition.rs:48` `archetype_upload` | divergent — upsert OK; validity lexical (G-05-01) |
| A14-4 | `get_archetype(ARCHETYPE_ID): ARCHETYPE`, err `artefact_does_not_exist` | ″ | `definition.rs:71` `archetype_get` | conformant (returns ADL source, G-05-03) |
| A14-5 | `list_archetypes(offset,fetch): List<ARCHETYPE_ID>` | ″ | `definition.rs:85` `archetype_list` | conformant |
| A14-6 | `list_matching_archetypes(id_pattern,…)`, err `invalid_id_pattern` | ″ | `definition.rs:98` `archetype_list_matching` | conformant (RE2, G-05-11) |
| A14-7 | `delete_archetype(ARCHETYPE_ID)`, Pre/Post | ″ | `definition.rs:113` `archetype_delete` | conformant |
| A14-8 | `has_opt(UUID): Boolean` | ″ | `definition.rs:141` `opt_exists` | conformant |
| A14-9 | `valid_opt(ARCHETYPE): Boolean` | ″ | `definition.rs:627` `valid_opt_xml` → register 10 | conformant (delegates to template seam) |
| A14-10 | `upload_opt(ARCHETYPE)`, Pre `valid_opt`, err `invalid_template` | ″ | `api/definition.rs:291` → `store_template` (register 10) | conformant (seam) |
| A14-11 | `get_opt(UUID): ARCHETYPE`, err `artefact_does_not_exist` | ″ | `definition.rs:153` `opt_get` | conformant (returns OPT XML) |
| A14-12 | `list_opts(offset,fetch): List<UUID>` | ″ | `definition.rs:187` `opt_list` | conformant |
| A14-13 | `list_matching_opts(id_pattern,…): List<ARCHETYPE_ID>` | ″ | `definition.rs:206` `opt_list_matching` | divergent-by-defect — returns `template_id` (G-05-08, spec defect) |
| A14-14 | `delete_opt(UUID)`, Pre/Post, err `invalid_template` | ″ | `definition.rs:225` `opt_delete` | conformant |
| A14-15 | `archetypes_count(): Integer` | ″ | `definition.rs:129` `archetype_count` | conformant |
| A14-16 | `opts_count(): Integer` | ″ | `definition.rs:242` `opt_count` | conformant |

### 1.2 `I_DEFINITION_ADL2` (`i_definition_adl2.adoc`)

Artefacts (archetype / template / operational_template) keyed by
`ARCHETYPE_HRID`. 14 operations. Validity is a register-10 concern
(`adl2_validation.rs`); this register owns only the store/list/count seam.

| # | Spec op | Cite | Code | Verdict |
|---|---------|------|------|---------|
| A2-1 | `has_artefact(HRID): Boolean` | i_definition_adl2 | `definition.rs:254` `adl2_exists` | conformant |
| A2-2 | `valid_artefact(AUTHORED_ARCHETYPE): Boolean` | ″ | `definition.rs:642` `valid_adl2_source` → register 10 | divergent — registration-subset only (G-05-02) |
| A2-3 | `upload_artefact(...)`, Pre `valid_artefact`, Post `has_artefact`, "replace it" | ″ | `definition.rs:268` `adl2_upload` | divergent — native replaces; wire 409 (G-05-12); `template_overlay` CHECK defect (G-05-07) |
| A2-4 | `get_artefact(HRID): AUTHORED_ARCHETYPE`, err `artefact_does_not_exist` | ″ | `definition.rs:326` `adl2_get` | conformant (returns ADL2 source, G-05-03) |
| A2-5 | `list_artefacts(offset,fetch): List<HRID>` | ″ | `definition.rs:340` `adl2_list` | conformant |
| A2-6 | `list_archetypes(): List<HRID>` (kind = AUTHORED_ARCHETYPE) | ″ | `definition.rs:353` `adl2_list_by_kind("archetype")` | conformant |
| A2-7 | `list_templates()` (kind = TEMPLATE) | ″ | ″ `("template")` | conformant |
| A2-8 | `list_opts()` (kind = OPERATIONAL_TEMPLATE) | ″ | ″ `("operational_template")` | conformant |
| A2-9 | `list_matching_artefacts(id_pattern,…)`, err `invalid_id_pattern` | ″ | `definition.rs:372` `adl2_list_matching` | conformant |
| A2-10 | `delete_artefact(HRID)`, err `artefact_does_not_exist` | ″ | `definition.rs:386` `adl2_delete` | conformant |
| A2-11 | `artefacts_count()` | ″ | `definition.rs:402` `adl2_count` | conformant |
| A2-12 | `archetypes_count()` | ″ | `definition.rs:412` `adl2_count_by_kind` | conformant |
| A2-13 | `templates_count()` | ″ | ″ | conformant |
| A2-14 | `opts_count()` | ″ | ″ | conformant |

### 1.3 `I_DEFINITION_QUERY` (`i_definition_query.adoc`; master04 §Registered Queries, §Query Formalism)

Queries keyed by qualified name `<namespace>::<query-name>` (or three-part
`<namespace>::<formalism>::<query-name>`); `"misc"` default namespace;
formalism string case-insensitive with optional `::version`. 8 operations +
`QUERY_DESCRIPTOR`.

| # | Spec op | Cite | Code | Verdict |
|---|---------|------|------|---------|
| DQ-1 | `has_query(name): Boolean` | i_definition_query | `definition.rs:462` `query_exists` | conformant |
| DQ-2 | `valid_query(text,type): Boolean` | ″ | `definition.rs:634` `valid_query_source` | divergent — AQL-only (G-05-06) |
| DQ-3 | `store_query(text,type,name?): QUERY_DESCRIPTOR`, Pre valid | ″ | `definition.rs:484` `query_store_sm` + `stored_query.rs:33` | conformant (naming defect handled G-05-09) |
| DQ-4 | `store_query_set(name?): UUID` — "TODO: determine details" | ″ | trait default `NotImplemented`/501 | conformant (spec TODO, G-05-10) |
| DQ-5 | `list_queries(offset,fetch): List<QUERY_DESCRIPTOR>` | ″ | `definition.rs:526` `query_list` | conformant |
| DQ-6 | `list_matching_queries(id_pattern,artefact_id_pattern,…)` | ″ | `definition.rs:553` `query_list_matching` | divergent — artefact pattern scans raw text (G-05-05) |
| DQ-7 | `delete_query(name)`, Pre/Post | ″ | `definition.rs:582` `query_delete` | conformant (deletes all versions of name) |
| DQ-8 | `queries_count(): Integer` | ″ | `definition.rs:606` `query_count` | conformant (distinct names) |
| DQ-D | `QUERY_DESCRIPTOR{qualified_query_name, version?, registration_time, formalism, source?}` | query_descriptor | `descriptor_from_row` `definition.rs:655`; type `sm/definition/query.rs` | conformant |
| DQ-N | Qualified-name default namespace `"misc"` | master04 §Registered Queries | `qualify` `definition.rs:722` | conformant |
| DQ-F | Formalism parse (`AQL`≡`aql`≡`AQL::1`, major 1 default) | master04 §Query Formalism | `is_aql_v1` `definition.rs:752` | conformant (parse); AQL-only accept (G-05-06) |
| DQ-3P | Three-part `<ns>::<formalism>::<query-name>` decomposition | master04 §Registered Queries | `split_qualified` first-`::` only `definition.rs:733` | **divergent** — formalism segment folded into name (G-05-04) |

### 1.4 `I_QUERY_SERVICE` (`i_query_service.adoc`; master08)

2 operations; `RESULT_SET` assembly. This is the execution-orchestration heart
of the register (the *engine* below is register 08).

| # | Spec op / attr | Cite | Code | Verdict |
|---|----------------|------|------|---------|
| Q-1 | `execute_stored_query(exec_spec, row_offset?, rows_to_fetch?, ehr_ids?): RESULT_SET`, err `ehr_id_does_not_exist` | i_query_service | `api/query.rs:23` → `aql_query.rs:41` `execute_aql`; stored resolve `stored_query.rs:100` | conformant |
| Q-2 | `execute_ad_hoc_query(exec_spec, …): RESULT_SET`, err `ehr_id_does_not_exist` | ″ | `api/query.rs:15` → `execute_aql` | conformant |
| Q-3 | `STORED_QUERY_EXECUTE_SPEC{qualified_query_name, version?, query_parameters}`; version latest when absent | stored_query_execute_spec | `sm query/service.rs` args + `AqlQueryRequest`; resolve `stored_query.rs:100` | conformant (partial-SEMVER = superset, G-05-13) |
| Q-4 | `ADHOC_QUERY_EXECUTE_SPEC{source, formalism="aql", query_parameters}` | adhoc_query_execute_spec | `AqlQueryRequest` (`request.rs`); formalism fixed to AQL | divergent-by-doc — formalism not carried on wire (G-05-06) |
| Q-5 | `row_offset`/`rows_to_fetch` (≤0 ⇒ zero/all) | i_query_service params | `compose_paging` `aql_query.rs:259`; collision-reject `400` | conformant |
| Q-6 | `ehr_ids: List<UUID>` scope; empty ⇒ population over `is_queryable` EHRs | i_query_service `_ehr_ids_` | `resolve_ehr_ids` `aql_query.rs:153`; `AqlQueryRequest.ehr_ids: Vec<String>` | conformant (multi-EHR realized — 08-query G-1 CLOSED) |
| Q-7 | err `ehr_id_does_not_exist` (well-formed but absent id) | ″ `.Errors` | `resolve_ehr_ids` `aql_query.rs:169` `SmError::ehr_not_found` | conformant (08-query G-2 CLOSED) |
| Q-8 | `RESULT_SET.columns: List<RESULT_SET_COLUMN{name, archetype_id?, path?}>` | result_set_column | `result_set_json` `aql_query.rs:320`; `ColumnMeta{name,path}` `aql/exec.rs:21` | divergent — `archetype_id` never populated (G-05-05b; optional [0..1]) |
| Q-9 | `RESULT_SET.id [1..1]` "unique identifier of this result set" | result_set | not emitted (`result_set_json` emits meta/q/columns/rows) | **missing** (G-05-03q; ITS-REST 1.0.3 also omits — divergence) |
| Q-10 | `RESULT_SET.creation_time [1..1]` | result_set | `meta._created` = `Timestamp::now()` `aql_query.rs:333` | conformant (as meta field) |
| Q-11 | `RESULT_SET.rows: List<RESULT_SET_ROW{values}>` | result_set_row | `result.rows` `aql_query.rs:338` | conformant |
| Q-12 | executed text = "all parameters substituted" | result_query_descriptor | `substitute_params` `aql_query.rs:295` → `meta._executed_aql` | conformant (08-query G-4 CLOSED) |

### 1.5 QUERY-spec service-level semantics (`master03-syntax.adoc`)

| # | Spec item | Cite | Code | Verdict |
|---|-----------|------|------|---------|
| QS-1 | Named parameters: `$` + identifier (letters/digits/underscore, not reserved) | AQL master03 §Parameters/Syntax (l.106) | `PARAM_REF` regex `aql_query.rs:283`; `build_params` `aql_query.rs:234` | conformant |
| QS-2 | Substitution quoting: strings/dates quoted, numbers/booleans not | AQL master03 §Parameters NOTE (l.113) | `render_param` `aql_query.rs:307` (Str single-quoted, doubled `'`; Int/Real/Bool bare; Null→NULL) | conformant |
| QS-3 | Parameter resolution is application/environment-supplied | AQL master03 §Parameter Resolution (l.132) | callers pass `query_parameters` map; unbound → engine rejects at plan (register 08) | conformant (seam boundary) |

### 1.6 BASE identification lexical law (`master05-identification_package.adoc`)

| # | Spec item | Cite | Code | Verdict |
|---|-----------|------|------|---------|
| ID-1 | `ARCHETYPE_ID` grammar (`issuer-model-class.concept{-spec}*.vN`) | BASE master05 §Archetype Identifiers | `extract_archetype_id` via `ArchetypeId::from_str` `definition.rs:776` | conformant (lexical) |
| ID-2 | `ARCHETYPE_HRID` (AOM2, superset with namespace + full `.vN.N.N`) | BASE master05 §104; AM referencing | `valid_adl2_hrid` reuses `ArchetypeId::from_str` `definition.rs:800` | divergent — accepts HRID shape but not strict AOM2 `version_status`/`build_count` (register-10 parser dep) |
| ID-3 | `TEMPLATE_ID` | BASE master05 §template_id.adoc | OPT `template_id` string keys `template_store` (register 10) | spec-silent-at-seam — validated at template ingestion (register 10) |
| ID-4 | Composite identifiers **case-insensitive** equality (§Composite Identifiers and Case) | BASE master05 §164-169 | qualified-name/HRID/archetype-id all compared **case-sensitively** in SQL | **divergent** — no case-fold on identity (G-05-14) |

---

## 2. Code with no spec item (dispositions)

| Code | What | Disposition |
|------|------|-------------|
| `DefinitionAdapter` (wire shapes: `template_*_list`, `*_example`, `query_store`, glob filters) `api/definition.rs:37-235` | ITS-REST wire-only rich shapes the SM interfaces do not express | **keep** — ITS/REST register concern; move to `service/definition/wire.rs` in the rewrite; drives no new spec item |
| `opt_get_by_template_id` `definition.rs:170` | wire address by `template_id` (SM keys OPTs by UUID) | keep — ITS-REST wire helper; spec-silent (no SM op), flag "wire address, our own" |
| `adl2_template_list` `definition.rs:431` | ADL2 `{template_id,created_timestamp}` wire metadata list | keep — wire/register-10 seam; flag spec-silent |
| `subject_scope` / `collect_attributes` on `AqlQueryRequest` `request.rs:56-63` | ABAC patient-scope + post-check attribute collection | keep — **flag: no openEHR spec governs this — our own access-control extension** |
| `EHRBASE_QUERY__TIMEOUT_MS` / `408` path `aql_query.rs:104-113,182` | per-query execution budget | keep — **flag: no openEHR spec governs a query timeout — our own operational extension** (ITS-REST `408` is the wire rendering) |
| AQL Prometheus metrics `aql_query.rs:205-212` | `aql_queries_total` / `aql_query_duration_seconds` | keep — spec-silent observability; our own design |
| `DEFAULT_QUERY_VERSION = "1.0.0"` no-version upsert `stored_query.rs:17` | auto-version scheme for no-version stores | keep with PORT NOTE — no auto-increment across differing text (spec-silent; ITS-REST `definition_query_store` sanctions "stores or updates") |

No delete/quarantine candidates: every function maps to an SM op, a wire
binding, or a flagged own-design extension.

---

## 3. G-row register

Consolidated from the two absorbed audits, re-scoped to the service seam and
re-verified against current code. **Severity**: H = spec MUST / conformance-
visible; M = spec SHOULD / behavioural divergence; L = cosmetic / documented
defect. **Disposition**: fix-in-rewrite / PORT NOTE / already-correct /
quarantine / delete.

| G-id | Item / cite | Sev | Disposition |
|------|-------------|-----|-------------|
| G-05-01 | ADL 1.4 archetype validity lexical, not AOM (`valid_archetype`/`upload_archetype`) — i_definition_adl14 | M | **PORT NOTE** — depends on register-10 ADL 1.4 source parser (untracked; file WORKLIST row). Keep structural check, label it honestly at `valid_archetype_source`. |
| G-05-02 | ADL2 validity is registration-subset, not AOM2 catalogue — i_definition_adl2; AOM2 master08 | M | **PORT NOTE** — subsumed by WORKLIST W-4 (register 10). Service seam records the fact only. |
| G-05-03 | Interchange form (String source/XML) vs SM AOM object signatures — i_definition_adl14/adl2 | L | **already-correct** — openEHR has no BMM for AOM instances; keep PORT NOTE. |
| G-05-04 | Three-part `<ns>::<formalism>::<query-name>` folded into name (first-`::` split) — master04 §Registered Queries | M | **fix-in-rewrite** — decompose into `(namespace, formalism?, query-name)`; formalism → `query_type`; store key = ns + bare name. |
| G-05-05 | `list_matching_queries` `artefact_id_pattern` scans raw source text, not extracted artefact ids — i_definition_query | M | **fix-in-rewrite (dep register 08)** — when the AQL engine can enumerate FROM/CONTAINS archetype/template ids, match against that set; until then tighten PORT NOTE. |
| G-05-05b | `RESULT_SET_COLUMN.archetype_id` never populated — result_set_column ([0..1]) | L | **fix-in-rewrite (dep register 08)** — populate from path analysis when known; else omit (optional). |
| G-05-06 | Non-AQL formalism rejected; `ADHOC_QUERY_EXECUTE_SPEC.formalism` not carried on wire — master04 §Query Formalism; adhoc_query_execute_spec; query_descriptor | M | **fix-in-rewrite** — choose accept-and-store (spec-aligned "any other string value") vs explicit typed reject; document at trait. Current `request.rs` fixes the query-side doc claim (08-query G-6 → option b). |
| G-05-07 | `template_overlay` ADL2 upload → 500 (CHECK `ck_adl2_artefact_kind` excludes it) — i_definition_adl2 | H | **fix-in-rewrite** — extend the CHECK or map `template_overlay`→`template`; an upload must never hit a DB constraint. *Storage `kind` enum is our own design — no openEHR spec governs it.* |
| G-05-08 | `list_matching_opts` typed `List<ARCHETYPE_ID>` for UUID-keyed OPTs — i_definition_adl14 (spec defect) | L | **PORT NOTE** — return `template_id` strings (already correct handling). |
| G-05-09 | `store_query` `Pre is_valid_query` vs actual `valid_query` (spec naming defect) — i_definition_query | L | **already-correct** — enforce `valid_query`, PORT NOTE. |
| G-05-10 | `store_query_set` "TODO: determine details" — i_definition_query | L | **already-correct** — `NotImplemented`/501 until spec defines it. |
| G-05-11 | "PERL regular expression" served by RE2 (`regex` crate) — list_matching_* | L | **PORT NOTE** — backref/lookaround patterns fail compile → `invalid_id_pattern` (correct status, narrower envelope). |
| G-05-12 | ADL2 wire upload 409 vs SM/master04 "replace it" — master04 §Archetypes and Templates; ITS-REST DEFINITION | L | **PORT NOTE** — wire/SM split; native replaces, wire 409 per ITS-REST. |
| G-05-13 | Partial-SEMVER stored-query resolution (`{major}`/`{major}.{minor}`→highest) — stored_query_execute_spec | L | **already-correct** — deliberate superset of "3-part semver, else latest"; PORT NOTE so not mistaken for drift. |
| G-05-03q | `RESULT_SET.id [1..1]` not emitted — result_set (MUST in SM; ITS-REST 1.0.3 omits) | M | **fix-in-rewrite** — emit a `uuidv7()`-derived id additively; PORT NOTE the SM/ITS-REST divergence. |
| G-05-14 | Composite identifiers compared case-**sensitively** (qualified names, HRIDs, archetype ids) — BASE master05 §Composite Identifiers and Case | M | **fix-in-rewrite** — case-insensitive equality (case-preserving storage) at the identity boundary; file the unregistered BASE finding. |
| G-05-15 | `template_does_not_exist` defined but `artefact_does_not_exist` emitted for missing OPT — definition_call_status_type | L | **fix-in-rewrite (cheap)** — emit `TemplateDoesNotExist`; no wire effect (both 404). |

**Counts by disposition:** fix-in-rewrite 7 (G-04, -05, -05b, -06, -07, -03q,
-14, -15 → note -15 cosmetic) · PORT NOTE 5 (G-01, -02, -08, -11, -12) ·
already-correct 4 (G-03, -09, -10, -13) · quarantine 0 · delete 0.
(Closed since 08-query.md was written: multi-EHR scope, `ehr_id_does_not_exist`,
executed-AQL substitution — verified against current `aql_query.rs`/`request.rs`.)

---

## 4. Target design

Mirror `app/ehrbase-sm/src/services/{definition,query}/` under
`app/ehrbase/src/service/{definition,query}/`; internal decomposition follows
the spec's own component split; every file ≤ ~700 lines.

```
app/ehrbase/src/service/definition/
  mod.rs            — module wiring + shared helpers (qualify, split, paginate,
                      page_bounds, compile_pattern) [was free fns in definition.rs]
  adl14.rs          — I_DEFINITION_ADL14 impl (archetypes + OPT-by-UUID seam)   ~230
  adl2.rs           — I_DEFINITION_ADL2 impl (adl2_artefact store/list/count)   ~180
  query_store.rs    — I_DEFINITION_QUERY + QUERY_DESCRIPTOR build + stored_query
                      CRUD (absorbs today's stored_query.rs)                    ~340
  wire.rs           — DefinitionAdapter (ITS-REST wire shapes, glob filters,
                      template list metadata) — ITS/REST register owns behaviour ~210
app/ehrbase/src/service/query/
  mod.rs            — module wiring
  execute.rs        — QueryService impl + execute_aql orchestration
                      (parse→plan→execute→assemble), paging composition,
                      ehr_ids resolution, timeout                              ~300
  result_set.rs     — RESULT_SET / RESULT_SET_COLUMN / RESULT_SET_ROW assembly,
                      param substitution                                      ~150
```

**Seams (TODO(w3f-integrate) candidates — do not implement here):**
- `crate::aql` engine (register 08): `plan`, `execute`, `expand_matches`,
  `collect_scope`, `ColumnMeta` (source of `archetype_id` for G-05-05b),
  FROM/CONTAINS artefact enumeration (source for G-05-05).
- `service::template` / `opt_validation` / `adl2_validation` (register 10):
  `store_template`, `validate_opt_structure`, `opt14::from_xml`,
  `validate_adl2_source` — consumed by A14-9/10/11, A2-2/3, `template_overlay`
  kind (G-05-07).
- `openehr_query::parser::parse_str` — stored-query AQL syntactic gate (kept).

**Key design decisions:**
1. **Traits are fixed; only the impl moves.** The `sm` catalog traits and
   `AqlQueryRequest`/`QueryOutcome`/`QueryDescriptor` are the contract — the
   rewrite re-homes the `EhrbaseService` impl bodies, it does not touch the
   seam.
2. **Stored-query store folds into `definition/query_store.rs`** — DEFINITION
   *owns* query registration (master04 §Registered Queries); QueryService only
   *resolves + executes* it. The current split (`stored_query.rs` vs
   `definition.rs`) is arbitrary and merges.
3. **`RESULT_SET` assembly isolated** in `query/result_set.rs` so SM-vs-ITS-REST
   shape divergences (G-05-03q `id`) live in one place, spec-cited.
4. **Identity canonicalisation at the store boundary** (G-05-14): a single
   `fn canonical_name`/`fn canonical_hrid` applied on write + lookup, so the
   BASE case-insensitivity rule is enforced once, not scattered.
5. **Formalism decomposition** (G-05-04, G-05-06): one `parse_qualified_name`
   producing `(namespace, formalism?, bare_name)` per master04, shared by
   store/has/delete so a three-part name round-trips.

---

## 5. PORT-NOTE residue (keep / re-verify / drop)

| PORT NOTE | Verdict |
|-----------|---------|
| Interchange form (ADL/OPT serialization vs AOM objects) — `definition.rs:11`, `sm definition/mod.rs` | **keep** — spec-true, re-cite i_definition_adl14/adl2 |
| `list_matching_opts` returns `template_id` (SM `List<ARCHETYPE_ID>` defect) — `definition.rs:202` | **keep** |
| PERL vs RE2 pattern compile — `definition.rs:706` | **keep** |
| ADL2 wire 409 vs native replace — `api/definition.rs:88` | **keep** — move to `wire.rs`, re-cite master04 + ITS-REST |
| `store_query` `is_valid_query` naming — `definition.rs:481`, `sm definition/query.rs` | **keep** |
| `store_query_set` spec TODO → 501 — `sm definition/query.rs` | **keep** |
| No-version auto-version scheme deferred — `stored_query.rs:14` | **re-verify** — decide auto-increment vs upsert-at-1.0.0 in rewrite; keep or resolve |
| `adl2_template_list` omits `concept`/`archetype_id` (no cADL parser) — `definition.rs:425` | **keep** — register-10 dep |
| Multi-EHR `ehr_ids` extension / `RESULT_SET.id` SM-vs-ITS-REST / formalism subset — 08-query §4 | **re-verify** — multi-EHR now realized (drop the "extension body only" framing); keep the SM/ITS-REST `id` divergence note; keep formalism-subset note |
| ABAC `subject_scope`/`collect_attributes`, query timeout — `request.rs`, `aql_query.rs` | **keep** — re-label "no openEHR spec governs this — our own extension" (drop any ADR framing) |
