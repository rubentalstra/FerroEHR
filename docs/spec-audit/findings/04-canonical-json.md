# 04 — Canonical JSON (ITS-JSON)

## Summary

The canonical-JSON layer is fundamentally sound and matches the ITS-JSON
contract on the **serialize** side: `_type` is emitted first, in uppercase RM
class names; `None`/empty containers are omitted (never `null`/`[]`); UIDs
serialize as `{_type, value}`; `DV_MULTIMEDIA.data` is inline base64 text; the
Integer/Real distinction is preserved by typed fields (whole Reals emit `x.0`);
and every definition in the vendored schema permits `_type`, so the
always-emit-`_type` policy is schema-safe. The fidelity gate
(`crates/openehr-its/tests/fidelity.rs`) proves readability, lossless
round-trip, and schema validation over the vendored corpus, with the RM
1.1↔1.2 divergence explicitly characterized via documented exclusions.

The findings are all on the **deserialize / tolerance** side. The
`OpenEhrType` derive is a *lenient* reader: it tolerates a missing `_type` and
silently ignores unknown fields. The vendored ITS-JSON schema is *strict*:
`_type` is `required` (at the root and on every polymorphic slot) and
`additionalProperties: false` on every class. That gap has one materially
important consequence — abstract slots are `#[serde(untagged)]` enums whose
dispatch is correct **only when `_type` is present**; with `_type` absent they
fall back to structural matching in declaration order, which can silently
mis-type a value rather than reject it (F-04-01). The remaining items are
leniency/diagnostics/versioning notes.

No `// @generated` files were edited; findings that touch generated shapes name
the emitter/derive as the fix site.

## Findings

### F-04-01: Untagged abstract-slot enums mis-dispatch (silent wrong type) when `_type` is absent, instead of rejecting

- **Severity:** major
- **Spec:** `docs/specs/openehr/ITS-JSON/README.adoc` §"Design choices"
  ("`if ...` construction for polymorphism … in the reference"); vendored
  schema `crates/openehr-its/schemas/json/openehr_rm_1.1.0_all.json` — root
  `allOf[0] = {"required":["_type"]}` and every polymorphic slot (e.g.
  `ELEMENT.uid`) wraps its `$ref` dispatch in `{"required":["_type"], …}`. The
  serialization rule (`.claude/rules/serialization.md` → Canonical JSON):
  "`_type` … is **required** whenever the statically declared field type is
  abstract."
- **Code:** `crates/openehr-derive/src/lib.rs:248-268` (Deserialize tolerates a
  missing `_type` — the `if let Some(t) = &shadow.__type` only checks *when
  present*); abstract slots such as
  `crates/openehr-rm/src/data_types/basic/data_value.rs:28-51`,
  `crates/openehr-base/src/base_types/identification/uid.rs` (`Uid`),
  `crates/openehr-rm/src/data_types/quantity/date_time/*` are
  `#[serde(untagged)]` enums.
- **Problem:** `#[serde(untagged)]` tries variants in declaration order and
  accepts the first that deserializes. Correct dispatch relies on each concrete
  variant *rejecting* a wrong payload — which today happens only via the
  `_type` mismatch check. When `_type` is **present** dispatch is correct
  (verified: a `DATA_VALUE` slot holding `{"_type":"DV_TIME","value":"12:00"}`
  is rejected by every earlier variant on `_type` mismatch and lands on
  `DvTime`; a `DV_CODED_TEXT` routes through the nested `DvText` enum
  correctly). But when `_type` is **absent**, structurally-identical variants
  (`DvDate`/`DvTime`/`DvDateTime`/`DvUri`/`DvParsable` all reduce to
  `{value: String}` + optional DV_ORDERED fields; `Uid`'s `InternetId`/`IsoOid`
  are both `{value: String}`) are disambiguated only by declaration order — so
  a `DV_TIME` value with no `_type` silently deserializes as the alphabetically
  earlier `DvDate`. That is **silent type corruption** of the stored node,
  worse than a clean rejection. The schema would reject the same input
  (`_type` required), but `validate_canonical` is not on the ingestion path
  (see F-04-05).
