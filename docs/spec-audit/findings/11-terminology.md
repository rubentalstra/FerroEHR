# 11 — Terminology (TERM 3.1.0 + terminology service)

## Summary

The `openehr-term` bundle is **spec-faithful and complete**. All three vendored
XML assets embedded in the crate (`en/es/ja/pt/zh` `openehr_terminology.xml`,
`openehr_external_terminologies.xml`, `PropertyUnitData.xml`) are **byte-identical**
to the vendored TERM 3.1.0 spec at `docs/specs/openehr/TERM/computable/XML/`
(verified by `diff`). Every internal vocabulary group (17: the 14 RM-referenced
groups + the 3 EHR_EXTRACT groups), every internal code set
(`normal_statuses`, `compression_algorithms`, `integrity_check_algorithms`),
and all four external code sets (`languages`/ISO 639-1, `countries`/ISO 3166-1,
`character_sets`/IANA, `media_types`/IANA) are parsed and reachable. The
`id=532` dual-rubric quirk (SPECPR-51: `532` = "complete" in
`version_lifecycle_state` vs "completed" in `instruction_states`) is handled
**correctly** — rubric lookup is group-scoped, proven by
`version_lifecycle_state_specpr51_quirk`, and the service layer emits the
correct group-specific rubric ("complete" for `ORIGINAL_VERSION.lifecycle_state`
code 532 in `versioned.rs`).

The bundle's public API is a flat idiomatic `OpenehrTerminology` rather than the
spec's three-interface shape (`TERMINOLOGY_SERVICE` / `TERMINOLOGY_ACCESS` /
`CODE_SET_ACCESS`). This is acceptable per ADR-006/008 (the spec governs
behaviour, not structure) and every spec operation has a semantic equivalent.

The defects are all on the **consumer side**: one real wire-conformance bug in
the service layer (audit `change_type` emits the rubric text as the CODE_PHRASE
`code_string` instead of the numeric group code), and several **RM
terminology-bound invariants the composition validator silently omits**
(`DV_TEXT.language`/`encoding`, `ISM_TRANSITION.transition`,
`TERM_MAPPING.purpose`, `DV_ORDERED.normal_status`, `PARTY_RELATED.relationship`)
despite the bundle exposing validators for every one of them.

Counts: **critical 0 · major 4 · minor 3 · info 2**.

## Findings

### F-11-01: `AUDIT_DETAILS.change_type` emits the rubric as the CODE_PHRASE `code_string`
- **Severity:** major
- **Spec:** TERM 3.1.0 `openehr_terminology.xml` group `audit_change_type`
  (`249` creation, `250` amendment, `251` modification, `252` synthesis,
  `523` deleted, `666` attestation, `816` restoration, `817` format conversion,
  `253` unknown); RM `common` `AUDIT_DETAILS.change_type: DV_CODED_TEXT` coded
  from that group; a `CODE_PHRASE.code_string` must be the group's **code**.
- **Code:** `crates/ehrbase/src/service/contribution.rs:213-224` (`audit_details`);
  callers pass rubric strings — `crates/ehrbase/src/service/vobject.rs:55-57`
  (`CREATION="creation"`, `MODIFICATION="modification"`, `DELETED="deleted"`) and
  `crates/ehrbase/src/service/contribution.rs:29-31`.
- **Problem:** `audit_details` sets **both** `change_type.value` and
  `change_type.defining_code.code_string` to the same `change_type` argument,
  which is the human rubric ("creation"/"modification"/"deleted"). So the emitted
  CODE_PHRASE is `{terminology_id: "openehr", code_string: "creation"}` — but
  `"creation"` is **not a valid code** in `audit_change_type`; the code is `249`.
  The correct pairs are `creation→249`, `modification→251`, `deleted→523`. Every
  `AUDIT_DETAILS` / `REVISION_HISTORY` returned by the REST API (VERSION
  `commit_audit`, CONTRIBUTION audit) therefore carries an invalid `code_string`
  and would fail a group-membership check / canonical conformance.
- **Fix:** separate the code from the rubric. Store the numeric group code
  (`"249"`/`"251"`/`"523"`) in the `change_type` DB column and as `code_string`,
  and set `value` to the group rubric (resolve via
  `openehr_term::bundle::openehr().rubric("audit_change_type", code, "en")`).
  Add a round-trip test asserting `code_string` is a member of
  `is_valid_audit_change_type`.
- [ ] fixed

