# W-3f area audit — Templates / OPT ingestion (the `ehrbase` platform crate)

**Scope.** OPT 1.4 ingestion, the operational-template store, the derived
runtime WebTemplate cache, and the identity law that governs archetypes and
templates as *authored resources*. Read-only audit; the sibling seams
(validation, the SM Definitions provisioning trait, the `template_store`
table) are named as `TODO(w3f-integrate)` candidates only.

**Method (spec-first).** The register skeleton below is enumerated **from the
vendored spec** — the resource model, the AM template/OPT model, the
architecture-overview archetypes chapter, and the identity package — and only
*then* is the existing code mapped onto each item with a `file:line` verdict.
Code that maps to no spec item is flagged (spec-silent / quarantine / delete).

---

## 1. Spec skeleton (the oracle)

### 1.1 Archetypes & OPTs are AUTHORED_RESOURCEs (BASE resource)

`docs/specs/openehr/BASE/docs/resource/master02-resource_package.adoc`:

- **S-01 — original language + translations.** `AUTHORED_RESOURCE` records the
  `_original_language_`; each translation adds a `TRANSLATION_DETAILS` to
  `_translations_`; `languages_available` lists all (§Natural Languages and
  Translation). *An archetype/OPT is a descendant of `AUTHORED_RESOURCE`, so
  these govern the artefact we ingest.*
- **S-02 — meta-data.** `RESOURCE_DESCRIPTION` / `RESOURCE_DESCRIPTION_ITEM`
  carry author, creation date, purpose, lifecycle; `_description_` is
  **optional** (partial-construction resources) (§Meta-data).
- **S-03 — controlled resource + revision history.** When change control
  applies, `_is_controlled_` = True and all changes carry an audit trail;
  controlled resources live in a versioned repository (§Revision History).

### 1.2 The AM template / OPT model (AM)

- **S-04 — a template is a specialised archetype** (`TEMPLATE`,
  `TEMPLATE_OVERLAY`); it inherits `AUTHORED_RESOURCE` meta-data, a
  `terminology` section, `rules`/`annotations`; it may only narrow, and its
  data conform to the referenced archetypes + RM
  (`AM/docs/AOM2/master10-templates.adoc` §Overview). Slot-filling is by
  specialisation; overlays are the template-local private components
  (§An Example).
- **S-05 — the OPT is the compiled, inheritance-flattened, standalone
  top-level artefact.** All archetype references resolved to **full 3-part**
  ids; no specialisation statement; no `use_node`; slot-fillers substituted;
  closed slots and `existence matches {0}` nodes removed; overlays applied;
  constituent terminologies gathered under `component_terminologies`
  (`AM/docs/OPT2/master02-overview.adoc` §Types of OPT;
  `master03-opt_raw.adoc` §Flattening/§Terminology).
- **S-06 — OPT safety rule.** "A production EHR … can safely run only using
  guaranteed *validated* templates and archetypes. No direct use of source
  artefacts should ever be made for reasons of safety."
  (`OPT2/master02-overview.adoc` §Purpose item 1).

### 1.3 Runtime role of archetypes & templates (Architecture Overview)

`docs/specs/openehr/BASE/docs/architecture_overview/master10-archetypes.adoc`:

- **S-07 — archetype repository is separate from data**, stored in its own
  repository; templates deploy archetypes **at runtime** (§Overview).
- **S-08 — a template's two runtime functions:** (a) validate data at
  capture/import so data conform to the RM *and* the archetypes; (b) the
  design basis for AQL paths (§Archetypes and Templates at Runtime).
- **S-09 — a compiled near-runtime form** that "incorporate[s] copies of the
  relevant archetypes", improving performance and guaranteeing only validated
  artefacts are used (§Deploying Archetypes and Templates). *This is the spec
  motivation for a derived runtime artefact — but the concrete shape is not
  specified.*

### 1.4 Identity law (BASE identification)

`docs/specs/openehr/BASE/docs/base_types/master05-identification_package.adoc`:

- **S-10 — `ARCHETYPE_ID`** is a multi-axial immutable id
  (`rm_originator-rm_name-rm_entity.domain_concept.vN`); version is *part of*
  identity — two versions are two distinct archetypes (§Archetype
  Identifiers + §Syntaxes grammar).
- **S-11 — `TEMPLATE_ID`** is a normal multi-axial identifier /GUID, same
  rules as archetypes (`AOM2/master10-templates.adoc` §Template Identifiers;
  `TEMPLATE_ID` class, identification package §Class Descriptions).
- **S-12 — composite identifiers are case-preserving *and*
  case-insensitive**: two ids differing only in case are the *same* id
  (§Composite Identifiers and Case).

### 1.5 OPT 1.4 wire form — spec-defect note

There is **no normative prose master for the OPT 1.4 structure** (the OPT2
masters describe the ADL2 successor with `component_terminologies`; blueprint
`03-am.md` §Spec defects). The OPT 1.4 canonical XML we ingest
(top-level `language / is_controlled / description / revision_history / uid /
template_id / concept / definition / ontology / component_ontologies /
annotations / constraints / view`) is governed by the **ITS-XML v1 Template
XSD** + AOM 1.4. Flag: *OPT-1.4-structure conformance is XSD-defined, not
prose-defined* — implement the evident meaning, cite the XSD.

