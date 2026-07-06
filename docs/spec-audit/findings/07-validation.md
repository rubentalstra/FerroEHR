# 07 — Composition validation (RM invariants + AOM constraints + terminology)

## Summary

Phase-15 built a three-pass composition validator in `openehr-flat::validation`
(`crates/openehr-flat/src/validation/`): (1) an RM class-invariant pass
(`openehr_rm::validate::validate_rm_value`, backed by ~43 `*_impl.rs` files),
(2) an openEHR-terminology pass, and (3) a WebTemplate archetype-conformance
walk (occurrences / cardinality / leaf domain constraints). It is wired into the
COMPOSITION create/update path (`crates/ehrbase/src/service/composition.rs`) and
maps failures to ITS-REST `422` with a `{message, validationErrors[]}` body.

The RM class-invariant pass is genuinely good: for the classes that have an
`*_impl.rs` the invariant conditions match the spec text closely (DV_PROPORTION
ratio kinds, ELEMENT null-flavour XOR, DV_AMOUNT accuracy, ISO-8601 well-formedness,
REFERENCE_RANGE simplicity). But the subsystem has structural gaps against the
spec, several of which the phase file itself pre-declared as "deferred follow-ups"
(F2/F3/F5/F6) — this audit confirms them against the vendored spec, adds spec
citations, and raises severity where the gap is a full bypass rather than a
missing edge check:

- **A whole commit path (CONTRIBUTION) skips validation entirely** — F-07-01.
- **A COMPOSITION with no declared `template_id` skips *all* validation**, including the template-independent RM-invariant and terminology passes — F-07-02.
- **Terminology validation covers 7 of ~17 RM-mandated coded slots**; the bundle validators for the other ~10 exist but are never called — F-07-03.
- **AOM `existence` constraints (and the VCOC cardinality/occurrences cross-check) are neither modeled nor checked** — F-07-04.
- The inherited DV_ORDERED invariants are unenforced, and DV_ORDINAL/DV_SCALE run no invariants at all — F-07-15.
- Missing DV_TEXT invariants, "empty-if-present" invariants, and invariant-name
  divergences from the spec — F-07-07/08/09.
- Two areas that are **underdetermined by the AOM 1.4 text** (not clear violations): rejection of unmatched/extra instance content (spec-silent in 1.4; only positive `valid_value` cascade is normative — F-07-05), and the DV_QUANTITY unit↔magnitude / DV_ORDINAL symbol↔value covariance (via `C_QUANTITY_ITEM`, partly covered — F-07-06).

Severity counts: **1 critical, 3 major, 8 minor, 3 info.**

The RM authority citations below are from `docs/specs/openehr/RM/docs/UML/classes/`
(the normative invariant tables) and `docs/specs/openehr/AM/docs/AOM1.4/`.

## Findings

### F-07-01: CONTRIBUTION-endpoint commits bypass composition validation entirely
- **Severity:** critical
- **Spec:** openEHR RM Common IM — Change Control (`docs/specs/openehr/RM/docs/common/`, CONTRIBUTION / VERSION / commit semantics); ITS-REST 1.0.3 `POST /ehr/{ehr_id}/contribution` (`docs/specs/openehr/ITS-REST/.../contribution`); CNF platform schedule master08-contribution. A CONTRIBUTION is a set of VERSIONs whose content must satisfy the same validity rules as a direct commit — there is no spec basis for content committed via a CONTRIBUTION being exempt from validation.
- **Code:** `crates/ehrbase/src/service/contribution.rs:43-109` (`create_contribution` → builds `Change::Create`/`Change::Modify` with `canonical: data` and calls `vobject::commit_contribution` directly); contrast `crates/ehrbase/src/service/composition.rs:18,118` which call `validate_composition_for_commit` on the direct path.
- **Problem:** `create_contribution` never invokes `validate_composition_for_commit` (or any validator). A COMPOSITION (or EHR_STATUS / FOLDER) POSTed inside a CONTRIBUTION is decomposed and persisted with no RM-invariant, terminology, or template-conformance checking. The entire phase-15 subsystem is dead on this path. The phase file lists this as a deferred follow-up ("CONTRIBUTION-path compositions bypass `create_composition`"), but it is a spec-conformance bypass of a required commit route, not a minor edge case.
- **Fix:** In `create_contribution`, for each `Change::Create`/`Change::Modify` whose `Kind::Composition`, run the same validation as the direct path before `commit_contribution` (share a `validate_for_commit(kind, &data)` helper). Reject the whole contribution atomically with `422` if any version fails. Add an e2e test (valid + invalid composition inside a contribution).
- [x] fixed