### F-11-02: Composition validator omits `DV_TEXT.Language_valid` / `Encoding_valid`
- **Severity:** major
- **Spec:** RM `data_types` `DV_TEXT` invariants —
  `Language_valid: language /= Void implies code_set(Code_set_id_languages).has_code(language)`
  and `Encoding_valid: encoding /= Void implies code_set(Code_set_id_character_sets).has_code(encoding)`
  (`org.openehr.rm.data_types.dv_text.adoc`).
- **Code:** `crates/openehr-flat/src/validation/terminology.rs:19-53,113-127`
  (the `Group` enum + `slots_for` cover only 7 groups; no `DV_TEXT` slot).
- **Problem:** `DV_TEXT`/`DV_CODED_TEXT` `language` and `encoding` are the most
  pervasive terminology-bound coded slots in a composition, yet neither is
  validated in any pass (the RM-invariant pass in `openehr-rm` explicitly defers
  terminology-bound invariants — see the module doc — and the terminology pass
  never adds them). The bundle already provides `is_valid_language` and
  `is_valid_character_set`; they are simply not called.
- **Fix:** add a `DV_TEXT`-family slot (all subtypes) validating `language`
  against `languages` and `encoding` against `character_sets`
  (`is_valid_external_code`). Note these are `CODE_PHRASE` slots without a
  `terminology_id: "openehr"` guard — resolve them against the external code
  sets directly, not `is_valid_code`.
- [ ] fixed

### F-11-03: Composition validator omits `ISM_TRANSITION.Transition_valid`
- **Severity:** major
- **Spec:** RM `ehr` `ISM_TRANSITION` invariant
  `Transition_valid: transition /= Void implies terminology(Terminology_id_openehr).has_code_for_group_id(Group_id_instruction_transitions, transition.defining_code)`
  (`org.openehr.rm.composition.ism_transition.adoc`).
- **Code:** `crates/openehr-flat/src/validation/terminology.rs:117` — the
  `ISM_TRANSITION` entry validates only `current_state`, not `transition`.
- **Problem:** The sibling `current_state` (instruction_states) **is** checked,
  but `transition` (instruction_transitions) is not, despite appearing on the
  same object and the bundle exposing `is_valid_instruction_transition`.
- **Fix:** add `("transition", Group::InstructionTransition)` to the
  `ISM_TRANSITION` slot list and a corresponding `Group` variant.
- [ ] fixed

### F-11-04: Composition validator omits `TERM_MAPPING.Purpose_valid`
- **Severity:** major
- **Spec:** RM `data_types` `TERM_MAPPING` invariant
  `Purpose_valid: purpose /= Void implies terminology(Terminology_id_openehr).has_code_for_group_id(Group_id_term_mapping_purpose, purpose.defining_code)`
  (`org.openehr.rm.data_types.term_mapping.adoc`).
- **Code:** `crates/openehr-flat/src/validation/terminology.rs:113-127` — no
  handling of `DV_TEXT.mappings[].purpose`.
- **Problem:** `TERM_MAPPING.purpose` (a `DV_CODED_TEXT` from the
  `term_mapping_purpose` group) can appear on any `DV_TEXT` in the composition
  and is unvalidated. Bundle validator `is_valid_term_mapping_purpose` exists
  and is unused.
- **Fix:** when walking a `DV_TEXT`, iterate `mappings[]` and validate each
  `purpose.defining_code` against `term_mapping_purpose` (guarded by
  `terminology_id == "openehr"`).
- [ ] fixed

### F-11-05: Validator omits `DV_ORDERED.normal_status` and `PARTY_RELATED.relationship`
- **Severity:** minor
- **Spec:** RM `DV_ORDERED` invariant
  `Normal_status_validity: normal_status /= Void implies code_set(Code_set_id_normal_statuses).has_code(normal_status)`
  (`org.openehr.rm.data_types.dv_ordered.adoc`); `PARTY_RELATED.relationship`
  coded from the `subject_relationship` group
  (`org.openehr.rm.common.party_related.adoc`, RM common §generic).
- **Code:** `crates/openehr-flat/src/validation/terminology.rs:113-127` (no slots).
- **Problem:** Both are terminology-bound RM slots the validator ignores; both
  have ready bundle validators (`is_valid_normal_status`,
  `is_valid_subject_relationship`). Lower frequency than F-11-02..04, hence minor.
- **Fix:** add a `DV_ORDERED`-family `normal_status` slot (against the
  `normal_statuses` code set) and a `PARTY_RELATED` `relationship` slot (against
  `subject_relationship`).
- [ ] fixed

