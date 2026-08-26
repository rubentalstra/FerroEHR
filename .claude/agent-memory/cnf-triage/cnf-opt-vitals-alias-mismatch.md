---
name: cnf-opt-vitals-alias-mismatch
description: RESOLVED — cnf.opt.vitals is now a real body_temperature OBSERVATION (template_id cnf.vitals)
metadata:
  type: project
---

SUPERSEDED (verified on the wire 2026-07-23). The 2026-07-22 defect (the
`cnf.opt.vitals` alias resolving to `minimal_action.en.v1`, an ACTION template)
has been fixed. `corpus/templates/vitals.opt` now
carries `<template_id><value>cnf.vitals</value></template_id>` and IS a
`openEHR-EHR-OBSERVATION.body_temperature.v1` carrier with a DV_QUANTITY
temperature leaf (units Cel) and a `_normal_range` REFERENCE_RANGE. The SF flat
fixtures `flat.normal_range.json` / `flat.all_types.json` commit against the
`vitals/body_temperature:0/any_event:0/temperature...` paths and the SUT
201-creates them (both interval bounds present).

**How to apply:** the old "SF vitals cases 422 on unknown simplified path"
attribution is dead — do NOT reuse it. The corpus alias/template are consistent
now. If an SF-MAP/vitals row is red today it is almost certainly the interval
bound-presence app defect (a one-sided reference range with default
`upper_unbounded=false`), see [[interval-and-coded-text-spec-overreach]].
