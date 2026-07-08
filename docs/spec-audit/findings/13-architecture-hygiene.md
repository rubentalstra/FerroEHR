# 13 — Architecture / duplication / hygiene

## Summary

The hand-written app + spec-runtime layers are, on the whole, well-factored and
idiomatic — the versioned-object machinery (`vobject`), the storage codec, the
content negotiation, the auth middleware, and the centralized error mapping are
clean, single-source designs. The owner's "many things are double or not
efficiently done, all over the place" instinct is nonetheless correct in a
handful of concrete, fixable places. The recurring themes are:

1. **A scattered `OBJECT_VERSION_ID` codec.** The `{uuid}::{system}::{version}`
   encode/decode is reimplemented 5 times across 3 files, each subtly
   different — no single owning type. (F-13-01, major.)
2. **Two `WebTemplate` caches + two resolution paths** — one in the service, one
   in the REST layer — that can diverge, plus a layering inversion where the
   REST FLAT glue fetches templates by calling the *definition* backend API.
   (F-13-02, major.)
3. **Boilerplate dispatch arms for entirely-unimplemented API groups**
   (demographic: 43 arms / ~393 LoC, query: 6 arms, admin: 3 arms) that all
   route to `NotImplemented` — pure scaffolding that should be one generic 501
   fallthrough. (F-13-03, major.)
4. **Dead dependency wiring** masked by a `cargo-machete` ignore list: `ehrbase`
   declares 6 path deps (incl. the empty `ehrbase-compat`) it never references.
   (F-13-04, major.)
5. **Helpers scattered across domain modules** rather than a shared service
   module (`ensure_ehr_exists` in `composition.rs`, `audit`/`with_uid`/
   `committer` in `ehr.rs`, `audit_details` in `contribution.rs`), and a
   **duplicated load→filter→deleted-check→with_uid read pattern** repeated
   across `composition.rs`/`directory.rs`/`ehr.rs`. (F-13-05, F-13-06.)
6. **The `Backend` seam returns bare `serde_json::Value`** with no header
   channel and forces typed↔Value↔typed round-trips at the wire edge.
   (F-13-07, major.)
7. Minor items: `try_get().unwrap_or_default()` row-mappers swallow DB errors
   (F-13-08); hand-rolled percent/form-url decoding despite `url` in the
   workspace (F-13-09); stale doc comments (F-13-10); empty served OpenAPI doc
   (F-13-11); `ehrbase-compat` is empty scaffolding while its intended contents
   live in `ehrbase-rest` (F-13-12).

Scope: hand-written code in `app/ehrbase`, `ehrbase-rest`, `ehrbase-compat`,
plus the crate-level findings for `openehr-flat`, `openehr-its` runtimes/opt14,
`openehr-query`, and `openehr-term` (sections F-13-20+ below, from the
per-crate audit passes). `// @generated` files were not style-reviewed.

## Findings

### F-13-01: `OBJECT_VERSION_ID` codec reimplemented 5× across 3 files, no owning type
- **Severity:** major
- **Code:**
  - encode: `app/ehrbase/src/service/ehr.rs:209` `object_version_id()` → `format!("{vo_id}::{system}::{sys_version}")`
  - decode: `app/ehrbase/src/service/api/ehr.rs:355` `parse_object_id()`, `:368` `parse_version_uid()`, `:379` `expected_from_if_match()`
  - decode again: `app/ehrbase/src/service/ehr.rs:279` `parse_expected_version()` (a near-duplicate of `expected_from_if_match`)
  - decode again: `app/ehrbase/src/service/contribution.rs:279` `parse_preceding()` (splits `::`, takes `nth(1)` for the version)
- **Problem:** The openEHR `OBJECT_VERSION_ID` has a defined three-part grammar
  (`object_id "::" creating_system_id "::" version_tree_id`). Here it is a
  bag of ad-hoc `split("::")`/`rsplit("::")` calls with divergent behaviour:
  `parse_object_id` takes the *last* `::` segment as the version;
  `parse_preceding` takes `nth(1)` (the middle-onward) — these disagree when the
  system id itself contains `::`, and none validate the middle system-id
  component. `expected_from_if_match` and `parse_expected_version` are two
  copies of the same If-Match parser with slightly different trimming. This is
  exactly the "same thing done several ways in several places" the owner flags,
  and it is correctness-relevant (If-Match / preceding-version parsing feeds
  optimistic concurrency).
- **Target design:** One `ObjectVersionId { object_id: Uuid, system_id: String,
  version: i32 }` value type (in a new `service/version_id.rs`, or better in a
  shared domain module) with `Display` (the encoder) and `FromStr` + a
  `from_if_match(&str)` constructor (the single decoder, tolerant of quotes and
  bare-integer If-Match). Every call site (`api/ehr.rs`, `ehr.rs`,
  `contribution.rs`) parses/formats through it. Deletes 4 of the 5 functions.
