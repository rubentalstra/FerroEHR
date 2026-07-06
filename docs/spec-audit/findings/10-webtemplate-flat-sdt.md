# 10 — WebTemplate / FLAT / STRUCTURED (SDT)

## Summary

`crates/openehr-flat` implements three simplified serialisation surfaces —
the **WebTemplate** builder (OPT 1.4 → Better `web-template` 2.3 JSON), the
**FLAT** (simSDT) `RM ⇄ flat-map` converter, and the **STRUCTURED** (structSDT)
`RM ⇄ nested-tree` converter — served from `crates/ehrbase-rest/src/dispatch/`
(`flat.rs` glue, `definition.rs` for the template GET; wired into COMPOSITION
create/get/update in `ehr.rs`). `crates/ehrbase-compat` is an empty stub; all
serving is in `ehrbase-rest`.

The implementation is careful, well-annotated Better/EHRbase-parity work. The
central spec question is *which authority governs it*:

1. **The SM "Serial Data Formats" and "Simplified IM (SIM-B)" specs are the
   normative openEHR authority for SDT, and they are almost entirely
   unfinished.** `SM/docs/serial_data_formats/master05` is literally `xxxx`;
   the concrete-format sections and the string-parser grammar are `TBD`. The
   parts that *are* normative — the DATA_VALUE syntax table
   (`serial_data_formats/master03-data_values.adoc`) and the SIM-B
   transformation rules (`simplified_im_b/master07-transformation_rules.adoc`)
   — define a **primary** string syntax (`DV_QUANTITY` = `"78.500,kg"`,
   `DV_ORDINAL` = `"1|[snomed_ct::…]"`, ODIN intervals `"|0 .. 5|"`,
   `Terminology_code` = `"[icd10AM::F60.1]"`) with the `|suffix` object forms
   listed only as *"EhrScape Variants"*.

2. **The vendored ITS-REST 1.0.3 contract we generate from DOES normatively
   define the WebTemplate JSON** (`schemas/web_template/WebTemplate.yaml` +
   `Tree/Child/Input/Validation/Range/TermBindings/…`) and mandates it as the
   `Accept: application/openehr.wt+json` alternative on
   `definition_template_adl1.4_get`. So the WebTemplate is *not* spec-silent —
   there is a schema to conform to.

3. **The CNF acceptance oracle uses the EhrScape/Better form, not the SM
   primary form.** The CNF platform test *schedule* does not test
   FLAT/STRUCTURED/WebTemplate at all (only canonical JSON/XML), but the CNF
   Robot *fixture* data (`CNF/tests/platform/robot/_resources/test_data_sets/
   flat_compositions/` and `.../compositions/FLAT/`) is written entirely in the
   `|magnitude`/`|unit`/`|code`/`ctx/…`/`:0` EhrScape convention that the
   implementation targets.

Net: implementing Better `web-template` semantics is the pragmatic, CNF-fixture-
aligned choice and is defensible. The findings below (a) reconcile the code
against the *one normative schema that exists* (ITS-REST `WebTemplate.yaml`),
(b) catalogue each place the code follows Better where the SM spec says
something else or is silent, and (c) flag correctness issues (fabricated
invalid terminology codes, a stale hardcoded `rm_version`, round-trip losses).
No critical findings; 1 major (fabricated invalid codes), the rest minor/info.

## What the vendored spec normatively defines

- **`SM/docs/serial_data_formats/master03-data_values.adoc`** — the only
  finished normative concrete-format text. Primitive JSON types; openEHR
  primitives-as-strings (`Iso8601_*`, `Uri`, `Terminology_code`
  `"[term::code]"`, `Terminology_term` `"[term::code|text|]"`); ODIN interval
  strings (`|N .. M|`, `|>= N|`, `|N ±M|`); and the DATA_VALUE forms:
  `DV_QUANTITY` = `"<value>,<unit>"`, `DV_ORDINAL`/`DV_SCALE` =
  `"<num>|<term_code|term>"`, `DV_PROPORTION` = `"<num>/<den>;<KIND>"`,
  `DV_MULTIMEDIA`/`DV_IDENTIFIER` = standard JSON objects. `|`-suffixed object
  forms (`|code`/`|value`/`|terminology`, `|id`, `|mediaType`, `|formalism`)
  are explicitly the **"EhrScape Variants"** — an alternative, not the primary.
