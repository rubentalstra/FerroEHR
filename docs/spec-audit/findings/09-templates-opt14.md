# 09 — Templates: OPT 1.4 / AOM 1.4

## Summary

Audited the OPT 1.4 template-ingestion subsystem against the vendored AOM 1.4
constraint XSD closure (`crates/openehr-its/schemas/xml/components/AM/Release-1.4/`
= `Template.xsd` → `OpenehrProfile.xsd` → `Archetype.xsd` → `Resource.xsd`), the
ITS-REST DEFINITION contract (`docs/specs/openehr/ITS-REST/specifications/`), and
the CNF `I_DEFINITION_ADL14` Robot suite
(`docs/specs/openehr/CNF/tests/platform/robot/I_DEFINITION_ADL14/`).

**Constraint-model fidelity is good.** Every AOM 1.4 / OpenehrProfile constraint
type in the XSD closure is present in the generated `opt14` model and dispatched
correctly by `xsi:type`: `C_COMPLEX_OBJECT`, `C_ARCHETYPE_ROOT`,
`C_SINGLE_ATTRIBUTE` / `C_MULTIPLE_ATTRIBUTE` (+ `CARDINALITY`),
`ARCHETYPE_SLOT` (`includes`/`excludes`), `CONSTRAINT_REF`,
`ARCHETYPE_INTERNAL_REF`, `C_PRIMITIVE_OBJECT` + all eight `C_PRIMITIVE` variants
(`C_BOOLEAN`/`C_STRING`/`C_INTEGER`/`C_REAL`/`C_DATE`/`C_DATE_TIME`/`C_TIME`/`C_DURATION`),
`C_DV_QUANTITY` (+ `C_QUANTITY_ITEM`), `C_CODE_PHRASE`, `C_CODE_REFERENCE`,
`C_DV_ORDINAL`, `C_DV_STATE` (+ `STATE_MACHINE`/`STATE`/`TRANSITION`), the
`ASSERTION`/`EXPR_*` tree, and the ontology/binding sets. Occurrences / existence
/ cardinality are modelled as the `IntervalOfInteger` shape with lenient defaults
for real-world exports that omit XSD-mandatory fields. All 91 vendored `.opt`
files parse (`opt14_corpus.rs`).

**The findings are concentrated in the REST ingestion behaviour and in two
lossiness / hygiene areas, not in the constraint model:**

1. **Duplicate-upload returns 201 (silent overwrite) instead of the
   CNF-mandated 409** — a direct, dedicated CNF test-case failure (critical).
2. `constraints` (`T_CONSTRAINT`) and `view` (`T_VIEW`) are dropped from the
   parsed model (skipped to `Value::Null`).
3. The 201 upload response omits the `Location` header and returns a JSON
   metadata descriptor rather than the OPT-XML representation / `Prefer`-driven
   body.
4. `opt14` duplicates the entire AOM 1.4 constraint model that `openehr-am`'s
   BMM-generated `am14` already carries — a maintainability hazard.

The stored artifact is the verbatim upload XML, and GET serves it verbatim
(`application/xml`, matching `200_Template_adl1_4_retrieved`), so the
model-level lossiness (F-09-02, F-09-05, F-09-06) does **not** corrupt the GET
round-trip; it only affects anything consuming the parsed `opt14` model or
re-serializing via the generated `ToXml`.

## Findings