- [x] fixed *(2026-07-06 W2-B — all four hand-rolled decoders deleted
  (`api/ehr.rs::parse_object_id`/`parse_version_uid`/`expected_from_if_match`,
  `ehr.rs::parse_expected_version`, the inline `split("::")`/`nth(1)` in
  `contribution.rs::parse_preceding`). The one decoder is
  `app/ehrbase/src/service/version_id.rs`, built **on the BASE value type**
  (`openehr_base::prelude::ObjectVersionId::from_str` — the strict three-part
  lexical parse of `object_version_id_impl.rs`); the module adds only the
  storage typing (object_id must be the UUID `vo_id`; trunk `i32`) and the
  If-Match extraction. Divergences resolved to the BASE-spec-correct parse: a
  `::`-carrying id must now be a *valid* 3-part `OBJECT_VERSION_ID` (`uuid::sys`
  and `a::b::c::3` are rejected instead of mis-split), and well-formed branch
  ids (`2.1.4`) are rejected with an explicit trunk-only error (PORT NOTE,
  F-06-09). The encoder stays the single `EhrbaseService::object_version_id`.
  `stored_query.rs`'s `split_once("::")` is the *qualified query name* grammar
  (`reverse_domain::semantic_id`), not an OBJECT_VERSION_ID — untouched.)*

### F-13-02: Two `WebTemplate` caches and two resolution paths; REST FLAT glue inverts layering
- **Severity:** major
- **Code:**
  - service cache + resolver: `app/ehrbase/src/service/mod.rs:49` (`EhrbaseService.web_templates: WebTemplateCache`) + `app/ehrbase/src/service/template.rs:25` `web_template_for()`
  - REST cache + resolver: `app/ehrbase-rest/src/state.rs:27` (`AppState.web_templates`) + `app/ehrbase-rest/src/dispatch/flat.rs:61` `web_template_for()`
- **Problem:** There are **two independent `WebTemplateCache` instances** for
  the same templates. The FLAT glue in `dispatch/flat.rs` builds/caches its own
  `WebTemplate` by calling `state.backend().definition_template_adl1_4_get(...)`
  to pull the OPT **XML back out through the REST DEFINITION API**, then
  re-parses it (`opt14::from_xml` + `build_web_template`) — duplicating the
  exact build the service already does and populating a *second* cache that
  never shares with the service's. So a template can be cached-and-built twice,
  and a composition validated by the service against WebTemplate A while the
  FLAT round-trip uses WebTemplate B. It is also a layering inversion: the REST
  layer reaches into a *definition* backend method to service a *composition*
  request. Per `rest-axum.md` FLAT/EhrScape are meant to live in
  `ehrbase-compat` on the shared service layer, not in `ehrbase-rest` with a
  private cache.
- **Target design:** WebTemplate resolution is a single service concern. Expose
  one method on the `Backend` seam (e.g. `web_template(template_id) ->
  Arc<WebTemplate>`, or FLAT conversion methods `composition_to_flat` /
  `composition_from_flat` that run in the service). Delete `AppState.web_templates`
  and `dispatch/flat.rs::web_template_for`; the REST layer calls the service,
  which owns the one cache. When P17 stands up `ehrbase-compat`, the FLAT glue
  moves there and still calls the same service method.
- [x] fixed *(2026-07-06 W2-K — one service-owned cache. New
  `WebTemplateService` seam trait (`ehrbase-rest/src/backend.rs`:
  `web_template(template_id) -> Arc<WebTemplate>`, part of `Backend`),
  implemented by `EhrbaseService` as a delegation to the existing
  `service/template.rs::web_template_for` (the one moka cache, shared with
  composition validation; unknown template → 422 per `422_COMPOSITION.yaml`).
  Deleted: `AppState.web_templates` + accessor, and
  `dispatch/flat.rs::web_template_for` including the layering-inverted
  `definition_template_adl1_4_get` XML re-fetch + `opt14::from_xml` re-parse;
  `dispatch/definition.rs`'s `wt+json` branch also serves through the seam
  (the DEFINITION GET still runs first so an unknown template stays a 404 on
  that surface). FLAT/STRUCTURED and `wt+json` now use the same WebTemplate
  instance validation uses. FLAT HTTP e2e (`flat_http.rs`) mocks the seam and
  stays green; moving the FLAT glue into `ehrbase-compat` remains P17
  (F-13-12).)*

### F-13-03: Full dispatch arms hand-written for entirely-unimplemented API groups
- **Severity:** major
- **Code:** `app/ehrbase-rest/src/dispatch/demographic.rs` (43 match arms, ~393 LoC), `dispatch/query.rs` (6 arms, 106 LoC), `dispatch/admin.rs` (3 arms). `EhrbaseService` implements `DemographicApi`/`QueryApi`/`AdminApi` as **empty impls** (`app/ehrbase/src/service/api/mod.rs:20-22`), so every one of those operations returns `NotImplemented` (501).
- **Problem:** ~500 lines of per-operation boilerplate (build `*Params`, decode
  body, call backend, negotiate response) exist solely to forward to a backend
  method that unconditionally 501s. The `demographic` group has no
  implementation at all yet is fully spelled out. This is scaffolding cost with
  zero behavioural value and a maintenance tax (every generated-contract change
  touches these dead arms).
- **Target design:** Route unimplemented groups through a single generic 501
  responder driven by the `ROUTES` table, e.g. in `dispatch/mod.rs` mount groups
  with a `not_implemented` dispatcher that returns
  `ApiError::NotImplemented(op)` for any op. Only hand-write per-op arms for a
  group once it has real handlers. `demographic`, `admin`, and (until P16)
  `query` collapse to one line each in `api_router()`.