- **`SM/docs/simplified_im_b/master07-transformation_rules.adoc`** — the RM→SIM
  transformation table (collapse/stringify rules): `OBSERVATION.data.events` →
  `S_OBSERVATION.data`; `EVENT.data.items` → `S_EVENT.data`; `ITEM_TREE.items`
  → `S_CLUSTER`; `DV_CODED_TEXT.defining_code.code_string` → `code`,
  `.terminology_id` → `terminology`; `DV_TEXT.formatting`/`.language`/
  `.encoding` → **skip**; `PARTY_PROXY.external_ref.id.value` → `id`, `.scheme`
  → `id_scheme`, `.namespace` → `id_namespace`. This is the compaction the
  WebTemplate/FLAT layer must realise.
- **`SM/docs/simplified_im_b/master06-app_context.adoc`** — an "application
  context model" of defaults exists as a *concept*, but the class defs are
  UML-include stubs; the concrete `ctx/…` key vocabulary is not spelled out.
- **`ITS-REST/.../simplified_data_template/master03-conceptual.adoc`** —
  conceptual (sOPT/SIM pipeline); `master04` = the sOPT visitor pseudocode;
  `master05-jdt_concrete_formats.adoc` = `NOTE: under development; currently
  just notes` + `xxxx`; `master06-instance_conversion.adoc` = empty.
- **`ITS-REST/specifications/schemas/web_template/*.yaml`** — a *normative*
  (if minimal) JSON schema for the WebTemplate resource, referenced by
  `responses/200_Template_adl1_4_retrieved.yaml`. `Tree` required:
  `id, name, localizedName, rmType, nodeId, min, max, localizedNames,
  localizedDescriptions, aqlPath, children`. `Child` required: `id, rmType,
  min, max, aqlPath`. `Input`: `{type, suffix, list, defaultValue,
  validation}` (no `listOpen`, no `terminology`). `List`: required
  `{value, label}`. `Range`: required `{minOp, min}`.
- **CNF**: platform test schedule (`CNF/docs/platform_test_schedule/`) does
  **not** exercise SDT — no FLAT/STRUCTURED/WebTemplate test category. Robot
  fixtures in the EhrScape form exist but are not scheduled conformance cases.
  → SDT is not CNF-conformance-gated (consistent with `PORT_MASTER_PLAN §7.4`).

## Findings

### F-10-01: FLAT/STRUCTURED concrete formats have no finished normative spec — Better is the de-facto authority (documented deviation)
- **Severity:** info
- **Spec:** SPEC-SILENT (Better convention). `SM/docs/serial_data_formats/master05` (`xxxx`), `master04-syntax.adoc` String Parser (`TBD: define`); `ITS-REST/.../simplified_data_template/master05-jdt_concrete_formats.adoc` (`under development; currently just notes`), `master06` (empty).
- **Code:** `crates/openehr-flat/src/lib.rs:1`; `flat/mod.rs`, `structured/mod.rs` (whole modules).
- **Problem:** The concrete FLAT/STRUCTURED JSON serialisation the server accepts/emits is defined nowhere in finished openEHR normative text. The implementation targets Better `web-template` `converter/*` semantics. This is the correct pragmatic choice (CNF Robot fixtures + ITS-REST examples use the same form), but it must be recorded as a deliberate reliance on a vendor convention over an unfinished spec, per the project's spec-authority rule.
- **Fix:** No code change. Keep the `//!`/`PORT NOTE` docs stating Better is the interop oracle *because* SM SDT is unfinished; re-evaluate on any SM-SDT spec release (add a `docs/VERSIONS.md` watch note for `SM` serial_data_formats/simplified_im_b reaching STABLE).
- [x] fixed *(2026-07-06 — `flat/mod.rs` module doc now records the Better-oracle
  reliance with the exact SM/ITS-REST "unfinished" citations + a re-evaluate-on-STABLE note.)*

