# 12 — RM/BASE generated types + spec functions/invariants

Audit scope: the generated spec crates `openehr-base` + `openehr-rm`, their
hand-written `*_impl.rs` behaviour siblings, the `validate.rs` glue, and the
emitter (`crates/openehr-codegen/src/emit.rs`). Oracle: vendored RM 1.2.0 +
BASE 1.3.0 spec text (`docs/specs/openehr/{RM,BASE}/`) and the vendored BMM
(`crates/openehr-codegen/vendor/bmm/components/{RM,BASE}/json/`).

## Summary

**Structural fidelity is excellent.** A ~15-class spot-check
(DV_PROPORTION, DV_QUANTITY, DV_DATE, DV_ORDINAL, POINT_EVENT/INTERVAL_EVENT,
FOLDER, PARTY_RELATED, EVENT_CONTEXT, ISM_TRANSITION, INSTRUCTION_DETAILS,
ARCHETYPED, LINK, FEEDER_AUDIT, OBJECT_VERSION_ID, the UID/OBJECT_ID
hierarchies) matched the BMM/spec on fields, optionality, container kind,
flattening of inherited attributes, `_type`, recursion boxing, and the
PATHABLE-not-LOCATABLE distinction (EVENT_CONTEXT / ISM_TRANSITION /
INSTRUCTION_DETAILS correctly carry no `name`/`archetype_node_id`). Emitter
degradations are almost nil: **exactly one** `serde_json::Value` fallback
exists in the whole generated RM crate, and **zero** in generated BASE.

**The gap is behaviour, not structure.** The `*_impl.rs` layer implements only
`validate_invariants` (a curated subset of RM class *invariants*) plus a couple
of private helpers. **Almost no spec *functions* are implemented anywhere** in
either crate — a repo-wide grep for `item_at_path`, `path_exists`, `parent()`,
`magnitude`, `less_than`, `is_strictly_comparable_to`, `is_equal`, `PartialOrd`
returns nothing. This is partly deliberate (ADR-008 pushes DV_ORDERED magnitude
to a Postgres `openehr_magnitude` SQL function; terminology-bound invariants are
deferred to the P15 validator + `openehr-term`), but several of the missing
pieces are load-bearing for phases already in flight or imminent (REST version
headers, AQL path resolution + ordering, composition validation of event
series). Those are recorded below as findings.

Nothing here is a hand-edit of a `// @generated` file — all fixes land in
`*_impl.rs` siblings (new behaviour) or the emitter (structural).

Severity counts: **critical 0 · major 5 · minor 7 · info 3**.

## Findings

### F-12-01: RM/BASE spec *functions* are essentially unimplemented across both crates
- **Severity:** major
- **Spec:** RM 1.2.0 + BASE 1.3.0 — BMM `functions` blocks on ~40 classes
  (e.g. `DV_QUANTITY.{add,subtract,multiply,less_than,is_integral}`,
  `DV_QUANTIFIED.{magnitude,is_equal,less_than}`,
  `DV_DATE_TIME.{magnitude,diff,add,subtract}`, `DV_DURATION.magnitude`,
  `ITEM_TABLE.{row_count,column_count,element_at_cell_ij,…}`,
  `ITEM_LIST.{item_count,named_item,…}`, `ITEM_TREE.element_at_path`,
  `DATA_STRUCTURE.as_hierarchy`, `VERSIONED_OBJECT.*` (16 fns),
  `REVISION_HISTORY.most_recent_version`, `TERM_MAPPING.{narrower,broader,…}`,
  `DV_URI.{scheme,path,fragment_id,query}`, `EVENT.offset`,
  `INTERVAL_EVENT.interval_start_time`, `HISTORY.is_periodic`).
- **Code:** all `crates/openehr-{rm,base}/src/**/*_impl.rs` (each contains only
  `impl Validate` + tests; no spec-function `impl` blocks).
- **Problem:** ADR-003/ADR-004 place spec *behaviour* (functions + invariants)
  in `*_impl.rs`. Only invariants were written. The concrete-type accessors and
  computed properties the spec defines do not exist on the Rust types, so any
  consumer (the AQL engine, the FLAT/WebTemplate builders, the service layer)
  must re-derive them ad hoc. This is not a wire-fidelity defect today, but it
  is a systematic spec-coverage gap that F-12-02..04 make concrete for the hot
  paths.
