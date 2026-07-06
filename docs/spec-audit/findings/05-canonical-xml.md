# 05 — Canonical XML (ITS-XML)

Audit of the canonical-XML surface against the vendored openEHR ITS-XML XSDs
(`crates/openehr-its/schemas/xml/`). Oracle = the XSD/spec only; EHRbase is
prior art, not authority.

Layers audited:
- Runtime: `crates/openehr-its/src/xml/runtime.rs`, `mod.rs`
- Generated impls: `crates/openehr-its/src/xml/generated/impls.rs`
- Emitter: `crates/openehr-codegen/src/{emit_xml.rs,xsd.rs}` + `src/emit.rs::xml_types`/`xml_fields`, wired in `src/main.rs::cmd_emit_xml`
- REST negotiation: `crates/ehrbase-rest/src/negotiate.rs`, `dispatch/ehr.rs`
- Gates: `crates/openehr-its/tests/xml_roundtrip.rs`, `xml_ehrbase.rs`, `xml_hash.rs`, `xml_smoke.rs`

## Summary

The XML codegen is largely faithful for the RM types that are actually present
in the curated v1 XSD set: element order, the attribute/element split
(`archetype_node_id` as an attribute on LOCATABLE subtypes), `xsi:type`
emission (only when concrete ≠ declared slot type), single-root namespace
injection, `StringDictionaryItem` shape for `Hash<String,String>`, and base64
carriage for `DV_MULTIMEDIA.data` are all correct. Verified good on
COMPOSITION, DV_QUANTITY (full DV_ORDERED→DV_AMOUNT→DV_QUANTITY order),
FEEDER_AUDIT, DV_INTERVAL, DATA_VALUE `xsi:type` dispatch.

The material problems come from **coverage gaps in the vendored input**, not the
emitter logic:

1. **The v1 1.0.2 ALL bundle has no `Ehr.xsd` and no demographic schema.** The
   emitter reads only `RM_FILES_V1` (BaseTypes, Structure, Content, Composition,
   Version, Extract, Resource). Every LOCATABLE-derived type outside that closure
   (16 types, incl. **EHR_STATUS** and **EHR_ACCESS**) falls back to
   BMM-field-order with **zero attributes**, so `archetype_node_id` is emitted as
   a child *element* instead of an XML *attribute* — a hard wire divergence from
   the XSD. **EHR_STATUS is actively served as canonical XML at the REST surface.**
2. **A structural asymmetry:** `ToXml` for XSD-covered types is driven by the XSD
   element set, while `FromXml` is driven by the BMM field set. Any BMM field not
   present in the vendored 1.0.2 XSD is silently dropped on XML output while kept
   in JSON — and **no gate can detect it** (round-trip stays "stable" because both
   serializations omit it).
3. **The C14N byte-parity gate against archie/EHRbase output is un-wired** — the
   two gates prove only internal `ToXml`↔`FromXml` consistency and read-robustness
   on real fixtures, not byte-parity and not field completeness.

Counts: 1 critical, 2 major, 4 minor, 4 info.

## Findings

### F-05-01: LOCATABLE subtypes outside the v1 XSD emit `archetype_node_id` as an element, not an attribute
- **Severity:** critical
- **Spec:** `its-xml-1.0.2-nsv1/ALL/Structure.xsd` — `LOCATABLE` declares
  `<xs:attribute name="archetype_node_id" type="archetypeNodeId" use="required"/>`
  (an XML **attribute**). Confirmed identical in v2
  `its-xml-2.0.0-nsv2/RM/Release-1.1.0/Common.xsd` `LOCATABLE`, and
  `Ehr.xsd` defines `EHR_STATUS`/`EHR_ACCESS` as `extension base="LOCATABLE"`.
- **Code:** `crates/openehr-its/src/xml/generated/impls.rs` — `EhrStatus` (impl at
  :7898, emits `archetype_node_id` as element at :7921), `EhrAccess` (:7788/:7811),
  and 14 more. Served as XML: `crates/ehrbase-rest/src/dispatch/ehr.rs:98,107,117`
  (`respond_rm::<EhrStatus>`). Root cause: `crates/openehr-codegen/src/xsd.rs`
  `RM_FILES_V1` (:216) has no `Ehr.xsd`/`Demographic.xsd` (none exists in the
  1.0.2 `ALL/` bundle); `crates/openehr-codegen/src/emit_xml.rs:114-125` falls
  back to BMM order + **empty attribute list** for any `spec` not in
  `xsd.types`.
