---
name: rm-common-master06-basic-semantics
description: Verified findings for RM common master06 §6.2 (Basic Semantics) — the IMPORTED_VERSION-never-materialized family (foreign commit times leak into time_created/Last-Modified/revision history, foreign contribution ref dropped), the unregistered §Digital Signature TBD, and what IS conformant in versioning/ + storage/version_repo/
metadata:
  type: feedback
---

Verified 2026-08-02 against RM 1.2.0
`docs/specs/openehr/RM/docs/common/master06-change_control_package.adoc`
lines 19–126 (§6.2 = Typing · Versioned Objects · Version and its Subtypes ·
The 'Virtual Version Tree' · Contributions · Committal and Audits · Digital
Signature · Attestation). Figures under `RM/docs/common/diagrams/` ARE vendored
now (`version_signature.png`, `instance_view_of_versioned_data.png`) — read
them; version_signature.png confirms the signed object is the whole
ORIGINAL_VERSION with `signature = (Void)`.

**CONFORMANT, do not re-check:**
- The signature signs the REASSEMBLED served bytes, not the client's submitted
  data (`versioning/change.rs:756-770` `version_signature` → `reassemble(&r.rows)`),
  so commit-time and read-time canonical forms are identical by construction —
  the obvious "client JSON vs reassembled JSON drift → strict 5xx on every read"
  trap is already solved.
- `canonical_form_of_json` drops top-level `signature` only (RFC 8785/JCS),
  `crates/openehr-rm/src/common/change_control/version_impl.rs:57`.
- `version_at_time` reads `sys_period` (LOCAL chronology), not the audit
  (`storage/version_repo/read.rs:248`) — §Copying's medico-legal requirement holds
  on THAT path.
- `Attestations_valid` / `Other_input_version_uids_valid` (never emit an empty
  array) — `versioning/wire.rs:281,361`.
- Attestations are per (vo_id, sys_version) so they never carry forward to a new
  version (§Attestation last paragraph).
- Generated `ImportedVersion<T>` = `{contribution, signature, commit_audit, item}`
  matches the XSD exactly (no `uid` field) — NOT a codegen gap.
- Contribution atomicity: one `tx` for the whole version set
  (`versioning/contribution.rs:589-628`).
- AMB-69 VERSIONED_PARTY `owner_id` IS NOW FIXED (`service/demographic/support.rs:267`
  emits `{local, SYSTEM, system_id}`, matching `service/message/export.rs:486`).
  The stale claim in `rm-common-master06-change-control-overview.md` is corrected.

**VERIFIED DEFECTS (the IMPORTED_VERSION family is the big one):**
- **We never materialise IMPORTED_VERSION, so foreign commit times leak into
  the container's own metadata.** `versioning/import.rs:331` stores the WRAPPED
  original's `commit_audit` as the version row's audit; `storage/version_repo/
  meta.rs:132 commit_bounds` derives `VERSIONED_OBJECT.time_created` + the
  `Last-Modified` header from `audit.time_committed` → an imported container
  reports the SOURCE system's instants. §Copying states the exact opposite
  ("the commit times always reflect the local (more recent) act of committal …
  rather than giving the illusion that recently copied Versions were there
  earlier than the time of local committal"), and §Committal grounds it ("from
  the point of view of the version container, the local commit audit and
  Contribution always correspond to the local act of committal"). Same leak in
  REVISION_HISTORY (`versioning/wire.rs:83`).
- **The wrapped original's `contribution` ref is DISCARDED** — `parse_imported_version`
  (`service/message/import.rs:487`) never reads it; the served ORIGINAL_VERSION's
  `contribution` names the LOCAL import contribution, i.e. a factual misstatement
  of "Contribution in which this version was added". §Committal requires both
  facts to survive.
- **`versioning/import.rs:17-18` cites a sanction that does not exist**:
  "master06 §Committal sanctions a non-distributed holder keeping only the
  ORIGINAL_VERSION content" — no such sentence in §Committal. AMB-89 covers only
  the CONTRIBUTION-route write shape, NOT the EHR-Extract import read shape.
- **The §Digital Signature `[.tbd]` is UNREGISTERED.** master06 marks the exact
  serialisation "To Be Determined … ODIN might be preferred"; we chose canonical
  JSON + RFC 8785 and stamp a `sha256:` prefix on the digest form. The parallel
  master04 attestation-proof TBD IS registered (AMB-181) — this one is not.
  Also `signature/signer.rs:17-18` quotes master06 as saying signature
  "algorithms are self-describing"; master06's actual words are different and
  the "self-describing" phrasing is BASE `architecture_overview/master07-security.adoc`
  §Digital Signature.
- **The signature does not cover commit-time-present attestations.**
  `build_original_version` has no `attestations` parameter; §Digital Signature
  says "the entire Version object" minus `signature`, and our commit route DOES
  accept accompanying attestations (`change.rs:709`).
- **`aggregate_change_type` (`contribution.rs:805`) misses the 250 case**:
  spec names `250|amendment|` for "a mixture of amendments and deletions"; we
  emit 251 for every non-uniform set. Spec calls the value approximate, so low.
- **Internal-doc citation in a NOTE**: `versioning/object_version_id.rs:144`
  points at `docs/spec-audit/rm-common-change-control`, a path that DOES NOT
  EXIST. Hard-rule violation.
- **Dangling `§Version tree`** (real heading: "The 'Virtual Version Tree'") still
  in 7 files incl. `object_version_id.rs:10,17` — and :17's quote ("To support
  branching, a further pair of numbers is added") is actually from §Version
  Identification → Local Versioning (line 226), a different chapter entirely.
- **Spec-internal slip, unregistered**: §Contributions writes
  "`ATTESTATION._commit_audit_._change_type_` is set to `666|attestation|`" —
  ATTESTATION IS an AUDIT_DETAILS and has no `commit_audit`.
- **`VERSION.Preceding_version_uid_validity` is unsatisfiable for a branch's
  first version**: `is_first ⟺ trunk_version = "1"` (BASE `version_tree_id.adoc`
  `Is_first_validity`), so `1.1.1` is `is_first` yet always has a preceding.
  Unregistered.
- **Spec functions absent from `openehr-rm`**: only `canonical_form` exists in
  `version_impl.rs`. `IMPORTED_VERSION.{uid,preceding_version_uid,lifecycle_state,
  data}` (declared "(effected)" with explicit Posts), `VERSION.{owner_id,is_branch}`,
  `ORIGINAL_VERSION.is_merged`, and all 14 `VERSIONED_OBJECT` functions have no
  `*_impl.rs` realization.
- **Latent contradictory `owner_id`**: `versioning/wire.rs:222` VersionedParty arm
  still emits `{local, EHR, ehr_id}`, reachable only via
  `service/message/export.rs:384` + `versioned_rm_type` (:93 maps the five party
  kinds) — contradicts the two fixed party surfaces.
