// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Greenfield decomposed node storage.
//!
//! The codec between canonical openEHR JSON and the stored `node` rows, plus
//! the row I/O for the versioned-object spine
//! (`vo_version`/`audit`/`contribution`/`vo_attestation`).
//!
//! No openEHR spec governs the physical storage — this is our own PG18-native
//! design (grounded on docs-verified
//! `PostgreSQL` physics: no partial jsonb detoast, GIN serves no ordering). The
//! change-control law the row I/O upholds is RM common master06; the identifier
//! forms preserved verbatim in each fragment are BASE `base_types` master05 and
//! `foundation_types` master03/05/06.
//!
//! One file per concern; consumers import each item from its defining
//! submodule (no re-exports):
//! - [`codec::decompose`] / [`codec::reassemble`] — the pure content
//!   transform.
//! - [`row::NodeRow`] / [`row::ReadRow`] / [`row::NodeContent`] — the write
//!   and lean read row shapes.
//! - [`structure::is_structure_type`] / [`structure::is_versioned_root_type`]
//!   / [`structure::archetype_parts`] — the decomposition granularity,
//!   delegated to the BMM-generated RM model.
//! - [`error::StorageError`] + the crate-internal `error::classify_sqlx` —
//!   the error surface and the SQLSTATE→SM-status bridge.
//! - [`promoted::PROMOTED_LEAVES`] — the promoted-leaf registry: the shared
//!   `(rm_type, path) → node column` mapping the write codec and the AQL read
//!   lowering both consult, so a hot leaf reads an indexed column.
//! - [`node_repo`] — `node`-table writes + the node→canonical reloads (single
//!   version, batched subtrees, first-version root).
//! - [`version_repo`] — the versioned-object spine (`vo_version`/`audit`/
//!   `contribution`/`vo_attestation` row I/O, folder-membership and
//!   event-outbox writes, the [`version_repo::read::StoredVersion`] read
//!   shape),
//!   itself one file per concern (commit / import / read / placement /
//!   attestation / meta / contribution).
//! - [`ehr_repo`] — `ehr`-table + `ehr_folder`-membership reads/writes (EHR
//!   root row, subject lookup, directory-slot resolution, the
//!   `is_modifiable` guard read).
//! - [`tag_repo`] — the `item_tag` store (EHR-scoped and demographic).
//!
//! The seam with the versioning layer is a value contract, not shared SQL:
//! versioning owns the *semantics* (classify, tree placement, lifecycle, sign,
//! attest, import policy) and calls these functions with plain inputs,
//! consuming [`version_repo::read::StoredVersion`] on read.

pub mod codec;
pub mod ehr_repo;
pub mod error;
pub mod node_repo;
pub mod promoted;
pub mod row;
pub mod structure;
pub mod tag_repo;
pub mod version_repo;