### F-10-02: `definition.rs` comment misstates the spec — WebTemplate on the adl1.4 GET is spec-defined, not an EHRbase extension
- **Severity:** minor
- **Spec:** `ITS-REST/specifications/operations/definition_template_adl1.4_get.yaml:6` (`Accept: application/openehr.wt+json` explicitly supported); `responses/200_Template_adl1_4_retrieved.yaml:62-64` → `schemas/web_template/WebTemplate.yaml`; `parameters/header/Accept_template.yaml:12`.
- **Code:** `crates/ehrbase-rest/src/dispatch/definition.rs:188-190` ("Serving `wt+json` on the spec `adl1.4/{id}` GET endpoint is a deliberate EHRbase-compatible extension (openEHR ITS-REST returns only the OPT itself).").
- **Problem:** The behaviour is correct, but the code's own spec citation is wrong: ITS-REST 1.0.3 *does* normatively define `wt+json` on this endpoint returning the WebTemplate schema. The inaccurate note undersells conformance and could mislead a future reviewer into treating a spec-required response as optional.
- **Fix:** Correct the comment to cite `definition_template_adl1.4_get.yaml` + `WebTemplate.yaml`; this is spec-conformant behaviour, not an extension.
- [ ] fixed

### F-10-03: WebTemplate output omits fields the ITS-REST `Tree` schema marks required
- **Severity:** minor
- **Spec:** `ITS-REST/specifications/schemas/web_template/Tree.yaml` (`required: id, name, localizedName, rmType, nodeId, min, max, localizedNames, localizedDescriptions, aqlPath, children`).
- **Code:** `crates/openehr-flat/src/webtemplate/model.rs:41-94` — `name`, `localizedName`, `nodeId`, `min` are `skip_serializing_if = "Option::is_none"`; `localizedNames`, `localizedDescriptions` are `skip_serializing_if = "IndexMap::is_empty"`; `children` is `skip_serializing_if = "Vec::is_empty"`.
- **Problem:** For the root `tree` node (and any node) with an empty rubric map, an unbounded-lower occurrence (`min = None`), or an empty node id, the emitted JSON omits members the ITS-REST `Tree` schema lists as `required`. A strict JSON-Schema validator (e.g. the drift/fidelity gate) would reject our WebTemplate. Better's own output omits them too and the schema is illustrative, but against the *vendored* schema this is a conformance gap.
- **Fix:** Either (a) accept it and note that the ITS-REST WebTemplate schema's `required` list is looser than real Better output (record as a documented deviation), or (b) for `Tree`/`Child` emit empty `localizedNames`/`localizedDescriptions: {}`, a present `nodeId: ""`, and `min: 0` when unbounded, to satisfy the schema. Prefer (b) for the root node at minimum.
- [x] fixed *(2026-07-06 — chose (b) for the root `Tree` node: new `model::serialize_root`
  (`#[serde(serialize_with)]` on `WebTemplate.tree`) fills every `Tree.required` member a
  sparse root would omit — `localizedNames`/`localizedDescriptions` ← `{}`, `nodeId` ← `""`,
  `min` ← `0`, `name`/`localizedName` ← the node id, `children` ← `[]` — without touching a
  well-formed root's output or the looser `Child` shape nested nodes serialize against. Two
  golden snapshots (Demo_Vitals, Diagnosis) gained the two spec-required empty rubric maps
  on their otherwise-complete root; medication_list unchanged.)*

### F-10-04: WebTemplate emits Better-2.3 fields beyond the ITS-REST schema (additive)
- **Severity:** info
- **Spec:** `ITS-REST/specifications/schemas/web_template/Input.yaml` (properties: `type, suffix, list, defaultValue, validation` — no `listOpen`, no `terminology`), `Child.yaml`/`Child1.yaml` (no `cardinalities`).
- **Code:** `model.rs:64-82` (`cardinalities`, `inContext`, `proportionTypes`, `termBindings`, `dependsOn` on nodes), `model.rs:156-213` (`listOpen`, `terminology` on `WebTemplateInput`; `ordinal`/`scale`/`localizedLabels`/`termBindings` on `WebTemplateCodedValue`), `builder.rs:204-215` (`semVer`, `otherDetails`, `version:"2.3"`).
- **Problem:** The impl follows the fuller real Better `web-template` 2.3 model, which carries fields the minimal ITS-REST schema does not list. The ITS-REST schemas set no `additionalProperties: false`, so extras are schema-legal, and they carry information consumers need (cardinality ids, dependsOn, term bindings). Recorded for completeness.
- **Fix:** None required. Keep the `model.rs` doc noting the ITS-REST schema is a subset of the Better 2.3 shape the impl emits.
- [x] fixed *(2026-07-06 — `model.rs` module doc now states the ITS-REST schema is a subset
  of the Better 2.3 shape, lists the additive fields, and notes the schemas set no
  `additionalProperties: false` so the extras are schema-legal.)*