### F-07-02: A COMPOSITION with no declared `template_id` skips *all* validation, not just template-conformance
- **Severity:** major
- **Spec:** RM invariants (e.g. `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_structures.representation.element.adoc` `Inv_null_flavour_indicated`; `.../org.openehr.rm.composition.composition.adoc` `Category_validity`, `Content_valid`) are properties of the RM instance and are **template-independent** — they hold whether or not an OPT is referenced. AOM `valid_value` (`docs/specs/openehr/AM/docs/AOM1.4/master04-constraint_model_package.adoc` §"Valid_value") is the *archetype*-conformance function; RM invariants are separate and always apply.
- **Code:** `crates/ehrbase/src/service/composition.rs:210-233` (`validate_composition_for_commit`): returns `Ok(())` immediately when `/archetype_details/template_id/value` is absent. The RM-invariant and terminology passes live *inside* `openehr_flat::validate_composition(composition, &wt)` (`crates/openehr-flat/src/validation/mod.rs:101-110`), which is only reached when a WebTemplate exists.
- **Problem:** The PORT NOTE correctly argues a *templateless* composition cannot be template-validated, but the implementation skips the whole validator, so it also skips the two template-independent passes (RM class invariants + RM-mandated terminology). A templateless composition with, e.g., an ELEMENT having both `value` and `null_flavour`, or an invalid `category` code, is accepted.
- **Fix:** Split the entry point. Always run `rm_invariant_pass` + `terminology_pass` (they need no WebTemplate). Only gate the WebTemplate/archetype-conformance pass on a resolved template. Expose an `openehr_flat::validate_rm_and_terminology(composition)` (no `wt`) and call it unconditionally in `validate_composition_for_commit`.
- [x] fixed

### F-07-03: Terminology pass covers 7 of ~17 RM-mandated coded slots; the bundle validators for the rest are never called
- **Severity:** major
- **Spec (each an RM invariant that resolves against openEHR terminology or an ISO code-set):**
  - `COMPOSITION.Territory_valid`: `code_set(Code_set_id_countries).has_code(territory)` — `org.openehr.rm.composition.composition.adoc:63`.
  - `COMPOSITION.Language_valid`: `code_set(Code_set_id_languages).has_code(language)` — `composition.adoc:66`.
  - `ENTRY.Language_valid` / `ENTRY.Encoding_valid` (`Code_set_id_character_sets`) — `org.openehr.rm.composition.entry.adoc:67,70`.
  - `DV_TEXT.Language_valid` / `DV_TEXT.Encoding_valid` — `org.openehr.rm.data_types.dv_text.adoc:62,65`.
  - `ISM_TRANSITION.Transition_valid` (`Group_id_instruction_transitions`) — `org.openehr.rm.composition.ism_transition.adoc:38`.
  - `AUDIT_DETAILS.Change_type_valid` (`Group_id_audit_change_type`) — `org.openehr.rm.common.generic.audit_details.adoc:39`.
  - `ATTESTATION.Reason_valid` (`Group_id_attestation_reason`) — `org.openehr.rm.common.generic.attestation.adoc`.
  - `PARTY_RELATED.Relationship_valid` (`Group_id_subject_relationship`) — `org.openehr.rm.common.generic.party_related.adoc:23`.
  - `TERM_MAPPING.Purpose_valid` (`Group_id_term_mapping_purpose`) — `org.openehr.rm.data_types.term_mapping.adoc:71`.
  - `DV_ORDERED.Normal_status_validity` (`Group_id_normal_statuses`); `DV_MULTIMEDIA.media_type` (IANA).
