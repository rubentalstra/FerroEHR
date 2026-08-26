---
name: rm-common-master04-class-tables
description: Verified findings for RM common master04 §4.3 (the nine generic-package class tables) — attestation description collapse, FLAT empty participation function, PARTICIPATION.time zero coverage, and what is already adjudicated (AMB-68 ordering)
metadata:
  type: feedback
---

Verified 2026-08-02 against RM 1.2.0. §4.3 is a bare include list (master04
lines 89–108) pulling nine `docs/specs/openehr/RM/docs/UML/classes/
org.openehr.rm.common.{party_proxy,party_self,party_identified,party_related,
participation,audit_details,attestation,revision_history,
revision_history_item}.adoc` files — read THOSE, master04 itself states no
class table.

**ALREADY ADJUDICATED — do NOT re-find:**
- REVISION_HISTORY ordering self-contradiction (Purpose "most-recent-first"
  vs `items` Meaning "most-recent-last") is **AMB-68**, `fixed_handling`,
  upstream #1512. The two Post-conditions (`items.last.version_id.value`,
  `items.last.audits.first.time_committed.value`) settle it: most-recent-LAST
  governs. Both builders emit oldest-first (`ORDER BY v.sys_version`,
  `storage/version_repo/meta.rs:71`) → CONFORMANT.
- `ATTESTATION.items` scope conflict = AMB-180 (`versioning/attestation.rs`
  NOTE); `proof` opacity = AMB-181. Both properly cited in code now.
- The §4.1/§4.2 defects I logged earlier are FIXED: the four generic-package
  `*_impl.rs` headers now cite the spec not archie; `flat/build.rs:1165` cites
  the UML class file; `ferroehr-rest/api/demographic/mod.rs:176` cites the real
  file; ATTESTATION now has 6 CNF cases; `versioning/wire.rs` revision history
  is typed through the codec.

**MODEL IS EXACT** for all nine classes (`crates/openehr-rm/src/common/generic/*.rs`
+ `model/data.rs`): every attribute name/type/multiplicity matches. Do not
re-walk it.

**VERIFIED DEFECTS (§4.3):**
- **`complete_attestation` collapses `description` to plain DV_TEXT**
  (`app/ferroehr/src/versioning/attestation.rs:363-376`:
  `d.as_str().or_else(|| d.get("value")…)` then `.map(dv_text)`), losing
  `defining_code`/mappings/formatting. Asymmetric with the sibling fix in
  `versioning/contribution.rs:767-775` (`decode_description`, test
  `coded_description_round_trips`). Wire-visible on
  `…/versioned_composition/revision_history`. ITS-REST `UpdateAttestation`
  is `allOf UpdateAudit` whose `description` is `$ref DvText`
  (`crates/openehr-its/vendor/rest-oas/ehr-codegen.openapi.yaml`), so a coded
  description IS expressible.
- **FLAT ctx fabricates `function: {"_type":"DV_TEXT","value":""}`**
  (`crates/openehr-its/src/flat/ctx.rs:346-349`,
  `function.unwrap_or("")`) when `ctx/participation_name`/`_id`/`_identifiers`
  is given without `ctx/participation_function`. Violates
  `DV_TEXT.Valid_value` (`not value.is_empty`); fails loud as a misattributed
  422, but a missing mandatory attribute must not be fabricated.
- **`PARTICIPATION.time` (0..1 DV_INTERVAL<DV_DATE_TIME>) has ZERO instances**
  anywhere (scripted over every JSON under `crates/openehr-its/tests/vendor`,
  `corpus`, `app/ferroehr/tests`: 212 PARTICIPATION
  nodes, key set always exactly `function/mode/performer`), and is not
  expressible through FLAT (`ctx.rs resolve_participations` builds no `time`).
  Contrast: `other_participations` IS covered in canonical JSON
  (`crates/openehr-its/tests/vendor/openehr_sdk/composition/canonical_json/
  other_participations.json`) — the sibling-audit claim that it is TDD-XML-only
  is FALSE.
- **`demographic_revision_history` never joins attestations**
  (`app/ferroehr/src/service/demographic/support.rs:170` hardcodes a 1-element
  `audits` vec) vs the EHR path (`versioning/wire.rs:84-89`). Vacuous today
  (demographic commits pass `attestations: Vec::new()`), latent.
- **`REVISION_HISTORY.most_recent_version()` /
  `most_recent_version_time_committed()` are unimplemented** (no
  `revision_history_impl.rs`); their semantics are re-derived inline at
  `ferroehr-rest/api/demographic/mod.rs:184` and `versioning/wire.rs:76`.
- Emitter nit: the generated header "Hand-written spec functions/invariants
  live in the sibling `*_impl.rs`" is emitted on `party_self.rs`,
  `participation.rs`, `revision_history.rs`, `revision_history_item.rs`, none
  of which HAS a sibling.
- Register row `REVISION_HISTORY_ITEM.Audit_valid` = "owned by the versioning
  layer's commit path" implies an active check; it is satisfied by
  construction only.
- `versioning/audit.rs:444` cites "RM common master04 §Audit Details" for
  `System_id_valid` — that §4.2 narrative heading does not state it; the
  invariant is in the class table.

**#1623 (`PARTY_IDENTIFIED.Identifiers_valid` Unrealized) is a CHOICE, not an
impossibility:** the wire-boundary venue already realizes the identical
present-but-empty rule for 10 class/attribute pairs
(`crates/openehr-its/src/flat/validation/mod.rs:872-907` RULES table) and
`versioning/audit.rs:483` already reads the raw `identifiers` array. The class
table lists it as a coequal peer of `Name_valid`.

**CNF gaps in this package:** no negative case for
`PARTICIPATION.Function_valid` (coded function out of `participation_function`),
none for `PARTY_RELATED.Relationship_valid` (out-of-group relationship, either
as content performer or as audit committer), none for the committer's
`PARTY_IDENTIFIED.Basic_validity`/`Name_valid`, none for
`ATTESTATION.attested_view`. `Mode_valid` and the ATTESTATION family ARE
covered.