- **Fix:** In the derive, make the `_type` check *mandatory when the value is
  being deserialized as part of an untagged/abstract dispatch*. Practical
  options: (a) emit the untagged enums with a required-`_type` guard (a small
  hand-rolled `Deserialize` that peeks `_type` and dispatches, instead of
  `#[serde(untagged)]` — this also fixes F-04-03); or (b) keep the derive
  lenient for concrete-slot use but have the enum-emitter add a
  `#[serde(deny_unknown_fields)]`-style pre-check; or (c) wire
  `validate_canonical` (which enforces `_type`) into the ingestion path so
  `_type`-less abstract payloads are rejected before decomposition. Option (a)
  is the spec-faithful mechanism (it mirrors the schema's `if _type == … then
  $ref` construction) and should be the emitter change.
- **Resolution (W2-D):** Implemented option (a). The `emit_enum` path in
  `crates/openehr-codegen/src/emit.rs` now emits a hand-rolled `Deserialize` for
  every abstract/polymorphic slot enum, dispatching on `_type` (via a
  `serde_json::Value` buffer + `serde_json::from_value`) using the same
  descendant→direct-variant map the XML runtime uses for `xsi:type`
  (`Model::xsi_dispatch`). It faithfully mirrors the schema's two shapes: an
  **abstract** slot (`self_data == false`: `DATA_VALUE`, `UID`, `VERSION`, …)
  *requires* `_type` and rejects a `_type`-less value; a **concrete
  polymorphic** slot (`self_data == true`: `DV_TEXT`) makes `_type` optional and
  defaults a `_type`-less value to the base type (matching the schema's `if not
  required _type then <base>` arm — this is why `name` DV_TEXT fields without
  `_type` still round-trip). Deep descendants route through the intermediate
  variant, which recurses. `Serialize` keeps `#[serde(untagged)]` (output
  byte-identical). Verified: corpus round-trip gates green + new tests in
  `crates/openehr-rm/tests/type_dispatch.rs` (`_type`-less abstract → error;
  wrong `_type` → error naming it; deep-descendant routing).
- [x] fixed

### F-04-02: Unknown fields are silently dropped on deserialize (lossy), diverging from the schema's `additionalProperties: false`

- **Severity:** minor
- **Spec:** vendored schema
  `crates/openehr-its/schemas/json/openehr_rm_1.1.0_all.json` — every class
  definition carries `additionalProperties: false` (confirmed for all 134
  definitions). ITS-JSON therefore *rejects* unknown keys.
- **Code:** `crates/openehr-derive/src/lib.rs:239-246` — the shadow struct has
  no `#[serde(deny_unknown_fields)]`, so serde ignores unknown keys; the doc
  comment at `lib.rs:12-13` states this is intentional ("Unknown fields are
  ignored (openEHR models evolve additively)").
- **Problem:** An incoming object with an extension/typo key
  (`{"_type":"DV_QUANTITY","magnitude":1.0,"units":"mg","foo":1}`) deserializes
  successfully and re-serializes **without** `foo` — data loss on the
  round-trip. The vendored schema would reject `foo` outright. So our reader is
  simultaneously *more lenient* (accepts non-conformant input) and *lossy*
  (drops the extra data) relative to the ITS-JSON contract. For a CDR that
  persists what it ingests, silently discarding client-supplied keys is a
  correctness/traceability concern.
- **Fix:** Decide the contract explicitly and record it as a `// PORT NOTE:` on
  the derive: either (a) keep leniency but reject-on-ingestion by running
  `validate_canonical` at the REST edge (strict, spec-matching — pairs with the
  F-04-01/F-04-05 fix), or (b) if additive tolerance is deliberately desired,
  document that it is a deliberate superset of the schema and that unknown keys
  are dropped. Do not leave it as an undocumented silent drop.
- **Resolution (W2-D):** Chose option (b) — documented tolerance — recorded as a
  `PORT NOTE` on the shadow struct in `crates/openehr-derive/src/lib.rs` (plus
  the crate-level doc). A blanket `#[serde(deny_unknown_fields)]` was
  implemented and tested first, but rejected because it broke the mandated
  corpus-read gate: the vendored SDK corpus itself ships fixtures with stray
  keys (`feeder_system_audit` placed directly on an `INSTRUCTION`/`ADMIN_ENTRY`,
  which is non-conformant), and the RM 1.2.0 types read an RM 1.1.0-era corpus
  (documented version skew). The PORT NOTE records that unknown keys are
  deliberately ignored as a superset of `additionalProperties: false`, and that
  the strict wire-shape contract remains available via `validate_canonical`
  (F-04-05) at the ingestion edge. The *polymorphic-slot `_type`* requirement —
  the one that caused silent type corruption — is now enforced unconditionally
  (F-04-01), independent of this key-leniency. Test:
  `unknown_keys_are_tolerated_on_deserialize` in `openehr-rm/tests/type_dispatch.rs`.
- [x] fixed

### F-04-03: Untagged-enum deserialization yields opaque "did not match any variant" errors for malformed abstract-slot payloads

- **Severity:** minor
- **Spec:** N/A (diagnostics quality); relevant to the ITS-REST 400/422 error
  bodies the CNF content suites expect
  (`docs/specs/openehr/CNF/docs/platform_test_schedule/` master15–17).
- **Code:** all `#[serde(untagged)]` enums (e.g.
  `crates/openehr-rm/src/data_types/basic/data_value.rs:29`); surfaced at the
  REST edge via `app/ehrbase-rest/src/negotiate.rs:260,308` (errors mapped to
  `ApiError::BadRequest`).
- **Problem:** `#[serde(untagged)]` discards each inner variant's real error
  (e.g. "missing field `units`" on a `DV_QUANTITY`) and reports only "data did
  not match any variant of untagged enum DataValue". A client submitting a
  malformed `DATA_VALUE` gets a 400 with a message that names neither the bad
  field nor the attempted type, degrading the openEHR error-body usefulness.
- **Fix:** Same emitter change as F-04-01 option (a): a hand-emitted
  `Deserialize` that reads `_type` first and dispatches to the one matching
  variant preserves that variant's precise error. (Resolving F-04-01 this way
  resolves F-04-03 for free.)