- **Code:** `crates/openehr-flat/src/validation/terminology.rs:113-127` (`slots_for` wires only `COMPOSITION.category`, `EVENT_CONTEXT.setting`, `ISM_TRANSITION.current_state`, `PARTICIPATION.function`/`.mode`, `EVENT.math_function`, plus a generic `null_flavour`). The bundle already exposes `is_valid_country`, `is_valid_language`, `is_valid_character_set`, `is_valid_instruction_transition`, `is_valid_audit_change_type`, `is_valid_attestation_reason`, `is_valid_subject_relationship`, `is_valid_term_mapping_purpose`, `is_valid_normal_status`, `is_valid_media_type` (`crates/openehr-term/src/bundle.rs:325-441`) — none are called.
- **Problem:** Confirms phase-15 F2. Ten+ RM terminology-bound invariants are silently unenforced despite the validators existing. `territory`/`language` are especially load-bearing (every COMPOSITION carries them).
- **Fix:** Extend `slots_for` / the terminology pass: handle the ISO code-set slots (`language`, `territory`, `encoding`) which use `code_set` not a group id (they are not `terminology_id == "openehr"` — the current `check_openehr_code` early-returns for non-`openehr` terminology, so these need a separate code-set branch keyed on `terminology_id` = `ISO_639-1`/`ISO_3166-1`/`IANA_character-sets`), and wire the remaining openEHR-group slots (`ISM_TRANSITION.transition`, `AUDIT_DETAILS.change_type`, `ATTESTATION.reason`, `PARTY_RELATED.relationship`, `TERM_MAPPING.purpose`, `DV_ORDERED.normal_status`, `DV_MULTIMEDIA.media_type`).
- [x] fixed

### F-07-04: AOM `C_ATTRIBUTE.existence` constraints are neither modeled nor validated
- **Severity:** major
- **Spec:** AOM 1.4 `docs/specs/openehr/AM/docs/AOM1.4/master04-constraint_model_package.adoc:33` — "an `existence` constraint indicates whether an object will be found in a given attribute field … existence is *always required*" and is distinct from cardinality (container membership) and occurrences (per-object). Line 70: `C_ATTRIBUTE` records "the existence and cardinality expressed by the constraint".
- **Code:** `crates/openehr-flat/src/webtemplate/model.rs` `WebTemplateNode`/`WebTemplateCardinality` carry `min`/`max` (occurrences) and cardinalities but **no existence field** (grep for `existence` in `crates/openehr-flat/src` is empty); the validator (`mod.rs`) has no existence check.
- **Problem:** A `C_ATTRIBUTE` with `existence {1..1}` on a single-valued RM attribute (the attribute field must be present) is not enforced. Occurrences on the child node partially overlaps this for archetype-node-identified children, but plain RM attributes constrained only by existence (e.g. a mandatory `value` on an ELEMENT the template requires present) are not checked. Default existence when unstated is `{1..1}` (`master05-cadl.adoc:210`), so the omission is not conservative. **Related:** the AOM 1.4 **VCOC** validity rule (`master05-cadl.adoc:324` — `(Σ occurrences.lower)..(Σ occurrences.upper)` must lie inside the container `cardinality` interval) is an archetype-internal check, not an instance check, so it belongs to OPT ingestion rather than here; noted for completeness.
- **Fix:** Capture `existence` on WebTemplate nodes during OPT ingestion (from `C_ATTRIBUTE.existence` in `openehr_its::opt14`; default `{1..1}`) and add an existence check to the walk (attribute present ⇒ within existence bounds; absent + existence min ≥ 1 ⇒ `Required`). Distinguish it from occurrences per `master04-constraint_model_package.adoc:33` (existence = "will the field be there at all"; cardinality = container membership; occurrences = per-object-block count).
- [x] fixed