### F-09-01: Duplicate template upload silently overwrites instead of returning 409 Conflict
- **Severity:** critical
- **Spec:** `ITS-REST/specifications/operations/definition_template_adl1.4_upload.yaml` (defines the `'409': 409_template_already_exists` response); `ITS-REST/specifications/responses/409_template_already_exists.yaml` ("409 Conflict is returned when a template with same template_id … already exists"); CNF `I_DEFINITION_ADL14/upload_opt/I_DEFINITION_ADL14.upload_opt-valid_opt_twice_conflict.robot` ("upload same OPT again" → "server rejected OPT with status code 409").
- **Code:** `crates/ehrbase/src/service/template.rs:73-88` (`store_template` uses `INSERT … ON CONFLICT (template_id) DO UPDATE SET … content = EXCLUDED.content`); `crates/ehrbase/src/service/api/definition.rs:23-34` (upload always yields the stored meta); `crates/ehrbase-rest/src/dispatch/definition.rs:60-70` (always renders `StatusCode::CREATED`).
- **Problem:** Re-uploading a template with an existing `template_id` succeeds with `201 Created` and **silently replaces the stored OPT content** (the `DO UPDATE` overwrites `content`, `concept`, `root_archetype`, and resets `created_at`). The ITS-REST contract and the CNF `upload_opt-valid_opt_twice_conflict` case both require `409 Conflict` (the ADL2 "twice without conflict" variant is explicitly tagged `future`/`NOT APPLICABLE FOR ADL 1.4`, so for ADL 1.4 the second upload must be rejected). This is a spec/CNF conformance failure and destroys the prior template version on collision.
- **Fix:** In `store_template`, detect the pre-existing `template_id` (either a plain `INSERT` that surfaces the unique-violation, or a `SELECT … FOR UPDATE` pre-check) and return a distinct `ServiceError::Conflict` mapped to `409` at the REST edge (add the variant if absent). Only insert when new; never `DO UPDATE` the content for adl1.4. The WebTemplate cache (`web_templates`) should be invalidated for that id when a legitimate replacement path is later added (admin), but on the adl1.4 endpoint the second upload must not mutate state.
- [x] fixed — `store_template` is now insert-only (`ON CONFLICT (template_id) DO
  NOTHING`; 0 affected rows → `ServiceError::Conflict` → 409); the stored OPT is
  never overwritten. Verified by `service_template.rs` (re-upload → 409, original
  XML untouched).

### F-09-02: `opt14` duplicates the full AOM 1.4 constraint model already generated as `am14`
- **Severity:** major
- **Spec:** AOM 1.4 constraint model (`docs/specs/openehr/AM/docs/ADL1.4/master05-cadl.adoc`); the same model is generated two ways — BMM → `crates/openehr-am/src/am14/aom14/**` and XSD → `crates/openehr-its/src/opt14/types.rs`.
- **Code:** `crates/openehr-am/src/am14/aom14/archetype/constraint_model/*.rs` (`CComplexObject`, `CObject`, `ArchetypeSlot`, `ArchetypeConstraint`, `CPrimitiveObject`, `CDefinedObject`, `ArchetypeInternalRef`, …) + `openehr_archetype_profile/{c_quantity,c_ordinal,c_coded_text}.rs` vs `crates/openehr-its/src/opt14/types.rs:159-372` (the same ~30 constraint types re-declared).
- **Problem:** There are now two parallel AOM 1.4 constraint models. `am14` is the BMM-generated, canonical-JSON (`#[derive(OpenEhrType)]`) differential model; `opt14` is an XSD-generated model carrying an XML codec plus the Ocean OPT-XML envelope types (`OPERATIONAL_TEMPLATE`, `C_ARCHETYPE_ROOT`, `T_COMPLEX_OBJECT`, `FLAT_ARCHETYPE_ONTOLOGY`, `C_CODE_REFERENCE`) that the AOM 1.4 BMM does not define. The envelope types justify a *scoped* adapter, but re-emitting the shared `C_*` constraint tree (rather than resolving to `am14` the way `emit-opt` already resolves RM leaves to `openehr_rm`/`openehr_base`) means the two models can silently drift, and any validation / path logic written against one will not match the other. The emitter comment (`emit_opt.rs:32-44`) documents only the *resource-metadata* divergence (`RESOURCE_DESCRIPTION.parent_resource` optionality), not the wholesale constraint-model duplication.
- **Fix:** Decide and document (an ADR / `// PORT NOTE:`) whether `opt14` is a deliberate throwaway legacy-OPT-XML adapter (acceptable, but say so and keep it minimal) or should resolve its shared `C_*` types to `openehr_am::am14` and generate only the OPT envelope + the ToXml/FromXml wire layer. If kept separate, add a cross-model consistency check so an AOM 1.4 spec bump regenerates both consistently. Minimum action: expand the `emit_opt.rs` header to state the constraint-model duplication and its rationale.
- [ ] fixed

