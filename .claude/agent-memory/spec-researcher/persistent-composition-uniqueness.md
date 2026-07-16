---
name: persistent-composition-uniqueness
description: Whether one-persistent-composition-per-template is spec-mandated (it is NOT) and where the spec/CNF discuss it
metadata:
  type: reference
---

"One live persistent COMPOSITION per template/OPT per EHR" is NOT a spec mandate — it is SILENT/under-debate.

- RM prose: `RM/docs/ehr/master04-ehr_package.adoc` §"Persistent Compositions"
  (~line 82) EXPLICITLY allows "more than one instance of some, e.g. multiple
  condition-specific problem lists". §"Compositions" defines category
  event/episodic/persistent.
- COMPOSITION class: `RM/docs/UML/classes/org.openehr.rm.composition.composition.adoc`
  — `category` DV_CODED_TEXT (431|persistent|), `is_persistent()` function; the
  ONLY invariants are Category/Territory/Language/Content/Is_archetype_root — NO
  uniqueness invariant.
- CNF names the gap directly: `CNF/docs/platform_test_schedule/master07-func_tc_ehr_composition.adoc`
  test case `create_composition-same_opt_twice` (~line 385-405): note says
  behaviour is "under debate in the openEHR SEC ... due to the lack of information
  in the openEHR specifications". Robot suite
  `CNF/tests/platform/robot/I_EHR_COMPOSITION/create_composition/I_EHR_COMPOSITION.create_composition-same_opt_twice.robot`
  is tagged `future` (not a required case); expects 2nd create → 400.
- CNF contribution schedule master08 (~line 130) also notes servers MAY reject a
  second persistent composition referencing the same OPT — "depending on the
  server implementation". So reject-on-duplicate is EHRbase prior art, not a mandate.