- [x] fixed *(2026-07-06 W3-B — `dispatch/demographic.rs` (393 LoC),
  `dispatch/query.rs` (106 LoC), and `dispatch/admin.rs` (47 LoC) deleted; the
  three groups mount on one generic `not_implemented` dispatcher in
  `dispatch/mod.rs` (~10 LoC), still driven by the generated `ROUTES` tables
  (routing/auth/admin-scope behaviour unchanged). The dead backend surface
  went with it: `Backend` is now `EhrService + DefinitionApi +
  WebTemplateService` — the empty `DemographicApi`/`QueryApi`/`AdminApi` impls
  on `EhrbaseService`, `StubBackend`, and the two test mocks are gone; each
  generated trait rejoins the seam in the phase that implements it (query at
  P16). Net ≈ −580 LoC. Wire behaviour verified by
  `http.rs::unimplemented_groups_answer_501_with_the_standard_error_body`
  (representative ops per group → `501` + the identical
  `{"error":"Not Implemented","message":"not implemented"}` body) plus the
  pre-existing per-group 501 smoke tests. One deliberate edge change: a
  malformed request to an unimplemented op (bad params/body) now answers `501`
  instead of a params-derived `400` — the operation is unimplemented
  regardless of payload.)*

### F-13-04: Dead dependency wiring hidden behind a `cargo-machete` ignore list
- **Severity:** major
- **Code:** `app/ehrbase/Cargo.toml:51` — `ignored = ["ehrbase-compat", "ehrbase-rest", "openehr-am", "openehr-base", "openehr-flat", "openehr-its", "openehr-lang", "openehr-query", "openehr-rm", "openehr-term"]`. Verified 0 references in `app/ehrbase/{src,tests}` for: `openehr_am`, `openehr_base`, `openehr_lang`, `openehr_query`, `openehr_term`, `ehrbase_compat`.
- **Problem:** Six path dependencies are declared and never used; `cargo-machete`
  would flag them, so an ignore list was added to silence it. That inverts the
  tool's purpose — it now hides real dead wiring instead of catching it.
  `ehrbase-compat` in particular is an empty crate (see F-13-12) wired into the
  binary for nothing. `openehr-query`/`sea-query` are pre-wired for P16 but have
  no consumer today.
- **Target design:** Remove the unused deps from `app/ehrbase/Cargo.toml` and
  the corresponding `ignored` entries; re-add each (with `dep.workspace = true`)
  in the phase that first consumes it (`openehr-query` at P16, `ehrbase-compat`
  at P17, etc.). Keep the ignore list only for genuinely
  wired-but-not-yet-referenced-in-source cases, documented per entry.
- [ ] fixed

### F-13-05: Service helpers scattered across domain modules instead of a shared module
- **Severity:** minor
- **Code:** `ensure_ehr_exists` defined in `service/composition.rs:154` but called from `item_tag.rs:62`, `directory.rs:17`, `contribution.rs:48`; `object_version_id`/`with_uid`/`audit`/`current_vo` + free fn `committer` defined in `service/ehr.rs:190-273`; `audit_details` defined in `service/contribution.rs:207` but called from `versioned.rs:56`; `change_type` constants in `vobject.rs:54` **duplicated** by `Action::change_type()` in `contribution.rs:27`.
- **Problem:** Cross-cutting helpers live wherever they were first needed, so
  `composition.rs` "owns" `ensure_ehr_exists` for the whole service and `ehr.rs`
  "owns" the audit/version-id builders. The two copies of the creation/
  modification/deleted code-strings (`vobject::change_type::*` vs
  `Action::change_type`) can drift.
- **Target design:** A `service/common.rs` (or extend `mod.rs`) holding the
  cross-object helpers: `ensure_ehr_exists`, `audit`/`committer`/`audit_details`,
  `with_uid`, and the version-id codec (F-13-01). Have `Action::change_type`
  return `vobject::change_type::*` constants rather than its own literals. Keeps
  domain modules (composition/directory/ehr) focused on their own logic.
- [ ] fixed

### F-13-06: Duplicated load→filter(ehr)→deleted-check→with_uid read pattern
- **Severity:** minor
- **Code:** repeated near-verbatim in `service/composition.rs:41-79` (`read_composition`, `composition_at_time`), `:167` (`ensure_composition_in_ehr`); `service/directory.rs:35-59` (`directory_at_time`), `:127` (`directory_at`); `service/ehr.rs:101-116` (`status_at`). Each does: `vobject::read_current/read_version/version_at → .filter(|r| r.ehr_id == ehr_id) → NotFound → if read.deleted { NotFound } → self.with_uid(...)`.
- **Problem:** The same 6-line ownership+liveness+uid dance is copy-pasted ~6
  times with only the "not found" label differing. Also `vobject`'s three read
  fns (`read_current`/`read_version`/`version_at`, `vobject.rs:469-552`) repeat
  an identical meta-fetch → `read_nodes` → build `VersionRead` body three times.
- **Target design:** A single `vobject::load(pool, selector, ehr_id) ->
  Result<VersionRead>` where `selector` is an enum `{Current, Version(i32),
  AtTime(Timestamp)}`, doing the meta query + `read_nodes` once, with an
  `owned_by(ehr_id)` + `live()` combinator on `VersionRead` returning the typed
  errors. Domain modules become `load(..).owned_by(ehr).live()?` one-liners.
- [ ] fixed