- **Fix:** (`*_impl.rs` level) incrementally add the functions each downstream
  phase needs as `impl <Type>` blocks in the siblings — do **not** try to emit
  them (bodies are non-computable from BMM). Track the set per class; the
  highest-value ones are itemised in F-12-02/03/04. Where a function is
  intentionally realised elsewhere (magnitude → SQL, ADR-008), record a
  `// PORT NOTE:` on the type so the omission is deliberate, not silent.
- [ ] fixed *(partial, 2026-07-06 W2-L — landed: the identification accessor
  layer (F-12-03/07), DV_ORDERED magnitude/comparison/`is_simple`/`is_normal`
  (F-12-04), PATHABLE paths (F-12-02), `EVENT.offset_from` +
  `INTERVAL_EVENT.interval_start_time` + `HISTORY.is_periodic` (F-12-05),
  `DV_URI.{scheme,path,query,fragment_id}`, `TERM_MAPPING.{narrower,broader,
  equivalent,unknown,is_valid_match_code}`, `DV_PARSABLE.size`,
  `DV_PROPORTION.{magnitude,is_integral}`, `DV_QUANTITY.is_integral`. Still
  missing (no current consumer): ITEM_TABLE/ITEM_LIST accessor suites,
  `DATA_STRUCTURE.as_hierarchy`, `VERSIONED_OBJECT.*`,
  `REVISION_HISTORY.most_recent_version`, DV_AMOUNT
  `add/subtract/multiply/negative` arithmetic — add per consuming phase.)*

### F-12-02: PATHABLE path functions (`item_at_path`, `path_exists`, `parent`, …) implemented nowhere
- **Severity:** major
- **Spec:** RM 1.2.0 `common.archetyped.PATHABLE` — functions `parent`,
  `item_at_path`, `items_at_path`, `path_exists`, `path_unique`, `path_of_item`
  (`docs/specs/openehr/BASE/docs/architecture_overview/master11-paths.adoc`
  defines the openEHR path syntax these implement).
- **Code:** `crates/openehr-rm/src/common/archetyped/pathable.rs` (generated
  untagged enum `Pathable`, no functions, no `parent` back-reference, no
  `*_impl.rs` sibling).
- **Problem:** `PATHABLE` is emitted purely as a closed-subtype enum. None of
  its six pathing functions exist, and there is no `parent()` mechanism
  (CLAUDE.md mandates `Weak`/index, but nothing is present). openEHR paths are
  the addressing primitive for AQL identified-path leaf extraction (P16),
  REST partial-update/`ehr/.../composition/...?path=`, and the P15 validator's
  per-node path prefixing (which currently synthesises paths in the validator
  rather than via the RM). Missing = a required capability absent from the
  domain model.
- **Fix:** (`*_impl.rs` level) add a `pathable_impl.rs` implementing
  `item_at_path`/`path_exists`/`path_of_item` over the RM tree (the node codec
  in `ehrbase::storage` and the AQL planner will consume it). `parent()` is a
  back-reference — implement via a path-index/visitor, never an owning ref.
  Confirm the P16 planner's intended source of truth (RM function vs. the
  nested-set `node` table) before duplicating.
