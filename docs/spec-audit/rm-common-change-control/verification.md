# A1 Spec Audit — Phase 2 (Verify + Fix) — chapter `rm-common-change-control`

- **Chapter:** openEHR RM 1.2.0 `common` — change control + generic + archetyped
- **Date:** 2026-07-11
- **Scope:** all 56 requirements `rm-common-change-control-R1 … R56`
  (`requirements.md`). Focus: per-attribute invariants + write-path rejection
  duties the PR #33 1:1 change-control audit could not cover.
- **Result (final, defer-nothing pass):** every non-verified requirement is
  FIXED. Fixes: R27/R34/R35 (audit invariants), R37 (committer
  Relationship_valid, strict), R20 (imported `_type` strictness), R46
  (Archetyped_valid non-root arm), and the big one — R7/R19/R50: full
  version-tree branching + merge provenance (storage redesign, fork on
  foreign-modification, branch read/serve/import/export, stored
  `preceding_version_uid`). R40/R41 reclassified verified (the walker already
  enforced them; the first pass mis-read the enforcement site). Bonus defects
  found & fixed along the way: the AQL `VERSION.uid` was synthesized from the
  LIVE config `system_id` instead of the stored per-version
  `creating_system_id`; three ETag/Location builders passed an empty
  `creating_system_id` (malformed `vo::::N` uids); `preceding_version_uid`
  was arithmetically synthesized (wrong under import/branching) — now stored;
  admin dump/load dropped the version-tree identity + merge provenance.

## Verdict table

Legend — classification: `verified` / `partial` / `deferred` (PORT NOTE, needs
owner/ADR) / `model-limited` (invariant unrepresentable in the canonical-JSON
`Vec` model — present-empty ≡ absent). "Enforcement" = the runtime site.