---

## 2. Code mapped onto the spec

| Code | Maps to | Verdict |
|---|---|---|
| `service/template.rs:110 store_template` | S-05, S-06, S-11 | conformant — parses OPT, extracts `template_id`/`concept`/root archetype, insert-only |
| `service/template.rs:249 validate_opt_structure` | S-05 (well-formedness) | conformant — rejects alien/duplicate top-level tags the tolerant codec would accept |
| `service/opt_validation.rs` (1322 L) + `opt_validation/` | S-06 (validated only) | conformant but **oversized** — AOM2/08 standalone-artefact validity; > 700-line rule |
| `service/template.rs:25 web_template_for` + `openehr-flat/src/cache.rs` | S-09 | conformant intent — cached derived runtime form; **format non-normative** (see §3) |
| `service/template.rs:78 template_example` + `openehr-flat/src/example.rs` | S-08(a)-adjacent | spec-silent (example generation is not spec-mandated) |
| `service/definition.rs` (adl14 impl) + `service/api/definition.rs` | S-04/S-05 provisioning | conformant — SM `I_DEFINITION_ADL14` on `template_store` |
| `migrations/ehr/0001_baseline.sql:149 template_store` | S-07 (repository) | conformant — DB schema is spec-silent by construction ("no openEHR spec governs SQL"); dual UUID/`template_id` identity documented |
| `template_store` case-sensitive `UNIQUE(template_id)` (text) | **S-12** | **divergent** — id equality is byte-exact, not case-insensitive |
| `openehr-flat/src/webtemplate/{builder.rs 49k, model.rs, inputs.rs, id.rs}` | S-09 | spec-silent — WebTemplate **format** is Better/SDT vendor practice, mirrors `WebTemplateIdBuilder.kt` |
| `service/tdd.rs`, `service/composition.rs:443`, `service/fhir/mod.rs`, `service/api/mod.rs` | S-08(a) consumers | conformant — validation/commit path consumes `web_template_for` |

**No orphan code found** — every template touchpoint traces to a spec item or
is an explicitly spec-silent runtime artefact. No delete/quarantine candidates.

---

## 3. WebTemplate machinery — spec-silent flag

The WebTemplate is the S-09 "compiled near-runtime form", which the spec
*blesses in principle* — but **the WebTemplate JSON format itself is not
openEHR-normative** (it is the Better `web-template` SDT format; the builder
comments cite `WebTemplateIdBuilder.kt`, not a spec). Disposition: **keep**,
but it must live in `openehr-flat` (a hand-written spec-adjacent crate), never
be presented as canonical, and its id-sanitisation (`id.rs` lowercasing) is a
*vendor* rule, not the S-12 identity law. The platform crate's `templates/`
module only *caches and resolves* it; it does not own the format.

---

## 4. G-row register

| id | citation / flag | severity | disposition |
|---|---|---|---|
| G-T01 | S-05/S-06 (OPT2 master02 §Purpose) | — | already-correct — `store_template` insert-only, parses+validates |
| G-T02 | S-05 (OPT2 master03 §Flattening) — structural well-formedness | — | already-correct — `validate_opt_structure` |
| G-T03 | S-06 (arch overview §Validation) — AOM2/08 catalogue | low | fix-in-rewrite (split only) — `opt_validation.rs` 1322 L → `artefact_validity/` sub-tree ≤ 700 L each |
| G-T04 | **S-12** (identification §Composite Identifiers and Case) | **med** | fix-in-rewrite — case-insensitive `template_id` equality + canonicalisation at the store boundary; matches blueprint BASE row 7 (unregistered gap) |
| G-T05 | S-09 (arch overview §Deploying) — runtime form | — | already-correct — moka cache + `Arc<WebTemplate>` |
| G-T06 | WebTemplate format — no openEHR spec governs this (our/Better design) | low | PORT NOTE — keep the flag; format stays in `openehr-flat` |
| G-T07 | S-11 dual identity (UUID handle + `template_id` wire) | — | already-correct — documented in DDL; both load-bearing |
| G-T08 | 422-vs-404 for unknown template (ITS-REST `422_COMPOSITION` / `404_unknown_template_id` + CNF) | — | PORT NOTE keep — re-verified against ITS-REST responses + CNF |
| G-T09 | Immutable OPT → 409 (`409_template_already_exists.yaml` + CNF `upload_opt…_conflict`) | — | PORT NOTE keep |
| G-T10 | `list_matching_opts` returns `template_id` not `ARCHETYPE_ID` (SM defect) | low | PORT NOTE keep — spec-defect, re-verified |
| G-T11 | OPT 1.4 has no prose master; XSD + AOM1.4 govern | low | PORT NOTE keep — cite ITS-XML Template XSD, never OPT2 masters |
| G-T12 | S-01/S-02/S-03 (AUTHORED_RESOURCE meta-data/translations/revision history) | low | PORT NOTE — OPT meta-data (language/description/revision_history) is parsed but **not surfaced/queried**; document that we index `template_id`/`concept`/root only (spec allows optional `_description_`) |

