# Phase 14 — WebTemplate builder + FLAT (simSDT)

- Status: in-progress (PR-A: WebTemplate builder + wt+json endpoint; PR-B: FLAT `RM ⇄ FLAT` converters + endpoints)
- Consumes: `openehr-am` (OPT model, P13), `openehr-term`
- Compile required: yes (compiling, tested increment)
- Decisions: ADR-006

## Objectives

Build the **WebTemplate** (the flattened, UI/validation-oriented view of an OPT)
from an ingested operational template — EHRbase's `OptVisitor` equivalent. The
WebTemplate drives composition validation (P15), the FLAT/STRUCTURED formats
(P17), and AQL semantic path analysis (P16). Cache built WebTemplates with
`moka`.

## Preconditions

- [x] P13 (OPT ingested — consumed via `openehr_its::opt14`)

## Scope

**In:** OPT → WebTemplate node tree (paths, rm types, cardinalities, terminology
bindings, inputs); `moka` cache keyed by template id; WebTemplate JSON export
(`application/openehr.wt+json`); Better `web-template` semantics as the oracle.
**Out:** validation logic (P15); FLAT/STRUCTURED conversion (P17).

## Tasks

- [x] WebTemplate model + OPT→WebTemplate builder — `openehr-flat::webtemplate` (model/id/inputs/builder); Better `web-template` shape (ids, aqlPath, inputs, compaction, post-processing) as the oracle.
- [x] `moka` cache — `WebTemplateCache` (`cache.rs`), wired into `ehrbase-rest` `AppState`.
- [x] WebTemplate JSON export endpoint — `GET /definition/template/adl1.4/{id}` serves `application/openehr.wt+json` on `Accept` (else the OPT XML).
- [x] Tests vs Better `web-template-tests` vectors — full 63-file Better set vendored + 91-file service corpus as a smoke gate (145 build, 0 builder failures); insta goldens + targeted assertions.

## Exit criteria

- [x] Reference OPTs produce WebTemplates matching Better's format (insta goldens for Demo Vitals / Diagnosis / medication_list; Better ships no stored WT vectors, so goldens are self-generated and format-matched, and the full-corpus gate proves the builder).
- [x] Cache hit path verified — `cache::tests::builds_once_then_serves_from_cache`.
- [x] Compiles + clippy-clean — `openehr-flat` + `ehrbase-rest` build, clippy-clean, `cargo fmt` clean.

## PR-A scope boundaries (recorded as `TODO(port)`)

- Required-RM-attribute injection (needs the BMM RM attribute model, P16),
  `ISM_TRANSITION`/careflow synthesis, "any"-element expansion, archetype
  internal-ref resolution, and node-level term bindings are deferred.
- 9 Better templates cannot be exercised yet: `openehr_its::opt14` (P13's
  generated parser) rejects them (`missing element node_id/occurrences/purpose`)
  — a P13 parser-strictness follow-up, not a WebTemplate-builder gap.

## PR-B — FLAT (simSDT) `RM ⇄ FLAT`

- [x] `openehr-flat::flat` — `to_flat` (RM canonical JSON → flat `path|suffix` map)
  and `from_flat` (flat map → canonical `COMPOSITION`), driven by the WebTemplate;
  17 `Dv*` leaf mappers (Better suffixes: `|magnitude`/`|unit` singular,
  `|code`/`|value`/`|terminology`, `|ordinal`/`|scale`, `|numerator`/`|denominator`,
  `|id`, `|mediatype`, …), `:i` repeat indexing (`isRepeating`), polymorphic-choice
  routing, and full `ctx/` context (language/territory/composer/time/setting).
- [x] `from_flat` re-materialises the compacted RM structure (HISTORY / single
  EVENT / ITEM_TREE / ELEMENT wrapper) + mandatory identity/occurrence fields so
  output deserialises as an `openehr-rm` `Composition`.
- [x] COMPOSITION FLAT endpoints (`ehrbase-rest`): `application/openehr.wt.flat+json`
  on create/update (body → `from_flat`, template id from `template_id`/`templateId`
  query or `openEHR-TEMPLATE_ID` header) and get/create/update (`Accept` → `to_flat`).
- [x] Tests: FLAT→RM→FLAT round-trip stable on 37 real `(canonical composition, OPT)`
  pairs (SDK + Better + service corpora); insta flat goldens; per-type key
  assertions; 4 HTTP endpoint tests (mock backend) incl. a full HTTP round-trip.

### PR-B scope boundaries (recorded as `TODO(port)`)

- `to_flat`/`from_flat` cover the core leaf + structural set; `INSTRUCTION.activities`
  /`ACTIVITY`, `ACTION.ism_transition` synthesis, archetyped `other_context`,
  feeder-audit / links / term-mappings / reference-range metadata, and STRUCTURED
  (structSDT) are deferred (P17). Reverse output is schema-valid RM for 24/37 of
  the corpus today (the remainder need the above ENTRY/multi-entry synthesis);
  the FLAT→RM→FLAT round-trip is stable for all 37.

## Decisions made this phase

- Better `web-template` semantics are the oracle; EHRbase quirks behind the
  `ehrbase-quirks` flag on `openehr-flat` (P17).
- FLAT template resolution: a flat body carries no template id, so create/update
  take it from a `template_id`/`templateId` query param or the
  `openEHR-TEMPLATE_ID` header (EHRbase-compatible); the WebTemplate is fetched
  from the DEFINITION store and cached.
