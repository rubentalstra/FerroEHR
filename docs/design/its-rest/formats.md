# Formats — Simplified Formats (FLAT / STRUCTURED) — compliance design (W-3e)

The ITS-REST **Simplified Formats** specification (the flat and structured
Web-Template JSON serializations) was, until Release 1.1.0 (Nov 2025),
carried as the *Simplified Data Template (SDT)* spec and treated in this
codebase as **DEVELOPMENT** — so `serialization.md` set the implementation
oracle to *Better's `web-template` semantics* ("Target Better's `web-template`
semantics as the primary oracle, since SDT … is still development"). That is no
longer true. SDT is **retired** and its content is **consolidated and promoted
to STABLE** as the Simplified Formats spec (SPECITS-61, 1.1.0; SPECITS-94
tightened it to 1.1.1, 28 Apr 2026). This document re-bases the FLAT/STRUCTURED
implementation onto the now-STABLE spec and is, per blueprint §2.3 row 17, the
**spec side of the deferred SIM-B/SDF transformation-rule audit** (P17).

**Spec oracle** (read before any change):

- `docs/specs/openehr/ITS-REST/docs/simplified_formats/master02-overview.adoc`
  — MIME types, field-identifier syntax, node-ID generation rules, instance
  indexing, attribute suffixes, the `_`-prefix RM-attribute rule, `|raw`, `ctx`,
  the Flat/Structured variants + syntax rules, level removal (attribute elision,
  always-collapsed wrapper types, conditional `EVENT` collapse), `|other` open
  value-sets, validation.
- `master03-design_rationale.adoc` — canonical vs simplified, historical
  lineage (TDS/ECISFLAT/Better WT/EHRbase SDT), the five viability requirements,
  the worked Flat + Structured blood-pressure example.
- `master04-basic_concepts.adoc` — (folded into master02 in this vendored
  build; the WT-metadata example + field-identifier detail).
- `master05-rm_mapping.adoc` — the class-by-class leaf-encoding tables
  (COMPOSITION, the ENTRY types, ELEMENT/CLUSTER, the `DV_*` data types,
  PARTY_*, PARTICIPATION, FEEDER_AUDIT, LINK, reference ranges, …) — the
  normative source for every `|suffix` and `/_attr` a conformant server must
  round-trip.
- `master06-context_information.adoc` — the full `ctx/` vocabulary and its
  RM targets/defaults.
