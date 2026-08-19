---
name: rm-common-ch234-fix-verification
description: Re-verification (2026-08-19) of the RM Common ch.2-4 audit fixes (#898/#899/#900) — which prior-review defects really closed, and the two enforcement-proof weaknesses that remain
metadata:
  type: feedback
---

Verified first-hand 2026-08-19 against the current tree. **All 24 fix issues
(#1479-#1481, #1616-#1622, #1627-#1631, #1636-#1640, #1644-#1647) closed by a
MERGED PR** (#1482, #1626, #1633, #1643, #1649). The audit/section issues
(#898/#899/#900/#983-#987) are hand-closed record issues with a posted
checklist comment — that is the intended #818 cadence, not a defect.

**CLOSED — do NOT re-report these from my older ch.3/ch.4 memories:**
`Archetype_node_id_valid` now model-derived over the concrete LOCATABLE closure
(`openehr-rm/src/v1_2/validate.rs:158` `locatable_node_id_violation` reading
`model::descendants("LOCATABLE")`, called at `openehr-its/src/wire_validate.rs:269`;
fast path declines for every unarmed LOCATABLE, `fast.rs:104-117`);
the RM pass runs on EVERY commit kind (`service/ehr/validation.rs:232-258`
`validate_for_commit`, both the CONTRIBUTION lane `versioning/contribution.rs:623,664`
and the direct lanes `status.rs:136`, `service.rs:76`, `directory.rs:89,296`);
root-identity + Links_valid (`validate.rs:390` in the recursive walk
`rm_instance/mod.rs:356`; `links: Option<NonEmptyVec<Link>>` makes `[]`
unrepresentable, so `NONEMPTY_LIST_RULES` is legitimately EMPTY);
the invariant register is DERIVED + seeded-negative-tested (`emit_validate.rs:161`,
`emitter_invariants.rs:682,746`), `ATTESTATION.Items_valid` archie row gone;
ATTESTATION commit_audit no longer flattened on EITHER lane (`audit.rs:341-361`,
`contribution.rs:1020`, import `message/import.rs:488`); coded descriptions
survive (`attestation.rs:391,413`; `audit.description` is jsonb + CHECK);
`"EHRbase"` identity literal gone (one `DEFAULT_SYSTEM_ID`, `service/mod.rs:115`);
all 35 `review doc` citations purged; REVISION_HISTORY functions realized
(`revision_history_impl.rs:38,49`); the misleading `*_impl.rs` headers on
party_self/participation/revision_history_item are gone. All promised CNF cases
exist (feeder_audit x3 + same_content, 5 LINK, 10 attestation, subject_external_ref,
participation_time_roundtrip + 4 invariant negatives + accepting twins).

**STILL OPEN (report these):**
- **The comment guard cannot see a NOTE in a doc comment.**
  `scripts/checks/comment-style.sh:119` gates the NOTE budget on `if (is_line)`
  and `is_doc` (`///`/`//!`) is excluded — so adjudication essays migrate into
  doc comments. 8+ over-budget in the ch.3/4 files alone; worst
  `openehr-rm/src/v1_2/validate.rs:341` (33 lines), `versioning/wire.rs:182` (17),
  `versioning/attestation.rs:15` (20), `paths.rs:41` (16).
- **#1616's totality is not test-proven**: `rm_validation.rs:214-225` iterates a
  HARDCODED 4-class fixture list (ITEM_TREE/LIST/SINGLE/EHR_STATUS) while its
  doc comment `:232` claims the demographic + EHR_EXTRACT LOCATABLEs — 22
  closure members never instantiated. Runtime is correct; the proof is not.
- **Register venue claims are substring-checked**: `emitter_invariants.rs:725`
  `text.contains(&a.name)` — a file that only MENTIONS an invariant satisfies an
  Impl/Wire/App venue claim (the Core arm is strong; these are not).
- **Section records are theme-level, not row-by-row**, and self-contradict on
  finding counts (#984 8 vs 7, #985 7 vs 5, #986 11 vs 5, #987 10 vs 4). No
  durable artifact records the CONFORMANT rows' evidence.
- **En route**: `openehr-its` has NO `canonical_json_literals` gate (it exists
  only in `app/ferroehr` + `app/ferroehr-rest` tests) while `flat` hand-builds
  34 `_type`-carrying `json!` fragments in production
  (`flat/map/structures.rs:435,459-464`, `flat/build.rs`).
