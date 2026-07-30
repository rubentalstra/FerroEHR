---
name: adl2-templates-chapter-location
description: Map of the TWO templates chapters (ADL2/master10 vs AOM2/master10), what each actually owns, and the confirmed example/grammar defects in the ADL2 one
metadata:
  type: reference
---

There are **two** `master10-templates.adoc` files in `docs/specs/openehr/AM/docs/` — do not confuse them:

- **`ADL2/master10-templates.adoc`** (641 lines, included by `ADL2/master.adoc` L141, anchor `#_templates`): only ~9 prose lines of normative content (Overview L3-11; Example prose L15, 144, 146, 148, 150, 152, 435). Everything else is the discharge-summary example (parent archetype L23-141, template+2 overlays L160-433, OPT L443-641) in at-coded/id-coded tab pairs. **Zero V-codes.** Owns: template=specialised archetype (L5), slot-fill-as-redefinition (L7), `{0}` multiplicity reduction (L9), overlays (L11, 146, 150), "the `template` keyword has no formal implications" (L144), flattening→one OPT + `component_terminologies` (L435), the only vendored `component_terminologies` ADL example (L515-537).
- **`AOM2/master10-templates.adoc`** (92 lines): the object-model side — TEMPLATE/TEMPLATE_OVERLAY classes, the "commonly used in templates" feature list incl. **default values** (L11-17), "no new nodes added" tooling convention (L19-21), the three referenced-entity kinds (L23-29), and **§Template Identifiers (L69-90)** = the ONLY place template id/versioning conventions live (`t_` prefix, `<root>-<parent_concept>_N` overlay ids; "suggested", non-normative).

Delegations OUT of ADL2/master10 (it is silent on all of these):
- closing slots → `ADL2/master09.05` §Slot Filling and Redefinition L921-948 (`allow_archetype X[id] closed`); narrowing → L950 replacement `allow_archetype`.
- filler node-id rule ("the original at-code … must also be mentioned, to indicate which slot the used archetype is filling") → `ADL2/master09.05` L849.
- unconstrained-attribute `use_archetype` → `ADL2/master09.05` §Unconstrained Attributes L952-1020 ("no slot, and is instead validity-checked against … the underlying reference model", L956).
- default values → `ADL2/master06-default_values.adoc` (165 lines, `_default =` pseudo-attribute).
- **VTPL** (the only template V-code: fillers must carry the root template's `original_language`) → `AOM2/master03-archetype_package.adoc` L271-274.
- OPT flattening steps + deleted-node removal → `OPT2/master02` L33-37, `OPT2/master03` L25, L45-49.
- effective-occurrences inference for OPT nodes → `AOM2/master04.5` L181.
- artefact-type grammar productions → `ADL2/master07.01-adl_introduction.adoc` L33-106.

**Confirmed released-text defects in ADL2/master10** (all example-side, all re-verified 2026-07-30):
1. at-coded template fillers use `at0000.1…at0000.9` (L189-195) but its own terminology (L206-233) and the at-coded OPT (L477) use `at0.1…at0.9`. id-coded twin is self-consistent (`id0.N`).
2. at-coded OPT shows `DV_CODED_TEXT[at0028]` (L471) where the at-coded source archetype has `DV_CODED_TEXT[at9000]` (L47).
3. Both OPT examples carry a `specialize` section (L446-447, L548-549) that **no** operational_template production allows (`adl2.g4` L52-60; `master07.01` L94-106).
4. Fillers are given fresh `id0.N` ids rather than the slot's own `id2`, contradicting master09.05 L849's "the original at-code … must also be mentioned".
5. L150 says overlays omit "`languages`" sections — the keyword is `language`.
6. L146 "The template, if saved as a file, contains all its overlays in one file" vs AOM2/master10 L29 "either as a separate file … or within the template source file".

**Grammar tensions vs `crates/openehr-adl/vendor/grammar/`** (`adl2.g4` L31-60, `cadl2.g4` L17/L34, `base_lexer.g4` L18-21):
- `cadl2.g4` `c_archetype_root` takes `ID_CODE` only and has **no** trailing `matches {}` block → the at-coded `use_archetype …[at0000.1, …]` form and the OPT inlined `SECTION[at0.1, <hrid>] occurrences matches {1} matches {…}` form are **both unparseable** by the vendored ADL2 grammar. OPT definition syntax has no production at all.
- `SYM_TEMPLATE_OVERLAY` = `H_CMT_LINE (WS|LINE)* 'template_overlay'` — the ≥8-dash separator line is **part of the token** (mandatory), while `master07.01` L71's `adl_template_overlay` production shows no separator.
- `adl2.g4` `template_overlay` allows only specialize+definition+terminology; `master07.01` L69-83 additionally allows optional `rules` and `rm_overlay`. Same mismatch on `rm_overlay` for `template`.