### F-10-05: SM "primary" DATA_VALUE string syntaxes are unsupported; only the EhrScape `|suffix` variants are implemented
- **Severity:** info
- **Spec:** `SM/docs/serial_data_formats/master03-data_values.adoc` — primary forms `DV_QUANTITY "78.500,kg"`, `DV_ORDINAL/SCALE "1|[snomed_ct::…]"`, `DV_PROPORTION "25.3/100;PERCENT"`, ODIN intervals `"|0 .. 5|"`, `Terminology_code "[icd10AM::F60.1]"`, `Terminology_term "[…|text|]"`; the `|`-object forms are the *EhrScape Variants*.
- **Code:** `crates/openehr-flat/src/flat/mappers.rs:54-157` (`leaf_to_flat`) + `:179-443` (`leaf_from_flat`) implement only the `|magnitude`/`|unit`/`|code`/`|value`/`|terminology`/`|numerator`/… EhrScape forms; there is no parser for the `"value,unit"`, `"n|[term::code]"`, `"n/d;KIND"`, ODIN-interval, or `"[term::code]"` string encodings.
- **Problem:** Against the one finished normative concrete-format table, the implemented forms are the *secondary* ("EhrScape") variants and the *primary* forms are absent. Impact is low: CNF Robot fixtures and the ITS-REST conceptual examples all use the EhrScape form, and no SM string-parser grammar exists yet (`master04-syntax.adoc` String Parser = `TBD`). But an input using the SM-primary syntax (`"78.500,kg"`) would be silently misparsed as a bare DV_TEXT-ish value.
- **Fix:** Document the accepted-form envelope as "EhrScape/Better `|suffix` variant only". Revisit if SM `serial_data_formats` publishes the string parser; do not add the primary forms speculatively (no grammar to conform to yet).
- [x] fixed *(2026-07-06 — `flat/mod.rs` doc states we implement only the EhrScape/Better
  `|suffix` envelope, cites the SM `master03` "EhrScape Variants" classification + the missing
  string parser, and defers the SM-primary forms until a grammar exists.)*

### F-10-06: `|formatting` is emitted/consumed for DV_TEXT, but the SM transformation rule says skip `formatting`
- **Severity:** minor
- **Spec:** `SM/docs/simplified_im_b/master07-transformation_rules.adoc` (RM Data types table): `DV_TEXT._formatting_` → `skip`, `._language_` → `skip`, `._encoding_` → `skip`.
- **Code:** `crates/openehr-flat/src/flat/mappers.rs:60-72` emits `put(out, base, "formatting", …)` for `DV_TEXT`/`DV_PARAGRAPH` and (`:81`) for coded text; `text_from_flat`/`coded_text_from_flat` (`:265-296`) read `|formatting` back. `language`/`encoding` are correctly not surfaced.
- **Problem:** The SM rule marks `formatting` as skipped in the simplified form; the impl surfaces it (to improve round-trip fidelity, matching Better). A minor deviation from the transformation table. `language`/`encoding` handling *does* follow the rule.
- **Fix:** Either drop `|formatting` to match the SM rule, or (preferred, for round-trip fidelity) keep it and add a `// PORT NOTE:` citing `master07` and the deliberate deviation. Low urgency — `formatting` is optional and its presence does not break canonical validity.
- [x] fixed *(2026-07-06 — kept `|formatting` for round-trip fidelity, with a `// PORT NOTE:`
  in `mappers.rs` citing `master07` and the deliberate deviation; `language`/`encoding` remain
  dropped per the rule.)*