### F-13-07: `Backend` seam is `serde_json::Value`-typed with no response-header channel
- **Severity:** major
- **Code:** the five generated `*Api` traits return `Result<Value, ApiError>` (see every method in `app/ehrbase/src/service/api/ehr.rs`); dispatch then re-types the `Value` into an `openehr-rm` type for XML (`negotiate::respond_rm`, `negotiate.rs:293`, `serde_json::from_value(value.clone())`); requests go XML→`openehr-rm`→`Value` (`negotiate::rm_value`, `:214`). No method can return `ETag`/`Location` (cross-referenced by F-01-01).
- **Problem:** The whole service↔REST contract passes untyped `Value`, so (a)
  the wire edge does `bytes → typed → Value` inbound and `Value → typed → bytes`
  outbound — two extra full serde round-trips and a `value.clone()` per XML
  response purely to recover the type information the body already implies; and
  (b) there is no channel for response metadata (version id, location), which is
  why no handler emits `ETag`/`Location` (a critical conformance gap tracked in
  chapter 01). The service *computes* the `object_version_id` then throws it
  away into the body `uid` only.
- **Target design:** This is a contract-shape decision that predates the app
  layer (the generated traits are `Value`-typed by ADR-005). Two levers:
  (1) Introduce a small `Created { body: Value, version_id: ObjectVersionId,
  location: String }` / `Retrieved { body, etag }` return envelope for the
  write/read ops (or a side-channel `ResponseMeta` the dispatch reads), so
  headers can be set without changing payload typing — the minimal fix for
  F-01-01. (2) Longer-term, consider having the generated contract carry the
  concrete RM type per operation so the XML path drops the Value round-trip.
  At minimum, stop cloning in `respond_rm` by taking the `Value` by value.
- [ ] fixed

### F-13-08: Row→JSON mappers swallow DB errors via `try_get().unwrap_or_default()`
- **Severity:** minor
- **Code:** `service/item_tag.rs:111` `tag_json` (6× `unwrap_or_default`); `service/template.rs:126` `template_json`; `service/stored_query.rs:87` `stored_query_json`.
- **Problem:** These three row-mappers default silently on a `try_get` failure
  (missing/renamed column, type mismatch), so a schema/column drift produces
  wrong data (empty strings, `Uuid::nil`) instead of an error — the rest of the
  service correctly propagates with `?`. Inconsistent error discipline for the
  same "map a `PgRow` to canonical JSON" task done three times.
- **Target design:** One generic `fn row_json(row: &PgRow, spec) -> Result<Value,
  ServiceError>` or just make each mapper return `Result<Value, ServiceError>`
  and use `?` (they are already called from `Result`-returning async fns —
  `list_templates`/`list_stored_queries` even wrap them in a `.map(Ok)`). Removes
  the silent-default footgun and the pattern triplication.
- [ ] fixed

### F-13-09: Hand-rolled percent-decoding / form-urlencoded splitting
- **Severity:** minor
- **Code:** `app/ehrbase-rest/src/params.rs:92` `form_urlencoded_pairs`, `:112` `percent_decode`, `:132` `hex_val`.
- **Problem:** ~40 lines reimplement `application/x-www-form-urlencoded` parsing
  and percent-decoding. The comment says it avoids "a dependency purely for
  query splitting", but `url` 2 (which re-exports `form_urlencoded`) and
  `percent-encoding` are already in the workspace lockfile, and `rust-style.md`
  says not to hand-roll what a crate provides. The hand-rolled version is also
  UTF-8-lossy and only handles the shapes seen so far.
- **Target design:** Use `form_urlencoded::parse(query.as_bytes())` for
  `form_urlencoded_pairs`/`query_param`. Delete `percent_decode`/`hex_val`.
- [ ] fixed

### F-13-10: Stale doc comments describing unfinished state that is now finished
- **Severity:** minor (info)
- **Code:** `app/ehrbase/src/service/api/ehr.rs:1-6` ("Methods not yet wired (revision history, time-travel reads, item tags, ehr_get_by_subject, contribution_create) inherit the generated NotImplemented default") — all of these are in fact implemented in the same file. `app/ehrbase-rest/src/negotiate.rs:278` and `:288` reference "(P12)" / "once typed payloads land (P12)" for behaviour that has landed.
- **Problem:** Module docs assert an unimplemented state contradicted by the code
  beneath them — misleads the next reader and rots trust in the comments.
- **Target design:** Update the `api/ehr.rs` header to reflect the implemented
  surface; drop the "(P12)"/"future" qualifiers in `negotiate.rs` now that the
  paths exist. Cheap, do it in the same pass as any of the above.
- [ ] fixed

### F-13-11: Served OpenAPI document carries no paths
- **Severity:** minor (info)
- **Code:** `app/ehrbase-rest/src/openapi.rs` — `#[derive(OpenApi)]` with `info(...)` only; no `paths(...)`.
- **Problem:** Swagger UI serves a title-and-description-only document (no
  operations), so the UI is effectively empty. It is intentionally a seam for a
  future code→OAS drift-check (ADR-005), but as shipped it is dead weight that
  looks like a feature.
- **Target design:** Either serve the vendored authoritative OAS JSON directly
  through Swagger UI (it is the source of truth per ADR-005), or gate the UI off
  until the drift-check seam is real. Don't ship an empty generated doc.
- [ ] fixed

### F-13-12: `ehrbase-compat` is empty scaffolding; its intended contents live in `ehrbase-rest`
- **Severity:** minor
- **Code:** `app/ehrbase-compat/src/lib.rs` (6 lines, doc comment only); wired as a dep of `ehrbase` (F-13-04) and never mounted. The FLAT/STRUCTURED glue that `rest-axum.md`/architecture assign to `ehrbase-compat` currently lives in `app/ehrbase-rest/src/dispatch/flat.rs` (F-13-02).
- **Problem:** An empty crate is carried in the graph, and the code that should
  eventually populate it sits in the wrong crate, coupling FLAT-interop concerns
  into the core ITS-REST server. This is planned P17 work, but the misplacement
  compounds F-13-02.
