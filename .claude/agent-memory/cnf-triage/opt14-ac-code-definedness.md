---
name: opt14-ac-code-definedness
description: What makes an OPT 1.4 ac-code "defined" (VTCBK/VACDF) and where a component archetype's ontology must live — the 2026-07-29 terminology-binding provisioning triage
metadata:
  type: project
---

An `ac` code in an OPT 1.4 is DEFINED only by a `constraint_definitions` entry
— a `CONSTRAINT_REF`/`constraint_bindings` mention is not a definition.

**Why:** three released sentences, all first-hand.
- AM `docs/ADL1.4/master08-adl.adoc` §Coded Term Validity, *VATDF*: "All
  constraint identifiers ('ac' codes) used in the definition part of the
  archetype must be defined in the constraint_definitions part of the
  ontology." Restated by *VACDF* on the next line.
- AM `docs/ADL1.4/master05-cadl.adoc` §Placeholder Constraints: the `[acNNNN]`
  placeholder — "Codes of this form are defined in the archetype ontology
  section, and can be mapped to query identifiers".
- ITS-XML `ALL/Archetype.xsd` `ARCHETYPE_ONTOLOGY`: sequence
  `term_definitions` (**minOccurs defaults to 1 → MANDATORY**),
  `constraint_definitions`, `term_bindings`, `constraint_bindings` — in that
  order. `CodeDefinitionSet` requires `language`; `ARCHETYPE_TERM` requires
  `code` + at least the `text`/`description` items (AOM1.4 master07 §Term and
  Constraint Definitions).

**Placement:** a COMPONENT archetype's ontology goes in a
`component_ontologies` entry keyed by its `archetype_id`; the top-level
`<ontology>` element is the ROOT archetype's — AM `docs/OPT2/master03-opt_raw.adoc`
§Terminology ("the flat form of the `terminology` section of each flattened
constituent archetype or template (other than the root template) is gathered
under the `component_terminologies` section"), mirrored by Template.xsd's
`ontology` / `component_ontologies` split. `opt14_convert.rs::ontology_for`
implements exactly that, so an ac-code defined in the WRONG ontology element
silently yields an unconstrained node (`map_constraint_ref` keeps the
constraint only when `defined_acs` contains the reference) — the constraint
disappears instead of erroring.

**How to apply:** any corpus OPT that carries an ac-code needs, in one
`component_ontologies` entry for the owning archetype, `term_definitions` +
`constraint_definitions` + `constraint_bindings` in XSD order. The app's VTCBK
gate (`validation/opt/terminology.rs`) is spec-grounded here despite carrying
the AOM2 code label; its sibling VACDF gate
(`validation/opt/invariants.rs::check_constraint_ref`) is deliberately lenient
(enforced only when the artefact declares any constraint vocabulary), so a
no-ontology OPT with a dangling `CONSTRAINT_REF` is accepted and its
constraint silently dropped. See [[results-evidence-locations]].