- **Problem:** 16 LOCATABLE-derived types serialize `<archetype_node_id>…</…>` as
  a child element in BMM field order, with no XML attribute. This is invalid
  canonical XML (fails XSD validation: `archetype_node_id` is a required
  attribute and is not a declared element) and diverges from what archie/EHRbase
  emit. The affected set:
  `EhrStatus`, `EhrAccess` (EHR package — **EhrStatus is served now**);
  `Address, Agent, Capability, Contact, Group, Organisation, PartyIdentity,
  PartyRelationship, Person, Role` (demographic — served in a later phase);
  `ExtractActionRequest, ExtractEntityChapter, GenericContentItem,
  OpenehrContentItem`. (23 other LOCATABLE types that *are* in the XSD closure
  correctly emit it as an attribute — e.g. `Composition` at impls.rs, the
  `__attrs.push(("archetype_node_id", …))` path.)
- **Fix:** (emitter) extend the XSD input so these types get an entry. The fix
  source is already vendored: parse the v2 `RM/Release-1.1.0/Ehr.xsd` +
  `Demographic.xsd` (and BASE where needed) **appended after** the v1 files in
  `cmd_emit_xml`/`XsdModel::parse_files` — `parse_files` uses `.or_insert`
  (:76), so shared types (CONTENT_ITEM, SECTION, COMPOSITION, …) keep their v1
  definitions and only the missing EHR/demographic types are added. The v1
  `LOCATABLE` (already in the model, base of these types via the flatten walk)
  then supplies the `archetype_node_id` attribute + canonical element order. The
  wire shape is identical across lineages bar the root `xmlns` — the emitter's own
  stated rationale for one impl set (main.rs:168-174) — so this is safe. Add a
  guard so a LOCATABLE subtype with an empty attribute set fails codegen.
- **Fix applied (emitter):** `crates/openehr-codegen/src/xsd.rs` — the emit-xml
  XSD input is now the v1 *served* core (`RM_FILES_V1_SERVED`: BaseTypes,
  Structure, Content, Composition, Version, Resource) merged with the v2
  RM-1.1.0 supplement (`RM_FILES_V2_SUPPLEMENT`: `Ehr.xsd`, `Demographic.xsd`,
  `EhrExtract.xsd`) via `xml_emit_files`, wired in `main.rs::cmd_emit_xml`. v1
  wins for shared/served types (`.or_insert`); the v2 files add the missing
  LOCATABLE subtypes, whose base chain reaches the v1 `LOCATABLE` and so picks up
  `archetype_node_id` as the required attribute + canonical element order. v1
  `Extract.xsd` (the stale RM-1.0.2 model, whose `EXTRACT_ITEM` does **not**
  extend LOCATABLE, contradicting the BMM) is dropped in favour of the
  BMM-consistent v2 `EhrExtract.xsd`. A guard (`emit_xml.rs::check_locatable_attr`,
  enforced in `emit_file`) now **fails codegen** if any emitted struct with an
  `archetype_node_id` field lacks an XSD attribute classification for it.
  Verified: all 16 affected types (EHR_STATUS/EHR_ACCESS, the 10 demographic
  types, the 4 extract subtypes) now emit `archetype_node_id` as an attribute;
  zero remaining element emissions; regression test
  `crates/openehr-its/tests/xml_locatable_attr.rs` (EHR_STATUS, EHR_ACCESS,
  PERSON, GENERIC_CONTENT_ITEM). All `openehr-its` gates green; drift idempotent.
- [x] fixed

### F-05-02: `ToXml` writes the XSD element set but `FromXml` reads the BMM field set — BMM-only fields are silently dropped on XML output
- **Severity:** major
- **Spec:** N/A (mechanism); consequence is loss of RM-1.2.0 fields absent from
  the vendored ITS-XML 1.0.2 shapes.