### F-07-05: Extra / unmatched instance content is not rejected (open-world walk) — spec-underdetermined in AOM 1.4
- **Severity:** minor
- **Spec:** AOM 1.4 defines only the *positive* recursive conformance function `valid_value` (`master04-constraint_model_package.adoc:60-62` — "a cascade of calls down the tree"). It is **silent on whether a present-but-unmatched instance node is an error** — a grep of `AM/docs/AOM1.4/` + `AM/docs/ADL1.4/` for `closed`/`extra`/`reject`/`unmatched` finds no normative rule. The "closed archetype" / RM-conformance closure semantics are formalized only in **AOM 2 / ADL 2** (`AM/docs/AOM2/master08-validation.adoc`, `c_conforms_to()`, VSONCT/VSONCO/VSONI). So rejecting extra content under an **OPT 1.4** is *prior-art* behaviour (EHRbase and the openEHR validators treat compositions as closed against the OPT), not compelled by AOM 1.4 text.
- **Code:** `crates/openehr-flat/src/validation/mod.rs:165-200` (`walk`/`check_group`) navigates *from* WebTemplate nodes *into* the instance — it only visits instance nodes that match a template node's path+predicate. Instance nodes with no matching template constraint are never visited and never flagged. So the walk reports *too few* / *out of range*, never *unexpected*.
- **Problem:** A composition can carry archetyped content (an extra ENTRY / unknown `archetype_node_id`) and pass. This diverges from de-facto openEHR CDR behaviour, but is **not** a clear AOM 1.4 violation — it is a decision point.
- **Fix:** Record an ADR / `// PORT NOTE:` decision. Recommended: adopt closed-archetype semantics (match prior art + the AOM2 direction), detecting instance children under a constrained (non-`any_allowed`) attribute whose `archetype_node_id` matches no sibling constraint and emitting a violation — but **only after** cross-checking the CNF composition fixtures (`docs/specs/openehr/CNF/tests/platform/robot/`) to confirm the expected tolerance and avoid over-rejecting RM-permitted-but-unconstrained metadata (`name`, `uid`, `links`).
- [ ] fixed

### F-07-06: DV_QUANTITY unit↔magnitude and DV_ORDINAL symbol↔value covariance only partly enforced
- **Severity:** minor
- **Spec:** **AOM 1.4 has no `C_ATTRIBUTE_TUPLE`/`C_PRIMITIVE_TUPLE`** (those are AOM2/ADL2). In OPT 1.4 the DV_QUANTITY covariance is expressed by `C_QUANTITY` + a `C_QUANTITY_ITEM` list, each item pairing one `units` string with its own `magnitude: Interval<Real>` (`AM/docs/UML/classes/org.openehr.am.aom14.c_quantity_item.adoc`; `AM/docs/ADL1.4/master09-customising_adl.adoc:42-67`) — or by alternative whole `C_COMPLEX_OBJECT` blocks under `value` (`master05-cadl.adoc:218-234`). DV_ORDINAL uses `C_ORDINAL` with an `ORDINAL{symbol: CODE_PHRASE, value: Integer}` list (`org.openehr.am.aom14.ordinal.adoc`) — the `(symbol, value)` pair must match one entry.
- **Code:** `crates/openehr-flat/src/validation/leaf.rs:150-160` (`unit_scoped_range`) **does** select the magnitude range for the instance's chosen unit — so the DV_QUANTITY units↔magnitude pairing *is* approximated (good). But: (a) the DV_ORDINAL `check_ordinal` (`leaf.rs:66-83`) validates only the coded symbol's membership, not the `symbol`↔`value` pairing; (b) alternative whole-block joint matching (the mph-vs-km/h pattern) is not modeled.
- **Problem:** A DV_ORDINAL whose `value` does not match its `symbol`'s ordinal entry is accepted; a value matching one alternative block's unit but another's magnitude could slip through.
- **Fix:** In `check_ordinal`, validate the `(symbol.defining_code, value)` pair against the `C_ORDINAL` list (carry the value alongside the code in the WebTemplate input). Lower priority — the common quantity case is already covered by `unit_scoped_range`.
- [ ] fixed

