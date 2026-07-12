# Definition Service (SM-2) — spec-compliance audit

Read-only audit (2026-07-12) of the openEHR SM **Definitions** package against
its realization in the tree. Scope: the three service interfaces
`I_DEFINITION_ADL14`, `I_DEFINITION_ADL2`, `I_DEFINITION_QUERY` and the
supporting structures `QUERY_DESCRIPTOR` / `DEFINITION_CALL_STATUS_TYPE`.

**Spec oracle** (read these before any change):

- `docs/specs/openehr/SM/docs/openehr_platform/master04-definition_package.adoc`
  (the chapter: overview, archetypes/templates, registered queries, the
  qualified-name scheme, query-formalism parsing)
- `docs/specs/openehr/SM/docs/UML/classes/` —
  `i_definition_adl2.adoc`, `i_definition_adl14.adoc`,
  `i_definition_query.adoc`, `query_descriptor.adoc`,
  `definition_call_status_type.adoc`
- Adjacent: `master02-overview.adoc` §List Handling (`item_offset` /
  `items_to_fetch` semantics); `docs/specs/openehr/AM/docs/AOM2/master08-validation.adoc`
  (the AOM2 validation catalogue the ADL2 surface can only partly enforce);
  the ITS-REST `DEFINITION` group OAS (the wire the service is bound to on
  the `/definition/template/*` and `/definition/query/*` routes)

**Current implementation** (verified 2026-07-12):

- Native catalog traits (one per SM interface, method-for-method):
  `app/ehrbase-sm/src/services/definition.rs` (456 lines) —
  `DefinitionAdl14Service`, `DefinitionAdl2Service`, `DefinitionQueryService`,
  each method defaulting to `NotImplemented`.
- `QUERY_DESCRIPTOR` native type: `app/ehrbase-sm/src/types.rs:180-196`;
  `DEFINITION_CALL_STATUS_TYPE`: `app/ehrbase-sm/src/error.rs:43-158`
  (all seven spec literals present, `error.rs:145-151`).
- Service logic: `app/ehrbase/src/service/definition.rs` (865 lines) —
  ADL 1.4 archetypes on `archetype_store`, ADL 1.4 OPTs on `template_store`,
  ADL2 artefacts on `adl2_artefact`, queries on `stored_query`.
- ADL2 registration-side validity:
  `app/ehrbase/src/service/adl2_validation.rs` (906 lines) — a structural +
  terminology-side subset of the AOM2 catalogue over unparsed source.
- Trait glue (`impl … for EhrbaseService`) + wire-shaped `DefinitionAdapter`
  extension: `app/ehrbase/src/service/api/definition.rs` (324 lines).
- REST dispatch (templates + queries only): `app/ehrbase-rest/src/dispatch/definition.rs`
  (406 lines). SM→HTTP status mapping: `app/ehrbase-rest/src/error.rs:51-83`.
- Schema: `archetype_store`, `adl2_artefact` in
  `app/ehrbase/migrations/ehr/0001_baseline.sql:495-523` (+ `template_store`,
  `stored_query` reused).

**Overall verdict — SUBSTANTIALLY COMPLIANT at the interface, PARTIAL on
validity depth.** Every one of the 38 operations the three interfaces define
is present on the native trait and (except the two the spec itself leaves
open) has a real service body; the qualified-name default namespace,
formalism-string parsing, `DEFINITION_CALL_STATUS_TYPE` coverage,
`QUERY_DESCRIPTOR` attribute set, list pagination, and SM→HTTP status mapping
are all faithful. The honest gaps are (1) *validity is structural, not
semantic* — no ADL 1.4 source parser and no AOM2 compiler, so `valid_*` and
the `upload_*` preconditions are lexical checks (the ADL2 side is the tracked
**WORKLIST W-4**, the ADL 1.4 side is untracked); (2) the three-part
`<namespace>::<formalism>::<query-name>` name scheme is not decomposed; (3)
non-AQL formalisms are rejected outright; (4) one storage-constraint defect
(`template_overlay`); (5) a handful of documented spec-defect / interchange
PORT NOTEs that are honest and correct. No fabricated capability; the
`NotImplemented` seams are exactly where the spec is a TODO or where a cADL
parser is absent.