### F-11-06: `is_valid_normal_status` uses a linear scan instead of an index
- **Severity:** minor
- **Spec:** n/a (implementation hygiene).
- **Code:** `crates/openehr-term/src/bundle.rs:382-385`.
- **Problem:** External code sets are indexed into `HashSet`s
  (`external_codes`, O(1) `is_valid_external_code`), but the internal
  `normal_statuses` code set is membership-tested by
  `cs.codes.iter().any(...)` — an O(n) scan. Minor and bounded (7 codes), but
  inconsistent with the rest of the API and re-scans on every DV_ORDERED.
- **Fix:** build the same `openehr_id → HashSet<value>` index for the canonical
  bundle's internal code sets, mirroring `external_codes`.
- [ ] fixed

### F-11-07: Spec identifier constants ("openehr", group ids) are not exposed; consumers hardcode strings
- **Severity:** minor
- **Spec:** RM support `OPENEHR_TERMINOLOGY_GROUP_IDENTIFIERS`
  (`Terminology_id_openehr = "openehr"`, `Group_id_* = "…"`) and
  `OPENEHR_CODE_SET_IDENTIFIERS` (`Code_set_id_* = "…"`) —
  `org.openehr.rm.support.openehr_terminology_group_identifiers.adoc`,
  `…openehr_code_set_identifiers.adoc`.
- **Code:** `crates/openehr-term/src/bundle.rs` (no id constants); consumers
  hardcode literals — `"openehr"` at `versioned.rs:98`, `contribution.rs:223`,
  `terminology.rs:96`; group-id string literals throughout the validators.
- **Problem:** The spec defines these ids as named constants precisely so
  consumers do not scatter magic strings; the bundle offers none, so each caller
  repeats `"openehr"` and the underscore group ids by hand. Also note the RM
  invariants reference groups by the **space** form (`"term mapping purpose"`)
  while the bundle indexes by the **underscore** `openehr_id`
  (`"term_mapping_purpose"`); both resolve correctly (the XML carries both, and
  `group_id(name)` bridges them), so this is not a conformance bug — only a
  robustness/DRY gap.
- **Fix:** expose the spec identifier constants (a `terminology_id::OPENEHR`,
  `group_id::*`, `code_set_id::*` module) and have consumers reference them.
- [ ] fixed

### F-11-08: Language assets are all embedded unconditionally (no feature gating)
- **Severity:** info
- **Spec:** n/a (VERSIONS.md / PORT_MASTER_PLAN §9.1 planned `lang-{de,es,fr,pt,ja}` features).
- **Code:** `crates/openehr-term/src/bundle.rs:38-59` (`LANGUAGE_ASSETS` includes
  all five via unconditional `include_str!`); `crates/openehr-term/Cargo.toml`
  (no `[features]`).
- **Problem:** All five vendored languages (`en/es/ja/pt/zh`) are compiled into
  every build; the planned per-language feature flags do not exist. The set
  matches the vendored spec exactly (no `de`/`fr` are vendored, so their planned
  flags are moot), and codes/validity resolve against the canonical `en` bundle,
  so this is purely a binary-size / design-intent note, not a conformance gap.
- **Fix:** none required for conformance; optionally feature-gate the non-`en`
  rubrics to match the documented design and shrink the binary.
- [ ] fixed

## Hygiene notes

- **Provenance is clean:** all embedded assets are byte-identical to the
  vendored TERM 3.1.0 spec (verified `diff`); no drift, no hand-edits.
- **Bundle test coverage is strong and spec-anchored** — group counts, the
  SPECPR-51 dual-rubric quirk, multi-language rubrics, external code sets, and
  property↔unit lookups are all asserted with codes pulled from the vendored XML.
  Consider adding assertions that pin the SPECPR-51 quirk against *both* the
  service emission (F-11-01 area) and the bundle.
- The bundle correctly resolves code **validity** against the canonical `en`
  bundle only (codes are language-independent) while rubric lookup is
  per-language — this matches spec intent.
- The bundle parses the three EHR_EXTRACT groups (`extract_content_type`,
  `extract_action_type`, `extract_update_trigger_event_type`) generically via
  `is_valid_code`; no convenience validators, which is fine (EHR_EXTRACT is
  experimental / out of Stage-1 scope).
- The FHIR `CodeSystem`/`ValueSet` mirror under
  `docs/specs/openehr/TERM/computable/FHIR/` is **not** consumed (the XML bundle
  is the single source) — correct; do not add a second parser path.
- Recommend a dedicated `15-validation` audit chapter cross-reference for
  F-11-02..05, since those are composition-validation completeness gaps that
  also belong to the P15 validation surface.