- [x] fixed *(2026-07-06 W2-L — `crates/openehr-rm/src/paths.rs`: `RmPath`
  parser (BASE master11-paths syntax: `atNNNN`/archetype-id predicates, the
  `,'name'` shortcut, explicit `name/value='…'` / `@archetype_node_id='…'`
  conjuncts; general comparison predicates rejected as
  `PathError::UnsupportedPredicate` — those are AQL, P16) + navigation over
  the canonical-JSON RM tree: `items_at_path` / `item_at_path` /
  `path_exists` / `path_unique` / `path_of_item` / `parent_of` (root-anchored
  parent lookup — no owning back-refs). Deliberately JSON-tree-based, not a
  typed-enum visitor: every consumer (node codec, P15 validator, FLAT) holds
  canonical JSON. **Consolidation plan:** `openehr-flat/src/flat/aql.rs`
  (`parse_path`) and the app-layer path parsers (F-13-20/21, W3-A) migrate
  onto this module — owned by those crates' waves, not W2-L.)*

### F-12-03: OBJECT_VERSION_ID / UID_BASED_ID / HIER_OBJECT_ID accessors + lexical-form handling missing
- **Severity:** major
- **Spec:** BASE 1.3.0
  `docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.base_types.object_version_id.adoc`
  (functions `object_id`, `creating_system_id`, `version_tree_id`, `is_branch`;
  lexical form `object_id '::' creating_system_id '::' version_tree_id`) and
  `…uid_based_id.adoc` (`root`, `extension`, `has_extension` + `Has_extension_valid`).
- **Code:** `crates/openehr-base/src/base_types/identification/object_version_id.rs`
  and `…/hier_object_id.rs` — both are bare `{ value: String }` with **no**
  `*_impl.rs` sibling; `uid_based_id.rs`/`object_id.rs` are enums with no
  accessors. Only `VERSION_TREE_ID` has a `*_impl.rs` (and it validates format
  but exposes none of its `trunk_version`/`is_branch`/`branch_*` functions).
- **Problem:** `OBJECT_VERSION_ID` is the identifier the ITS-REST contract puts
  in `ETag`/`Location` and version paths, and the versioning service must split
  it into (uid, system, version-tree) and detect branches. None of the parsing
  accessors or the `UID_BASED_ID` root/extension split exist, and there is no
  invariant checking the `::`-delimited lexical form (BMM lists no invariant for
  OBJECT_VERSION_ID, but a well-formedness check is required for round-trip and
  is exactly the kind of structural guarantee the ISO-8601 `Value_valid`
  invariants already stand in for elsewhere). Downstream (P12 service, REST
  headers) is re-parsing these strings by hand.
- **Fix:** (`*_impl.rs` level) add `object_version_id_impl.rs` +
  `uid_based_id_impl.rs`/`hier_object_id_impl.rs` implementing the accessor
  functions and a `Value_format_valid`-style lexical invariant (mirror the
  `version_tree_id_impl.rs` pattern), then wire OBJECT_VERSION_ID into the
  `validate_rm_value` dispatcher. Complete the VERSION_TREE_ID accessor
  functions while there.
- [x] fixed *(2026-07-06 W2-L — new `openehr-base` identification siblings:
  `lexical.rs` (shared `IdError` thiserror type + UID-subtype builder),
  `uid_based_id_impl.rs` (`root`/`extension`/`has_extension` on
  HIER_OBJECT_ID, OBJECT_VERSION_ID and the `UidBasedId` enum + `value()`),
  `object_version_id_impl.rs` (`object_id`/`creating_system_id`/
  `version_tree_id`/`is_branch`, strict three-part `FromStr`, and a
  `Value_format_valid` invariant wired into `validate_rm_value`), and
  `version_tree_id_impl.rs` extended with `trunk_version`/`is_branch`/
  `is_first`/`branch_number`/`branch_version` + `FromStr` (branch segments now
  require ≥ 1 per the spec's `Branch_*_valid`). **Migration targets (owned by
  the app-crate agents, F-13-01/W2-B):** the 5 hand-rolled `::`-splitters in
  `app/ehrbase`/`ehrbase-rest` should move to
  `openehr_base::…::ObjectVersionId::{from_str, object_id, creating_system_id,
  version_tree_id, is_branch}`.)*

### F-12-04: DV_ORDERED comparison / magnitude unimplemented → interval + reference-range ordering not enforced
- **Severity:** major
- **Spec:** RM 1.2.0 `DV_ORDERED.{less_than,is_strictly_comparable_to,is_simple,is_normal}`,
  `DV_QUANTIFIED.magnitude`, `DV_INTERVAL` invariant `Limits_consistent`
  (lower ≤ upper), `REFERENCE_RANGE.is_in_range`, and the ordered-magnitude
  semantics for DV_DATE/TIME/DATE_TIME/DURATION/ORDINAL/SCALE/PROPORTION/COUNT.
- **Code:** `crates/openehr-rm/src/data_types/quantity/dv_interval_impl.rs`
  (only boundary-flag invariants; `Limits_consistent` explicitly skipped),
  `reference_range_impl.rs` (`is_in_range` not implemented), and the absent
  comparison functions on every `DV_ORDERED` subtype.
- **Problem:** The `dv_interval_impl.rs` `// PORT NOTE:` correctly defers
  `lower ≤ upper` to the P16 `openehr_magnitude` SQL work, but the consequence
  is that **no RM-level ordering exists at all**: an inverted `DV_INTERVAL`
  (e.g. a reference range low > high) passes RM validation, `REFERENCE_RANGE.is_in_range`
  can't be evaluated in-process, and AQL `ORDER BY`/comparison over DV_ORDERED
  leaves has no Rust fallback. The base `Proper_interval` *does* implement
  `Limits_consistent` via `PartialOrd`, so the two interval families diverge in
  strictness. Acceptable as a deliberate ADR-008 split only if the SQL layer
  provably covers every path; today it is an enforcement hole.
- **Fix:** (`*_impl.rs` level) implement the openEHR ordered-magnitude
  comparison once (a `magnitude()`/`is_strictly_comparable_to()`/`less_than()`
  surface on the DV_ORDERED subtypes) so `DV_INTERVAL<T: DV_ORDERED>` can enforce
  `Limits_consistent` and `REFERENCE_RANGE.is_in_range` works in-process; keep
  the SQL `openehr_magnitude` for indexed query paths but do not leave RM
  validation blind. Confirm the accept-set against the AQL spec's DV_ORDERED
  ordering rules.
- [x] fixed *(2026-07-06 W2-L — `dv_ordered_impl.rs`: `magnitude()` per the
  spec on every DV_QUANTIFIED subtype (DV_DATE days since 0001-01-01,
  DV_TIME/DV_DATE_TIME seconds, DV_DURATION nominal seconds via
  `Iso8601_duration.to_seconds` with `Average_days_in_year` 365.24 /
  `Average_days_in_month` 30.42 per BASE `Time_definitions`),
  `is_strictly_comparable_to` + `less_than` per subtype (units-gated for
  DV_QUANTITY, kind-gated for DV_PROPORTION) and on the `DvOrdered` enum,
  `is_simple`/`is_normal`, and an `OrderedLimit` comparison trait.
  `dv_interval_impl.rs` now enforces `Limits_consistent` (incomparable or
  inverted limits are violations; undecidable — `Value` elements or malformed
  magnitudes — are left to the element's own `Value_valid`) and provides
  `has()`; `reference_range_impl.rs` gains `is_in_range()`; every DV_ORDERED
  subtype now also runs `Normal_range_and_status_consistency`. The P16
  `openehr_magnitude` SQL function stays the indexed-path realisation and
  must stay aligned with this module (PORT NOTE recorded on the module).)*

### F-12-05: EVENT / POINT_EVENT / INTERVAL_EVENT invariants missing and not in the validator dispatcher
- **Severity:** major
- **Spec:** RM 1.2.0 `EVENT` (`Offset_validity1`), `INTERVAL_EVENT`
  (`Math_function_validity`, `Interval_start_time_valid`), `HISTORY`
  (`Events_valid` ✓, `Period_consistency`).
- **Code:** no `point_event_impl.rs` / `interval_event_impl.rs`; the
  `validate_rm_value` dispatcher in `crates/openehr-rm/src/validate.rs:352-399`
  handles `HISTORY` but has no `POINT_EVENT`/`INTERVAL_EVENT`/`EVENT` arm.
- **Problem:** Events are ubiquitous OBSERVATION content. Their non-terminology
  invariants (interval-event start-time consistency, offset validity) are never
  checked, and the P15 composition validator can't reach them because they are
  not dispatched. `HISTORY.Period_consistency` is likewise unimplemented (archie
  marks `Periodic_validity` `ignored`, but `Period_consistency` is a distinct,
  non-ignored invariant per BMM). This is a real validation-coverage gap in the
  clinically hottest structure.
- **Fix:** (`*_impl.rs` level) add event `*_impl.rs` with the BMM invariants and
  add `POINT_EVENT`/`INTERVAL_EVENT` arms to `validate_rm_value` (they are
  generic; dispatch with `serde_json::Value` element type as HISTORY/DV_INTERVAL
  already do). Cross-check `Period_consistency` against the spec before deciding
  whether it, like `Periodic_validity`, is genuinely un-enforceable.
- [x] fixed *(2026-07-06 W2-L — new `event_impl.rs` (`Event::{time,data,
  offset_from}`; offset computed from an explicit origin, no parent back-ref),
  `point_event_impl.rs` / `interval_event_impl.rs` (`Validate` +
  `INTERVAL_EVENT.interval_start_time()` = `time - width`, preserving the
  value's own timezone suffix) with `POINT_EVENT`/`INTERVAL_EVENT` dispatcher
  arms. `HISTORY` gains `is_periodic()` and **does** enforce
  `Period_consistency` (event offsets must be whole multiples of `period`,
  1 µs tolerance; malformed times are the value's own `Value_valid` problem) —
  cross-checked: it is a distinct, non-ignored BMM invariant, and per ADR-008
  the spec wins over archie's omission. `EVENT.Offset_validity1` and
  `INTERVAL_EVENT.Interval_start_time_valid` hold by construction for the
  computed functions (PORT NOTEs recorded); `Math_function_validity` is
  terminology-bound → P15 validator + `openehr-term`, per the crate policy.)*

### F-12-06: Curated invariant set omits several non-terminology RM invariants
- **Severity:** minor
- **Spec:** RM 1.2.0 BMM invariants not covered by any `*_impl.rs` / not in the
  dispatcher: `LOCATABLE.{Links_valid, Archetyped_valid}` (only
  `Archetype_node_id_valid` is applied), `ITEM_LIST.Valid_structure`,
  `DV_PARSABLE.{Formalism_valid,Size_valid}`,
  `DV_ORDERED.{Other_reference_ranges_validity, Is_simple_validity, Normal_range_and_status_consistency}`,
  `ITEM_TAG.{Inv_key_valid,Inv_value_valid}`, `EHR.*`, `EHR_ACCESS.Scheme_valid`,
  `VERSIONED_COMPOSITION.Persistent_validity`, `ORIGINAL_VERSION.*`,
  `REVISION_HISTORY_ITEM.Audit_valid`.
- **Code:** `crates/openehr-rm/src/validate.rs` dispatcher table + the absent
  siblings.
- **Problem:** The implemented invariant set is a deliberate, composition-content
  subset (documented in `validate.rs` header). Several omissions are
  non-terminology and cheap to check (ITEM_LIST structural uniformity, DV_PARSABLE
  size/formalism, DV_ORDERED reference-range/normal-status consistency, ITEM_TAG
  key/value). LOCATABLE `Links_valid`/`Archetyped_valid` apply to every locatable.
  None are wire-breaking, but they widen the accept-set beyond the spec.
- **Fix:** (`*_impl.rs` level) add the cheap non-terminology invariants and their
  dispatcher arms; for `EHR`/version-family, confirm whether validation is meant
  to live at the service layer (P12) before adding here. Record any deliberately
  omitted (e.g. archie-`ignored`) invariants with a `// PORT NOTE:` + spec cite,
  as `composition_impl.rs` already does.
- [ ] fixed *(partial, 2026-07-06 W2-L — added: `DV_PARSABLE`
  (`Formalism_valid`; `Size_valid` ≥ 0 holds by construction for a byte
  length), `ITEM_TAG` (`Inv_key_valid` incl. the is_justified
  no-leading/trailing-whitespace rule, `Inv_value_valid`), and
  `DV_ORDERED.Normal_range_and_status_consistency` on all 9 subtypes
  (unlocked by F-12-04), each with dispatcher arms. Not added, with reasons:
  `ITEM_LIST.Valid_structure` — structural (`items: Vec<Element>` cannot hold
  a non-ELEMENT); `LOCATABLE.Links_valid` and
  `DV_ORDERED.Other_reference_ranges_validity` — `Vec` fields cannot
  distinguish Void from empty on our model, so `absent implies non-empty` is
  unexpressable; `LOCATABLE.Archetyped_valid` — `is_archetype_root` is
  defined by `archetype_details /= Void`, making the xor definitional;
  `DV_ORDERED.Is_simple_validity` — definitional for the computed
  `is_simple()`. `EHR.*` / `EHR_ACCESS` / `VERSIONED_COMPOSITION` /
  `ORIGINAL_VERSION.*` / `REVISION_HISTORY_ITEM` remain open pending the
  service-layer-vs-RM decision the Fix note calls for (version-family objects
  are constructed by the P12 service, not ingested as composition content).)*

### F-12-07: BASE/RM identifier & terminology accessor functions missing (ARCHETYPE_ID, TERMINOLOGY_ID, LOCATABLE_REF)
- **Severity:** minor
- **Spec:** BASE 1.3.0 `ARCHETYPE_ID.{qualified_rm_entity,domain_concept,rm_originator,rm_name,rm_entity,specialisation,version_id}`,
  `TERMINOLOGY_ID.{name,version_id}`, `LOCATABLE_REF.as_uri`.
- **Code:** `crates/openehr-base/src/base_types/identification/{archetype_id,terminology_id,locatable_ref}.rs`
  (bare structs, no `*_impl.rs`).
- **Problem:** These parse the structured identifier strings (archetype-id
  segments, terminology name/version, locatable-ref URI form). The AQL engine's
  archetype-id matching in CONTAINS predicates and template handling will need
  `ARCHETYPE_ID` decomposition; REST/`DV_EHR_URI` handling needs `LOCATABLE_REF.as_uri`.
  Currently absent, so consumers re-split strings.
- **Fix:** (`*_impl.rs` level) add accessor impls when P16/template work needs
  them; low priority until then, but flag so it is not silently re-implemented
  in the application layer.
- [x] fixed *(2026-07-06 W2-L — `archetype_id_impl.rs`
  (`qualified_rm_entity`/`domain_concept`/`rm_originator`/`rm_name`/
  `rm_entity`/`specialisation`/`version_id` + strict `FromStr`),
  `terminology_id_impl.rs` (`name`/`version_id`), `locatable_ref_impl.rs`
  (`as_uri`). The AQL CONTAINS matcher (P16) and template handling should
  consume these instead of re-splitting strings.)*

### F-12-08: `DV_ORDERED.normal_range` generic parameter is inconsistently monomorphised
- **Severity:** info
- **Spec:** RM 1.2.0 `DV_ORDERED.normal_range: DV_INTERVAL<DV_ORDERED>`.
- **Code:** `DvQuantity.normal_range: Option<Box<DvInterval<DvQuantity>>>`
  (`dv_quantity.rs:19`), `DvProportion.normal_range: …<DvProportion>`
  (`dv_proportion.rs:22`), but `DvDate`/`DvOrdinal.normal_range: …<DvOrdered>`
  (`dv_date.rs:74`, `dv_ordinal.rs:31`). Emitter: `emit.rs` generic bound-fill.
- **Problem:** The emitter fills the `DV_INTERVAL<DV_ORDERED>` type parameter
  with the *self* type for some subtypes and the *bound* (`DvOrdered`) for
  others. Not wire-breaking — the payload is the same and `DvOrdered` is a
  superset enum containing each variant — but it is an internal inconsistency
  that surprises consumers and makes `normal_range` handling type-branch by
  subtype. The spec declares one shape (`DV_INTERVAL<DV_ORDERED>`).
- **Fix:** (emitter level) make the bound-fill deterministic — emit
  `DvInterval<DvOrdered>` uniformly for the inherited DV_ORDERED `normal_range`
  (matches the spec declaration and the majority of the emitted types).
- [ ] fixed

### F-12-09: single `serde_json::Value` degradation — `X_VERSIONED_OBJECT` payload in ehr_extract
- **Severity:** minor
- **Spec:** RM 1.2.0 `ehr_extract.openehr_extract.OPENEHR_CONTENT_ITEM.item: X_VERSIONED_OBJECT<T>`.
- **Code:** `crates/openehr-rm/src/ehr_extract/openehr_extract/openehr_content_item.rs:40`
  — `item: Option<XVersionedObject<serde_json::Value>>`.
- **Problem:** The only `serde_json::Value` fallback in the entire generated RM
  crate (BASE has none). It is the ADR-004 monomorphization artifact for a
  version-family generic whose parameter can't be resolved. Confined to
  `ehr_extract`, which is experimental and out of the Stage-1 CNF conformance
  surface, so impact is low; note only that any node reaching this type loses
  typed (de)serialization and invariant dispatch.
- **Fix:** (emitter level, low priority) resolve the `X_VERSIONED_OBJECT`
  parameter or leave as-is with a `// PORT NOTE:`; acceptable while ehr_extract
  is out of scope. Confirm no in-scope canonical payload deserialises through it.
- [ ] fixed

### F-12-10: validator dispatches generic containers with `Value` element type, losing element-specific checks
- **Severity:** minor
- **Spec:** RM 1.2.0 — container invariants on `DV_INTERVAL`/`HISTORY` vs. their
  element types.
- **Code:** `crates/openehr-rm/src/validate.rs:368,372` —
  `run::<DvInterval<Value>>` and `run::<History<Value>>`.
- **Problem:** Documented in the `validate.rs` doc-comment as "enough for their
  own (non-child) invariants". True today because the element-typed invariants
  aren't implemented, but once F-12-04 (DV_INTERVAL `Limits_consistent` over
  DV_ORDERED) lands, dispatching with `Value` will silently skip it. This is a
  latent coupling between the dispatcher shortcut and the missing comparison.
- **Fix:** (`validate.rs` level) when F-12-04 lands, dispatch DV_INTERVAL through
  a DV_ORDERED-typed element (or run a `_type`-directed element pass) so the
  ordering invariant is actually reached. No action needed before then; recorded
  so the two findings are fixed together.
- [x] fixed *(2026-07-06 W2-L, together with F-12-04 — the `DV_INTERVAL`
  dispatcher arm now deserializes `DvInterval<DvOrdered>` first (reaching
  `Limits_consistent`) and falls back to `DvInterval<Value>` (boundary-flag
  invariants only) when the limits are not typed `DV_ORDERED` payloads.)*

### F-12-11: foundation-type BMM functions/invariants unimplemented (Iso8601_timezone, Time_Definitions, Statistical/Math/Env utilities)
- **Severity:** info
- **Spec:** BASE 1.3.0 `Iso8601_timezone` (functions + `Min_hour_valid`/`Sign_valid`
  invariants), `Time_Definitions.valid_iso8601_*`, and the `Statistical_evaluator`/
  `Math`/`Env`/`Locale`/`Quantity_converter` utility classes.
- **Code:** `crates/openehr-base/src/foundation_types/time/iso8601_*.rs` (bare
  `{ value: String }`, no impls); utility classes not emitted as files at all.
- **Problem:** The `Iso8601_*` foundation types are structurally emitted but
  unused on the wire (RM date/time carry `value: String`, validated by the
  hand-written ISO-8601 helpers in `rm/validate.rs`), so their missing functions
  have near-zero fidelity impact. The utility classes (`Math`, `Env`, statistical
  aggregation) are AQL/expression-language helpers, not RM data — legitimately
  out of scope for these crates.
- **Fix:** none required for conformance. If P16 needs `Time_Definitions`
  validity predicates or `Statistical_evaluator` for AQL aggregates, implement
  them in the engine/`*_impl.rs` at that point. Record as a known non-gap.
- [x] fixed *(2026-07-06 W2-L — recorded as a known non-gap per the Fix note.
  The `Time_definitions` constants the duration-magnitude rule needs
  (`Average_days_in_year` 365.24, `Average_days_in_month` 30.42,
  `Days_in_week`) now exist as spec-cited constants in
  `openehr-rm/src/data_types/quantity/dv_ordered_impl.rs`; the remaining
  foundation utilities stay unimplemented until a consumer appears.)*

### F-12-12: composite-identifier equality was case-sensitive (BASE R10)
- **Severity:** minor
- **Spec:** BASE 1.3.0
  `docs/specs/openehr/BASE/docs/base_types/master05-identification_package.adoc`
  §"Composite Identifiers and Case" (lines 164–177): all composite identifiers
  MUST be **case-preserving** *and* **case-insensitive** — "two identifiers
  identical apart from case are considered to be identical, and therefore to
  identify the same thing"; §"Composite Identifiers and Language" (lines
  179–183) restricts the human-readable sections to the basic latin character
  set, and the Case section carves out languages where case does not exist (the
  Turkish `I/i` caveat).
- **Code:** `crates/openehr-base/src/base_types/identification/uid_based_id_impl.rs`.
- **Problem:** the `UID_BASED_ID` family (`HIER_OBJECT_ID`, `OBJECT_VERSION_ID`,
  the `UidBasedId` enum) exposed only the derived byte-exact `PartialEq`; two
  `OBJECT_VERSION_ID`s differing only in UUID hex case (`…4E3D…` vs `…4e3d…`, or
  a case-flipped `creating_system_id`) compared unequal, violating R10. This was
  the only *unregistered* conformance gap surfaced by the BASE/TERM blueprint
  chapter (`docs/blueprint/02-base-term.md` item 1 / R10).
- **Fix:** added case-**insensitive** `is_equal(&self, other)` to
  `HierObjectId` / `ObjectVersionId` (via the `uid_based_id_accessors!` macro)
  and to the `UidBasedId` enum, using `str::eq_ignore_ascii_case` — the
  locale-safe fold the spec's basic-latin restriction + Turkish caveat call for.
  The stored `value` is untouched (case-*preserving* rule holds). The CDR's
  version/EHR lookups were already case-insensitive on the UUID `object_id`
  because the service/REST decoders parse it through `uuid::Uuid`
  (`app/ehrbase/src/service/version_id.rs`, `app/ehrbase-rest/src/version_id.rs`),
  which normalises hex case; a regression test in `version_id.rs` pins that a
  case-flipped-hex `OBJECT_VERSION_ID` resolves to the same `vo_id`.
- [x] fixed *(2026-07-09 B2 task 7 — `is_equal` added + spec-cited tests;
  storage-boundary lookups verified case-insensitive via the `uuid::Uuid`
  decode. Closes `docs/blueprint/02-base-term.md` §Remaining-work item 1.)*

## Hygiene notes

- **Emitter degradation surface is minimal and worth stating positively:** one
  `serde_json::Value` in generated RM, none in generated BASE, and version-family
  generics (`ORIGINAL_VERSION.data: Option<T>`, `IMPORTED_VERSION.item: OriginalVersion<T>`)
  stay properly generic rather than collapsing to `Value` — the ADR-004
  "monomorphized version-family carries `data: Value`" caveat did **not**
  materialise in the mainline change_control types.
- **Invariant message fidelity is good:** `invariant_failed` reproduces archie's
  `"Invariant <Name> failed on type <RM_TYPE>"` verbatim, and `*_impl.rs` files
  carry accurate `// PORT NOTE:` records for archie-`ignored` invariants and
  terminology-deferred ones. This makes the omissions auditable — the gap is
  breadth of coverage, not mislabeled behaviour.
- **Deliberate ADR-008 splits are documented but scattered:** the DV_ORDERED
  magnitude → SQL decision is noted in `dv_interval_impl.rs` but not on the
  DV_ORDERED subtypes themselves; a consumer reading `DvQuantity` sees no comparison
  and no note. Consider a single `// PORT NOTE:` anchor (e.g. on `DvOrdered`) so
  the "comparison lives in `openehr_magnitude`" decision is discoverable from the
  type.
- **`validate.rs` header is the de-facto coverage spec** for the invariant layer;
  keep the dispatcher table and that doc-comment in sync as F-12-05/06 add arms,
  and consider generating a coverage report (implemented vs. BMM invariant set)
  as a drift guard, since the curated subset is otherwise invisible.
- All fixes above are `*_impl.rs`/emitter/`validate.rs` changes — no
  `// @generated` file needs hand-editing.