### F-09-03: `T_CONSTRAINT` (`constraints`) and `T_VIEW` (`view`) are dropped from the parsed model
- **Severity:** minor
- **Spec:** `Template.xsd` — `OPERATIONAL_TEMPLATE.constraints` (`type="T_CONSTRAINT"`) and `.view` (`type="T_VIEW"`); `T_CONSTRAINT` carries `T_ATTRIBUTE` → `T_COMPLEX_OBJECT` with `default_value` (`DATA_VALUE`) and `differential_path`.
- **Code:** `crates/openehr-codegen/src/emit_opt.rs:59` (`OPAQUE_TYPES = ["T_CONSTRAINT", "T_VIEW"]`); `crates/openehr-its/src/opt14/types.rs:526-527` (`constraints: Option<serde_json::Value>`, `view: Option<serde_json::Value>`); `crates/openehr-its/src/xml/runtime.rs:474-481` (`FromXml for serde_json::Value` returns `Value::Null` after skipping the subtree); `impls.rs:4487-4492`.
- **Problem:** The template-level `constraints` overlay — which in the Ocean OPT format is where node **`default_value`s** and additional differential constraints (keyed by `differential_path`) live — and the `view` presentation block are parsed as `Null`; their subtrees are skipped. Any consumer of the parsed `OperationalTemplate` (WebTemplate builder, FLAT default-value population) never sees these defaults. GET is unaffected (verbatim XML is stored/served), and operational (flattened) templates usually fold constraints into `definition`, so impact is low — but `default_value`s carried only in `<constraints>` are lost to the model. The emitter comment calls this "skipped losslessly-enough"; it is not lossless.
- **Fix:** If WebTemplate/FLAT ever needs template default values, model `T_CONSTRAINT`/`T_ATTRIBUTE`/`T_COMPLEX_OBJECT` properly (the anonymous inline `T_VIEW.constraints` complexType is the only genuinely awkward part and can stay `Value`). Otherwise, keep the scope boundary but change the header note from "losslessly-enough" to an explicit `// PORT NOTE:` that `default_value` overlays are dropped, and confirm no default-value corpus case depends on it.
- [ ] fixed

### F-09-04: 201 upload response omits `Location` header and returns JSON metadata instead of the OPT representation
- **Severity:** minor
- **Spec:** `ITS-REST/specifications/responses/201_Template_adl1_4_upload.yaml` — "Server assigned `template_id` SHOULD be returned as part of the `Location` response header", "An `ETag` … MAY be present", body per `Prefer` is either empty or the full `application/xml` `OperationalTemplate` representation.
- **Code:** `crates/ehrbase-rest/src/dispatch/definition.rs:60-70` (`negotiate::respond(h, StatusCode::CREATED, &meta)`); `crates/ehrbase/src/service/api/definition.rs:33` (returns the JSON meta descriptor); no `Location` anywhere in `crates/ehrbase-rest/src`.
- **Problem:** The upload returns a JSON metadata object as the 201 body regardless of `Accept`/`Prefer`, sets no `Location` header (SHOULD), and does not honour `Prefer: return=representation`/`return=minimal` (the CNF conflict suite sets `Prefer=return=representation`). The spec's 201 body content-type is `application/xml` (the OPT) or empty. This is a wire-shape divergence, not a hard MUST, but it fails the representation/`Location` expectations the CNF harness is written around.
- **Fix:** Set `Location: {base}/definition/template/adl1.4/{template_id}` on 201; honour `Prefer` — empty body for `return=minimal`, the stored OPT XML (`application/xml`) for `return=representation`. Drop the JSON-metadata body from this endpoint.
- [ ] fixed