- `master00-amendment_record.adoc` / `master01-preface.adoc` — maturity +
  conformance statement ("the available calls … are the same as for other
  openEHR serialisation formats … with a different representation format
  indicated by setting the appropriate `Content-Type`").
- `docs/specs/openehr/ITS-REST/docs/simplified_data_template/master01-preface.adoc`
  — the retirement notice ("as of Release 1.1.0 is retired in favour of the
  Simplified Formats specification"); the `simplified_data_template/` folder is
  now a stub preface only, **not** masters 02–06.

**Current implementation** (verified 2026-07-12):

- Converters: `crates/openehr-flat/src/flat/` — `to_flat.rs` (81 lines,
  RM→FLAT walk), `from_flat.rs` (526, FLAT→RM builder), `mappers.rs` (509,
  per-`DATA_VALUE` leaf encoding both directions), `context.rs` (480, the
  `ctx/` vocabulary), `graph.rs` (89, structural-node re-materialisation),
  `sub.rs` (115, key parsing + `FlatView`), `defaults.rs` (25, RM defaults).
- STRUCTURED: `crates/openehr-flat/src/structured/mod.rs` (342) — a pure,
  WebTemplate-independent nesting transform composed over the FLAT converter.
- Node-ID / WebTemplate: `crates/openehr-flat/src/webtemplate/id.rs` (268,
  json-id derivation + dedup), `builder.rs` (1214, OPT→WebTemplate + the
  compactor that performs level removal), `model.rs`, `inputs.rs`.
- Example generator + TDD: `example.rs` (715), `tdd.rs` (815).
- Validation: `crates/openehr-flat/src/validation/` (leaf.rs 821, mod.rs 948,
  terminology.rs, subtype.rs).
- Public API: `crates/openehr-flat/src/lib.rs` — `to_flat`/`from_flat`,
  `to_structured`/`from_structured`, `flat_to_structured`/`structured_to_flat`,
  `build_web_template`, `example_composition`, `from_tdd`, `validate_*`.
- Wire: `app/ehrbase-rest/src/overview/negotiate.rs:43-127` defines the media
  types (`application/openehr.wt+json`, `…wt.flat+json`, `…wt.structured+json`)
  and the `wants_*`/`is_*_body` selectors + `flat_json_body`/
  `structured_json_body` responders. FLAT/STRUCTURED composition I/O is wired
  at `app/ehrbase-rest/src/dispatch/ehr.rs:377-380,443-446` (create/update
  body) and `:899-905` (read), delegating to
  `app/ehrbase-rest/src/dispatch/flat.rs:62-152`; the template-example
  endpoint at `app/ehrbase-rest/src/dispatch/definition.rs:145-152`.
- Feature flag: `crates/openehr-flat/Cargo.toml:30-31` `ehrbase-quirks = []`.
- ECC: no dedicated Formats area; FLAT/STRUCTURED exercised only by
  `app/ehrbase-rest/tests/flat_http.rs` + `example_http.rs`.

The bar this document sets: our converters target Better semantics faithfully,
but Better is now **prior art, not the oracle** (ADR-008 discipline). Where the
STABLE Simplified Formats spec defines a suffix/attribute/rule that Better omits
or spells differently, the spec wins — the gaps below are where the two diverge.

---

## 1. Gap register (what is not spec-true today)

Every gap cites the governing spec text and the implementing file:line. The
recurring root cause: the converters implement Better's **data-entry** subset
(the leaves a form fills), whereas the STABLE spec's mapping tables also
mandate the **metadata / audit / accuracy / reference-range** leaves — chiefly
the whole `_`-prefixed optional-RM-attribute family — for lossless round-trip.

| # | Gap | Spec citation | Today |
|---|-----|---------------|-------|
| G-1 | **The entire `_`-prefixed optional-RM-attribute family is dropped — neither emitted nor parsed.** Every ENTRY table lists `/_uid`, `/_link:i`, `/_feeder_audit` (COMPOSITION/OBSERVATION/EVALUATION/INSTRUCTION/ACTION/ADMIN_ENTRY/CLUSTER); ELEMENT adds `/_null_flavour`, `/_null_reason`; DV_TEXT/DV_CODED_TEXT add `/_language`, `/_encoding`, `/_mapping:i`; every `DV_ORDERED` adds `/_normal_range`, `/_other_reference_ranges:i`; PARTY_* add `/_identifier:i`; INSTRUCTION/ACTION/OBSERVATION add `/_guideline_id`, `/_work_flow_id`, `/_other_participation:i`, `/_provider`. A composition carrying any of these does **not** FLAT-round-trip. | master02 §"RM Attributes prefix"; master05 per-class tables (COMPOSITION `/_link:i`,`/_feeder_audit`,`/_uid`; ELEMENT `/_null_flavour`,`/_null_reason`; DV_TEXT `/_language`,`/_encoding`,`/_mapping:i`; DV_QUANTITY `/_normal_range`,`/_other_reference_ranges:i`; PARTY_IDENTIFIED `/_identifier:i`) | Not surfaced on write (`flat/mappers.rs:165-174` note; `flat/mod.rs:25` lists `uid`/`normal_range`/`other_reference_ranges`/`mappings` as dropped) and **no `_`-prefix branch exists** in `flat/from_flat.rs` / `flat/sub.rs` (parse ignores it). The only `_`-family attributes handled at all are the `ctx/_*` context ones via `context.rs`. |
| G-2 | **`|raw` canonical-JSON embed mechanism unimplemented.** The spec makes `<path>\|raw: { "_type": …, … }` a first-class bypass to embed pre-serialized canonical JSON for complex/pre-existing RM structures. | master02 §"Raw canonical JSON" ("`\|raw` … enables direct embedding of pre-serialized openEHR canonical JSON … must include the `_type` property") | Zero handling anywhere in `crates/openehr-flat` (grep `raw` → only doc-comment matches). A `\|raw` key routes to nothing on write and is never produced on read. |
| G-3 | **`DV_QUANTITY` / `DV_COUNT` / `DV_PROPORTION` / `DV_DURATION` secondary attributes dropped.** Spec tables define `\|accuracy`, `\|accuracy_is_percent`, `\|normal_status` (all four), `\|precision` (QUANTITY/PROPORTION), the computed magnitude + `\|magnitude_status` (PROPORTION), and `\|units_system`/`\|units_display_name` (QUANTITY). | master05 §§DV_QUANTITY, DV_COUNT, DV_PROPORTION, DV_DURATION | RM→FLAT (`flat/mappers.rs:96-124,136-138`) emits QUANTITY magnitude/unit/precision/magnitude_status (units_system/display only behind `ehrbase-quirks`, `:109-113`), COUNT magnitude+magnitude_status, PROPORTION numerator/denominator/type, DURATION bare value only. `accuracy`/`accuracy_is_percent`/`normal_status` never emitted; PROPORTION precision/magnitude_status/computed-magnitude never emitted. FLAT→RM (`quantity_from_flat:382-409`, `count_from_flat:412-421`, `proportion_from_flat:423-435`) reads the same reduced set. |
| G-4 | **The date/time `DV_*` family loses `/_accuracy`, `\|magnitude_status`, `\|normal_status`, and reference ranges.** | master05 §§DV_DATE, DV_DATE_TIME, DV_TIME (`/_accuracy` as DV_DURATION; `\|magnitude_status`; `\|normal_status`; `/_normal_range`; `/_other_reference_ranges:i`) | `flat/mappers.rs:136-138` treats `DV_DATE_TIME`/`DV_DATE`/`DV_TIME`/`DV_DURATION`/`DV_URI`/`DV_EHR_URI` as a bare `value` only; `bare_typed` (`:506-509`) reconstructs value only. |
| G-5 | **`DV_MULTIMEDIA` reduced to `uri`/`\|mediatype`/`\|alternatetext`/`\|size`.** The spec also defines `\|compression_algorithm`, `\|integrity_check`, `\|integrity_check_algorithm`, `\|data` (inline base64), and the `/_thumbnail`, `/_charset`, `/_language` sub-paths. | master05 §DV_MULTIMEDIA | `flat/mappers.rs:150-164` emits uri/mediatype/alternatetext/size (inline `data` explicitly not surfaced); `multimedia_from_flat:469-504` reads uri/mediatype/alternatetext/size. |
| G-6 | **`DV_TEXT`/`DV_CODED_TEXT` `_language`/`_encoding`/`_mapping:i` and `DV_PARSABLE` `_charset`/`_language` dropped.** | master05 §§DV_TEXT, DV_CODED_TEXT, DV_PARSABLE, TERM_MAPPING, CODE_PHRASE | `flat/mappers.rs:60-95` keeps only `value`/`formatting`/`code`/`terminology`/`preferred_term`; `parsable_from_flat:267-274` keeps value+formalism. The `_`-sub-paths fall under G-1. |
| G-7 | **`\|other` open-value-set discriminator: the two spec MUST-rejects are unenforced, and read-side emission ignores `listOpen`.** Spec: `\|other` MUST be rejected on a closed list (`listOpen: false`); `\|other` MUST NOT be combined with `\|code`/`\|value`/`\|terminology`/`\|preferred_term`; on read, emit `\|other` only when the DV_TEXT sits in an **open** coded slot. | master02 §"Open Value-Sets and the `\|other` Suffix"; master05 §"When a `DV_CODED_TEXT` becomes a `DV_TEXT`" | `\|other` is emitted whenever a `DV_TEXT` value occupies any `DV_CODED_TEXT` slot (`flat/mappers.rs:60-65`, no `listOpen` check) and consumed on write (`:242-243`), but no closed-list rejection and no `\|other`+`\|code` mutual-exclusion check is present in `validation/` (grep found none). |
| G-8 | **The `ctx/` vocabulary is a narrow round-trip subset on output.** master06 defines `work_flow_id`, `provider_*`, `history_origin`, `activity_timing`, `action_time`, `action_ism_transition_current_state`, `instruction_narrative`, `participation_identifiers`, `id_scheme`, `link`. `emit_ctx` produces only language/territory/composer/time/end_time/setting/location/health_care_facility + participation `name`/`function`/`mode`/`id`. The rest are input-only (`apply_ctx`) and never re-emitted; `participation_identifiers`, `id_scheme`, and `ctx/link` are not emitted at all. | master06 §§Workflow ID, provider, participation (`participation_identifiers`), link, action_*, activity_timing, history_origin, instruction_narrative | `flat/context.rs:39-125` (`emit_ctx`) vs `:137-397` (`apply_ctx`) — asymmetric. Input side handles most keys; output side omits them. |
| G-9 | **Node-ID duplicate-suffix form diverges from the STABLE spec's stated rule.** The spec's worked example maps a duplicate "Blood Pressure" to **`blood_pressure_1`** (underscore separator, from `_1`). Our dedup (Better `NumericSuffixIdDeduplicator`) yields **`blood_pressure2`** (no separator, from `2`). | master02 §"Node ID Generation Rules" (table: "Blood Pressure (duplicate) → blood_pressure_1") | `webtemplate/id.rs:31-45` (`Deduplicator::unique`) starts at `2` and concatenates with no underscore. The sanitize algorithm itself (`id.rs:50-74`) matches the spec's seven rules. |
| G-10 | **`PARTICIPATION.time` interval and `PARTY_RELATED` performer `relationship` not surfaced; wire-level `_participation:i`/`_other_participation:i` absent.** | master05 §PARTICIPATION (`time` note; `/relationship` for a `PARTY_RELATED` performer) + §EVENT_CONTEXT (`/_participation:i`) | `context.rs` participation emit/build drops `identifiers`, `relationship`, and `time`; the non-context `_other_participation:i` / `_participation:i` wire leaves fall under G-1. |
| G-11 | **`ctx/time` default is epoch, not `now()`.** The spec: "`ctx/time` will be set to `now()` if not set explicitly." The reverse converter fabricates `1970-01-01T00:00:00Z` for the RM-mandatory temporal fields when the FLAT omits them. | master06 §time ("`ctx/time` will be set to `now()`"); master02 §Context ("defaults to the current server time (`now()`)") | `flat/defaults.rs:20` `DEFAULT_TIME = "1970-01-01T00:00:00Z"`, used in `context.rs:70,198` and `graph.rs`. |
| G-12 | **The terse coded form `terminology::code\|value\|` is accepted but is not defined by the STABLE Simplified Formats spec** — it is an SM/SIM-B acceptance carried over. Accepting undefined input is a fail-open divergence to record (not necessarily to remove). | master05 defines only the suffixed `\|code`/`\|value`/`\|terminology` form; the terse form is SM `simplified_im_b` master04 `S_DV_CODED_TEXT`, outside this spec | `flat/mappers.rs:328-366` (`coded_parts`/`parse_terse_coded`) parses the terse form when no `\|code` suffix is present. |
| G-13 | **No dedicated conformance evidence for the format.** The spec's conformance statement says the *same* calls are tested with the format selected by `Content-Type`; there is no Formats ECC area asserting the mapping tables. | master01 §Conformance | Only `app/ehrbase-rest/tests/flat_http.rs` + `example_http.rs` smoke-test the endpoints; no per-`DV_*` mapping assertions, no ECC area, no round-trip corpus keyed to master05. |

### What is already spec-true (not re-litigated)

Verified aligned with the STABLE spec, kept as-is:

- **MIME types** — both `application/openehr.wt.flat+json` and
  `…wt.structured+json` (master02 §MIME Types) are recognised on `Accept` and
  `Content-Type` and set on responses (`overview/negotiate.rs:48-50,64-127`).
- **Node-ID sanitisation** — the seven normalisation rules (char→`_`, collapse,
  lowercase, trim, empty→`id`, digit→`a`-prefix, sibling uniqueness) match
  master02 §"Node ID Generation Rules" (`webtemplate/id.rs:50-120`); only the
  suffix *spelling* diverges (G-9).
- **Level removal** — container-attribute elision + always-collapsed wrappers
  (`ITEM_TREE`/`ITEM_LIST`/`ITEM_SINGLE`/`ITEM_TABLE`/`HISTORY`) + the
  conditional single-`EVENT` collapse match master02 §"Level Removal"
  (`webtemplate/builder.rs:39-62` `ALWAYS`/`SINGLE_COMPACTABLE`, compaction at
  `:432-495`; re-materialisation in `flat/graph.rs`).
- **Flat ⇄ Structured** — the pure, WebTemplate-independent nesting transform
  (arrays throughout, `|suffix` keys, `ctx` object, `:index` from array
  position) matches master02 §§Structured format / Conversion Between Formats
  (`structured/mod.rs`).
- **`|unit` singular, `|preferred_term`, `|formatting`, `|scale`/`|ordinal`,
  `composer_self`/`composer_name`/`composer_id`, setting default
  "other care"** — all match the 1.1.1 tables + master06 (`flat/mappers.rs`,
  `flat/context.rs`, `flat/defaults.rs:24`).

---

## 2. Target design

The work is localised to `crates/openehr-flat` (converters + validation) plus a
new ECC area; no new REST routes are needed (the format is selected by
`Content-Type`/`Accept`, already wired). Every change is spec-cited; Better
stays a cross-check, not the authority.

### 2.1 Complete the leaf mapping tables (`flat/mappers.rs`) — G-3..G-6

Extend `leaf_to_flat` / `build_for` to the full master05 attribute set per type,
driven by a small per-type suffix table so RM→FLAT and FLAT→RM cannot drift:

- **`DV_QUANTITY`**: add `|accuracy`, `|accuracy_is_percent`, `|normal_status`
  (both directions). Keep `|units_system`/`|units_display_name` behind
  `ehrbase-quirks` (correct — no STABLE-spec suffix exists; the RM fields stay
  first-class in canonical JSON/XML). Cite master05 §DV_QUANTITY.
- **`DV_COUNT`**: add `|accuracy`, `|accuracy_is_percent`, `|normal_status`.
- **`DV_PROPORTION`**: add `|precision`, `|accuracy`, `|accuracy_is_percent`,
  `|magnitude_status`, `|normal_status`, and the computed bare `magnitude` on
  output (spec: "calculated on output").
- **`DV_DURATION`**: add `|accuracy`, `|accuracy_is_percent`,
  `|magnitude_status`, `|normal_status`.
- **date/time family** (`DV_DATE`/`DV_DATE_TIME`/`DV_TIME`): add `/_accuracy`
  (DV_DURATION), `|magnitude_status`, `|normal_status` (G-4).
- **`DV_MULTIMEDIA`**: add `|compression_algorithm`, `|integrity_check`,
  `|integrity_check_algorithm`, the inline `|data` (base64), and the
  `/_thumbnail` (recursive DV_MULTIMEDIA), `/_charset`, `/_language`
  sub-paths (G-5).
- **`DV_TEXT`/`DV_CODED_TEXT`**: add `/_language`, `/_encoding`, `/_mapping:i`
  (TERM_MAPPING: `|match`, `/target` CODE_PHRASE, `/purpose` DV_CODED_TEXT);
  `DV_PARSABLE`: add `/_charset`, `/_language` (G-6).
- **reference ranges** (`/_normal_range` = DV_INTERVAL<T>, `/_other_reference_
  ranges:i` = REFERENCE_RANGE<T>): a shared emitter/parser parameterised on the
  leaf's `T`, with `|lower_unbounded`/`|upper_unbounded`/`|lower_included`/
  `|upper_included` and the `/meaning` DV_TEXT (master05 §§DV_INTERVAL,
  REFERENCE_RANGE). This is a sub-case of the `_`-family (§2.3).

### 2.2 The `_`-prefixed RM-attribute family (`flat/mappers.rs`, `flat/from_flat.rs`, `flat/sub.rs`) — G-1

The largest gap. Design a **generic `_`-attribute layer** that sits above the
leaf mappers, symmetric in both directions:

- **RM→FLAT**: after a node's data-entry leaves are emitted, walk its RM value
  for the optional attributes the template did not surface and emit them under
  `<path>/_attr` (or `<path>|attr` for the LINK/OBJECT_REF/DV_IDENTIFIER
  scalar-suffix cases), per the master05 tables: `_uid`, `_link:i` (LINK:
  `|type`/`|meaning`/`|target`), `_feeder_audit` (FEEDER_AUDIT tree),
  `_null_flavour`/`_null_reason` (on ELEMENT), `_identifier:i` (PARTY),
  `_guideline_id`/`_work_flow_id`/`_instruction_details`/`_wf_definition`,
  `_other_participation:i`, `_provider`, and the reference ranges from §2.1.
- **FLAT→RM**: `sub.rs`/`from_flat.rs` gain a `_`-prefix recognition path — a
  key segment starting `_` addresses an optional RM attribute of the *current*
  node rather than a template child; it is rebuilt via a dedicated
  `rm_attr_from_flat` dispatch (LINK, FEEDER_AUDIT, DV_IDENTIFIER, CODE_PHRASE,
  the reference-range builder, a raw string for `_uid`). This is a new routing
  branch, not a leaf mapper.
- **Multiplicity** for `_link:i`/`_identifier:i`/`_other_participation:i` reuses
  the instance-index machinery; the `is_multiple` hard-coded set
  (`from_flat.rs:46-48`) should ultimately be driven by the P16 BMM RM model
  (already TODO-flagged, `from_flat.rs:39-45`).

*No openEHR spec governs the internal dispatch shape — our own design; the wire
shape (`<path>/_attr…`) is fixed by master02/master05.*

### 2.3 `|raw` embed (`flat/from_flat.rs`, `flat/to_flat.rs`) — G-2

- **write**: a `<path>|raw` key carries a canonical-JSON object (with `_type`);
  the converter routes it to the target node and inserts the object verbatim as
  that node's RM value, bypassing leaf decomposition (validating only that
  `_type` is present and the type is admissible at that slot).
- **read**: `|raw` is a client-authored escape hatch; the server is not required
  to *produce* it. Keep RM→FLAT decomposing; document that `|raw` is
  write-only for us (a spec-permitted choice — the spec mandates acceptance,
  not emission). Cite master02 §"Raw canonical JSON".

### 2.4 `|other` MUST-rules (`flat/validation/`, `flat/mappers.rs`) — G-7

- reject `<path>|other` when the leaf's WT constraint is a **closed** coded list
  (`listOpen: false`) → a typed FLAT validation error;
- reject `<path>|other` co-occurring with `|code`/`|value`/`|terminology`/
  `|preferred_term` on the same leaf;
- on RM→FLAT, gate the `DV_TEXT`-in-coded-slot → `|other` emission
  (`mappers.rs:60-65`) on the slot actually being `listOpen: true`; a `DV_TEXT`
  in a closed slot is a data defect, not an `|other`. Cite master02
  §"Open Value-Sets", master05 §"When a `DV_CODED_TEXT` becomes a `DV_TEXT`".

### 2.5 Symmetric `ctx/` output + `now()` default (`flat/context.rs`, `flat/defaults.rs`) — G-8, G-11

- extend `emit_ctx` to the master06 keys that have an unambiguous output home:
  `id_scheme`, `participation_identifiers`, `ctx/link:i`, and the
  `PARTY_RELATED` participation `relationship`. The genuinely
  per-entry-ambiguous input defaults (`provider`, `work_flow_id`,
  `history_origin`, `activity_timing`, `instruction_narrative`,
  `action_ism_transition_current_state`) stay input-only, but that asymmetry is
  documented as a spec-permitted producer choice (master06 frames them as input
  shortcuts) rather than silent.
- `ctx/time` unset → `now()` at conversion (thread a `now` instant in, replacing
  the epoch `DEFAULT_TIME` for the *context* start-time; the structural
  `HISTORY.origin`/`EVENT.time` fallbacks can still derive from it). Cite
  master06 §time.

### 2.6 Node-ID suffix (`webtemplate/id.rs`) — G-9

Reconcile the dedup spelling with master02. The spec shows `blood_pressure_1`;
Better shows `blood_pressure2`. Two honest options, decide with a CNF/corpus
cross-check:

1. **follow the STABLE spec** (underscore + from `_1`) and gate the Better
   spelling behind `ehrbase-quirks` — spec-first, but breaks parity with any
   Better-generated WT a client already holds; or
2. **keep Better's spelling as the default** and record a cited PORT NOTE that
   the STABLE spec's example is illustrative and interop tooling universally
   emits the Better form (SPECITS-94 did not touch the dedup form).

Recommendation: (2) with the PORT NOTE, unless a CNF fixture asserts the
underscore form — WebTemplate json-ids are a shared contract with existing
Better/EHRbase clients and a silent change would break stored form definitions.

### 2.7 Conformance (`tools/conformance`) — G-13

A **Formats ECC area** that drives the same EHR/COMPOSITION calls with
`Content-Type: application/openehr.wt.flat+json` / `…structured+json`, plus a
**mapping-table round-trip corpus**: for each master05 `DV_*` example, assert
canonical-JSON → FLAT → canonical-JSON stability (value-equality) and the exact
`|suffix`/`/_attr` set the spec table lists. The master03 blood-pressure Flat +
Structured examples land verbatim as golden fixtures. Zero `skipped` outcomes
(W-2 ruling) — the endpoints are executable.

---

## 3. Work plan (execution order under W-3e)

1. **Leaf-table completion** (§2.1): per-type suffix tables in `mappers.rs`,
   both directions; `insta` golden vectors from the master05 examples. (G-3,
   G-4, G-5, G-6)
2. **Reference-range emitter/parser** (§2.1/§2.3): shared `DV_INTERVAL<T>` /
   `REFERENCE_RANGE<T>` layer — the first `_`-attribute consumer.
3. **The `_`-attribute layer** (§2.2): routing in `sub.rs`/`from_flat.rs` +
   `rm_attr_from_flat`; LINK/FEEDER_AUDIT/`_uid`/`_null_*`/`_identifier`/
   participation/OBJECT_REF; round-trip tests from the master05 full examples.
   (G-1, G-10)
4. **`|raw`** (§2.3): write-path embed + validation. (G-2)
5. **`|other` MUST-rules** (§2.4): closed-list + mutual-exclusion rejection;
   `listOpen`-gated emission. (G-7)
6. **`ctx/` symmetry + `now()`** (§2.5): `emit_ctx` extension, `now` threading.
   (G-8, G-11)
7. **Node-ID suffix decision** (§2.6): CNF/corpus check, then follow-spec-or-
   PORT-NOTE. (G-9)
8. **Formats ECC area + mapping corpus** (§2.7); record G-12 (terse coded form)
   as an accepted extension. Close every G-row (code or re-verified cited PORT
   NOTE). (G-13, G-12)

Exit: every G-row closed; `to_flat`/`from_flat`/`to_structured`/
`from_structured` round-trip the master05 tables + the master03 examples;
workspace suites + clippy green; ECC zero-drift with the new Formats area; the
website book FLAT/STRUCTURED page updated (same-PR docs rule).

---

## 4. Standing PORT NOTEs after the work (the honest residue)

- **`|units_system`/`|units_display_name`** stay `ehrbase-quirks`-gated — the
  STABLE spec defines no such suffix; the RM 1.2.0 fields remain first-class in
  canonical JSON/XML (`mappers.rs:100-113` PORT NOTE already stands).
- **Terse coded form `terminology::code|value|`** (G-12) is accepted as an
  SM/SIM-B-derived **extension** beyond the STABLE Simplified Formats spec —
  fail-open input tolerance, recorded, not removed.
- **`|raw` is write-only** for us (the spec mandates acceptance, not emission);
  RM→FLAT always decomposes.
- **Node-ID dedup spelling** (G-9) — if we keep Better's `2`/no-underscore form,
  a cited PORT NOTE records that the STABLE spec's `blood_pressure_1` is an
  illustrative example and interop tooling emits the Better spelling; the WT
  json-id is a shared contract with existing clients.
- **Per-entry `ctx/` input defaults** (`provider`, `work_flow_id`,
  `history_origin`, `activity_timing`, `instruction_narrative`,
  `action_ism_transition_current_state`) remain input-only, not re-emitted on
  output — a spec-permitted producer choice (master06 frames them as input
  shortcuts), documented rather than silent.
- **`DV_TEXT._formatting`** is kept on FLAT for round-trip fidelity even though
  the SM transformation table marks it *skip* (`mappers.rs:71-78` PORT NOTE
  stands) — it is optional, so canonical validity is unaffected.
- **`is_multiple` hard-coded attribute set** (`from_flat.rs:46-48`) stays a
  TODO(port) until the P16 BMM RM attribute model lands to drive multiplicity;
  the `_link:i`/`_identifier:i`/`_other_participation:i` arrays from §2.2 must be
  added to it in the interim.