- **Code:** serialize path `crates/openehr-codegen/src/emit_xml.rs:113-125,148-153`
  — when `spec` is in the XSD, `elems` come **only** from `xsd.flattened(spec)`;
  a BMM field whose `wire_name` is not among the XSD attrs/elems is never written
  (and is not recorded in `unmatched`, which tracks only the reverse direction,
  main.rs:206-209). Deserialize path `emit_xml.rs:271-305` builds accumulators
  from the **BMM** `fields`, i.e. reads every BMM field regardless of the XSD.
- **Problem:** the model is RM 1.2.0 but the driving XSD is ITS-XML 1.0.2
  (`docs/VERSIONS.md` notes this skew). Any field RM added since 1.0.2 to a type
  that *is* in the XSD is kept in canonical JSON but dropped from canonical XML —
  a silent JSON/XML divergence. No gate catches it: `xml_roundtrip.rs` and
  `xml_ehrbase.rs` assert only that `to_xml(x)==to_xml(from_xml(to_xml(x)))`;
  since the field is dropped on *every* `to_xml`, the two serializations agree and
  the gate is green while data is lost. (No dropped field was confirmed in the
  RM types spot-checked — FEEDER_AUDIT, ISM_TRANSITION, DV_QUANTITY, COMPOSITION
  are complete — so this is a latent risk created by the version skew + a gate
  blind spot, not a proven live data loss.)
- **Fix:** (emitter) after emitting the XSD-ordered fields, emit any remaining
  BMM fields not matched to an XSD attr/elem (appended, or at least warn), so an
  XSD/BMM drift surfaces instead of dropping data; and record BMM-field-without-
  XSD-match in a second `unmatched` list that the `codegen-drift`/`check-xsd`
  step reports. Alternatively pin the XML driver to the v2 (RM-1.1.0/BASE-1.2.0)
  XSDs which are closer to the BMM.
- **Fix applied (emitter, W2-I):** `emit_xml.rs` now reconciles the two field
  sets per XSD-covered struct. `bmm_only_fields()` computes (BMM fields ∖ XSD
  attrs∪elems); a guard (`check_bmm_field_coverage`, enforced in `emit_file` the
  way `check_locatable_attr` is) **fails codegen** listing every BMM field with
  no XSD slot unless it is on the `XML_BMM_ONLY_ALLOWLIST` — each allowlist entry
  citing the RM-1.2.0/BASE-1.3.0-vs-vendored-ITS-XML spec delta. Allowlisted
  fields are appended as **deterministic trailing canonical-XML elements** in BMM
  order (`emit_to_xml`), so nothing is silently dropped; the reconciliation is
  reported to stderr on every `emit-xml`. **The reconciliation surfaced 44
  previously-dropped fields** (the ToXml/FromXml asymmetry was live, not merely
  latent): BASE 1.3.0 AUTHORED_RESOURCE.uid/annotations + the 10 RESOURCE_-
  DESCRIPTION additions + 2 TRANSLATION_DETAILS additions + the `accreditaton`
  (sic) BMM spelling + CODE_PHRASE.preferred_term; RM 1.2.0 ENTRY.workflow_id
  (renamed from `work_flow_id`, ×5 subtypes), DV_QUANTITY.units_system/
  units_display_name, ELEMENT.null_reason, ISM_TRANSITION.reason,
  FEEDER_AUDIT_DETAILS.other_details, FOLDER.details, EHR.tags; EhrExtract
  includes_*→include_* renames (EXTRACT_SPEC/EXTRACT_VERSION_SPEC); and the
  VERSIONED_OBJECT base fields (uid/owner_id/time_created) on the four
  VERSIONED_* container types (base defined only in un-merged v2 Common.xsd).
  All now emitted with their RM-1.2.0 canonical names (matching canonical JSON).
- [x] fixed

