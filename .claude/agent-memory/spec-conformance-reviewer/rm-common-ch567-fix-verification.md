---
name: rm-common-ch567-fix-verification
description: Re-verification (2026-08-19) of the RM Common ch.5-7 audit fixes (#901/#902/#903) — every fix issue closed by a merged PR, which prior-review defects really closed, and the four residues that remain
metadata:
  type: feedback
---

Verified first-hand 2026-08-19. **All 38 fix issues closed by a MERGED PR**
(#1658, #1669, #1676, #1689, #1757, #1779, #1785, #1804) — zero hand-closes.
Sibling of [[rm-common-ch234-fix-verification]].

**CLOSED — do NOT re-report from my older ch.5/6/7 memories:**
- FOLDER.details: the JSON write path IS typed-decoded now
  (`ferroehr-rest/src/overview/negotiate.rs:594 rm_value::<Folder>` on BOTH
  branches, called at `api/ehr/directory.rs:85,151`), and the model-driven
  `check_declared_slot_type` (`openehr-rm/src/v1_2/validate.rs:286`) runs at
  EVERY slot of the walk (`openehr-its/src/rm_instance/mod.rs:388-395,402,422`)
  — so the predicted JSON-201-then-XML-500 asymmetry is structurally gone. The
  bespoke `EHR_STATUS.other_details` check is gone too (generic pass covers it).
- Served XML default is **V2** (`negotiate.rs:366`, owner ruling #1666); the
  self-deriving XSD gate exists (`openehr-its/tests/it/xml_xsd_validity.rs`).
- `VERSIONED_OBJECT` declares **16** functions (12 query + 4 committal) — my
  older "14" was wrong; `versioned_object_impl.rs` is a documentation-only
  module explaining why none is value-realizable, and it no longer claims
  `commit_original_merged_version` is realized.
- 523+data refused on both content arms (`versioning/change.rs:565,622` →
  `lifecycle.rs:111`); `lifecycle_state` required on every non-attest
  CONTRIBUTION member (`contribution.rs:516`); DELETE with a non-523
  `openehr-version` refused 400 (`overview/committal.rs:206`, wired on
  composition/directory/party/relationship deletes).
- IMPORTED_VERSION-shaped member refused by a CLOSED member vocabulary
  (`contribution.rs:289-355`: 7 declared keys, `item`/`uid` named refusals,
  `_type` limited to UPDATE_VERSION|ORIGINAL_VERSION).
- Attesting an IMPORTED_VERSION refused (`versioning/attestation.rs:120`, over
  `wrapped_original IS NOT NULL` from `storage/version_repo/attestation.rs:98`).
- `uq_vo_version_trunk_position` added; archie-as-authority residue gone (one
  prior-art mention left, correctly labelled, `validation/opt/rm_conformance.rs:142`);
  the latent VersionedParty `{local,EHR,ehr_id}` arm is gone with a NOTE.
- Tags: generated `UpdateItemTag` (deny_unknown_fields) IS the decode seam on all
  three PUTs (`negotiate::typed_json_vec`); `pending_item_tags`
  (`api/ehr/mod.rs:404`) validates BEFORE the commit on all four wrapper seams;
  'FOLDER' in `ck_item_tag_target_type`; empty collection emits NO echo header
  (`overview/params.rs:376`); all 23 tag ops ATNA-classified
  (`ferroehr-rest/src/system_log/classify.rs:146`).

**RESIDUES (report these):**
- **Contradictory lifecycle on a CONTRIBUTION delete member is still silently
  discarded**: `contribution.rs:684-709` builds `Change::Delete` without
  `lifecycle_state`, and nothing compares it to `change_type 523` — the exact
  leniency class #1745 names, fixed only on the header seam.
- **#1789's strictness refusals have no CNF case** — asserted only in
  `ferroehr-rest/tests/it/item_tag_http.rs:168,197,237`. The runner CAN express it
  (`tags:` is passed through raw, see `composition_tags_update-non_array_body`).
- **Audit-record honesty**: §6.3/§6.4 close comments claim "every entry classified
  with file:line + test/case evidence (the walk comment above)" while the posted
  walk comments carry only aggregate counts + named exceptions; #995/#996/#997
  carry ticked criteria and ZERO comments (the ch.7 record lives on #903 only);
  #988/#989/#990 are theme-level (pre-#1784, same class as the adjudicated #991).
- `contribution.rs:1115` stale doc: "None → the commit path defaults to 532" is
  unreachable since :516.