- **Target design:** Leave `ehrbase-compat` empty and *unwired* until P17 (drop
  the dep per F-13-04). At P17, move `dispatch/flat.rs` (and EhrScape) into
  `ehrbase-compat` calling the shared service method from F-13-02.
- [ ] fixed

---

## `openehr-flat` (FLAT / STRUCTURED / WebTemplate / validation)

This crate is the single largest duplication cluster in the workspace. It runs
the whole SDT + validation pipeline on `serde_json::Value` and has grown several
parallel implementations of the same operations internally.

### F-13-20: Two AQL-path parsers inside `openehr-flat` (plus a third in `openehr-query`)
- **Severity:** major
- **Code:** `flat/aql.rs:26` `parse_path` / `:54` `parse_seg` (→ `AqlSeg { attr, node_id, name }`) **and** `validation/mod.rs:415` `parse_segments` / `:437` `parse_segment` (→ `Segment { attr, pred: Pred }`). A third, full-grammar parser lives in `openehr-query` (`parser.rs:99` `path_parsers`, → `ObjectPath`).
- **Problem:** `flat/aql.rs` and `validation/mod.rs` are byte-for-byte the same
  algorithm (bracket-depth split on `/`, split predicate on `,`, trim quotes),
  differing only in the output struct and in gratuitous edge details
  (`rfind(']')` vs `trim_end_matches(']')`; `'` only vs `'`+`"`). Two copies to
  keep in sync, already diverging.
- **Target design:** Promote `flat::aql` to a crate-internal `path` module with
  one `Seg`/`Pred` type; delete `validation::{parse_segments, parse_segment,
  Pred, Segment}` and route the validator through it. Longer-term (P16) share a
  single RM-path abstraction with `openehr-query` rather than three parsers.
- [x] fixed *(2026-07-06 W3-A — both flat parsers deleted. Parsing +
  predicate-matching + per-step navigation now route through the single
  canonical implementation in `openehr_rm::paths` (BASE master11-paths), reached
  via a new thin crate-local `openehr-flat/src/path.rs` (`parse` / `relative` /
  `navigate` / `resolve`). Deleted: `flat/aql.rs` (`AqlSeg`, `parse_path`,
  `parse_seg`, `matches_pred`, `resolve`, `relative`) and
  `validation/mod.rs`'s `Pred`, `Segment`, `parse_segments`, `parse_segment`,
  `get_attr`, `navigate_trailing`, `node_id`, `instance_name`. `openehr-rm`
  gained only spec-general surface: `Predicate::matches`/`is_empty` made public
  and a `select_children` per-step primitive (the step `items_at_path` already
  iterated). Semantic differences resolved to the spec — see F-13-21.)*

### F-13-21: Predicate-matching + RM-instance navigation duplicated alongside the parsers
- **Severity:** major
- **Code:** `flat/aql.rs:97` `matches_pred` / `:120` `resolve` vs `validation/mod.rs:396` `Pred::matches`, `:464` `get_attr`, `:474` `navigate_trailing`, `:486` `node_id`, `:492` `instance_name`.
- **Problem:** Both sides independently check `archetype_node_id` + `name/value`
  against a predicate and walk single-object/array RM attributes — `resolve` and
  `navigate_trailing`+`get_attr` are the same descent, maintained twice.
- **Target design:** Fold into the shared `path` module: one `matches(node,
  &Pred)`, one `resolve(rm, &[Seg]) -> Vec<&Value>`, one `instance_name`/`node_id`.
- [x] fixed *(2026-07-06 W3-A — predicate matching is now the single
  `openehr_rm::paths::Predicate::matches`; navigation is `select_children`
  (per-step) composed by `openehr-flat/src/path.rs::navigate`. Both flat copies
  gone. **Two SPEC-behaviour differences fell out and were resolved to the
  spec, both tightening the validator's previous leniency (a valid canonical
  composition is unaffected — the divergence only shows on non-canonical or
  malformed input, which is itself an RM violation):**
  1. **Predicate re-checked on single-valued attributes.** The old flat
     `resolve`/`navigate_trailing` descended a single-valued (object) attribute
     *without* re-testing the segment predicate; the RM step
     (`select_children`) re-tests it. BASE
     `master11-paths` §"Predicate Expressions" ("predicate expressions are
     often possible even on single-valued attributes, and can be used …") — a
     predicate constrains the matched node regardless of the attribute's
     cardinality, so re-checking is correct. Effect: a single-valued node whose
     `archetype_node_id` disagrees with the path predicate no longer resolves
     (was silently descended before).
  2. **`name/value` matches only the canonical `DV_TEXT` form.** The old
     `instance_name` also accepted a bare-string `name`; RM `Predicate::matches`
     matches `LOCATABLE.name.value` (a `DV_TEXT`) exactly, per BASE
     `master11-paths` §"Name-based Predicate" (Xpath `name/value='…'`).
     Canonical JSON always encodes `name` as a `DV_TEXT` object, so this drops a
     non-canonical leniency only.)*