---

## 1. Gap register (what is not spec-true today)

Each row cites the governing spec text and the code evidence. G-1/G-2 are the
substance of "validity is not real yet"; G-4/G-6/G-7 are genuine behavioural
divergences; the remainder are documented spec defects, deliberate interchange
choices, or correct handling of spec TODOs (recorded for completeness).

| # | Gap | Spec citation | Today (file:line) |
|---|-----|---------------|-------------------|
| G-1 | **ADL 1.4 archetype validity is lexical, not AOM validation.** `valid_archetype` / `upload_archetype` ("The archetype must be valid to succeed") only confirm the source opens with the `archetype` keyword line and that line 2 parses as a well-formed `ARCHETYPE_ID`. No ADL 1.4 source parser, no cADL/AOM constraint checking, no terminology-code validation. **Not covered by W-4** (which is ADL2-only) — this is an untracked validation gap. | `i_definition_adl14.adoc` (`valid_archetype`, `upload_archetype`) | `valid_archetype_source` delegates to `extract_archetype_id().is_some()` (`service/definition.rs:620-622`); `extract_archetype_id` is header-keyword + `ArchetypeId::from_str` on the next line (`service/definition.rs:776-789`). |
| G-2 | **ADL2 artefact validity is a registration-surface subset, not the AOM2 catalogue.** `valid_artefact` / `upload_artefact` `Pre_valid` run only the structural + terminology-decidable rules an uncompiled source permits (STCNT, VARAV/VARRV, VARDT, VARCN, VACSD, VOLT/VOTM/VTLC, VATDF/VACDF, value-set + binding rules). The cADL-semantic, RM-conformance (`VCxxx`), and specialisation-flattening (`VSxxx`) families are not run — no ADL2/cADL source parser or flattener exists. **Tracked as WORKLIST W-4** ("ADL2 — full implementation, spec-exact"); reference it, do not re-plan here. | `i_definition_adl2.adoc` (`valid_artefact`, `upload_artefact`); `AOM2/master08-validation.adoc` | Whole module `service/adl2_validation.rs` (see its head PORT NOTE, lines 39-45); driven from `service/definition.rs:268-321`, `:642-645`. |
| G-3 | **Interchange form diverges from the SM object signatures — deliberate.** The SM types params/returns as AOM objects (`upload_archetype(an_arch: ARCHETYPE)`, `get_artefact(): AUTHORED_ARCHETYPE`, `valid_opt(an_opt: ARCHETYPE)`). The native API exchanges the **serializations the platform ingests** — ADL 1.4 source text, ADL2 source text, OPT 1.4 canonical XML — because openEHR has no BMM meta-model for AOM instances. Spec-justified; residue only. | `i_definition_adl14.adoc`, `i_definition_adl2.adoc` (signatures) | Documented at `services/definition.rs:19-28` and `:213-221`; every trait method takes/returns `String`. |
| G-4 | **The three-part qualified-name scheme is not decomposed.** master04 defines two forms: `<namespace>::<query-name>` and `<namespace>::<formalism>::<query-name>`. The store splits on the **first** `::` into `(reverse_domain_name, semantic_id)`, so a three-part name like `task_planning::aql::chemotherapy_plans` stores `semantic_id = "aql::chemotherapy_plans"` — the formalism segment is silently folded into the name rather than recognised. | `master04-definition_package.adoc` §Registered Queries | `qualify` (`service/definition.rs:722-728`) and `split_qualified` split on the first `::` only (`service/definition.rs:733-737`). |
| G-5 | **`list_matching_queries` matches the raw source text, not extracted artefact ids.** The spec's `artefact_id_pattern` is a regex "on archetype / template identifiers referenced in the query"; with no AQL artefact extractor the impl substring-scans the whole stored query text, so the pattern can match (or miss) on incidental text rather than on the query's referenced artefacts. | `i_definition_query.adoc` (`list_matching_queries`) | `service/definition.rs:553-575` (+ PORT NOTE `:549-552`): `d.source … re.is_match(src)`. |
| G-6 | **Non-AQL formalisms are rejected.** `valid_query` returns `false` for anything but AQL major-1, so `store_query` for any other formalism is `invalid_query` (→ 422) — even though the chapter's own naming examples include `task_planning::aql::…` and `QUERY_DESCRIPTOR.formalism` explicitly permits "any other string value", and the service is described as storing "queries, and query sets" for "any model-like or reference artefacts". The service can only hold AQL. | `master04-definition_package.adoc` §§Overview, Query Formalism; `query_descriptor.adoc` (`formalism`) | `valid_query_text` = `is_aql_v1(a_type) && openehr_query parse` (`service/definition.rs:745-747`, `:752-765`). |
| G-7 | **`template_overlay` ADL2 uploads fail on the storage CHECK constraint (→ 500).** The validator classifies and returns `kind = "template_overlay"` for a `template_overlay` header, but `adl2_artefact.kind` only permits `archetype`/`template`/`operational_template`, so the `INSERT` violates `ck_adl2_artefact_kind` and surfaces as an internal error instead of a clean store or a typed reject. A real defect. | `i_definition_adl2.adoc` (`upload_artefact`: templates are archetype instances) | `validate_adl2_source` returns `"template_overlay"` (`service/adl2_validation.rs:99`); `adl2_upload` binds `meta.kind` (`service/definition.rs:309-319`); CHECK excludes it (`migrations/ehr/0001_baseline.sql:520`). |
| G-8 | **`list_matching_opts` return type is a spec defect, handled.** The SM types the return `List<ARCHETYPE_ID>` though ADL 1.4 OPTs are UUID-keyed; the impl returns the OPTs' `template_id` strings (the meaningful identifier a pattern is useful against). Correct pragmatic reading; recorded as spec defect. | `i_definition_adl14.adoc` (`list_matching_opts`) | `service/definition.rs:199-220` (PORT NOTE `:202-205`). |
| G-9 | **`store_query` precondition naming inconsistency, handled.** The SM precondition is written `is_valid_query(a_query_text)` but the actual function is `valid_query(text, type)`. The impl enforces `valid_query` and rejects invalid queries as `invalid_query` (→ 422). Spec defect, correctly resolved. | `i_definition_query.adoc` (`store_query` `Pre_valid_query`) | `service/definition.rs:481-495`; trait doc `services/definition.rs:387-390`. |
| G-10 | **`store_query_set` unimplemented — correct handling of a spec TODO.** The SM entry is an explicit "TODO: determine details"; the method keeps the trait default (`NotImplemented` → 501) rather than inventing semantics. | `i_definition_query.adoc` (`store_query_set`: "TODO: determine details") | Trait default not overridden (`services/definition.rs:403-413`); noted `service/api/definition.rs:298-299`. |
| G-11 | **"PERL regular expression" served by an RE2 engine.** The `regex` crate is RE2-class (no backreferences / lookaround); a PERL pattern using them fails to compile and surfaces as `invalid_id_pattern` (→ 400). A legitimate PERL pattern can therefore be rejected. Documented PORT NOTE; the outcome (unusable-pattern → `invalid_id_pattern`) is the correct SM status even if the acceptance envelope is narrower. | `i_definition_query.adoc` / `i_definition_adl2.adoc` / `i_definition_adl14.adoc` (`list_matching_*`: "PERL regular expression") | `compile_pattern` (`service/definition.rs:706-717`). |
| G-12 | **ADL2 wire upload = 409 vs the SM/master04 "replace it".** master04 says ADL2 `upload_artefact` must **replace** an existing same-identifier artefact. The native `upload_artefact` does replace (upsert), but the REST adapter returns `409` on an existing HRID to satisfy the ITS-REST `409_template_already_exists` response. Deliberate wire/SM split; worth flagging because the wire behaviour contradicts the chapter prose. | `master04` §Archetypes and Templates (`upload_artefact` "replace it"); ITS-REST `DEFINITION` OAS | native replace (`service/definition.rs:264-321`); wire 409 (`service/api/definition.rs:67-89`). |
| G-13 | **ADL2 `example` / `version` wire operations → 501.** Both need a cADL/AOM2 source model (example generator / `OperationalTemplateV2` JSON projection) the tree lacks. ADL2 is OPTIONAL for CNF and these are untested; tied to W-4. | ITS-REST `DEFINITION` OAS (`/definition/template/adl2/{id}/example`, `…/{version}`) | `dispatch/definition.rs:210-221` (`NotImplemented`). |
| G-14 | **`template_does_not_exist` is defined but never emitted for a missing OPT.** `DEFINITION_CALL_STATUS_TYPE` has a distinct `template_does_not_exist`, but a missing OPT reports `artefact_does_not_exist`. Cosmetic only — both map to `404` (`error.rs:63-67`). | `definition_call_status_type.adoc` (`template_does_not_exist`) | `opt_get` / `opt_delete` use `ArtefactDoesNotExist` (`service/definition.rs:159-164`, `:232-237`); enum literal present unused (`error.rs:102,151`). |
| G-15 | **ADL 1.4/ADL2 source-archetype and all `*_count` operations have no wire — native-API only.** `has_archetype`, `upload_archetype`, `get_archetype`, `list_archetypes`, `list_matching_archetypes`, `delete_archetype`, `has_artefact`, `get_artefact`, `delete_artefact`, and every `*_count` are reachable only through the native trait; the 39 generated `DEFINITION` routes cover only templates (OPTs) + queries. **Consistent with ITS-REST 1.0.3** (which defines no archetype-source or count endpoints) — a scope note, not a defect. | ITS-REST `DEFINITION` OAS (no archetype-source routes); `master04` (interfaces are the abstract model) | `dispatch/definition.rs:66-289` handles only `definition_template_*` + `definition_query_*`; no `/definition/archetype` route exists. |