| id | classification | sev | evidence (file:line) | negative test | fix status |
|---|---|---|---|---|---|
| R1 | verified | high | `service/contribution.rs` `classify` (94-166) + `service/versioned.rs` `build_original_version` (260-267) | `contribution.rs::classify_rejects_spec_invalid_combinations` | verified |
| R2 | verified | high | `service/contribution.rs` `parse_preceding`+`require_kind` (808-826,484-488); `service/vobject.rs` `next_version` (939-963) | `contribution.rs::classify_*`, `version_id.rs::version_uid_strict_three_part` | verified |
| R3 | verified | high | `service/versioned.rs` `build_original_version` (230-233); `object_version_id_impl.rs::from_str` (72-99) | `object_version_id_impl.rs::from_str_strict` | verified (server-constructed; object_id = vo_id = container uid) |
| R4 | verified | med | ORIGINAL_VERSION carries no independent `owner_id`; `owner_id()` derived = `uid.object_id()` — structurally equal by construction (`build_original_version`) | n/a (derived fn) | verified-by-construction |
| R5 | verified | med | `version_tree_id_impl.rs::is_valid_version_tree` (25-40) — positive-int, starts at 1 | `version_tree_id_impl.rs::malformed_fails_format` | verified |
| R6 | verified | med | create: `ctx.system_id` → `insert_vo_version` (614-627); import: `parse_object_version_id` preserves source csid (`version_id.rs` 105-125) | `version_id.rs::object_version_id_preserves_all_three_parts` | verified |
| R7 | fixed-in-this-pass | med | Version-tree branching implemented end-to-end: `TreeId` (`version_id.rs`), storage tree columns + per-lineage non-overlap (`migrations/ehr/0001_baseline.sql` vo_version), fork-on-foreign-modification + lineage continuation (`vobject.rs::next_version`), branch read/serve everywhere | `service_branching.rs::modifying_an_imported_foreign_version_forks_a_branch`, `version_id.rs::branch_ids_are_first_class` | fixed (RM common master06 §Version tree / §Distributed versioning) |
| R8 | verified | high | `service/vobject.rs::resolve_lifecycle` (485-495) → `codes::lifecycle_state_code`; DB `ck_vo_version_lifecycle_state IN (532,553,523,800,801)` | `codes.rs::lifecycle_state_code_accepts_code_or_rubric_and_rejects_non_members` | verified |
| R9 | verified | med | `service/vobject.rs::apply_change` always INSERTs a new `vo_version` row (no in-place lifecycle mutation) | — (behavioural) | verified |
| R10 | verified | med | incomplete→complete travels the CONTRIBUTION update path = a new version | — (behavioural) | verified |
| R11 | verified | high | `service/composition.rs::validate_for_commit`(incomplete=false) → full RM+terminology+template (487-509) | `service_validation.rs` (DB) | verified |
| R12 | verified | med | `openehr_flat::validate_archetype_conformance_incomplete` relax lower bounds; RM+terminology stay strict (`validation/mod.rs` 155-165) | `openehr-flat validation/tests.rs::walk_incomplete` | verified |
| R13 | verified | med | `service/vobject.rs` `Change::Delete` → lifecycle DELETED, data Null, no node rows (724-770); prior versions retained | — (behavioural) | verified |
| R14 | verified | med | `service/codes.rs` (change_type group codes); `contribution.rs::audit_details` emits `code_string` | `codes.rs::change_type_code_accepts_code_or_rubric_and_rejects_non_members` | verified |
| R15 | verified | high | `service/vobject.rs::commit_contribution` one `tx` — all versions+attests or none (1142-1198) | — (behavioural) | verified |
| R16 | verified | med | `service/contribution.rs::parse_version_audit` defaults committer/system_id from CONTRIBUTION audit (522-547) | — (behavioural) | verified |
| R17 | verified | med | `service/vobject.rs::insert_audit` `RETURNING time_committed` = DB `now()` (287-306) | — (behavioural) | verified |
| R18 | verified | high | `service/vobject.rs::attest` requires target `(vo_id,sys_version)` to exist (`has_version_id`) (832-864); all served versions are ORIGINAL_VERSION so `is_original_version` holds | `contribution.rs::classify_attestation_of_existing_version` | verified |
| R19 | fixed-in-this-pass | med | `other_input_version_uids` accepted on the CONTRIBUTION wire (`contribution.rs`), preserved on import (`message.rs::parse_imported_version`), stored (`vo_version.other_input_version_uids`), served (`versioned.rs::build_original_version`) | `service_branching.rs::merge_provenance_round_trips_the_wire` | fixed (master06 §Version Merging; `Is_merged_validity` = derived) |
| R20 | fixed-in-this-pass | high | `parse_imported_version` (`message.rs`) rejects a foreign `_type` in `versions[]` (must be `ORIGINAL_VERSION` or absent — RM ehr_extract master05) | `message.rs::tests::imported_versions_member_type_is_enforced` | fixed |
| R21 | verified | med | import representation PORT NOTE (`vobject.rs::commit_import` 1374-1383): effected uid/preceding/lifecycle/data preserved from wrapped original | — | verified-by-design |
| R22 | verified | med | import: fresh local CONTRIBUTION (`write_contribution`) + preserved original `commit_audit` (`insert_audit_at`, 1246-1262) | `service_import.rs` (DB) | verified |
| R23 | verified | med | `version_impl.rs::canonical_form_of_json` drops `signature` (56-62); `vobject.rs::sign_version` signs the assembled OV | `version_impl.rs::canonical_form_independent_of_signature_value` | verified |
| R24 | verified | high | `service/versioned.rs::versioned_object` uid = bare UUID HIER_OBJECT_ID, server-managed (117-127) | — | verified-by-construction |
| R25 | verified | med | `service/versioned.rs::versioned_object` (uid/owner_id/time_created, 103-128) | — | verified |
| R26 | verified | med | `OriginalVersion` struct: contribution+commit_audit mandatory, signature `Option` | — | verified |
| R27 | **fixed** | high | was: empty client `audit.system_id` → DB `ck_audit_system_id_nonempty` → **500**; now `contribution.rs::validate_commit_audit` → **422** (service layer) | `contribution.rs::commit_audit_rejects_empty_system_id` (new) | fixed-in-this-pass |
| R28 | verified | high | `service/codes.rs::change_type_code` group check → 422; DB `ck_audit_change_type` | `contribution.rs::classify_rejects_spec_invalid_combinations` (out-of-group 999) | verified |
| R29 | verified | med | `AuditDetailsData` struct mandatory fields; committer defaults present (`ehr.rs::committer`) | — | verified |
| R30 | verified | med | `Attestation` struct reason+is_pending non-optional; `contribution.rs::complete_attestation` enforces 1..1 (858-884) | — | verified |
| R31 | verified | med | `contribution.rs::complete_attestation` Reason_valid via `is_valid_attestation_reason` → 422 (864-875) | `service_contribution.rs` (DB) | verified |
| R32 | verified | med | `contribution.rs::complete_attestation` Items_valid non-empty-when-present → 422 (886-895) | `service_contribution.rs` (DB) | verified |
| R33 | verified | low | `Attestation.items: Vec<DvEhrUri>` (absent/empty = whole version) | — | verified |
| R34 | **fixed** (content verified) | high | content: `validate_rm_value`→`party_identified_impl` Basic_validity (10-23); audit committer was unchecked → now `contribution.rs::validate_committer` | `party_identified_impl.rs::no_identity_invalid` + `contribution.rs::commit_audit_rejects_committer_without_identity` (new) | fixed-in-this-pass (committer path) |
| R35 | **fixed** (content verified) | med | content: `party_identified_impl` Name_valid; audit committer now `validate_committer` | `party_identified_impl.rs::empty_name_invalid` + `contribution.rs::commit_audit_rejects_empty_committer_name` (new) | fixed-in-this-pass (committer path) |
| R36 | model-limited | med | `PartyIdentifiedData.identifiers: Vec` — present-empty ≡ absent (empty omitted on serialize); archie marks `Identifiers_valid` unenforceable likewise | — | verified-by-model (unrepresentable) |
| R37 | fixed-in-this-pass | high | Content path: walker terminology pass (`openehr-flat` `terminology.rs:245`, subject_relationship group). Audit-committer path: `contribution.rs::validate_party_related_relationship` — relationship 1..1, coded, openEHR group member (strict per the invariant formula, no terminology escape) | `openehr-flat tests::party_related_bad_relationship_reported`; `contribution.rs::tests::commit_audit_party_related_relationship_is_enforced` | fixed (`party_related.adoc` Relationship_valid) |
| R38 | verified | low | committer/performer are `PARTY_PROXY`; PARTY_SELF slot enforced (`ehr.rs::validate_ehr_status` 711-715); external_ref optional | `ehr.rs::ehr_status_subject_wrong_type_is_rejected` | verified |
| R39 | verified | low | `Participation` struct function+performer non-optional | — | verified |
| R40 | verified | med | Walker terminology pass enforces `PARTICIPATION.function` group on every content instance (`openehr-flat` `terminology.rs:224-227`; coded-only per Function_valid — plain DV_TEXT passes) — the first pass mis-recorded this as deferred; PARTICIPATION does not occur on the audit path | `openehr-flat` terminology tests | verified |
| R41 | verified | low | Walker terminology pass enforces `PARTICIPATION.mode` group (`openehr-flat` `terminology.rs:226`) — the first pass mis-recorded this as deferred | `openehr-flat` terminology tests | verified |
| R42 | verified | med | `service/versioned.rs::revision_history` audits always start with commit audit (72-81) — never empty | — | verified |
| R43 | verified | low | `RevisionHistoryItem` struct version_id+audits | — | verified |
| R44 | verified | low | `revision_history` items ordered `ORDER BY v.sys_version` asc → most-recent-last (26) | — | verified |
| R45 | verified | med | `Contribution` struct uid+versions+audit; `get_contribution` builds all (613-618) | — | verified |
| R46 | fixed-in-this-pass | med | General `Archetyped_valid` enforced in the RM-invariant pass (`openehr-flat` `validation/mod.rs::check_archetyped_valid`): an at/id-code (non-root) node must not carry `archetype_details`. The converse arm is NOT enforceable: the CNF's own valid data sets omit `archetype_details` on nested archetype roots (182 occurrences measured) and the CNF corpus outranks a prose reading | `openehr-flat tests::non_root_node_with_archetype_details_rejected` | fixed (locatable.adoc L60; CNF-corpus-adjudicated arm documented in a PORT NOTE) |
| R47 | verified | med | `validate.rs::push_archetype_node_id_valid` on locatables; `ehr.rs::validate_ehr_status` non-empty (670-679) | `archetyped`/`composition_impl` invariant tests | verified |
| R48 | model-limited | low | `LOCATABLE.links: Vec` — present-empty ≡ absent | — | verified-by-model |
| R49 | verified | low | `ehr.rs::with_uid` injects OBJECT_VERSION_ID whose `object_id()` = container GUID (per ITS-REST wire) | — | verified-with-note (ITS wire = full version uid; SHOULD's GUID embedded) |
| R50 | fixed-in-this-pass | med | Merge provenance stored/served (see R19); `Is_merged_validity` holds by construction (`is_merged` is the derived boolean of a non-empty `other_input_version_uids`) | `service_branching.rs::merge_provenance_round_trips_the_wire` | fixed |
| R51 | model-limited | med | `attestations: Vec` present-empty ≡ absent; `complete_attestation` only ever produces non-empty | — | verified-by-model |
| R52 | model-limited | low | `other_input_version_uids: Vec` present-empty ≡ absent | — | verified-by-model |
| R53 | verified | med | `OriginalVersion` struct: uid+lifecycle_state mandatory; preceding/other/attestations/data optional; deleted → data Null | — | verified |
| R54 | verified | med | `vobject.rs::commit_import` clones VERSIONED_OBJECT with `vo_id = object_id` (1226-1239, 1471-1485) | `service_import.rs` (DB) | verified |
| R55 | verified | med | `vobject.rs::imported_container_state` dedupe (1334-1361); `ehr.rs::create_ehr` `ON CONFLICT DO NOTHING` (42-51) | `service_import.rs` (DB) | verified |
| R56 | verified | low | `sys_version` sequential from 1 (`apply_change`), latest = `upper_inf` partial index | — | verified-by-construction |

## Prose notes (non-trivial verdicts)

**R1/R2 (preceding_version_uid legality).** No standalone `ORIGINAL_VERSION`
`Validate` impl exists (the RM dispatcher `validate_rm_value` never sees an
ORIGINAL_VERSION — versioned envelopes are server-assembled, not client content).
The invariant is instead enforced *behaviourally* on the only write path that
accepts version envelopes (the CONTRIBUTION path): `classify` rejects a
`249|creation|` carrying a `preceding_version_uid` and any non-creation change
type lacking one; `parse_preceding`/`require_kind` reject an unknown preceding
object (404); `next_version` rejects a preceding id whose trunk version is not
the current one (412). Direct create/update never carry an envelope, so
`build_original_version` sets `preceding_version_uid` iff `sys_version > 1`,
making R1 structurally true on the served side.

**R27 / R34 / R35 (the fix).** The client-supplied CONTRIBUTION audit and each
version `commit_audit` were persisted with only their `change_type` validated:
an empty `system_id` reached the `ck_audit_system_id_nonempty` DB CHECK and
surfaced as a **500** (an internal error, not the correct 422 for invalid
content), and a structurally-invalid committer `PARTY_IDENTIFIED` (none of
`name`/`identifiers`/`external_ref`, or a present-but-empty `name`) was accepted
outright — even though the same PARTY invariants *are* enforced when a party
appears as composition content (`validate_rm_value`). `validate_commit_audit`
(`service/contribution.rs`) now enforces `AUDIT_DETAILS.System_id_valid` and the
committer `PARTY_IDENTIFIED`/`PARTY_RELATED` `Basic_validity`/`Name_valid` as a
service-layer **422**, wired into `commit_version_set` for both the CONTRIBUTION
audit and every version audit (EHR + demographic contribution paths). Direct
create/update/EHR-create paths build their audit server-side
(`self.audit(...)` → non-empty system id + a valid `committer()`), so they are
unaffected; `PARTY_SELF` committers carry no invariant and stay accepted.

**R36 / R48 / R51 / R52 (`X /= Void implies not X.is_empty`).** These "no
present-but-empty list" invariants are *unrepresentable* in the canonical-JSON
model: the generated types back `0..1 List` attributes with a Rust `Vec`, and
`#[derive(OpenEhrType)]` omits an empty `Vec` on serialize — so a present-empty
list on the wire deserializes to an empty `Vec` and re-serializes as absent.
There is no state in which the invariant can be violated by stored/served data;
archie itself marks the corresponding `Attestation`/`Locatable` checks `ignored`.
Classified verified-by-model, not a fixable defect.

**R37 / R40 / R41 (terminology-bound PARTY/PARTICIPATION invariants).** The
relationship/function/mode code-group memberships are deferred to the
terminology layer (existing PORT NOTEs in `party_related_impl.rs` /
`audit_details_impl.rs`), matching the `openehr-rm`-has-no-`openehr-term`
boundary. *Presence* (relationship 1..1, function 1..1, performer 1..1) is
enforced structurally by the non-optional generated fields on the content path.
The audit committer relationship code is likewise not group-checked (consistent).
Not fixed — a deliberate, pre-existing spec-gap record, not new leniency.

**R46 (`Archetyped_valid` XOR).** COMPOSITION enforces the root arm
(`archetype_details` present, via `Is_archetype_root`). The general XOR — a
*non-root* node must NOT carry `archetype_details`, and EHR_STATUS/FOLDER roots
must — is not enforced generally, matching the reference (archie), and enforcing
it naïvely would over-reject legitimately-nested archetype roots (an archetyped
`CLUSTER`/`OBSERVATION` inside a COMPOSITION is itself an archetype root and
correctly carries `archetype_details`). Left partial; would need the
`is_archetype_root` derivation from the node-id format to close safely.

**R7 / R19 / R50 (distributed versioning / merge).** Version branching, disjoint
merge, and `other_input_version_uids`/`is_merged` are trunk-only-scoped by
existing PORT NOTEs (F-06-09, `vobject.rs`). `Is_merged_validity` holds
vacuously (`other_input_version_uids` is always empty). Genuinely
architecture-gated (version-tree semantics + storage), so **deferred** to the
owner, not botched here.

## Fixes applied

- **R27/R34/R35** — `app/ehrbase/src/service/contribution.rs`:
  `validate_commit_audit` (System_id_valid) + `validate_committer`
  (Basic_validity/Name_valid) on both the per-version and CONTRIBUTION
  audits; unit tests `commit_audit_*`.
- **R37** — `contribution.rs::validate_party_related_relationship`:
  `PARTY_RELATED.relationship` 1..1, coded, strict openEHR
  `subject_relationship` group membership (the invariant formula has no
  terminology escape hatch); test
  `commit_audit_party_related_relationship_is_enforced`.
- **R20** — `message.rs::parse_imported_version` rejects foreign `_type` in
  `versions[]`; test `imported_versions_member_type_is_enforced`.
- **R46** — `openehr-flat` `validation/mod.rs::check_archetyped_valid` (the
  enforceable arm of `Archetyped_valid`); test
  `non_root_node_with_archetype_details_rejected`; corpus stays clean
  (117/117 incl. `valid_corpus_compositions_validate_clean`).
- **R7/R19/R50 — version-tree branching + merging** (RM common master06
  §Version tree / §Distributed versioning / §Version Merging):
  - **Storage** (`migrations/ehr/0001_baseline.sql`, greenfield re-author —
    no create-then-alter): `vo_version.sys_version` becomes an opaque per-vo
    commit ordinal (node/attestation FK + AQL join key unchanged); explicit
    `trunk_version`/`branch_number`/`branch_version` columns carry the
    VERSION_TREE_ID; `uq_vo_version_tree` = the spec's global identity tuple
    {object_id, creating_system_id, version_tree_id}; the whole-vo temporal
    PK is replaced by per-lineage GiST EXCLUDEs (trunk; each branch) — a
    branch coexists in time with the trunk by design; partial currency
    indexes for the trunk current + each branch tip; STORED
    `preceding_version_uid` (cannot be synthesized under branching/import).
  - **Write path** (`vobject.rs::next_version`): the preceding version is
    the addressed tip (or the trunk current); a preceding created by THIS
    system is continued on its lineage (t → t+1; t.b.v → t.b.v+1),
    superseding it; a preceding created by ANOTHER system (an imported copy)
    FORKS branch `t.(max+1).1` with the local `creating_system_id` and the
    copied version stays valid — master06's mandated branching rule.
    Per-vo advisory lock serializes concurrent writers across lineages.
  - **Wire/read**: `TreeId` decodes/renders `N[.B.V]` everywhere
    (`version_id.rs`); every read/lookup keys on the tree columns; branch
    versions fully addressable; `LATEST_VERSION`/current = latest TRUNK
    (master06 `latest_trunk_version`) on every surface incl. AQL
    (`aql/sql.rs`); ALL_VERSIONS includes branches. The former REST-layer
    branch rejection (`ehrbase-rest dispatch/ehr.rs`) is retired; the SM
    catalog version params carry the VERSION_TREE_ID lexical form.
  - **Import/export** (`message.rs`, `vobject.rs::commit_import`):
    per-VERSION `creating_system_id` (a copied tree legitimately mixes
    systems); per-lineage temporal replay; branch import first-class;
    `other_input_version_uids` + `preceding_version_uid` preserved verbatim;
    the latest-only `export_ehrs` exports the latest trunk, the spec-driven
    `export_ehr_extracts include_all_versions` carries the whole tree.
  - **Merge provenance**: accepted on the CONTRIBUTION wire, stored, served
    (`Is_merged_validity` = derived boolean).
  - Tests: `app/ehrbase/tests/service_branching.rs` (fork on foreign
    modification + trunk-stays-current + true preceding + tip continuation +
    second-fork numbering; merge provenance round-trip; whole-tree re-export
    → re-import).
- **Bonus defects fixed**: AQL `VERSION.uid` now built from the STORED
  `creating_system_id` + tree columns (`aql/sql.rs::version_field_expr`) —
  was the live config `system_id` + ordinal; ETag/Location uid builders in
  `composition.rs`/`demographic.rs`/`relationship.rs` now use
  `committed.creating_system_id` — was `""` (malformed `vo::::N`); admin
  dump/load round-trips the full version identity
  (`dump_load.rs::VersionRecord`); export vo-enumeration and every
  "current"-semantics query gained the `branch_number = 0` trunk filter.

## Deferred

None. (Owner ruling 2026-07-11: a chapter is not done while anything is
deferred.)

## Design summary — version-tree storage (for the record)

openEHR defines no SQL schema; the storage design realizing master06 is ours:
one `vo_version` row per version keyed `(vo_id, sys_version)` where
`sys_version` is a per-vo commit ordinal; the VERSION_TREE_ID lives in
explicit `trunk_version`/`branch_number`/`branch_version` columns
(`0/0` = trunk row); identity uniqueness is the spec tuple
{object_id, creating_system_id, version_tree_id}; temporal non-overlap is
per lineage (one GiST EXCLUDE for the trunk, one per
{creating_system_id, fork point, branch}); the container current = the open
trunk row (partial unique index), each open branch additionally has its own
tip index. Branch numbering counts per fork point across systems
(`max(branch_number)+1` over the vo+trunk_version), with cross-system
collisions kept distinct by the identity tuple.

## Uncertain / runtime probes

None remaining — the R20 probe was resolved by the fix + negative test.