### F-10-07: FLAT→RM context rebuild fabricates invalid terminology codes (`openehr::0`) that will fail validation
- **Severity:** major
- **Spec:** RM invariants + terminology binding (`docs/specs/openehr/RM` `EVENT_CONTEXT`/`PARTICIPATION`/`ISM_TRANSITION`; TERM group codes) — a `DV_CODED_TEXT.defining_code` must resolve to a real openEHR terminology code; P15 composition validation enforces this.
- **Code:** `crates/openehr-flat/src/flat/context.rs:336-345` (participation `mode` built with `code_phrase("openehr", "0")`), `:434-439` (`apply_entry_defaults` ISM `current_state` built with `code_phrase("openehr", "0")`). Contrast `crates/openehr-flat/src/flat/from_flat.rs:415-421` and `graph.rs:83-86` which correctly use `openehr::524 "initial"` for ISM `current_state`.
- **Problem:** `openehr::0` is not a valid openEHR terminology code. Any composition rebuilt from a FLAT/STRUCTURED body that carries `ctx/participation_mode` or `ctx/action_ism_transition_current_state` will contain an invalid `defining_code`, which composition validation (P15) and terminology binding will reject — and it is internally inconsistent with the `524` used elsewhere for the same ISM field.
- **Fix:** In `context.rs`, for participation `mode` use a valid openEHR "participation mode" group code (default e.g. `openehr::193` "not specified", or leave `mode` absent since it is optional on `PARTICIPATION`), and for ISM `current_state` use `openehr::524 "initial"` to match `from_flat.rs`/`graph.rs`. Factor the ISM default into `graph::fill_structural_mandatory` so there is one source of truth.
- [x] fixed

### F-10-08: DV_MULTIMEDIA is lossy and diverges from the SM object form
- **Severity:** minor
- **Spec:** `SM/docs/serial_data_formats/master03-data_values.adoc` — `DV_MULTIMEDIA` primary form is a JSON object `{integrityCheckAlgorithm, mediaType, compressionAlgorithm, uri}`; EhrScape variant prefixes each with `|`. `mediaType` example `"IANA_media-types::text/plain"`.
- **Code:** `crates/openehr-flat/src/flat/mappers.rs:131-145` (to_flat: emits bare `uri`, `|mediatype` from `media_type.code_string`, `|alternatetext`, `|size`; inline `data` base64 **dropped**) and `:402-437` (from_flat: hardcodes `media_type` terminology `"IANA_media-types"`, no `integrity_check`/`compression` reconstruction).
- **Problem:** (a) Round-trip loss — inline `DV_MULTIMEDIA.data` (base64) present in canonical JSON is not surfaced in FLAT and cannot be recovered, so `RM → FLAT → RM` drops embedded media. (b) The suffix set (`|mediatype`/`|alternatetext`/`|size`) is Better's, not the SM object form (`mediaType`/`integrityCheckAlgorithm`/`compressionAlgorithm`/`uri`). (c) `integrity_check_algorithm`/`compression_algorithm` (RM-optional but sometimes present) are dropped.
- **Fix:** Document the media-data round-trip boundary (Better also omits inline data). Keep Better suffixes (CNF-aligned) but note the SM divergence. Consider preserving `|size` only when >0 (already done) and adding `|integrity`/`|compression` for fidelity if a corpus case needs it.
- [x] fixed *(2026-07-06 — the inline `DV_MULTIMEDIA.data` (base64) round-trip loss is now
  listed in the central `flat/mod.rs` "non-surfaced RM attributes" boundary (F-10-10); the
  `mappers.rs` DV_MULTIMEDIA arm already cites the Better mapper it follows. `|integrity`/
  `|compression` left out — no corpus case needs them yet.)*