---

## 2. What is faithful (evidence, not intent)

Recorded so the audit is honest about strengths, not only gaps:

| Claim | Evidence |
|-------|----------|
| All 38 interface operations present on the native traits with SM-exact names, params, spec-cited pre/post-conditions | `services/definition.rs` — ADL14 (16 methods), ADL2 (14), Query (8), each doc-commented with its `.adoc` origin |
| `DEFINITION_CALL_STATUS_TYPE` fully modelled and correctly mapped to HTTP | all seven literals at `error.rs:145-151`; mapping `invalid_*`→422, `*_does_not_exist`→404, `invalid_id_pattern`→400, `not_implemented`→501 (`ehrbase-rest/error.rs:51-83`) |
| `QUERY_DESCRIPTOR` complete: `qualified_query_name`, `version`, `registration_time`, `formalism`, `source` | `types.rs:180-196`; built from the `stored_query` row at `service/definition.rs:655-680` |
| Qualified-name default namespace `"misc"` applied per master04 | `qualify` (`service/definition.rs:722-728`); unit test `:810-815` |
| Formalism string parsed case-insensitively with optional `::version`, major "1" default (`AQL` ≡ `aql` ≡ `AQL::1`) | `is_aql_v1` (`service/definition.rs:752-765`); unit test `:818-828` |
| List pagination honours master02 (`item_offset`, `items_to_fetch=0 ⇒ all`) | `Page` (`types.rs:29-59`); `paginate`/`page_bounds` (`service/definition.rs:684-701`) |
| ADL 1.4 OPT upload replace/conflict, `Prefer`, `Location`/`ETag` handled at the wire | `dispatch/definition.rs:75-130`; ADL2 `Prefer` body `:311-337` |
| ADL2 registration validity, while a subset, cites each AOM2 rule code by number and rejects with `invalid_artefact` + the code | `service/adl2_validation.rs` (STCNT/VAR*/VARCN/VACSD/VOLT/VOTM/VTLC/VAT*/VTVS*/VTTBK) |

