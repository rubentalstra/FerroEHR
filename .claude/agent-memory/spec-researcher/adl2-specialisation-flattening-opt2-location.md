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
