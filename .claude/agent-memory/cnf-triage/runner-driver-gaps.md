---
name: runner-driver-gaps
description: Confirmed instrument driver gaps that surface as red rows the SUT passed correctly
metadata:
  type: project
---

Three confirmed runner-machinery gaps in Veredictum's `src` (2026-07-22
baseline). In all, the SUT was spec-correct; the runner misdrove/miscompared.

1. **No option-gating at drive time.** `run.rs` main loop (`for case in
   ordered`, ~L103) gates only on status / `fully_unrealized` / `requires.server`
   exclusivity — it never sees the statement, so it DRIVES cases whose
   `case.option` is not in `statement.options` and records real `failed` rows.
   `verdict.rs::effective_outcome` (L372-376) already deselects them, so the
   VERDICT is unaffected, but results.json shows a spurious red row. Hit
   `I_DEFINITION_ADL14.get_opt-retrieve_latest_version` (option
   `adl14-duplicate-versioned`; statement selected `adl14-duplicate-conflict`).
   Fix: thread the statement into run.rs and NotApplicable-gate on option.

2. **No header-mutation variant support.** `driver.rs` uses `step.variant` ONLY
   to select a binding variant (`binding_for_variant`, L100-123); its own doc
   says a header-mutation variant with no dedicated binding "falls back to the
   variant-less binding". So `no_template_id` (omit openehr-template-id) and
   `deprecated_media_type` (send a pre-1.1.0 SDT-era Content-Type) send the
   NORMAL valid request → SUT correctly 201-creates → cases expecting
   422/415 fail. Hit SF-FLAT-missing_template_id, SF-DEPRECATED-media_unsupported.
   Fix: per-step header set/omit mechanism in driver.rs.

3. **`equivalent` comparator doesn't fold ctx/ input keys to RM paths.** ctx/
   keys are INPUT convenience (ITS-REST simplified_formats master06 L3: "set
   context values which set default values in the rm-tree"); wt-flat read-back
   expresses them on RM paths (committed `ctx/participation_name` →
   read-back `minimal/context/_participation:0|name`). The `equivalent
   to: committed` assertion diffs keys literally and fails. Hit
   SF-FLAT-commit_roundtrip_ctx_defaults step 3. Fix: normalize ctx/ → RM-path
   in the ctx_defaults ignore/normalization (exec assertions/compare path).

**How to apply:** all three are runner defects, NOT app defects — the fix is in
Veredictum's src, never app code or the catalogue expectation. Verify the
gap still exists (grep the cited symbols) before re-attributing; an implementer
may have added the mechanism.
