---
name: adl2-specialisation-flattening-opt2-location
description: Where ADL2 specialisation semantics, flattening, templates, OPT2, and the specialisation-phase validation rule codes live in the vendored AM spec
metadata:
  type: reference
---

ADL2 specialisation / flattening / templates / OPT2 navigation (AM spec, `docs/specs/openehr/AM/docs/`):

- **Specialisation semantics** — `ADL2/master09.01`–`09.10`:
  09.01 overview (covariance rule), 09.02 concepts (flat vs differential form, spec levels, spec paths, path congruence, **Redefinition Concepts table** = the MANDATE/EXCLUDE/REFINE/ADD/SPECIALISE/FILL/CLOSE catalogue), 09.04 attribute redef (existence/cardinality/ordering before-after), 09.05 object redef (node ids, occurrences, cloning rule, exhaustive redef, RM type refinement, use_node, external ref, slot filling, primitive/terminology/tuple redef), 09.06 rules (`and then`), 09.07 languages, 09.08 description (replaces), 09.09 terminology (value_sets replace; sum of term_definitions), 09.10 bindings.
- **Template semantics** — `ADL2/master10-templates.adoc`: template = specialised archetype with slot fillers; `use_archetype`, `template_overlay`, overlays inline in one file, `operational_template` + `component_terminologies` example.
- **OPT2** — `OPT2/master02-overview` (raw vs profiled, the removed/resolved list), `master03-opt_raw` (flattening extra steps, ANTLR `adl_operational_template` rule, `component_terminologies`), `master04-opt_profiled` (annotations/language/binding filtering, terminology substitution — has a TBD), `master05-file_formats` = **STUB (headers only, no content)**; file extensions `.opt/.optx/.optj` come from master02.
- **Validation phases + flattening algorithm** — `AOM2/master08-validation.adoc`: 3-phase model (phase1 standalone, phase2 vs flat parent, phase3 on flat form) with inline rule codes; `== Flattening` section lists the algorithm specifics.
- **Rule-code full definitions with meanings** — `AOM2/master04.5-constraint_model-class_definitions.adoc`: VSANCE/VSANCC/VSAM/VDIFV/VDIFP (~L135-172), VSONT/VSONCT/VSONIN/VSONCO/VSONPT/VSONPI/VSONPO/VSSM (~L342-391), VARX*/VDS*/VUN*/VSUNT slot+proxy (~L410-488), VACMCU/VACMCO cardinality (~L155-159). VSONCO collective-occurrences algorithm is the load-bearing one (L359-379).

Note: file_formats master05 empty; profiled-OPT node-level terminology substitution is flagged TBD in spec ("no way to do this in ADL2").

**Plain-flatten vs OPT-flatten deletion handling (CONFIRMED spec gap):** OPT flattening EXPLICITLY removes deleted nodes — `OPT2/master03-opt_raw.adoc` §Flattening L45 ("all deleted nodes are removed... `C_ATTRIBUTE` `existence matches {0}` and `C_OBJECT` `occurrences matches {0}`") + master02 L35 + intro L3 ("fully flattened, with all deleted nodes removed"). PLAIN archetype flattening: `AOM2/master08-validation.adoc` §Flattening (L114-125) only LISTS "deletions (`existence matches {0}`, `occurrences matches {0}`)" as a consideration — it does NOT state whether plain flat form keeps the deleted node visible-but-stripped-of-children vs removes it. That "keep visible, strip children" behaviour is SPEC-SILENT in the vendored text (implementation/tooling detail, ADL Workbench = reference). Flag as `// NOTE:` decision point.
Sibling-order flatten rule = `ADL2/master09.04` §Ordering of Sibling Nodes L268-279 (anchor = flat-parent sibling id or local redefinition; no markers → redefined-in-place, extensions-at-end; anchor-loss defaults: before→first conforming, after→LAST conforming). Cloning trigger = `ADL2/master09.05` L316-323 (the exact `clone not needed` predicate).
