---
name: rm-common-validate-paths-templates
description: Confirmed defects in the openehr-rm common/validate/paths spec-behaviour templates (#2255) — the ISO-8601 duplicate validators, the ATTESTATION terminology gap, and the stale optional-container NOTEs
metadata:
  type: feedback
---

Verified 2026-08-11 against `docs/specs/openehr/{RM,BASE}` while auditing
`tools/openehr-codegen/templates/openehr-rm/{common,validate,paths.rs,ehr_extract,integration}`
(issue #2255). Re-verify before acting.

- **THREE independent ISO-8601 duration readers disagree in-tree.** RM
  `validate.rs::is_valid_iso_duration` (the `DV_DURATION.Value_valid` gate)
  enforces neither designator ORDER nor at-most-once, so `P1D1M`/`P1Y1Y`
  commit; `dv_ordered_impl::iso_duration_to_seconds` then ACCUMULATES the
  repeats into a fabricated magnitude, while BASE `iso8601_parse::parse_duration`
  (slot ratchet, `last_slot`) rejects the same string, so `DvDuration::add`
  returns None. The production `P[nnY][nnM][nnW][nnD][T[nnH][nnM][nnS]]`
  (BASE `time_definitions.adoc` §Functions) admits none of them.
  Same shape for seconds `60`: RM accepts (`in_range(sec,0,60)`), BASE
  `validate_time` rejects. BASE's own timezone/duration code is NO LONGER the
  lenient side (my older BASE note is stale on both points).
- **`terminology::slots_for` is keyed by EXACT `_type`, so inherited
  terminology invariants are lost at subtypes.** The one live gap is
  `ATTESTATION` (ancestors = AUDIT_DETAILS) missing `change_type` →
  `Change_type_valid` unenforced, while `attestation_impl.rs` claims that
  table realizes it. Every other terminology-bound declarer has all its
  concrete descendants listed by hand (rot risk, not a live gap).
- **The optional-container NOTEs did not follow the `Option<NonEmptyVec>`
  migration.** `attestation_impl` ("List emits as a Vec, absent and empty are
  one value"), `party_identified_impl`/`party_related_impl` (contradict each
  other about `Identifiers_valid`), `revision_history_impl` (Option returns for
  two `1..1` functions), and generated `validate/generated.rs:41` all still
  describe `Vec`/`Option<Vec<T>>`. `NonEmptyVec: Deserialize` refuses `[]` at
  PARSE (`containers.rs:224`) — check the FIELD TYPE before believing any such
  NOTE.
- **`templates/openehr-rm/validate.rs` is invisible to
  `scripts/checks/comment-style.sh`**: the guard skips any file containing the
  literal `@generated`, and that template mentions it in prose. Every stamped
  copy under `crates/` carries `@generated-from-template`, so nothing checks it.
- The master05 `[hospital episodes]` vs master11 bare-bracket conflict IS
  registered (ambiguities.yaml ~l.7705) — do not re-file it. The
  attribute-less `ehr:/<ehr_id>/<OVID>` locator accepted "because the CNF
  fixtures use it" (paths.rs) is NOT registered and has no released ground.