### F-10-09: Rebuilt compositions hardcode `rm_version = "1.0.4"` while the project pins RM 1.2.0
- **Severity:** minor
- **Spec:** `docs/VERSIONS.md` (RM 1.2.0 is the pinned/target version); `ARCHETYPED.rm_version` should reflect the emitting system's RM version.
- **Code:** `crates/openehr-flat/src/flat/from_flat.rs:377` (`a.insert("rm_version".into(), json!("1.0.4"))`) and `:455` (`ensure_template_id` → `"rm_version": "1.0.4"`).
- **Problem:** FLAT→RM produces `ARCHETYPED.rm_version = "1.0.4"`, inconsistent with the workspace's RM 1.2.0 pin (and with what canonical serialisation elsewhere would report). `1.0.4` is the archie/EHRbase-era value; harmless for parsing but a version-provenance inaccuracy that could confuse the fidelity gate or downstream consumers.
- **Fix:** Source `rm_version` from a single crate constant tied to the RM pin (or take it from the template/OPT `rm_release` where available), not a literal.
- [x] fixed *(2026-07-06 — both `1.0.4` literals in `from_flat.rs` now read the single
  `flat::defaults::RM_VERSION = "1.2.0"` constant, tied to the RM pin (`docs/VERSIONS.md`) and
  matching the RM spec's ARCHETYPED.rm_version = "version used to create this object". The
  taking-from-OPT-`rm_release` variant is left for when OPT ingestion surfaces it.)*

### F-10-10: RM constructs outside the web-template are silently dropped (round-trip boundary undocumented)
- **Severity:** info
- **Spec:** SPEC-SILENT (Better convention) — the SM SDT design explicitly makes data "less self-standing" (`master03-conceptual.adoc:205`), so lossiness is intended; but the exact boundary is a local decision.
- **Code:** `crates/openehr-flat/src/flat/to_flat.rs:45-80` (`walk` visits only web-template nodes) + `mappers.rs:146-156` (reference ranges not surfaced, noted in-code).
- **Problem:** `LINK`, `FEEDER_AUDIT`, non-root `uid`, `DV_ORDERED.normal_range`/`other_reference_ranges`, and `DV_TEXT.mappings` have no web-template node and are dropped on `RM → FLAT`. The tested contract is `from_flat → to_flat` stability (round-trip of *FLAT-expressible* data), not full RM fidelity. This is correct for FLAT but is not stated as the explicit boundary anywhere central.
- **Fix:** Add a short "FLAT round-trip scope / non-surfaced RM attributes" list to `flat/mod.rs` docs so the boundary is explicit and testable.
- [x] fixed *(2026-07-06 — `flat/mod.rs` now has a "FLAT round-trip scope" paragraph listing
  the non-surfaced RM attributes (`LINK`, `FEEDER_AUDIT`, non-root `uid`, DV_ORDERED reference
  ranges, `DV_TEXT.mappings`, inline `DV_MULTIMEDIA.data`) and stating the tested contract is
  `from_flat → to_flat` stability of FLAT-expressible data.)*

### F-10-11: `is_multiple` attribute set is a hardcoded list, not derived from the RM model, and includes a non-attribute (`actions`)
- **Severity:** minor
- **Spec:** RM attribute multiplicity (`docs/specs/openehr/RM`) — the authoritative multi-valued-attribute set should come from the BMM RM model (the same model ADR-008/P16 mandates for AQL path analysis).
- **Code:** `crates/openehr-flat/src/flat/from_flat.rs:38-44` (`is_multiple` matches `content | items | events | activities | actions`).
- **Problem:** The reverse converter decides which AQL steps are array levels from a fixed string set. `actions` is not an RM attribute (INSTRUCTION has `activities`, ACTION has none named `actions`) — dead arm. More importantly, any other genuinely multi-valued attribute reachable in a template (e.g. `credentials`, `other_participations`, nested cluster/section variants) is not covered and would be rebuilt as a single object, mis-shaping the RM. Works for the common COMPOSITION/HISTORY/ITEM path but is not RM-general.
- **Fix:** Drive multiplicity from the generated BMM RM attribute model (P16) instead of the hardcoded list; drop the `actions` arm. Until then, add a `// TODO(port):` referencing the RM-model dependency.
- [x] fixed *(2026-07-06 — dropped the dead `actions` arm from `from_flat::is_multiple`; added
  a `// TODO(port):` citing the ADR-008/P16 BMM-RM-model dependency as the real fix. The
  BMM-model-driven multiplicity is P16 work.)*

