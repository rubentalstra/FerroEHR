# Feature — Simplified Formats (FLAT + STRUCTURED), spec-exact greenfield rewrite

- Status: **not started** (this document is the approved design; execution
  follows the task list below)
- Owner rulings baked in: full fresh rewrite (no quick fixes, no wrappers,
  no legacy compatibility layers); **only the official openEHR specs are
  referenced** — no vendor semantics as an oracle; Accept/Content-Type
  negotiation only (no `?format=` query parameter — the spec defines none).
- Resolves: [issue #95](https://github.com/rubentalstra/ehrbase-rs/issues/95)
  (format support via Accept header). Adjacent: [issue #94](https://github.com/rubentalstra/ehrbase-rs/issues/94)
  (example generator completeness — separate item; this feature only fixes
  the example endpoint's *format* surface).

## 1. The spec oracle and its authority chain

All normative text is vendored under `docs/specs/openehr/`. Ranked:

1. **ITS-REST Simplified Formats** — `ITS-REST/docs/simplified_formats/`
   (`master02-overview` … `master06-context_information`). **STABLE.**
   This is *the* wire-format authority: path syntax, node-id generation,
   per-RM-type mapping tables, `ctx/` vocabulary, level removal, `|other`,
   `|raw`, FLAT↔STRUCTURED algorithms, MIME types.
2. **ITS-REST OpenAPI** (same pinned commit) —
   `ITS-REST/specifications/…` (`docs/overview/Resources.md` §Simplified
   Formats, `headers/ContentType_LOCATABLE.yaml`,
   `parameters/header/Accept_LOCATABLE.yaml`, the per-operation files).
   Defines *where* the formats apply, the negotiation/status-code rules,
   and the `openehr-template-id` header requirement.
3. **SM SIM-B** (`SM/docs/simplified_im_b/`) and **SDF**
   (`SM/docs/serial_data_formats/`) — **DEVELOPMENT**, with open TBDs.
   SIM-B's transformation rules (`master07-transformation_rules.adoc`)
   formalize and *agree with* the ITS-REST level-removal and attribute
   inlining; they are useful as the model-level cross-check. SDF's terse
   single-string leaf encodings (`"78.500,kg"`, ODIN terminology-code
   strings, ODIN interval strings) **conflict** with the STABLE ITS-REST
   suffix encoding and are **not implemented**.
4. **SDT** (`ITS-REST/docs/simplified_data_template/`) — **RETIRED**
   (`master01-preface.adoc §Status`: retired as of Release 1.1.0 in favour
   of the Simplified Formats specification). Historical only.

Rule: where 1 and 2 speak, they win. Where they are silent, flag the
decision explicitly (`no openEHR spec governs this — our own design`) —
never fill the gap from another CDR's behaviour.

## 2. Why a rewrite

The existing `crates/openehr-flat` was authored against Better's
`web-template` semantics as the reference, before the Simplified Formats
specification reached STABLE, and carries that lineage: a vendor-quirk
feature gate (`ehrbase-quirks`), vendor-fixture-driven tests as the primary
oracle, and duplicated FLAT and STRUCTURED conversion paths. The REST
surface wires FLAT/STRUCTURED only into COMPOSITION I/O and the template
example endpoint, while the vendored OpenAPI declares the media types much
more broadly. The rewrite re-authors the layer **from the spec text**, with
spec examples as the primary test vectors, one conversion core, the full
endpoint matrix, and no vendor gates. Existing corpus tests are retained
as *regression* vectors only (they exercise real OPTs; they are not an
oracle).

## 3. The binding wire contract (from the vendored spec)

### 3.1 Media types

`simplified_formats/master02-overview.adoc §MIME Types` +
`specifications/docs/overview/Resources.md §Simplified Formats`:

| Media type | Meaning |
|---|---|
| `application/openehr.wt.flat+json` | Simplified FLAT JSON data instance |
| `application/openehr.wt.structured+json` | Simplified STRUCTURED JSON data instance |
| `application/openehr.wt+json` | Operational Template rendered as Web Template JSON (template resource only) |

Explicitly **not** implemented:

- `application/openehr.wt.flat.schema+json` / `…structured.schema+json` —
  deprecated by `Resources.md §Simplified Formats` (NOTE) and
  `§Alternative data formats`; requests naming them get `406`/`415` like
  any other unsupported type.
- `application/openehr.nc.flat+json` (ECISFLAT) and
  `application/openehr.tds2+xml` — listed under `Resources.md
  §Alternative data formats` as legacy/experimental. No EhrScape surface
  is built; that legacy API family is out of this product's scope.

### 3.2 Negotiation rules (`Resources.md §Simplified Formats`)

- Content negotiation works exactly as for canonical JSON/XML, with the
  media types above.
- Unsupported request `Content-Type` → **415 Unsupported Media Type**
  (MUST). Unfulfillable `Accept` → **406 Not Acceptable** (MUST).
- Every non-`204` response carries a correct `Content-Type` (MUST).
- There is **no format query parameter** anywhere in the spec; `?format=`
  stays unrecognized (documented in the book).
- `Requests_and_responses.md §openehr-template-id`: the
  `openehr-template-id` request header **MUST** be used when committing a
  COMPOSITION via `PUT`/`POST` in a simplified format (the payload carries
  no `archetype_details.template_id`). Missing header on a simplified
  commit → `422` with a diagnostic naming the header.

### 3.3 Endpoint matrix (from the per-operation OAS files)

Simplified data-instance media types (`…wt.flat+json`,
`…wt.structured+json`) appear in `Accept_LOCATABLE.yaml` /
`ContentType_LOCATABLE.yaml`, referenced by:

| Surface | Ops (OAS files) | Posture |
|---|---|---|
| COMPOSITION | `composition_create/get/update` | **Full FLAT/STRUCTURED I/O** — the mapping tables govern COMPOSITION content |
| Template example | `definition_template_adl1.4_example_get`, `definition_template_adl2_example_get` | **Full output support** (generate canonical RM example → serialize per Accept) |
| CONTRIBUTION | `contribution_create/get` | Envelope stays canonical JSON; only each `versions[i].data` is simplified (`contribution_create.yaml §Simplified Formats`). Supported where the inner payload is a COMPOSITION; see the spec-silence posture below for other kinds |
| EHR / EHR_STATUS | `ehr_create`, `ehr_create_with_id`, `ehr_status_*` | Media types declared in the OAS, but the Simplified Formats spec defines **no mapping** for EHR_STATUS (see 3.4) → `415`/`406` with a precise diagnostic |
| Directory (FOLDER) | `directory_*` | Same: no FOLDER mapping defined → `415`/`406` |
| Demographic (PERSON/AGENT/GROUP/ORGANISATION/ROLE) | `person_*`, `agent_*`, … | Same: no mapping defined → `415`/`406` |
| Template definition | `definition_template_adl1.4_get` (+ upload response) | `application/openehr.wt+json` returns the Web Template rendering (`Accept_template.yaml`, `ContentType_Template.yaml`) |

### 3.4 Spec-silence posture (flagged, not guessed)

The Simplified Formats mapping chapter (`master05-rm_mapping.adoc`) covers
COMPOSITION and every class reachable from it. It defines **nothing** for
EHR_STATUS, FOLDER, or demographic PARTY types — and structurally cannot,
because field identifiers are generated from an Operational Template
(`master02 §Relationship to Other Specifications`), which those resources
do not have. Decision: **no openEHR spec governs simplified serialization
of non-templated resources — we reject with `406` (output) / `415`
(input)** and a diagnostic that names the supported types for that
endpoint. If a future ITS-REST release defines those mappings, the reject
branch is replaced, not patched around. Each reject site carries a
`// PORT NOTE:` citing `ITS-REST/docs/simplified_formats/master05` scope.

## 4. Format semantics to implement (checklist of normative rules)

Everything below cites `docs/specs/openehr/ITS-REST/docs/simplified_formats/`.

**Path syntax** (`master04 §Field Identifiers`…`§Attribute Suffixes`):
segments `/`-separated; zero-based `:i` instance indices where `max > 1`
or `max = -1`; `|attr` suffixes; `_`-prefixed optional RM attributes
(`_uid`, `_link:i`, `_normal_range`, …); `|raw` embeds canonical JSON
(value must carry `_type`); the exact 7-step node-id generation algorithm
(`§Node ID Generation Rules`) including the published example table.

**Level removal** (`master04 §Level Removal`): the fixed list of elided
container attributes (`COMPOSITION.content`, `SECTION.items`,
`OBSERVATION.data/state/protocol`, …, `CLUSTER.items`); always-collapsed
wrappers (`ITEM_STRUCTURE` family, `HISTORY`); conditionally-collapsed
`EVENT` (collapsed iff `max = 1` and it is the only EVENT node in its
HISTORY); `ELEMENT.value` replaced by the attribute suffix.

**Per-type leaf mappings** (`master05-rm_mapping.adoc`, one table per
class): DV_TEXT, DV_CODED_TEXT (incl. `|other`), CODE_PHRASE,
TERM_MAPPING, DV_ORDINAL, DV_BOOLEAN, DV_URI, DV_EHR_URI, DV_IDENTIFIER,
DV_QUANTITY, DV_PROPORTION, DV_COUNT, DV_DATE, DV_DATE_TIME, DV_TIME,
DV_DURATION, DV_PARSABLE, DV_MULTIMEDIA, DV_INTERVAL, REFERENCE_RANGE,
COMPOSITION, EVENT_CONTEXT (incl. flattened archetyped `other_context`),
OBSERVATION, EVALUATION, INSTRUCTION (`current_activity`), ACTION,
ADMIN_ENTRY, ELEMENT, CLUSTER, LINK, FEEDER_AUDIT, FEEDER_AUDIT_DETAILS,
ACTIVITY, ISM_TRANSITION, INSTRUCTION_DETAILS, POINT_EVENT,
INTERVAL_EVENT, PARTY_SELF/IDENTIFIED/RELATED/PROXY, OBJECT_REF,
PARTICIPATION. Each table's Required column and Notes (input-defaulting,
output-only-when-non-default flags on intervals, proportion `magnitude`
calculated on output, PARTICIPATION `time` not representable) are the
implementation contract, item by item.

**Open value-sets** (`master04 §Open Value-Sets and the |other Suffix`):
`|other` writes persist as DV_TEXT; reads of a DV_TEXT under a
`listOpen: true` DV_CODED_TEXT constraint SHOULD emit `|other`; `|other`
is mutually exclusive with `|code`/`|value`/`|terminology` (MUST reject);
MUST be rejected when the constraint is closed.

**Context vocabulary** (`master06-context_information.adoc`): the full
`ctx/` key set with landing sites and defaults — `language`/`territory`
(mandatory), `time` (defaults to `now()`), `setting` (defaults to
`"other care"`), `composer_name`/`composer_self`/`composer_id`,
`id_namespace`/`id_scheme`, `work_flow_id|…`, `participation_*:i`
(compact `issuer::assigner::id::TYPE;…` and non-compact forms),
`health_care_facility|…`, `end_time`, `history_origin`, `action_time`,
`activity_timing`, `provider_*`, `action_ism_transition_current_state`
(code or value), `instruction_narrative`, `location`, `link:i|…`.

**STRUCTURED variant** (`master04 §Structured format`): nested objects;
arrays **always**, even for `0..1`/`1..1`; suffixes as `"|attr"`
properties; `ctx` as one object; empty objects omitted. FLAT↔STRUCTURED
follow the exact algorithms in `master04 §Conversion Between Formats`.

**Validation** (`master04 §Validation`): resolve the WT for the target
template; map every field; mandatory `ctx/language` + `ctx/territory`;
types/cardinality/terminology per the OPT; `_`-paths valid against the RM.

## 5. Architecture (greenfield)

The seams stay where the workspace already draws them — the wire format is
a serialization concern, the service stays canonical-RM — but the crate
content is re-authored fresh, per file, from the spec.

### 5.1 `crates/openehr-flat` — fresh rewrite, one conversion core

Old files are deleted and re-authored (the per-folder fresh-rewrite
method). Target module layout:

- `sim/` — **the single internal model**: `SimNode` — the
  STRUCTURED-shaped tree (nested objects, arrays-always, `|attr` leaf
  properties, `ctx` object). Both wire variants are pure codecs over it:
  - `sim/flat.rs` — FLAT keys ↔ `SimNode` (template-free, exactly
    `master04 §Conversion Between Formats`).
  - `sim/structured.rs` — serde shape ↔ `SimNode` (near-identity).
  This kills the duplicated flat/structured conversion paths: RM
  conversion is written **once** against `SimNode`.
- `path.rs` — the typed path model: segment (+ optional `:i`), `|suffix`,
  `_` RM-attribute marker, `ctx/` namespace; parser + printer with exact
  spec syntax.
- `webtemplate/` — the Web Template builder from the OPT: the node-id
  algorithm (`master04 §Node ID Generation Rules`), aqlPath computation,
  inputs, multiplicity, `inContext` marking, per the published document
  shape (`master04 §Web Template Metadata`). Flag retained in-code: the
  Web Template *document* itself has no standalone normative spec — the
  shape follows the example in `master04` and serves
  `application/openehr.wt+json`.
- `map/` — the per-RM-type mapping tables as data-driven codecs, one
  module per `master05` table group (data values, parties, entries,
  composition/context). Each mapping entry cites its table row.
- `ctx.rs` — the `master06` vocabulary: typed context struct, parsing of
  both participation-identifier forms, defaulting engine (applied on
  build, never stored).
- `build.rs` — Sim → RM: template-driven construction with level
  re-materialization (HISTORY/ITEM_STRUCTURE/EVENT wrappers, ELEMENT
  wrappers, structural mandatories), `ctx` defaulting, `|raw` embedding,
  `|other` branch, typed errors for every MUST-reject in §4.
- `flatten.rs` — RM → Sim: template-driven walk with level removal and
  the output-side rules (non-default-only interval flags, `|other`
  emission, proportion magnitude).
- `example.rs` — the RM example generator (kept; its completeness is
  issue #94's own item), emitting canonical RM that the normal
  serializers then render in any negotiated format.
- `validation/` — the `master04 §Validation` SHOULD-list, integrated with
  the existing template/RM validation passes.
- **Deleted:** the `ehrbase-quirks` feature and everything it gates. The
  STABLE spec absorbed the useful parts (`master05 §DV_QUANTITY` shows
  `|units_system`/`|units_display_name`); sibling-uniqueness is the
  numeric-suffix rule of `master04 §Node ID Generation Rules` step 7, one
  algorithm, no alternate form. No vendor flags remain in the crate.
- **Deleted with it:** any remaining TDD/EhrScape-flavoured shims that do
  not serve the SM `I_EHR_EXTRACT`/TDD service surface (the TDD import
  used by `service::message` is a consumer of this crate and is re-pointed
  at the new API, not rewritten here).

### 5.2 `app/ehrbase-rest` — one negotiation module, full matrix

- `overview/negotiate.rs` re-authored: one `WireFormat` enum —
  `CanonicalJson | CanonicalXml | Flat | Structured | WebTemplate` — with
  a single Accept parser (q-values per RFC 9110 §12) and a single
  Content-Type classifier used by **every** endpoint. No per-endpoint
  ad-hoc `wants_*` predicates.
- `formats/` re-authored as the simplified-payload adapter: request side
  (parse body per Content-Type + resolve template id from
  `openehr-template-id`; absence on a simplified commit → 422), response
  side (serialize any canonical-RM payload per the negotiated format).
  CONTRIBUTION gets the envelope-canonical / `versions[i].data`-simplified
  composition treatment from `contribution_create.yaml`.
- Every endpoint in the §3.3 matrix dispatches through this one seam —
  including the reject branches for the spec-silent resources, so the 406
  and 415 diagnostics are uniform.
- The service layer (`app/ehrbase`) is untouched in shape: payloads cross
  the seam as canonical RM; the service keeps owning the
  `WebTemplateCache`.

### 5.3 What does **not** exist afterwards

- No `ehrbase-quirks` feature, anywhere.
- No EhrScape module, routes, or MIME types (`…nc.flat+json`), and no plan
  row for them — building a retired vendor API contradicts the product's
  spec-only mandate. If demand ever materializes it is a new, separately
  justified feature.
- No `?format=` query parameter handling.
- No acceptance of the deprecated `.schema+json` media types.

## 6. Verification (the gate battery)

1. **Spec-example fixtures**: every JSON example in `master04`, `master05`,
   and `master06` becomes a checked-in test vector (FLAT, STRUCTURED, ctx,
   `|raw`, `|other`, node-id table, level-removal worked example). These
   are the primary oracle.
2. **Round-trip properties**: FLAT → RM → FLAT key-set equality modulo the
   spec's own output rules; STRUCTURED ↔ FLAT symmetry per the `master04`
   algorithms; RM → FLAT → RM canonical-JSON equality over the OPT corpus
   (retained as regression vectors).
3. **Reject-path tests**: every MUST-reject in §4 (`|other` combinations,
   closed value-sets, missing mandatory ctx, unknown paths, bad suffixes)
   returns a typed error mapping to 422 at the wire.
4. **HTTP negotiation matrix**: endpoint × media type × direction —
   correct 200/201/406/415/422, response `Content-Type` always present,
   `openehr-template-id` MUST-rule, contribution envelope rule, template
   `wt+json` rendering, deprecated/legacy types rejected.
5. **Workspace gates**: crate + workspace clippy and nextest green; ECC
   run with **zero drift** against the current baseline (simplified
   formats are additive; the canonical surfaces must not move).
6. **Docs**: book pages for content negotiation and the simplified formats
   updated in the same PR; `CHANGELOG.md` entry; OAS assembly untouched
   (the vendored contract already declares the media types).

## 7. Task list

- [ ] Re-vendor check: confirm `docs/specs/openehr/ITS-REST` pin includes
      the Simplified Formats chapters used here (it does at the current
      pin; re-run `scripts/vendor-spec-docs.sh` only if bumping).
- [ ] `openehr-flat` rewrite: `path.rs` + `sim/` core + FLAT/STRUCTURED
      codecs with spec-example vectors.
- [ ] `webtemplate/` builder re-authored on the node-id algorithm +
      document shape; `application/openehr.wt+json` body unchanged at the
      wire where already correct.
- [ ] `map/` per-type codecs from the `master05` tables (each entry cites
      its table); `ctx.rs` from `master06` with the defaulting engine.
- [ ] `build.rs` / `flatten.rs` against the WebTemplate, with level
      removal/re-materialization and `|raw`/`|other`.
- [ ] `validation/` per `master04 §Validation`; delete `ehrbase-quirks`
      and vendor-gated code; migrate the OPT corpus tests onto the new
      API as regression.
- [ ] `ehrbase-rest`: `WireFormat` negotiation core; `formats/` adapter;
      wire the full §3.3 matrix incl. CONTRIBUTION and the uniform
      spec-silent reject branches; `openehr-template-id` enforcement.
- [ ] Gate battery (§6) green; book + changelog; PR.

## 8. Exit criteria

- [ ] Every normative rule in §4 is implemented and covered by a test
      that cites its spec section.
- [ ] The §3.3 endpoint matrix behaves exactly as tabled, verified by the
      HTTP negotiation matrix suite.
- [ ] `ehrbase-quirks` and all vendor-oracle framing are gone from the
      workspace.
- [ ] Workspace clippy + nextest green; ECC zero drift; issue #95 closed
      with the Accept-header answer documented in the book.