### F-05-03: C14N byte-parity gate against archie/EHRbase output is un-wired
- **Severity:** major
- **Spec:** ITS-XML canonical form; `serialization.md` rule ("C14N uses
  `xmllint --c14n` … for now").
- **Code:** `crates/openehr-its/tests/` — no reference to `xmllint`/`c14n`
  anywhere; `xml_ehrbase.rs` docstring explicitly states a "byte-for-byte C14N
  compare against the fixture is not the bar here; that awaits archie-canonical
  vectors / the live parity harness."
- **Problem:** the two XML gates prove (a) internal `ToXml`↔`FromXml` round-trip
  stability on the corpus (`xml_roundtrip.rs`) and (b) that stock-EHRbase
  fixtures parse and self-round-trip (`xml_ehrbase.rs`). Neither proves our
  output is byte-identical (post-C14N) to archie/EHRbase, nor field-complete
  (F-05-02). So wire-parity for XML is currently **unverified** — divergences
  like F-05-01/04/05 pass the suite.
- **Fix:** wire an archie-canonical vector set (or the live parity harness) with
  `xmllint --c14n` comparison as the acceptance gate for XML (Stage-1). Track as a
  known gap until then.
- **Fix applied (W2-I):** new gate `crates/openehr-its/tests/xml_c14n.rs`. It
  takes the vendored **CNF** canonical-XML COMPOSITION fixtures
  (`docs/specs/openehr/CNF/.../compositions/CANONICAL_XML/*.xml`), parses each
  via `FromXml`, re-serializes via `to_canonical_xml`, canonicalizes both the
  fixture and our output with `xmllint --noblanks --c14n` (verified available on
  the machine), and **byte-compares**. Result: our output is byte-identical to
  all 4 valid fixtures for element order, values, `archetype_node_id`
  attributes, namespaces, and text — the **only** residual is the cabolabs
  generator's *verbose* `xsi:type` (redundant on the document root + every
  concrete-typed slot, e.g. `<name xsi:type="DV_TEXT">`), whereas we (and
  archie/EHRbase, the Stage-1 parity target) emit the **minimal** set. This one
  cited serialization-convention axis is normalized: a fixture is classed an
  `xsi:type`-convention match only if stripping every `xsi:type` from both sides
  is byte-identical **and** our `xsi:type` set is a subset of the reference's;
  **any other difference is a hard failure** (the comparison is not loosened for
  content). The 3 `__invalid_*` fixtures are skip-listed (validation-negative
  inputs). Gate result: 0 strict, 4 xsi:type-convention, 3 skipped, 0 failed.
  (True *strict* byte-parity awaits an archie-minimal-convention canonical vector
  set; the gate is wired and live now, catching any structural/value/order/attr
  regression.)
- [x] fixed

### F-05-04: interval inclusivity flags always emitted; archie omits them at the unbounded default
- **Severity:** minor
- **Spec:** `ALL/BaseTypes.xsd` `DV_INTERVAL` — `lower_included`/`upper_included`
  are `minOccurs="0"` (:59-60); `Interval` likewise (:526-527).
- **Code:** `impls.rs` `DvInterval<T>` ToXml (:6104-6137) writes
  `lower_included`/`upper_included` unconditionally
  (`self.lower_included.write_xml(...)`), because the RM fields are non-optional
  `bool`. `FromXml` tolerates their absence via the `default` mechanism
  (`emit.rs::xml_fields` `default`, `emit_xml.rs:343-346`).
- **Problem:** for an unbounded boundary archie omits the corresponding
  `_included` flag; we always emit it. Round-trip is stable (we always write /
  always tolerate), but output is not byte-identical to archie — a C14N-parity
  divergence (masked by F-05-03).
- **Fix:** (emitter) when a field carries a `default` (the archie-omitted-at-
  default set), skip writing it on `ToXml` when the value equals that default,
  mirroring the tolerant read. Verify against archie vectors once F-05-03 lands.
- **W2-I assessment (left):** does **not** block the F-05-03 byte-parity gate —
  none of the 4 CNF canonical COMPOSITION fixtures carry a `DV_INTERVAL`, so the
  flags are never exercised there and our output is byte-identical without this
  change. Deferred (minor); revisit when an archie interval vector is added.
- [ ] fixed

### F-05-05: number/boolean lexical forms not gated for parity; `f32` vs `f64` asymmetry
- **Severity:** minor
- **Spec:** XSD `xs:double`/`xs:float` (e.g. `DV_QUANTITY.magnitude`
  BaseTypes.xsd:114, `DV_AMOUNT.accuracy` :95) — lexical `double`/`float`.
- **Code:** `runtime.rs:217-229` — `f64` emits `format!("{self:.1}")` for whole
  values (`120.0`), else `to_string()`; `f32` uses bare `Display` (:215, no
  `.0`). Booleans use `bool` `Display` → `true`/`false` (correct). ISO-8601
  temporals are carried as `String` and emitted verbatim (correct — preserves
  partial precision).
