---
name: unconstrained-attribute-validation
description: Where the openEHR "unconstrained/open attribute = any RM-valid value allowed" rule is stated (AOM1.4, ADL1.4 cADL, CNF platform schedule)
metadata:
  type: reference
---

Topic: what an archetype/template NOT constraining an RM attribute means for validation.

Owning spec locations (verbatim decisive lines confirmed):
- `AM/docs/ADL1.4/master05-cadl.adoc` §"Any" Constraints (~line 326-351):
  line 338 — an "any" constraint == "no constraint being stated at all" when
  existence is unchanged; means any value permitted by the underlying info model.
- `AM/docs/AOM1.4/master04-constraint_model_package.adoc` §Any_allowed (line 52):
  `any_allowed` = "any value permitted by the reference model ... is allowed",
  the "completely open constraint". AOM1.4 defines only the POSITIVE `valid_value`
  fn (line 60-62); it is SILENT on closed-world rejection (that is AOM2 territory).
- `AM/docs/ADL1.4/master02-overview.adoc` line 100-102: templates add only
  "further compatible constraints" + "do not introduce any new semantics".
- CNF DECISIVE: `CNF/docs/platform_test_schedule/master15-content_tc_composition.adoc`
  line 38 — "When there is no constraint defined for an attribute, it means
  anything is allowed on that attribute. It is recommended to include data not
  defined by the archetype, but valid in the RM, when generating the data
  instances." Also line 46 ("not constraining ... at all" == open/any-allowed,
  "anything, even nothing, is accepted"). master16 lines 22/114/329 same pattern.
- CNF validate_open cases (`master17.*`): an attribute matching {*} / not present
  in the OPT → only RM-mandatory sub-fields cause rejects (e.g. DV_COUNT
  magnitude NULL rejected "RM/Schema magnitude is mandatory"), every RM-valid
  value accepted. Confirms OPT does NOT materialize a constraint per attribute.

Our settled reading of the above: closed-world validation applies ONLY to
archetype-slot competition and to archetyped (at-coded) children; plain RM
attributes the template does not constrain remain open and are never flagged.
Nothing in the spec text above authorizes rejecting content under an
unconstrained RM-mandatory attribute (e.g. ACTION.description).

The validate_open behaviour is exercised by the committed CNF catalogue
(Veredictum's `artifacts`), which is where the expectation lives.

No CNF case found for committing a *server-generated example* composition
(grep empty); master15:38 is the only instance-generation guidance.