### F-07-07: DV_TEXT class invariants are entirely unenforced
- **Severity:** minor
- **Spec:** `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.dv_text.adoc:59-71` — `Valid_value: not value.is_empty`; `Mappings_valid: mappings /= void implies not mappings.is_empty`; `Formatting_valid: formatting /= void implies not formatting.is_empty` (plus the terminology `Language_valid`/`Encoding_valid` under F-07-03).
- **Code:** There is no `dv_text_impl.rs`, and `DV_TEXT`/`DV_CODED_TEXT` are absent from the dispatcher `match` in `crates/openehr-rm/src/validate.rs:352-399`. So no DV_TEXT invariant runs. (DV_CODED_TEXT's nested `defining_code` *is* checked, because `CODE_PHRASE` is dispatched when the recursion reaches it — `Code_string_valid` is covered.)
- **Problem:** Every `name` field and every free-text value skips `Valid_value` (empty text accepted) and the empty-if-present mappings/formatting checks. Confirms phase-15 F3.
- **Fix:** Add `dv_text_impl.rs` (`Valid_value`, `Mappings_valid`, `Formatting_valid`) and add `"DV_TEXT" => run::<DvTextData>` / `"DV_CODED_TEXT" => run::<DvCodedText>` to the dispatcher. (DV_CODED_TEXT inherits DV_TEXT's invariants.)
- [ ] fixed

### F-07-08: "Not-empty-if-present" / container invariants the spec mandates are unenforced
- **Severity:** minor
- **Spec:** (all in `docs/specs/openehr/RM/docs/UML/classes/`)
  - `LOCATABLE.Links_valid: links /= Void implies not links.is_empty` — `org.openehr.rm.common.locatable.adoc:57` (applies to **every** LOCATABLE in the composition).
  - `COMPOSITION.Content_valid: content /= Void implies not content.is_empty` — `org.openehr.rm.composition.composition.adoc:69`.
  - `ENTRY.Other_participations_valid: other_participations /= Void implies not other_participations.is_empty` — `org.openehr.rm.composition.entry.adoc:76`.
  - `SECTION.Items_valid` (items non-empty if present) — `org.openehr.rm.composition.section.adoc`.
  - `INSTRUCTION.Activities_valid` (non-empty if present) — `org.openehr.rm.composition.instruction.adoc`.
  - `ITEM_LIST` structure (all items are ELEMENT, `Valid_structure`) — `org.openehr.rm.data_structures.item_structure.item_list.adoc` (largely structurally guaranteed by the generated `Element`-typed field; the empty-if-present style checks are not).
- **Code:** No LOCATABLE-level shared check for `Links_valid` (`crates/openehr-rm/src/validate.rs` provides only `push_archetype_node_id_valid`); `composition_impl.rs`, `section_impl.rs`, `instruction_impl.rs` explicitly note the archie-`ignored` invariants are skipped ("archie's own `Section.Items_valid` is `ignored`").
- **Problem:** Per ADR-008 the **spec** — not archie — is the oracle; these are normative spec invariants that archie merely marks `ignored`. A composition with `"links": []` or `"content": []` on the wire violates the spec yet validates. Since canonical JSON omits empty collections on *output*, the empty-array case arises exactly on *input* — where the validator sits. `Links_valid` in particular applies to every LOCATABLE and is cheap. Confirms phase-15 F5.
- **Fix:** Add a shared `push_links_valid` applied by every LOCATABLE impl; add `Content_valid`, `Items_valid`, `Activities_valid`, `Other_participations_valid` to the respective impls. Keep an ADR-003 `// PORT NOTE:` distinguishing "spec-defined but archie-ignored" so the deliberate over-strictness relative to archie is documented.
- [ ] fixed

### F-07-09: Invariant-failure message names diverge from the spec (and from each other)
- **Severity:** minor
- **Spec:** The LOCATABLE invariant that requires `archetype_details` on a root is `Archetyped_valid: is_archetype_root xor archetype_details = Void` (`org.openehr.rm.common.locatable.adoc:60`). `Is_archetype_root` is the *function* redefinition (returns True for COMPOSITION/ENTRY), not the invariant name.
- **Code:** `crates/openehr-rm/src/composition/composition_impl.rs:23` emits `"Invariant Is_archetype_root failed on type COMPOSITION"`, while every ENTRY subtype emits `"Is_archetypeRoot"` (`evaluation_impl.rs:14`, `instruction_impl.rs:14`, `observation_impl.rs:20`, `action_impl.rs:13`, `admin_entry_impl.rs:13`).
- **Problem:** Inconsistent between COMPOSITION (`Is_archetype_root`) and ENTRY (`Is_archetypeRoot`), and neither matches the spec invariant name `Archetyped_valid`. These strings are surfaced to clients in the `422` body, so the error identity is non-conformant/unstable. Also the `xor` reverse direction (a non-root node carrying `archetype_details` — allowed since any node may be an archetype root; only the "root ⇒ details present" half is checked, which is correct in effect). Confirms phase-15 F6.
- **Fix:** Standardize on the spec name `Archetyped_valid` (or a single consistent name) across COMPOSITION + all ENTRY impls; update the co-located tests. Same pass: `Accuracy_valid`→`Accuracy_validity` if pursuing archie-name fidelity is dropped in favour of spec names.
- [ ] fixed

### F-07-10: ARCHETYPE_SLOT fills are not validated against the slot constraint
- **Severity:** minor
- **Spec:** AOM 1.4 `ARCHETYPE_SLOT` (`includes`/`excludes` archetype-id assertion lists) — slot-filling content must match the slot's id patterns.
- **Code:** `crates/openehr-flat/src/webtemplate/builder.rs:313-315` — `CObject::ArchetypeSlot(_) | CObject::ConstraintRef(_)` produce no WebTemplate node ("Unfilled slot / constraint ref: no node"). So an OPT that leaves a slot open (or content filled at commit time) carries no constraint into the walk.
- **Problem:** Content placed into an open slot is unconstrained by this validator (and, via F-07-05, also not rejected). For a fully-flattened OPT this is usually moot, but externally-referenced/openly-slotted templates lose slot validation.
- **Fix:** When a slot is unfilled, either resolve the referenced archetype (out of scope Stage-1) or at minimum record the slot's rm_type + occurrences so the walk can still occurrence/type-check the slot position. Document as a scope boundary if deferred.
- [ ] fixed

### F-07-11: C_STRING pattern matching silently passes patterns the `regex` crate cannot compile
- **Severity:** minor
- **Spec:** cADL `C_STRING` regex (`docs/specs/openehr/AM/docs/ADL1.4/master05-cadl.adoc`), which may use PCRE features (backreferences) that Rust `regex` rejects.
- **Code:** `crates/openehr-flat/src/validation/leaf.rs:277-287` (`matches_pattern`): a pattern that fails `Regex::new` returns `true` (treated as matching).
- **Problem:** A value violating a pattern that happens to use an unsupported regex feature is accepted. Rare, but a silent false-negative. `CLAUDE.md` lists `fancy-regex` for exactly cADL backreferences.
- **Fix:** Fall back to `fancy-regex` for patterns `regex` rejects; only skip if both fail, and log at debug. Low priority.
- [ ] fixed

### F-07-12: `422` body flattens structured `{path, message}` violations into `"<path>: <message>"` strings
- **Severity:** info
- **Spec:** ITS-REST 1.0.3 `responses/422_COMPOSITION.yaml` declares **no** content schema; the reused `schemas/others/Error.yaml` has `validationErrors: [string]`. So the current shape is *permitted*.
- **Code:** `crates/ehrbase-rest/src/error.rs:57-60` maps `ValidationFailed(errors)` into `validationErrors` strings; `crates/ehrbase/src/service/composition.rs:225-231` builds `ValidationError{path,message}` from the validator's structured `ValidationMessage{path,message,kind}`.
- **Problem:** No conformance defect (spec schema is empty), but the validator's rich per-node path/kind is collapsed to a string, and machine-readable path granularity is lost. Documented PORT NOTE.
- **Fix:** None required for conformance. Optionally retain structured objects if a future CNF fixture constrains the 422 body.
- [ ] fixed

### F-07-13: Type-conformance uses a hardcoded partial subtype map (permissive), not the BMM RM model
- **Severity:** info
- **Spec:** RM type hierarchy (BMM `openehr_rm_1.2.0.bmm.json`); conformance = instance type is a descendant of the constraint type.
- **Code:** `crates/openehr-flat/src/validation/subtype.rs:14-161` — a hand-maintained `descendants` map; unknown pairings are treated as conformant (`conforms` returns `true`). The module header notes the BMM-generated model arrives with the AQL engine (P16).
- **Problem:** Some wrong-typed content conforms silently until the generated RM model lands (e.g. a novel/renamed abstract slot not in the map). Documented and deliberately permissive to avoid over-rejection.
- **Fix:** Replace with the P16 `emit-rm-model` BMM-derived subtype relation when available; until then, keep the map in sync with the RM slots that appear as WebTemplate `rmType`s.
- [ ] fixed

### F-07-14: DV_INTERVAL / DV_ORDERED cross-value ordering invariants deferred
- **Severity:** info
- **Spec:** `INTERVAL.Limits_consistent` (`lower <= upper`); `DV_ORDERED.Normal_range_and_status_consistency`; `DV_QUANTITY`/ordered comparison uses openEHR ordered-magnitude semantics, not Rust `PartialOrd`.
- **Code:** `crates/openehr-rm/src/data_types/quantity/dv_interval_impl.rs` documents `Limits_consistent` is not implemented (needs the P16 `openehr_magnitude` machinery); `dv_quantity_impl.rs` defers `Normal_range_and_status_consistency`.
- **Problem:** Interval/reference-range ordering and normal-status/normal-range consistency are unchecked pending P16. Legitimately blocked; tracked here so it is closed when magnitude machinery lands.
- **Fix:** Implement once `openehr_magnitude` (P16) exists; add the cross-value checks in the composition validator (they need typed magnitude comparison, not per-node invariants).
- [ ] fixed

### F-07-15: Inherited DV_ORDERED invariants unenforced; DV_ORDINAL / DV_SCALE run no invariants at all
- **Severity:** minor
- **Spec:** `org.openehr.rm.data_types.dv_ordered.adoc` defines four invariants inherited by **DV_QUANTITY, DV_COUNT, DV_PROPORTION, DV_ORDINAL, DV_SCALE**: `Other_reference_ranges_validity` (`other_reference_ranges /= Void implies not …is_empty`), `Is_simple_validity`, `Normal_status_validity` (terminology — see F-07-03), `Normal_range_and_status_consistency` (needs magnitude comparison — see F-07-14).
- **Code:** `crates/openehr-rm/src/data_types/quantity/dv_quantity_impl.rs` and `dv_count_impl.rs` push only the DV_AMOUNT/DV_QUANTIFIED helpers (`push_dv_amount_invariants`) — the DV_ORDERED `Other_reference_ranges_validity` and `Is_simple_validity` are not applied. `DV_ORDINAL`/`DV_SCALE` have **no `*_impl.rs`** and are **absent from the dispatcher** (`validate.rs:352-399`), so they run zero RM invariants (only their nested `symbol/defining_code` CODE_PHRASE is checked via recursion).
- **Problem:** `Other_reference_ranges_validity` (an empty-if-present check applicable to every ordered value) and `Is_simple_validity` are unenforced across all ordered types; DV_ORDINAL/DV_SCALE additionally skip everything.
- **Fix:** Add a shared `push_dv_ordered_invariants` (Other_reference_ranges non-empty, Is_simple) applied by every ordered impl; add `dv_ordinal_impl.rs`/`dv_scale_impl.rs` and dispatch `"DV_ORDINAL"`/`"DV_SCALE"`.
- [ ] fixed

## Hygiene notes

- **RM-invariant pass quality is high** where impls exist: DV_PROPORTION kind/denominator rules, ELEMENT null-flavour XOR + null_reason, DV_AMOUNT accuracy/percent, REFERENCE_RANGE `Range_is_simple`, HISTORY `Events_valid`, ISO-8601 well-formedness (`validate.rs`) all track the spec text accurately, with co-located unit tests. The gaps are omissions (missing classes / missing passes), not wrong logic.
- **`collect-all` (non-fail-fast) design is correct** and matches archie's `RMObjectValidator` — good for `422` bodies that report every violation.
- **The 422-vs-400 split is well-reasoned and spec-cited** in `composition.rs:188-209` (converts-but-does-not-validate → 422). No defect; keep the PORT NOTE.
- **Dispatcher coverage:** `validate.rs:352-399` dispatches ~35 types; notable absentees are `DV_TEXT`/`DV_CODED_TEXT` (F-07-07), `ISM_TRANSITION` (structural presence only; terminology handled separately), and `ITEM_LIST`/`ITEM_TREE`/`ITEM_SINGLE` (no `Valid_structure` — F-07-08).
- **`is_valid_category` vs `is_valid_composition_category`** in `bundle.rs` are aliases — harmless, but pick one to avoid drift.
- **Test surface:** per-rule unit tests (`validation/tests.rs`) are solid for occurrences/cardinality/leaf; add negative tests once F-07-01/02/03 are fixed (contribution-path invalid composition; templateless invalid composition; bad territory/language codes), and a "extra unmatched ENTRY rejected" test for F-07-05.
- **Phase-15 self-audit (F2/F3/F5/F6) is accurate** — this audit confirms each against the vendored spec and escalates F-07-01/02 (bypass paths) above the "edge check" framing the phase file used.