**Counts:** already-correct 4 · fix-in-rewrite 2 (one is split-only) ·
PORT-NOTE 6. No quarantine, no delete.

---

## 5. Target design — `app/ehrbase/src/templates/`

Derived from the spec decomposition **resource identity/lifecycle → operational
form → derived runtime artefact**. All files ≤ ~700 lines.

```
app/ehrbase/src/templates/
  mod.rs           facade + EhrbaseService inherent-method surface; module docs
  store.rs         template_store CRUD: store_template (insert-only/409),
                   get_template_xml, get_template_meta, list_templates,
                   opt_get / opt_get_by_template_id / opt_list  (S-05/S-07/S-11)
  ingest.rs        OPT 1.4 XML → OperationalTemplate parse + validate_opt_structure
                   (top-level well-formedness, S-05)                 (~250 L)
  identity.rs      TEMPLATE_ID/ARCHETYPE_ID parse + S-12 case-insensitive
                   equality & canonicalisation at the store boundary (G-T04)
  artefact_valid/  the AOM2/08 catalogue, split from opt_validation.rs (G-T03):
    mod.rs         dispatch (VCOC/VACMCO, VATID/VTLC, VTTBK/VTCBK, VCORM/…)
    reference_model.rs   VCORM/VCARM/VCAEX/VCACA/VCAM (RM conformance)
    terminology.rs       VATID/VTLC/VTTBK/VTCBK
    structure.rs         VCOC/VACMCO + tests
  runtime.rs       web_template_for (cache resolve, hit/miss metric) +
                   template_example — thin over openehr_flat            (~150 L)
```

`openehr-flat` keeps the WebTemplate **format** (builder/model/inputs/id +
cache); the platform crate only stores, resolves, and caches (G-T06).

## 6. Seams (`TODO(w3f-integrate)` candidates — not touched here)

- **validation/** — the commit/import path consumes `web_template_for`
  (`service/composition.rs`, `service/tdd.rs`) → S-08(a).
- **service/definition/ (SM)** — `DefinitionAdl14Service` provisioning trait
  is the FIXED seam; `templates/store.rs` is its backing.
- **storage/** — the `template_store` table (dual identity) is owned by the
  storage layer; `templates/` calls it, does not define DDL.
- **service/fhir/** + **service/api/** — read-only WebTemplate consumers.

---

## W-3f closure (2026-07-13)

`template.rs` re-grounded into `src/templates/` (`store.rs`, `ingest.rs`, `identity.rs`, `runtime.rs`, `mod.rs`); the AOM2 artefact-validity catalogue landed in the sibling `validation/opt/` module (register 09) rather than a templates-local `artefact_valid/`, per the cross-register validation-ownership ruling.

| G | Disposition | Evidence |
|---|---|---|
| G-T01 | already-correct | `store_template` insert-only, parses+validates — `templates/store.rs` |
| G-T02 | already-correct | `validate_opt_structure` well-formedness — `validation/structure.rs` (invoked via `templates/ingest.rs`) |
| G-T03 | FIXED (split) | AOM2/08 catalogue split from the 1,322-line file into `validation/opt/*` (`rm_conformance.rs`, `terminology.rs`, `invariants.rs`, `interval.rs`, `primitive.rs`), each ≤700 lines |
| G-T04 | FIXED in code | case-insensitive `template_id` equality + store-boundary canonicalisation — `templates/identity.rs:38` `lower()`-normalised form + migration `migrations/ehr/0007_template_id_ci_unique.sql` (`ux_template_store_template_id_ci`) |
| G-T05 | already-correct | moka cache + `Arc<WebTemplate>` runtime form — `templates/runtime.rs` |
| G-T06 | PORT NOTE | WebTemplate format stays in `openehr-flat` (no openEHR spec) — `templates/runtime.rs` |
| G-T07 | already-correct | dual identity (UUID handle + `template_id`) documented in DDL — `templates/store.rs` |
| G-T08 | PORT NOTE | 422-vs-404 for unknown template (ITS-REST + CNF re-verified) — `templates/store.rs` |
| G-T09 | PORT NOTE | immutable OPT → 409 — `templates/store.rs` |
| G-T10 | PORT NOTE | `list_matching_opts` returns `template_id` not `ARCHETYPE_ID` (SM defect) — `templates/store.rs` |
| G-T11 | PORT NOTE | OPT 1.4 governed by XSD + AOM1.4 (cite ITS-XML Template XSD) — `templates/ingest.rs` |
| G-T12 | PORT NOTE | OPT meta-data parsed but not surfaced/queried (index `template_id`/`concept`/root only) — `templates/store.rs` |

Open residue: none — G-T03 split into `validation/opt/`, G-T04 fixed in code + migration, the rest kept as cited PORT NOTE / already-correct.