---

## 3. Target design (to close the gaps)

Ordered by value. G-2 and G-13 are **subsumed by WORKLIST W-4** (the full
spec-exact ADL2 implementation: ADL2 + cADL2 + ODIN parser, AOM2 master08
semantic validation on parsed artefacts, specialisation flattening, OPT2,
template semantics) — this audit does not re-plan that work; it only records
the interface-level facts W-4 must satisfy. The items below are the
Definition-specific fixes that are **independent of W-4**.

### 3.1 Storage-constraint defect (G-7) — immediate

Extend `ck_adl2_artefact_kind` to permit `template_overlay` (the validator
already produces it and it is a legitimate ADL2 artefact keyword,
`adl2_validation.rs:99`), **or** map `template_overlay` onto `template` for
storage with a recorded reason. Either way an upload must not reach a DB
constraint violation. Add an upload test for each of the four ADL2 keywords.
*No openEHR spec governs the storage `kind` enumeration — our own design; the
fix aligns it with the artefact keywords master04/AOM2 recognise.*

### 3.2 Qualified-name three-part scheme (G-4)

Parse the query name per master04 §Registered Queries into
`(namespace, formalism?, query-name)`: on a three-`::` split, the middle
segment is the formalism (validated against the `a_type` parameter when both
are supplied). Persist the namespace and the bare query-name as the store key
so `has_query`/`delete_query` round-trip a three-part name; keep the formalism
in `stored_query.query_type`. This removes the silent folding in
`split_qualified` (`service/definition.rs:733-737`).