### F-09-05: `StringDictionaryItem` groups collapse to `BTreeMap`, losing element order and de-duplicating ids; no ToXml round-trip gate
- **Severity:** minor
- **Spec:** `Resource.xsd` `StringDictionaryItem` (`<x id="k">v</x>`, `maxOccurs="unbounded"`); used by `ARCHETYPE_TERM.items`, `ANNOTATION.items`, `RESOURCE_DESCRIPTION.original_author`/`other_details`, `RESOURCE_DESCRIPTION_ITEM.*`, `TRANSLATION_DETAILS.*`.
- **Code:** `crates/openehr-codegen/src/emit_opt.rs:46-50,258-272` (repeated `StringDictionaryItem` → `BTreeMap<String,String>`); `crates/openehr-its/src/opt14/types.rs:17,90,533,552,595`; corpus gate `crates/openehr-its/tests/opt14_corpus.rs` asserts parse-only.
- **Problem:** Modelling a repeated id/value element group as `BTreeMap<String,String>` (a) reorders entries alphabetically on any `ToXml` re-serialization (the XSD is an ordered `sequence`), and (b) silently drops a later entry if two items share an `id`. For OPT `other_details` the id set is typically unique, and GET serves verbatim XML so the *endpoint* is unaffected — but the generated `ToXml` impls (which exist for every `opt14` type) would not round-trip these groups faithfully, and nothing tests that: the corpus gate only checks `from_xml(...).is_ok()` plus a 2-file field spot-check.
- **Fix:** Prefer a `Vec<(String, String)>` (or an order-preserving multimap) for `StringDictionaryItem` groups to preserve order and duplicates; or, if the map is intentional, add a `// PORT NOTE:` recording the order/dup loss. Add a parse → `ToXml` → re-parse structural-equality gate over the corpus so model-level losslessness is actually asserted, not assumed.
- [ ] fixed

### F-09-06: `VALIDITY_KIND` and `OPERATOR_KIND` enumerations carried as raw `String` codes
- **Severity:** minor
- **Spec:** `Archetype.xsd` `VALIDITY_KIND` (integer enum: `1001` mandatory / `1002` optional / `1003` disallowed) used by `C_DATE/C_TIME/C_DATE_TIME.timezone_validity`; `OPERATOR_KIND` (integer enum `2001`…`2024`) used by `EXPR_OPERATOR.operator`.
- **Code:** `crates/openehr-codegen/src/emit_opt.rs:185-188` (a named `xs:simpleType` restriction resolves to `Resolved::Primitive("String")`); `crates/openehr-its/src/opt14/types.rs:203,212,368` (`timezone_validity: Option<String>`), `391,424` (`operator: String`).
- **Problem:** The integer enumeration codes are carried as their literal wire text (`"1001"`, `"2001"`), not decoded to a typed enum or a semantic value. This round-trips as text and does not lose data, but it defers all validity/operator semantics to the consumer and diverges from how these `*_KIND` enums are modelled elsewhere. Low impact for template ingestion (these fields are rarely read by the WebTemplate builder), but a fidelity gap versus the AOM enum semantics.
- **Fix:** Optionally emit typed enums (or `#[serde(transparent)]` newtypes) for named `xs:simpleType` integer restrictions with `enumeration` facets, mapping code→symbolic name from the XSD `id=` attributes. Otherwise record a `// PORT NOTE:` that `*_KIND` values are carried verbatim as wire codes.
- [ ] fixed

### F-09-07: Lenient `occurrences`/`existence` default of `0..1` can misrepresent mandatory nodes
- **Severity:** minor
- **Spec:** `Archetype.xsd` — `C_OBJECT.occurrences` and `C_ATTRIBUTE.existence` are XSD-mandatory (`type="IntervalOfInteger"`, no `minOccurs="0"`); AOM 1.4 multiplicity semantics (`AM/docs/ADL1.4/master05-cadl.adoc`).
- **Code:** `crates/openehr-codegen/src/emit_opt.rs:67-79` (`lenient_default` → `0..1` present/optional-single); applied at `212-299`.
- **Problem:** When a real-world OPT omits `occurrences`/`existence`, the reader fills `0..1` (optional, single). That keeps imperfect exports parsing (a legitimate goal for a wire adapter), but `0..1` is a *guess*: a node that should be mandatory (`1..1`) or multi-valued is silently made optional-single. If composition validation (P15) ever consumes these multiplicities from the `opt14` model, the guess could weaken constraints. Conformant OPTs always include these elements, so the default only fires on non-conformant input.
- **Fix:** Keep the leniency, but document (`// PORT NOTE:`) that a defaulted multiplicity is a fallback, and ensure validation resolves multiplicity from `definition` (or the archetype) rather than trusting a defaulted `0..1`. Consider logging when the default fires.
- [ ] fixed