### F-13-22: `CODE_PHRASE` / `DV_CODED_TEXT` / `DV_DATE_TIME` JSON builders triplicated + inlined
- **Severity:** major
- **Code:** `code_phrase()` defined identically in `flat/graph.rs:21` and `flat/context.rs:36`, plus a third variant `mappers.rs:312` `code_phrase_obj`; also hand-inlined as `json!({"_type":"CODE_PHRASE",...})` in `from_flat.rs:387-438` and `context.rs:38-42`. `dv_date_time` (`graph.rs:16`) is likewise re-inlined across `from_flat.rs` (L279…L412) and `context.rs` (L208, L211).
- **Problem:** `graph.rs` documents itself as "the single source of truth for
  those defaults", but its helpers are used only within `graph.rs`; everyone else
  re-inlines the same JSON shapes.
- **Target design:** One shared `builders` module (`code_phrase`, `dv_coded_text`,
  `dv_date_time`, `empty_item_tree`), called from `from_flat`, `context`,
  `mappers`. Delete the duplicate/inlined copies.
- [ ] fixed

### F-13-23: Three overlapping "fill RM-mandatory fields" passes in the reverse converter
- **Severity:** major
- **Code:** `from_flat.rs:258` `new_struct`, `:317` `finish_identity`, and `:93` `complete_tree` → `graph.rs:49` `fill_structural_mandatory`.
- **Problem:** The same compacted-away structural defaults (HISTORY `origin`/
  `events`, EVENT `time`, ITEM_TREE `items`, ENTRY-family blocks, ISM_TRANSITION)
  are filled across three order-dependent passes; `new_struct`/`finish_identity`
  never call the `graph` helpers that are documented as canonical. POINT_EVENT
  `time` is filled in all three; INTERVAL_EVENT `width`/`math_function` only in
  `graph.rs` — impossible to reason about which pass wins.
- **Target design:** Two clearly-scoped traversals — one for structural mandatory
  fields (`graph.rs`), one for locatable identity (name / archetype_details /
  ENTRY language-encoding-subject). Delete the field-filling bodies of
  `new_struct`/`finish_identity`; route through the graph helpers.
- [ ] fixed

### F-13-24: Four different hand-rolled recursive JSON tree walks
- **Severity:** major
- **Code:** `from_flat.rs:93` `complete_tree`, `context.rs:411` `walk_entry_defaults`, `validation/mod.rs:128` `rm_invariant_pass`, `validation/terminology.rs:56` `terminology_pass` (which also threads an unused `_parent_type` param).
- **Problem:** Each re-implements "recurse object fields (skipping `_`-prefixed)
  + array elements" with gratuitous differences (some skip `_`-keys, one recurses
  them, one uses a fixed whitelist).
- **Target design:** One generic RM-instance visitor (`visit(&Value, path, &mut
  FnMut(&Value,&str))` + a `_mut` sibling) reused by all four passes; drop the
  dead `_parent_type` argument.
- [ ] fixed

### F-13-25: `ehrbase-quirks` feature is declared but never gates anything — quirk is hard-coded on the default path
- **Severity:** major (rule violation)
- **Code:** `mappers.rs:93-94` emits `|unit_system`/`|unit_display_name` and `:334-335` reads them back **unconditionally**; no `#[cfg(feature = "ehrbase-quirks")]` exists anywhere despite the feature being declared in `Cargo.toml`.
- **Problem:** `serialization.md` explicitly says these Better-only extras must
  live behind the `ehrbase-quirks` flag and "never hard-code a quirk into the
  default path." As shipped, every FLAT output carries the quirk.
- **Target design:** Gate the two lines behind `#[cfg(feature = "ehrbase-quirks")]`,
  or drop the feature declaration until quirks are wanted.
- [x] fixed *(2026-07-06 — both the emit (`leaf_to_flat`) and read-back (`quantity_from_flat`)
  `|unit_system`/`|unit_display_name` paths are now `#[cfg(feature = "ehrbase-quirks")]`; the
  default FLAT path is quirk-free. `ehrbase-compat` (the EHRbase/Better compat surface) enables
  the feature so CI exercises it; core ITS-REST serving stays clean.*
  **Spec-backing decision:** `DV_QUANTITY.units_system`/`units_display_name` ARE genuine
  RM 1.2.0 fields (openehr-rm `dv_quantity.rs:59,65`; emitted in canonical JSON + XML per W2-I),
  but their FLAT `|unit_system`/`|unit_display_name` *suffix representation* is a Better vendor
  extra beyond the common EhrScape suffix set — no normative SDT concrete format exists
  (SM serial_data_formats unfinished; F-10-01/05), so serialization.md's classification of them
  as `ehrbase-quirks` extras stands. Gating (not default-on) is the correct fix; the RM fields
  keep their first-class canonical-JSON/XML home regardless. No test references these suffixes,
  so no snapshot changed.)*

### F-13-26: RM/terminology magic constants hand-inlined instead of sourced from one place
- **Severity:** major
- **Code:** category `433`/"event" (`from_flat.rs:430`), setting `238`/"other care" (`context.rs:28`), ISM `524`/"initial" (`from_flat.rs:418`, `graph.rs:85`), math_function `146`/"mean" (`graph.rs:70`), `rm_version` `"1.0.4"` (`from_flat.rs:377`,`:455`) — while `validation/tests.rs` uses `"1.0.2"` (inconsistent).
- **Problem:** These are exactly the codes `validation/terminology.rs` validates
  against; re-typed as JSON literals in many places, they can (and already do for
  `rm_version`) drift.