### 3.3 Non-AQL formalism handling (G-6)

Decide and document one of:
- **(a) accept-and-store** any formalism string (per `QUERY_DESCRIPTOR`
  "any other string value"), with `valid_query` returning `true` for a
  recognised formalism and a syntactic pass only for AQL; or
- **(b) explicit typed reject** for unsupported formalisms with a distinct
  message (not a blanket `invalid_query`), and a recorded PORT NOTE that this
  build stores AQL only.

master04 leaves the *set* of supported formalisms to the implementation but
does not sanction rejecting the qualified-name scheme that references them;
(a) is the more spec-aligned target. Either choice is documented at the trait.

### 3.4 `artefact_id_pattern` over extracted artefacts (G-5)

When the AQL engine can enumerate a query's referenced archetype/template
ids (it parses `FROM`/`CONTAINS` already), match `artefact_id_pattern`
against that set rather than the raw text (`service/definition.rs:553-575`).
Until then keep the substring approximation but with the PORT NOTE tightened
to say it is a text scan, not an artefact-reference scan.

### 3.5 ADL 1.4 archetype-source validity (G-1)

Untracked today. Either (a) fold ADL 1.4 source archetypes into the W-4 parser
work (an ADL 1.4 → AOM adapter), or (b) file a dedicated WORKLIST row. Until a
parser exists, keep the structural check but record it explicitly as a PORT
NOTE at `valid_archetype_source` (it currently reads as if validity were real).

### 3.6 `template_does_not_exist` (G-14)

Cosmetic: emit `TemplateDoesNotExist` for a missing OPT so the SM abstract
status matches the artefact kind. No wire effect (both are 404); do it when
next touching `opt_get`/`opt_delete`.

---

## 4. Standing PORT NOTEs (the honest residue after the fixes)

These are correct and remain, each spec-cited in place:

- **Interchange form** (G-3): the native API exchanges ADL/OPT serializations,
  not AOM objects — openEHR publishes no BMM meta-model for AOM instances, so
  parsing happens inside the service exactly as the ITS-REST wire does.
- **`list_matching_opts` return type** (G-8): SM `List<ARCHETYPE_ID>` for
  UUID-keyed OPTs is a spec defect; we return `template_id` strings.
- **`store_query` precondition naming** (G-9): SM `is_valid_query` vs the
  real `valid_query` — a spec inconsistency; we enforce `valid_query`.
- **`store_query_set`** (G-10): a spec TODO — stays `NotImplemented`/501 until
  the SM defines it.
- **PERL vs RE2** (G-11): backreference/lookaround patterns fail to compile
  and are reported `invalid_id_pattern` — narrower acceptance, correct status.
- **ADL2 wire 409 vs SM replace** (G-12): the ITS-REST OAS declares
  `409_template_already_exists`; native SM callers keep master04 replace. The
  divergence is between the wire contract and the abstract chapter prose.
- **ADL2 semantic validation + example/version** (G-2, G-13): the
  registration surface enforces only what an uncompiled source permits; full
  AOM2 validation, specialisation flattening, and the example/version wire ops
  land with **WORKLIST W-4** (`docs/plans/adl2.md`, to be authored). The
  cADL-semantic (`VCxxx`), redefinition (`VSxxx`), and cADL-syntax (`Sxxx`)
  rule families are inapplicable at this surface, not silently skipped
  (`adl2_validation.rs:39-45`).