- **Problem:** whole-`f64` → `120.0` matches the openEHR convention (RM Reals are
  `f64`, so `f32` is effectively unused for RM), but exact number parity vs archie
  (e.g. very large/small magnitudes, `-0.0`, precision-bearing decimals) is not
  gated (an in-code `PERF(port)` note flags this). The `f32`/`f64` `.0`
  asymmetry is a latent trap if any field is ever `f32`.
- **Fix:** add an exact-number-parity check to the XML gate (F-05-03) over the
  corpus; make `f32` use the same whole-number formatting as `f64` for
  consistency.
- **W2-I assessment (partially addressed):** the new C14N gate (F-05-03) now
  byte-checks number/boolean lexical forms for every value present in the CNF
  canonical fixtures (a `120` vs `120.0` regression would fail it) — that part of
  the fix is effectively landed. The `f32`/`f64` `.0` asymmetry in `runtime.rs`
  is untouched (runtime is out of the emitter-only scope of this task and no RM
  field is `f32`); left as a latent-trap note.
- [ ] fixed

### F-05-06: VERSION-family / CONTRIBUTION XML is refused (406) though ITS-XML defines the shape; the payload is an untyped monomorphization artifact
- **Severity:** minor
- **Spec:** `ALL/Version.xsd` defines `VERSION`, `ORIGINAL_VERSION`,
  `IMPORTED_VERSION` canonical XML; `ORIGINAL_VERSION.data` is `xs:anyType`
  (:20).
- **Code:** `crates/ehrbase-rest/src/negotiate.rs::respond` (:270-284) returns
  `ApiError::NotAcceptable` (406) for XML on VERSION-family/revision-history/
  contribution responses, stating they "have no spec-defined canonical-XML
  shape." They partly do (Version.xsd), but the wrapped `data` maps to
  `serde_json::Value` in the RM (ADR-004 monomorphization) and would serialize as
  JSON-text — see F-05-08.
- **Problem:** a 406 where the spec defines a container shape is a missing
  capability. It is the *safe* choice given F-05-08 (a typed VERSION XML would
  emit a JSON blob for the wrapped object), so this is a scope note, not a crash.
  The 406 message references a stale "(P12)" milestone.
- **Fix:** either type the version-family `data` (resolve the ADR-004
  monomorphization for `X_VERSIONED_*`) and serve VERSION XML properly, or keep
  the 406 and document it as an intentional gap; update the stale message text.
- [ ] fixed

### F-05-07: `serde_json::Value` slots serialize as JSON-text on write and are discarded (→ Null) on read
- **Severity:** info
- **Spec:** N/A — these slots (version-family `data`, BMM `Any`) are ADR-004/005
  monomorphization artifacts with no spec canonical-XML shape.
- **Code:** `runtime.rs:246-258` (write JSON `to_string()` as element text) and
  `:474-482` (`skip_element` → `Value::Null`).
- **Problem:** documented scope boundary. Consequence: any XML path that reaches
  such a slot is lossy (writes a JSON blob, reads back Null). It is why VERSION
  XML cannot be offered conformantly (ties to F-05-06). These slots do not occur
  on the RM composition/EHR-status wire, so it does not affect the served
  COMPOSITION/EHR_STATUS paths.
- **Fix:** none required for the current served surface; revisit if the
  monomorphized types are made precise.
- [ ] fixed

### F-05-08: `xsi:type` value is written unprefixed; read strips any prefix
- **Severity:** info
- **Spec:** ITS-XML uses the default namespace for RM instances; a bare
  `xsi:type="DV_TEXT"` resolves against the default `xmlns`.
- **Code:** write `emit_xml.rs:111` (`"xsi:type", "<SPEC>"`); read
  `runtime.rs::StartTag::xsi_type` (:284-289) strips any prefix on the value
  (`v1:CLUSTER` → `CLUSTER`).
- **Problem:** none for canonical (default-namespace) documents — write is
  correct, read is tolerant of prefixed inputs. Noted for completeness.
- **Fix:** none.
- [ ] fixed