- **Target design:** A single `defaults` module (or `openehr-term` accessors) for
  RM-mandated code defaults; construct via `openehr-rm` types where practical.
- [ ] fixed

### F-13-27: `builder.rs` oversized (895 LoC) mixing concerns; generated-enum accessors hand-maintained
- **Severity:** minor
- **Code:** `webtemplate/builder.rs` — ontology/term collectors + node walk + per-RM post-processing + two-phase compactor + three 13-arm `CObject` matches (`:795` `object_rm_type`, `:813` `object_node_id`, `:840` `object_occurrences`).
- **Problem:** One 895-line module spans five responsibilities; the three
  `CObject` accessor matches are boilerplate over a **generated** `opt14` enum,
  hand-maintained here where they will drift from the emitter.
- **Target design:** Split into `builder/{walk,compactor,ontology}.rs`; emit the
  `CObject` accessor trait from `openehr-codegen` on the `opt14` types instead of
  matching by hand.
- [ ] fixed

### F-13-28: Small repeated helpers + dead variants in `openehr-flat`
- **Severity:** minor
- **Code:** `.split('<').next()` generic-strip repeated 7× (`from_flat.rs:443` `strip_generic`, `subtype.rs:140` `base`, inline in `to_flat.rs:38`, `mappers.rs:180`, `inputs.rs:41`, `leaf.rs:21`, `id.rs:79`); FLAT-key parsing duplicated (`sub.rs:27` `parse_key` vs `structured/entry.rs:51` `convert_key_segment`); dead `FlatError` variants `OptParse`/`Serialize`/`UnknownPath` (`error.rs`, 0 constructions); dead `WebTemplateInputType` variants `Duration`/`Quantity`/`Count`/`Proportion` (`model.rs:149-153`); stale `openehr-am`/`openehr-base` deps in `Cargo.toml` (machete-ignored, appear unused).
- **Target design:** One `base_rm_type(&str)->&str`; one FLAT-key tokenizer; delete
  the 3 unused `FlatError` variants (or wire `UnknownPath` where `from_flat`
  silently drops unroutable keys); remove/`PORT NOTE` the dead input-type variants;
  drop unused deps + their machete ignores.
- [ ] fixed

---

## `openehr-its` (hand-written runtimes; opt14 is generated)

Correction to the audit premise: the entire `src/opt14/` tree **is `// @generated`**
(by `emit-opt`), reuses the single `xml::runtime`, and must not be hand-edited —
so there is **no** hand-written opt14 boilerplate problem and no XML-runtime
duplication. The genuine hand-written surface (~730 LoC) is healthy. Only minor
items:

### F-13-40: Dead / speculative public items in `openehr-its` runtimes
- **Severity:** minor
- **Code:** `json.rs:28` `to_canonical_json_pretty` (0 uses workspace-wide — dead public API); `xml/runtime.rs:22` `Namespace::V2` (never constructed; the whole `ns` threading exists for a V2 that is never selected — documented-speculative per ADR-005 but dead today); `src/bmm.rs` (7-line placeholder exporting nothing, `pub mod bmm` in `lib.rs:24`).
- **Target design:** Remove `to_canonical_json_pretty` (or add a caller/`#[cfg(test)]`); collapse `Namespace` to a const until the v2 trait actually lands; delete `bmm.rs` (move its note into `lib.rs`) or mark it a documented stub.
- [ ] fixed

### F-13-41: `ApiError::into_response` renders plain text, contradicting `ValidationError`'s doc promise
- **Severity:** minor
- **Code:** `rest/runtime.rs:76` `IntoResponse` renders `self.to_string()` (plain text); the structured `{message, validationErrors[]}` body promised by the `ValidationError` doc (`:16`) is only produced by `ehrbase-rest/src/error.rs:58`.
- **Problem:** The runtime's own `IntoResponse` never emits the validation-errors
  array — a latent trap for any caller that renders `ApiError` directly rather
  than through `ehrbase-rest`'s override.
- **Target design:** Either fix the doc to say the structured body is the REST
  layer's job, or have the runtime `IntoResponse` build the same structured body.
  (Related: `json.rs:60` `validate_canonical` returns stringly `Vec<String>`
  while the REST layer has structured `ValidationError { path, message }` — have
  `validate_canonical` return `{instance_path, message}` records; callers already
  format `e.instance_path()`.)
- [ ] fixed

### F-13-42: Stale phase refs + missing `opt14` in `openehr-its` `lib.rs` docs
- **Severity:** minor (info)
- **Code:** `lib.rs:12` "(Implementation is P5.)", `:15` "is `ehrbase-rest` (P6)" — pre-renumber phase numbers (XML/REST are done; REST landed P11); the module list (`:7-18`) omits `opt14`, now a first-class public module (`:26`).
- **Target design:** Refresh the header; list `opt14`. ApiError variants are all
  used, `status()` covers every variant — no dead-variant issue there (good).
- [ ] fixed

---

## `openehr-query` (AQL lexer / parser / AST)

The AST is clean and does **not** re-model RM types (good). Issues:

### F-13-50: `VersionPredicate::Standard` AST variant is never constructed (half-built grammar arm)
- **Severity:** major
- **Code:** variant at `ast.rs:329`; the parser's `version_predicate` (`parser.rs:341-344`) only yields `Latest`/`All` — the grammar's `versionPredicate : … | standardPredicate` arm is unimplemented.
- **Target design:** Either wire the standard-predicate arm in `class_operand`
  (`parser.rs:345`), or drop the variant until P16 implements it. Don't ship an
  AST shape no input can produce.