- **Resolution (W2-D):** Resolved with F-04-01. The dispatcher deserializes the
  one matching variant via `serde_json::from_value`, whose error is mapped
  through `serde::de::Error::custom`, so the real inner error survives. Proven
  by `malformed_variant_surfaces_the_real_inner_error` in
  `openehr-rm/tests/type_dispatch.rs`: a `units`-less `DV_QUANTITY` now yields an
  error that names `units` and is *not* the opaque "did not match any variant".
- [x] fixed

### F-04-04: `DV_COUNT.magnitude` is `i64`; openEHR `Integer` is 32-bit

- **Severity:** info
- **Spec:** vendored schema types `DV_COUNT.magnitude` as `{"type":"integer"}`
  (unbounded JSON integer); openEHR BASE Foundation Types define `Integer` as
  32-bit and ADR-004 fixes the emission `Integer → i32`
  (`docs/ADRs/ADR-004-spec-driven-codegen.md` §3, "Strong typing"). RM spec
  `DV_COUNT.magnitude: Integer` (`docs/specs/openehr/RM/` Data Types,
  Quantity package).
- **Code:** `crates/openehr-rm/src/data_types/quantity/dv_count.rs:43`
  (`pub magnitude: i64`).
- **Problem:** `magnitude` is `i64` while the ADR-004 convention and the 32-bit
  `Integer` type imply `i32`. This is a harmless widening for JSON round-trip
  (JSON has no fixed integer width and the schema is unbounded), but it is an
  inconsistency worth confirming against the BMM input: if the RM 1.2.0 BMM
  actually declares `Integer64` here the `i64` is correct and this is a
  non-issue; otherwise it is an emitter override drift from the `Integer → i32`
  rule.
- **Fix:** Verify the BMM property type for `DV_COUNT.magnitude`. If `Integer`,
  align the emitter so it maps to `i32` like every other `Integer`; if
  `Integer64`, add a one-line note so the divergence is not read as a bug.
- [ ] fixed

### F-04-05: The ITS-JSON schema validator is a test-only gate, not wired into the ingestion path

