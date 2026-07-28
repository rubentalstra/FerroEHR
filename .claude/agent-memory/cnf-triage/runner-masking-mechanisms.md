---
name: runner-masking-mechanisms
description: Two ways a red row hides — patched_body's silent RM-invalid EHR_STATUS fallback, and verdict.rs treating `errored` as no evidence
metadata:
  type: project
---

Both confirmed in the 2026-07-28 full-catalogue run.

**1. `exec/driver.rs::patched_body` (the `from_capture:` read-modify-write).**
When the named capture is absent it does NOT fail loud for
`status_body`/`ehr_status`: it substitutes a hardcoded minimal EHR_STATUS that
carries `archetype_node_id` with **no `archetype_details`** — the third site of
the family in [[ehr-status-archetype-root-invariant]], so the SUT 422s
correctly and the row reads as a fake app failure. The fallback's legitimate
scope is only the "no resource to GET" negatives (`-bad_ehr`), where the SUT
404s before validating. Any case core using a `from_capture` binding must
capture the base body explicitly (`capture: { status_body: ok.body }`) — the 12
working `set_ehr_queryable-*` cases all do.

**2. `verdict.rs::capability_evidence` absorbs `Effective::Errored`.** It
returns `Failed` on any failed case and `Passed` on any passed case; an errored
(inconclusive) case contributes NOTHING. So a capability with one passing case
publishes `passed` while a wire-visible defect sits in an errored row —
observed: `CompositionOps: passed` with both
`update_composition-prefer_minimal`/`-prefer_absent` errored on an unmapped 200.

**3. `report_only` ambiguities silently de-gate.** A case whose `ambiguities:`
names a `report_only` entry is non-gating (`verdict.rs::is_report_only`). AMB-136
is `report_only`, so EVERY `I_ITS_REST_VERSIONED_PARTY` case is non-gating even
though the binding's own comment records AMB-136 as re-adjudicated under the
AMB-161 pseudo-interface mechanism — re-check the disposition before assuming a
red demographic-container row moves any verdict.