### F-09-08: `TemplateMetadata.concept`/`archetype_id` emitted as JSON `null`; `version` never emitted
- **Severity:** minor
- **Spec:** `ITS-REST/specifications/schemas/definition/TemplateMetadata.yaml` — `template_id`, `concept`, `archetype_id`, `created_timestamp` are all `required` strings; `version` is optional.
- **Code:** `crates/ehrbase/src/service/template.rs:67-71` (`concept`/`root_archetype` become `None` when empty), `126-137` (`template_json` emits `concept`/`archetype_id` as `null` when absent).
- **Problem:** If an OPT has an empty `concept` or root archetype id, the list/metadata JSON emits `null` for a field the schema marks as a required string — a schema violation. Also `version` is never populated (acceptable, it is optional). Real conformant OPTs always carry a non-empty concept and root archetype, so this only bites on degenerate input, but the `.then_some(...)` → `null` path is a latent contract break.
- **Fix:** Emit `""` (or reject the OPT at upload as invalid content, `400`) rather than `null` for the required `concept`/`archetype_id`; an OPT with no concept/root archetype is arguably invalid template content per `400_invalid_template_content`.
- [ ] fixed

## Hygiene notes

- **Corpus gate is parse-only.** `opt14_corpus.rs::every_opt_template_parses`
  asserts `from_xml(...).is_ok()`; `key_fields_populated` spot-checks two files.
  There is **no** parse → `ToXml` → re-parse (or C14N) round-trip gate, so the
  generated `ToXml` impls and the model-level losslessness (F-09-03, F-09-05) are
  untested. Given that OPT storage is verbatim-XML this is low-risk today, but the
  `ToXml` code path is entirely unexercised.

- **`opt14` uses plain `#[derive(serde::Serialize/Deserialize)]`, not
  `#[derive(OpenEhrType)]`.** Its enums (`CObject`, `CAttribute`, `CDomainType`,
  `CPrimitive`, `State`, `ExprItem`, `ArchetypeConstraint`, `AuthoredResource`)
  are therefore externally-tagged Rust JSON (`{"CComplexObject": {…}}`) with no
  `_type` self-tag — **not** canonical openEHR AOM JSON. This is harmless while
  `opt14` is only ever used via the XML codec (WebTemplate consumes the typed
  model directly; GET serves verbatim XML), but the derived JSON is effectively
  dead and would mislead anyone who serializes `opt14` to JSON expecting the
  canonical shape. Consider dropping the serde-JSON derives or noting they are
  non-canonical.

- **`C_DEFINED_OBJECT` is emitted as a concrete enum variant** (`CObject::CDefinedObject`,
  `ArchetypeConstraint::CDefinedObject`) because `Archetype.xsd` does not mark
  `C_DEFINED_OBJECT` `abstract="true"` even though AOM 1.4 defines it abstract.
  Harmless (it will never appear as an `xsi:type` on the wire) but a spec-vs-XSD
  nit; the XSD, being the codegen input, wins here.

- **ADL2/OPT2 upload correctly 501s** (`dispatch/definition.rs:88-100`,
  `service/api/definition.rs:1-5`) — appropriate, since ADL2 is OPTIONAL and
  untested by the current CNF kit. The `delete_opt`/`validate_opt` CNF suites are
  admin-scoped (not in the stable ITS-REST adl1.4 trait), so their absence here is
  correct for Stage 1.

- **Namespace handling is robust.** The XSD closure targets
  `http://schemas.openehr.org/v2` while real OPT exports (and the ITS-REST
  examples) use `http://schemas.openehr.org/v1` and sometimes `ns2:` prefixes;
  the runtime strips element-name prefixes (`runtime.rs:389-393`) and the
  `xsi:type` value prefix (`xsi_type`, `runtime.rs:280-289`), so all 91 corpus
  files (including the `ns2:`-prefixed `non_unique_aql_paths.opt`) parse. Good.