- **Severity:** info
- **Spec:** `.claude/rules/serialization.md` ("Validate output against
  `openehr_rm_1.1.0_all.json`"); the schema enforces `_type` and
  `additionalProperties:false` (the strictness F-04-01/F-04-02 rely on).
- **Code:** `crates/openehr-its/src/json.rs:60` (`validate_canonical`) is
  referenced only from `crates/openehr-its/tests/fidelity.rs`; the REST
  ingestion path (`app/ehrbase-rest/src/negotiate.rs:173-262`,
  `rm_value`/`json_value`) deserializes with bare `serde_json::from_slice` /
  `from_str` and never calls `validate_canonical`.
- **Problem:** Because incoming bodies are only run through the lenient
  `OpenEhrType` deserializer, the two leniencies above (missing `_type`,
  unknown fields) are *observable at the API surface*: a client can POST a
  composition with a `_type`-less `DATA_VALUE` (mis-typed per F-04-01) or extra
  keys (dropped per F-04-02) and get a 201 rather than a 400/422. Semantic
  validation (P15) covers RM invariants/terminology but is not a substitute for
  the wire-shape (`_type`-present, no-unknown-keys) contract the ITS-JSON
  schema defines.
- **Fix:** This is a design decision to make explicitly, not necessarily a code
  bug: either wire `validate_canonical` (or the targeted `_type`-presence check
  from F-04-01) into the ingestion edge for the strict interpretation, or
  document (PORT NOTE + a phase note) that wire-schema validation is
  deliberately deferred and that the deserializer's leniency is the accepted
  ingestion contract. Cross-check the CNF content suites (master15–17): if any
  case asserts a 400/422 for a `_type`-less or extra-field payload, the strict
  path is required for conformance.
- [ ] fixed

### F-04-06: `validate_canonical` validates RM 1.2.0 output against the RM 1.1.0 schema; the version ceiling is undocumented in the entry point

- **Severity:** info
- **Spec:** `docs/VERSIONS.md` (RM pinned at 1.2.0; ITS-JSON has no numbered
  release / no RM 1.2.0 schema — only 1.0.3/1.0.4/1.1.0 `_all` files are
  vendored); `docs/specs/openehr/ITS-JSON/README.adoc` §"Available components".
- **Code:** `crates/openehr-its/src/json.rs:14`
  (`RM_SCHEMA_JSON = include_str!(".../openehr_rm_1.1.0_all.json")`), hardcoded
  to 1.1.0 though `schemas/json/openehr_rm_1.0.3_all.json` and `1.0.4` are also
  present.
- **Problem:** The generated types are RM 1.2.0 but the only available ITS-JSON
  schema is 1.1.0, so `validate_canonical` necessarily validates 1.2.0 payloads
  against a 1.1.0 contract. The fidelity test (`fidelity.rs:332-338` +
  `excluded()`) *does* document and handle this (RM-1.1-era corpus files that
  omit RM-1.2-mandatory `LOCATABLE` fields are excluded, and the comment states
  the per-class definitions still accept 1.2.0 output). The gap is only that
  the *library entry point* `json.rs` does not state this ceiling — a caller
  reading `validate_canonical`'s doc would assume full 1.2.0 conformance
  checking. This is the same "parity note" flagged in ADR-004/ADR-005 and
  VERSIONS.md.
- **Fix:** Add a doc note on `RM_SCHEMA_JSON`/`validate_canonical` recording
  that it is the RM 1.1.0 contract (no 1.2.0 schema published) and therefore
  cannot catch 1.2.0-only structural constraints — so the gate proves 1.1.0
  compatibility, not full 1.2.0 conformance. No behavioural change.
- [ ] fixed

## Hygiene notes

- **Serialize side is clean and spec-conformant** — verified against
  `.claude/rules/serialization.md` and the vendored schema:
  - `_type` emitted first via `serialize_map` (`openehr-derive/src/lib.rs:230`),
    uppercase RM class names from `#[openehr(type_name=...)]`.
  - `None` omitted, empty `Vec` omitted, no `null`/`[]` emitted
    (`lib.rs:126-144`) — matches the "nulls omitted entirely" rule.
  - UIDs (`OBJECT_VERSION_ID`, `HIER_OBJECT_ID`) serialize as `{_type, value}`,
    never bare strings (`openehr-base/.../object_version_id.rs`,
    `hier_object_id.rs`).
  - `DV_MULTIMEDIA.data` is `Option<String>` holding inline base64
    (`dv_multimedia.rs:*`) — matches "inline base64, not a reference".
  - Integer vs Real is carried by the field types (`DvQuantity.magnitude: f64`,
    `DvCount.magnitude: i64`, `DvProportion.numerator/denominator: f64`,
    `DvProportion.type: i32`); the fidelity `semantic_eq` correctly treats
    `5` and `5.0` as equal magnitudes (`fidelity.rs:131-132`).
  - Every one of the 134 schema definitions lists `_type` as an allowed
    property, so the always-emit-`_type` policy never trips
    `additionalProperties:false`.
  - `Interval` inclusivity/boundedness flags are re-materialized at canonical
    defaults via `#[openehr(default=...)]` (`dv_interval.rs`), and the fidelity
    gate treats those as information-preserving (`fidelity.rs:147-151`).
- **`serde_json` `preserve_order` is enabled** workspace-wide
  (`Cargo.toml:120`), so `serde_json::Value` round-trips and
  `validate_canonical` are order-stable. Note the custom `Serialize` emits keys
  in struct order regardless, so serialization order does not depend on this
  feature; it matters only for `Value`-based paths (`respond_rm`, schema
  validation).
- **Untagged-enum ordering** currently disambiguates derived-before-base by
  alphabetical variant naming (e.g. `DvCodedText` before `DvText`,
  `PartyRelated` before `PartyIdentified`), which happens to place the
  more-constrained subtype first. This is *incidental*, not enforced; if the
  F-04-01 fix keeps `#[serde(untagged)]` rather than an explicit `_type`
  dispatcher, add an emitter invariant that orders most-derived variants first
  so the missing-`_type` structural fallback cannot regress.
- **Generated-file discipline respected:** all serde-shape issues here are
  fixable in the emitter (`openehr-codegen`) or the `OpenEhrType` derive, never
  by hand-editing the `// @generated` type files.
- **Fidelity gate is strong** (readability + lossless round-trip + schema
  validation over the corpus, `fidelity.rs`); its exclusion lists are
  individually justified with spec reasons and do not mask defects. Keep it as
  the acceptance instrument; consider adding a *negative* case proving a
  `_type`-less abstract-slot payload is rejected once F-04-01 is fixed.
