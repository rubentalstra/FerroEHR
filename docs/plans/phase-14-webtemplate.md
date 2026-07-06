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

### Completeness (after PR-C + the leftover-closure pass)

- **STRUCTURED (structSDT) done** (PR-C): the pure `flat ⇄ structured` nesting
  transform composed with FLAT; `wt.structured+json` wired. STRUCTURED round-trip
  37/37 stable; RM-validation **37/37**.
- **FLAT reverse output is schema-valid RM 37/37** (PR-C closed the earlier 24/37
  gap: `INSTRUCTION`/`ACTIVITY`/`INTERVAL_EVENT`/`ISM_TRANSITION` structural
  completion synthesized from the WebTemplate in `flat/graph.rs`; DV_MULTIMEDIA/
  DV_PARSABLE mappers). Full `ctx/` coverage both directions.
- **WebTemplate `termBindings` (node + coded-value) + `compactMultipleCodedTexts`
  done** (Better parity). **opt14 tolerates real-world OPT laxity** (omitted
  mandatory node_id/occurrences/purpose default leniently) → 151/154 build.

### Deferred follow-ups (scoped, phase-blocked — tracked, NOT stubs)

Accepted as scoped follow-ups (2026-07-06); each depends on infrastructure not
yet built or is a niche trade-off — none is a shortcut in shipped code:

- **WebTemplate required-RM-attribute synthesis** (structural RM attributes an OPT
  omits) → needs the **BMM RM model** (P16 `emit-rm-model`). Note in
  `webtemplate/builder.rs`.
- **Coded-label rubric lookup** for non-`local` terminologies → needs an
  **openehr-term bundle loader** (not built). Note in `webtemplate/inputs.rs`.
- **WebTemplate `defaultValue`** (from `CPrimitive.assumed_value`) +
  **`otherTerminologies`** — small/niche, doable without new infra.
- **3 Better edge fixtures** (Apgar_1 / Request_for_Pancreas / test_statuses)
  carry incomplete embedded RM values the strict (correct) RM deserializer
  rejects; the 91-file real corpus all parses. Making opt14's embedded-RM slots
  lenient `Value` (→154/154) trades away typing there; deferred deliberately —
  the shared RM parser was NOT weakened.

## Decisions made this phase

- Better `web-template` semantics are the oracle; EHRbase quirks behind the
  `ehrbase-quirks` flag on `openehr-flat` (P17).
- FLAT template resolution: a flat body carries no template id, so create/update
  take it from a `template_id`/`templateId` query param or the
  `openEHR-TEMPLATE_ID` header (EHRbase-compatible); the WebTemplate is fetched
  from the DEFINITION store and cached.