### F-05-09: `DV_INTERVAL` bounds pass the literal generic name `"T"` as the declared slot type
- **Severity:** info
- **Spec:** `BaseTypes.xsd` `DV_INTERVAL` — `lower`/`upper` are `DV_ORDERED`
  (abstract), so a concrete bound requires `xsi:type`.
- **Code:** `impls.rs` `DvInterval<T>` (:6122-6127) writes
  `v.write_xml(w, "lower", Some("T"))`.
- **Problem:** works as intended by accident — because no openEHR type is named
  `T`, the concrete bound's `xml_type_name()` always differs from `"T"`, so
  `xsi:type` is always emitted (correct for a `DV_ORDERED` slot). Fragile: it
  relies on the generic parameter never colliding with a real spec type name.
- **Fix:** (emitter) emit the actual declared slot type (`DV_ORDERED`) for a
  generic bound instead of the parameter name, so intent is explicit.
- [ ] fixed

### F-05-10: `DV_MULTIMEDIA.data` / `integrity_check` base64 handled correctly (verification)
- **Severity:** info
- **Spec:** `BaseTypes.xsd` `DV_MULTIMEDIA` — `data`/`integrity_check` are
  `xs:base64Binary` (:299,302).
- **Code:** RM `DvMultimedia.data`/`integrity_check` are `Option<String>`
  (`crates/openehr-rm/src/data_types/encapsulated/dv_multimedia.rs:22,28`) holding
  the base64 text; `impls.rs:6230-6244` emits them as text elements via the
  `String` `ToXml`. Element order matches the XSD flatten
  (charset, language, alternate_text, uri, data, media_type, …).
- **Problem:** none — base64 is carried verbatim as a string, correct for both
  JSON and XML. Recorded as a positive verification.
- [ ] fixed

### F-05-11: `xs:token` normalization not applied
- **Severity:** minor
- **Spec:** `BaseTypes.xsd` — `OBJECT_ID.value` (:358), `OBJECT_REF.namespace`
  and `OBJECT_REF.type` (:405-406) are `xs:token` (whitespace-collapsed).
- **Code:** these are `String` fields emitted/parsed verbatim (`runtime.rs`
  `String` impls).
- **Problem:** no `xs:token` whitespace collapse on read/write. In practice the
  values (UIDs, namespaces, type names) contain no collapsible whitespace, so the
  divergence is theoretical; a pathological input with internal/edge whitespace
  would not be normalized.
- **Fix:** low priority — normalize `xs:token`-typed leaves if strict XSD
  conformance on malformed input is required; otherwise document as out of scope.
- [ ] fixed

## Hygiene notes

- **The emitter's "v1 shape serves both lineages" rationale is sound but the v1
  input is incomplete.** The single-impl-set decision (main.rs:168-174) is
  correct (wire shape differs only by root `xmlns`), but it silently assumes the
  v1 `ALL/` bundle is a complete RM closure. It is not — no `Ehr.xsd`, no
  demographic schema — which is the root of F-05-01. The vendored v2 bundle *does*
  have both; the emitter should draw the missing shapes from there rather than
  degrade to attribute-less BMM order.
- **`unmatched` is one-directional.** `cmd_emit_xml` reports "XSD-only elements
  without a BMM field" but never the reverse (BMM field without an XSD element,
  F-05-02). Both directions should be reported and drift-gated.
- **Gate naming vs guarantee.** `xml_roundtrip.rs` is a *self-consistency* gate,
  not a *conformance* gate; its name and the `ok > 10` assertion can read as
  stronger than they are. The header comments are honest about this, but the
  suite provides no field-completeness or byte-parity guarantee (F-05-02/03).
- **`RM_FILES_V2`/`v2_files` are `#[allow(dead_code)]`** (`xsd.rs:256-273`) —
  reserved for a future v2 trait. They are the natural vehicle for the F-05-01
  fix (pull EHR/demographic shapes from v2).
- **Stale milestone text** in `negotiate.rs::respond` (":278, "(P12)") and the
  `respond` docstring reference an old phase; harmless but should be trimmed.
- No `unwrap`/`expect` outside tests in the audited runtime; error handling maps
  cleanly to `XmlError`/`ApiError` (the non-RM XML path is a clean 406, not a 500).