### F-10-12: FLAT/STRUCTURED composition commit/retrieve media types are not in the vendored ITS-REST composition operations (extension)
- **Severity:** info
- **Spec:** ITS-REST — `wt.flat+json`/`wt.structured+json` appear only in `Resources.md` data-representation prose and on the *template* GET; a grep of `operations/`, `requestBodies/`, `responses/` finds them only in `definition_template_adl1.4_get.yaml`, not in any COMPOSITION operation.
- **Code:** `crates/ehrbase-rest/src/dispatch/ehr.rs:167-230` wires `composition_from_flat`/`_structured` and `composition_flat_response`/`_structured_response` onto COMPOSITION create/get/update via `negotiate::wants_flat`/`is_flat_body`/`wants_structured`.
- **Problem:** Committing/retrieving a COMPOSITION in FLAT/STRUCTURED is an EHRbase-compatible extension of the ITS-REST COMPOSITION endpoints (the vendored OAS defines only canonical JSON/XML there). Not CNF-gated. Behaviourally aligned with EHRbase; just record it as an extension surface.
- **Fix:** None functional. Note in `dispatch/flat.rs` docs that COMPOSITION FLAT/STRUCTURED I/O is an EHRbase extension beyond the vendored ITS-REST COMPOSITION operations.
- [ ] fixed

### F-10-13: `ctx/` vocabulary and synthesised PARTY/participation defaults are EHRbase convention (SPEC-SILENT)
- **Severity:** minor
- **Spec:** SPEC-SILENT (Better/EHRbase). `SM/docs/simplified_im_b/master06-app_context.adoc` posits an application-context model but leaves the class defs as UML-include stubs; the concrete `ctx/language`, `ctx/health_care_facility|id`, `ctx/participation_*:i`, `ctx/setting|code` keys are EHRbase's SDT Context doc, not normative openEHR.
- **Code:** `crates/openehr-flat/src/flat/context.rs` (entire module) — esp. `party_identified` (`:264-296`) inventing `scheme:"id_scheme"`, `namespace:"EHR"`, `type:"PERSON"/"ORGANISATION"`; `participations_from_ctx` (`:299-349`) forcing `function` to an empty-string `DV_TEXT` and defaulting party type.
- **Problem:** The `ctx/` shortcut set and its default-filling are a vendor convention faithfully realised, but several synthesised values (`GENERIC_ID.scheme = "id_scheme"`, an empty-string participation `function`) are placeholders that may not pass RM/terminology validation and are invisible to the client. Combined with F-10-07 these are the main correctness risks of the reverse path.
- **Fix:** Where a synthesised value is not clinically meaningful, prefer omission over a placeholder when the field is RM-optional (participation `function` is 1..1 `DV_TEXT` so an empty string is at least structurally valid; `mode` is optional and should be omitted rather than defaulted — see F-10-07). Document the `ctx/` set as an EHRbase convention with a spec-watch note.
- [ ] fixed

### F-10-14: Default context data (epoch time, `openehr::238` setting) injected into committed compositions
- **Severity:** minor
- **Spec:** SPEC-SILENT (Better `ConversionContext` defaults). RM: `EVENT_CONTEXT.start_time` and `.setting` are mandatory.
- **Code:** `crates/openehr-flat/src/flat/context.rs:27-30` (`DEFAULT_TIME = "1970-01-01T00:00:00Z"`, `DEFAULT_SETTING_CODE = "238"`); applied in `apply_ctx` (`:204-214`) and mirrored on output in `emit_ctx` (`:74-98`); `DEFAULT_TIME` is also independently defined in `from_flat.rs:27`.
- **Problem:** To satisfy the RM-mandatory `start_time`/`setting`, the converter fabricates an epoch timestamp and `openehr::238 "other care"` when the FLAT body omits `ctx/time`/`ctx/setting`. This writes clinically meaningless data into stored compositions. It is round-trip-stable (emit mirrors apply) and matches Better, but a committed composition dated 1970-01-01 is a data-quality hazard. `DEFAULT_TIME` duplicated across two modules invites drift.
- **Fix:** Consolidate the defaults into one place (e.g. `context.rs`, re-used by `from_flat.rs`). Consider requiring `ctx/time` (400 on absence) rather than fabricating epoch, matching stricter EHRbase configs; at minimum document the default clearly.
- [x] fixed *(2026-07-06 — the duplicated `DEFAULT_TIME` (and the setting defaults) now live in
  one `flat/defaults.rs` module consumed by both `context.rs` and `from_flat.rs`; documented
  there. The fabricate-epoch-vs-400-on-absence behaviour is deliberately unchanged (Better
  parity) — only the drift-prone duplication is resolved.)*