- [x] fixed *(already resolved by W2-E, verified in W3-A: `parser.rs`
  `version_predicate` now yields `VersionPredicate::Standard` from the
  standard-predicate arm (F-08-02), and `NodePredicate::Standard` /
  `PathPredicate::Standard` are constructed for bare comparisons (F-08-10). No
  unconstructable variant remains.)*

### F-13-51: `path_parsers()` built twice, each call discarding half; `query()` oversized
- **Severity:** minor
- **Code:** `parser.rs:287` `let (identified, _predicate) = path_parsers();` and `:340` `let (_ip2, predicate2) = path_parsers();` — two full copies of the path grammar because each caller wants one half. `query()` (`:286-509`) is ~220 lines under `#[allow(clippy::too_many_lines)]`.
- **Target design:** Build `path_parsers()` once and share both halves. Extract
  `select_clause`/`contains_expr`/`where_expr`/`order_limit` sub-grammar builders
  (mirroring `path_parsers`), removing the lint allow.
- [x] fixed *(2026-07-06 W3-A — the duplicate `path_parsers()` build in
  `query()` is removed: it is now built once and its three halves
  (`identified` for the SELECT/terminal side; `predicate`/`standard` for the
  FROM `classExprOperand` / VERSION predicate) are shared. The
  `query()`-oversized / sub-grammar-extraction half is a larger refactor with
  W2-E-regression risk and is left as a P16 cleanup — the `#[allow(too_many_lines)]`
  stays.)*

### F-13-52: Silent lossy numeric parsing + stale doc + span-collapsing wrapper
- **Severity:** minor
- **Code:** `parser.rs:69-72`,`:314`,`:476` use `.parse().unwrap_or_default()` — an overflowing integer literal (lexer only checks `[0-9]+`) silently becomes `0`, likewise `TOP`/`LIMIT`/`OFFSET`. `parser.rs:11-13` doc claims `// TODO(port):` markers exist — none do. `parser.rs:40` `parse_str` collapses `lex`/`parse` spans into a joined `String`.
- **Target design:** Propagate a parse error (or saturate with a documented
  decision) instead of defaulting to zero; fix the stale doc; ensure the P16
  engine calls `parse` (span-rich), not `parse_str`.
- [x] fixed *(2026-07-06 — the lossy `.parse().unwrap_or_default()` numeric
  parses were already replaced by W2-E: integer/real primitives use
  `.ok().map(...)`/`.ok()?` (→ parse error, not `0`) and `TOP`/`LIMIT`/`OFFSET`
  use `.try_map(… .map_err(Simple::new))` (overflow = parse error). W3-A fixed
  the remaining stale doc claim (the module header no longer claims non-existent
  `// TODO(port):` markers; it states the overflow-is-an-error behaviour). The
  `parse_str` span-collapse is a convenience wrapper for tests/CLI; the P16
  engine using span-rich `parse` is P16 guidance, left as a note.)*

---

## `openehr-term` (terminology bundle + access)

The "inlined data" concern does **not** apply — data is asset-driven via
`include_str!` (`bundle.rs:38`), which is the target design already. The tiny
`terminology/*.rs` files are `// @generated` per-BMM-class and must **not** be
consolidated. Issues:

### F-13-60: Internal code sets unindexed — `is_valid_normal_status` does a linear scan
- **Severity:** minor
- **Code:** `bundle.rs:382` `is_valid_normal_status` uses `cs.codes.iter().any(...)` (O(n)), while `is_valid_code` (`:263`) and `is_valid_external_code` (`:414`) use precomputed `HashMap`/`HashSet` indexes; internal code sets got no index in `load()`.
- **Target design:** Build an `internal_codes: HashMap<String, HashSet<String>>`
  index in `load()` alongside `external_codes`; route `is_valid_normal_status`
  through it. Consistent + O(1).
- [ ] fixed

### F-13-61: `status` fields always `None` — `TerminologyStatus` carries no runtime data
- **Severity:** minor
- **Code:** `bundle.rs:548`,`:569` (and `Code` construction `:549`) hardcode `status: None`, never reading a status attribute — so `Code.status`, `CodeSet.status`, `TerminologyGroup.status`, `TerminologyConcept.status`, and the `TerminologyStatus` newtype are all vestigial at runtime.
- **Target design:** If the vendored XML has no status attributes, add a
  `// PORT NOTE:` recording that these are intentionally unpopulated; if it does,
  fix the parser to read them. Either way, document it.
- [ ] fixed

### F-13-62: Wide unused API surface + missing-attribute masking
- **Severity:** minor (info)
- **Code:** ~13 of ~20 `is_valid_*` wrappers (`bundle.rs:275-366`) and most navigation methods are unused outside the crate's own tests (only 7 consumed, by `openehr-flat/validation/terminology.rs`); `attr().unwrap_or_default()` throughout the parsers (`:529`,`:546`,`:553`,`:567`,`:602`…) turns a missing *required* attribute into an empty string rather than a `Malformed` error.
- **Target design:** Defer a prune pass until P15 validation settles which
  bindings it needs (the wrappers double as spec-citation docs, so keep for now);
  tighten the required-attribute reads to error on absence if asset integrity
  matters. `lib.rs:1` doc calls the crate "@generated module tree" but the
  substantive logic is the hand-written `bundle.rs` — minor doc fix.
- [ ] fixed
