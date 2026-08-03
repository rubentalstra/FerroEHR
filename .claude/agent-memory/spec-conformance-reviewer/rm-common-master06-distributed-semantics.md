---
name: rm-common-master06-distributed-semantics
description: Verified findings for RM common master06 §6.4 (Semantics in Distributed Systems) — the IMPORTED_VERSION family is now materialised and largely conformant; the live defects are the OAS-required `data` missing from the served IMPORTED_VERSION, the unenforced AMB-89 import-member refusal (silent downgrade), and zero released-wire CNF coverage of an imported version read
metadata:
  type: feedback
---

Verified 2026-08-03 against RM 1.2.0
`docs/specs/openehr/RM/docs/common/master06-change_control_package.adoc`
lines 242–334 (§6.4 = Copying [The Copy Operation · Subsequent Local
Modifications] · Version Merging · Disjoint Merging · Moving Version
Containers).

**SUPERSEDES the §6.2 memory's "we never materialise IMPORTED_VERSION"
claim** — #1689 + #1739 fixed the whole family. Do not re-report it.

**CONFORMANT, do not re-check:**
- `vo_version.wrapped_original jsonb` is the IMPORTED_VERSION discriminator:
  the ROW's `contribution_id`/`audit_id`/`signature` carry the LOCAL act, the
  jsonb carries the foreign `{contribution, commit_audit, signature?}`
  (`migrations/ehr/0001_baseline.sql:376,404,457`).
- `VERSIONED_OBJECT.time_created` + `Last-Modified` + REVISION_HISTORY all read
  the ROW's own audit (`storage/version_repo/meta.rs:132 commit_bounds`,
  `versioning/wire.rs:53 revision_history`) → local chronology (B16).
  `version_at` reads `sys_period` (`storage/version_repo/read.rs:268`), and
  the import stamps a synthetic per-lineage chain from the DB transaction
  timestamp (`versioning/import.rs:245-247,355-364`) → B17/B18 hold.
- Read side: `versioning/wire.rs:285 version_envelope` serves IMPORTED_VERSION,
  `:333 original_version` UNWRAPS for the extract export (X_VERSIONED_OBJECT
  is `List<ORIGINAL_VERSION>`), ETag falls through to `item.uid` and
  Last-Modified takes the WRAPPER's audit
  (`ferroehr-rest/src/api/ehr/mod.rs:94,120`).
- Merge provenance is PRODUCE-only and the refusal is real: 400 at
  `versioning/contribution.rs:385`, CNF
  `commit_contribution-merge_provenance_member`, test
  `service_branching.rs:341`. The §6.3 memory's "accepted on the wire" defect
  is FIXED. So is `lifecycle_state` optional (`contribution.rs:394`).
- EHR clone: `service/message/import.rs:113,128-130` reuses the source ehr_id
  and stamps OUR `effective_system_id` (S11, ehr master04 L209).

**VERIFIED DEFECTS:**
- **The served IMPORTED_VERSION omits the OAS-required `data`.**
  `versioning/wire.rs:464 build_imported_version` emits only
  `{_type, contribution, commit_audit, item, signature?}`, but every released
  `UMImportedVersionOf*` lists `data` under `required`
  (`vendor/rest-oas/ehr-html.openapi.yaml:3752-3775`). RM makes `data` a
  DERIVED function and ITS-XML's `IMPORTED_VERSION` has no `data` element
  (`schemas/xml/.../Common.xsd:159`) — a genuine OAS-vs-RM/ITS-XML conflict,
  UNREGISTERED in ambiguities.yaml.
- **AMB-89's stated refusal is not implemented — it silently downgrades.**
  `versioning/contribution.rs` reads only `data`/`commit_audit`/
  `other_input_version_uids`/`lifecycle_state`/`signature`/
  `preceding_version_uid` from a member (`:321,346,385,414,1006`); `_type`,
  `item` and `uid` are never read, so an `_type: IMPORTED_VERSION` member with
  a `data` copy COMMITS as a local ORIGINAL_VERSION under a freshly minted
  uid. Asymmetric with AMB-196 one property over.
- **Zero coverage of an imported version on a RELEASED route.** Every
  IMPORTED_VERSION CNF case is on the `/message` extension
  (`schedule/messaging/import_ehr-local_chronology`,
  `export_ehrs-reexport_of_an_import`); nothing asserts the served
  IMPORTED_VERSION shape, ETag, Last-Modified or XML root on
  `get_versioned_composition/version`, and no case combines import with an
  as-of read (B17's illusion test exists only in prose).
- `uq_vo_version_tree UNIQUE (vo_id, creating_system_id, trunk_version, …)`
  admits `sysA::2` AND `sysB::2` as two trunk rows of one container — the
  trunk position is not unique. Blocked on both live write paths, open to the
  archive load.
- Stale SQL comment: `0001_baseline.sql:378` still says
  `other_input_version_uids` is "accepted on the wire".
- `versioned_object_impl.rs:31-34` claims `commit_original_merged_version` is
  realized by `versioning::change` — that path can never produce a merge
  (provenance is refused there).

**A9 is real and we sit on the §6.4.1.2 side:** `versioning/change.rs:340`
forks a BRANCH whenever the tip's `creating_system_id` differs from ours, so
§6.4.4's post-move trunk continuation (`sysB::3` after `sysA::2`) is
unrepresentable through the commit engine. Import can write it (per-VERSION
`creating_system_id`), local commits cannot.