### F-10-15: STRUCTURED shape is a pure Better convention with only an internal round-trip contract
- **Severity:** info
- **Spec:** SPEC-SILENT (Better `RawToStructuredConverter`/`FlatToStructuredConverter`). `ITS-REST/.../simplified_data_template/master03-conceptual.adoc:136-194` shows a structSDT *example* but no normative shape rules.
- **Code:** `crates/openehr-flat/src/structured/mod.rs` + `structured/entry.rs`.
- **Problem:** `flat ⇄ structured` is implemented as a WebTemplate-independent nesting transform (root object, arrays-of-objects, `|suffix` keys, `ctx` object), verified only by internal round-trip unit tests, not against any spec fixture. Reasonable given no normative structSDT, but there is no external oracle; a Better `web-template-tests` structured fixture would strengthen confidence.
- **Fix:** Add a structured fixture from Better `web-template-tests` (or a CNF FLAT fixture round-tripped through structured) as an `insta`/corpus test to pin the shape against an external reference.
- [ ] fixed

## Hygiene notes

- **Duplicated `code_phrase`/`DV_CODED_TEXT` constructors.** At least four
  hand-rolled builders: `flat/context.rs:36` (`code_phrase`),
  `flat/graph.rs:21-36` (`code_phrase`/`dv_coded_text`),
  `flat/mappers.rs:312` (`code_phrase_obj`), plus inline `json!` code phrases in
  `from_flat.rs` (`:387-388`, `:416-420`) and `webtemplate/builder.rs`. Consider
  a single small `openehr-flat` helper module (or reuse `openehr-rm`
  constructors) so the terminology defaults (F-10-07) cannot drift again.

- **`DEFAULT_TIME` defined twice** (`flat/context.rs:27`, `flat/from_flat.rs:27`)
  with the same value — one source of truth (see F-10-14).

- **Two coded-text-with-`other` compaction paths.** `builder.rs:358-387`
  (`post_process_element` / `compact_to_coded_with_other`, build phase) and
  `builder.rs:494-514` (`compact_coded_text_with_other`, compaction phase)
  implement nearly identical DV_CODED_TEXT+DV_TEXT→coded-with-other merges. They
  fire at different tree stages (pre- vs post-compaction), which is why both
  exist, but the near-duplicate logic is a maintenance trap — factor the shared
  "make coded input list-open + append `other` text input + drop text sibling"
  into one helper.

- **Local AQL-path parser duplicates path-handling concerns.**
  `crates/openehr-flat/src/flat/aql.rs` hand-parses `/attr[predicate]` segments
  independently of the full AQL front-end in `crates/openehr-query` (which has a
  `logos`+`chumsky` lexer/parser) and of the materialised-path handling in
  `crates/ehrbase/src/storage/codec.rs`. Three places parse openEHR paths. The
  flat parser is a deliberately narrow subset (predicates only, no full AQL), so
  reuse may not be worth it, but this should be a conscious decision — note it,
  or expose a shared `openehr-query` path-segment API the flat converter can
  consume, especially before the P16 RM-model work (F-10-11) lands.

- **`ehrbase-compat` is an empty stub** (`crates/ehrbase-compat/src/lib.rs`, 6
  lines). The EhrScape/WebTemplate/FLAT endpoints the crate is slated to own
  (per `architecture.md` and `rest-axum.md`) currently live in
  `ehrbase-rest/src/dispatch/flat.rs` + `definition.rs`. Not a defect, but the
  crate-boundary intent in the docs and the actual code location have drifted;
  reconcile (either move the compat glue into `ehrbase-compat` at P17 or update
  the architecture docs to say it lives in `ehrbase-rest`).

- **`serde_json::Value` for WebTemplate range bounds.** `model.rs:259-269`
  (`WebTemplateRange.min`/`max`) uses `Value` to hold int-or-decimal-or-ISO
  bounds, which is correct for the polymorphic Better shape but means the
  ITS-REST `Range.min: integer` schema type (F-10-04) is not statically
  enforced; fine, just noted.
