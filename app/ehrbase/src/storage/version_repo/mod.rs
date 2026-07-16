//! Row I/O for the versioned-object spine: `vo_version`, `audit`,
//! `contribution`, `vo_attestation`, plus the folder-membership and
//! event-outbox writes that ride along inside the same commit transaction.
//!
//! No openEHR spec governs the SQL schema (`docs/architecture.md` §Storage) —
//! this module is pure plumbing. All **semantics** (change classification,
//! version-tree placement, lifecycle, signing, attestation policy, import
//! policy) stay in the versioning layer, which hands these functions plain
//! value inputs and consumes the [`StoredVersion`] read shape. The change
//! control law these rows realize is RM common master06 (§Contributions,
//! §Committal and Audits, §Version tree, §Copying); `AUDIT_DETAILS`/
//! `ATTESTATION` are master04. Every write runs inside a caller-owned `sqlx`
//! transaction so a version + nodes + contribution + audit (+ outbox) commit
//! atomically (master06 §Committal: "similar to nested transactions").
//!
//! One file per concern, all surfaced here (the module is the seam the
//! versioning/service layers import from):
//! - [`commit`] — the local write path: audit/contribution inserts, the
//!   folded one-statement version commit, lineage-tip close, and the
//!   ride-along folder-membership + event-outbox writes.
//! - [`import`] — the EHR-Extract / archive-load write path: explicit
//!   `sys_period` version rows, lineage close-at, container-state read.
//! - [`read`] — the full version reads ([`StoredVersion`]: metadata + body +
//!   attestations) by current / ordinal / tree id / instant.
//! - [`placement`] — the version-tree placement reads (lineage tip, next
//!   ordinal, transaction timestamp); the placement *decision* stays in
//!   versioning.
//! - [`attestation`] — `vo_attestation` writes and reads.
//! - [`meta`] — the lean metadata-only reads (`ETag`/`If-Match`, revision
//!   history, existence/kind/count lookups) that skip node reassembly.
//! - [`contribution`] — CONTRIBUTION reads (audit, affected versions,
//!   listing/counting).
//
// The versioning layer owns the value types (`AuditInput`, `Kind`, `TreeId`,
// `VersionRead`, `Committed`) and maps them onto the plain inputs and the
// [`StoredVersion`] output here (e.g. `Kind::as_str` → `kind: &str`,
// `TreeId::columns()` → the three tree ints). Storage never depends upward on
// versioning — this decoupling is deliberate and stays.

mod attestation;
mod commit;
mod contribution;
mod import;
mod meta;
mod placement;
mod read;

pub use attestation::{
    AttestTargetRow, attestation_target, insert_attestation, read_attestations_all,
};
pub use commit::{
    AuditRow, FoldedVersion, VersionRow, advisory_lock, close_ordinal_at_now, commit_new_version,
    commit_version_into, insert_audit, insert_audit_at, insert_contribution,
    insert_ehr_folder_rank, write_contribution, write_outbox,
};
pub use contribution::{
    ContributionAudit, contribution_audit, contribution_version_refs, count_contributions,
    ehr_contribution_count, list_contributions,
};
pub use import::{
    ContainerStateRow, ImportedVersionRow, VerbatimVersionRow, close_lineage_at,
    imported_container_state, insert_imported_vo_version, insert_version_verbatim,
};
pub use meta::{
    CurrentCompositionMeta, CurrentDemographicMeta, CurrentMeta, CurrentVoRow, VersionMeta,
    all_version_meta, composition_count, current_composition_meta, current_demographic_meta,
    current_version_meta_by_kind, current_version_meta_scoped, current_vo, current_vo_ids,
    ehr_exists, object_kind, object_kinds, time_created, vo_owner,
};
pub use placement::{Placement, TipRow, lineage_tip, next_branch_number, next_placement, tx_now};
pub use read::{StoredVersion, read_current, read_version, read_version_by_ordinal, version_at};

/// `other_input_version_uids` stores NULL when empty (`Is_merged_validity`),
/// else the JSON array.
fn optional_json_array(uids: &[String]) -> Option<serde_json::Value> {
    if uids.is_empty() {
        None
    } else {
        Some(serde_json::json!(uids))
    }
}
