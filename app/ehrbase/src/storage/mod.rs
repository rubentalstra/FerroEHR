//! Greenfield decomposed node storage — the codec between canonical openEHR
//! JSON and the stored `node` rows, plus the row I/O for the versioned-object
//! spine (`vo_version`/`audit`/`contribution`/`vo_attestation`).
//!
//! No openEHR spec governs the physical storage — this is our own PG18-native
//! design (`docs/architecture.md` §Storage; grounded on docs-verified
//! `PostgreSQL` physics: no partial jsonb detoast, GIN serves no ordering). The
//! change-control law the row I/O upholds is RM common master06; the identifier
//! forms preserved verbatim in each fragment are BASE `base_types` master05 and
//! `foundation_types` master03/05/06.
//!
//! Layers:
//! - [`decompose`] / [`reassemble`] — the pure content transform (`codec`).
//! - [`NodeRow`] / [`ReadRow`] — the write and lean read row shapes (`row`).
//! - [`is_structure_type`] / [`is_versioned_root_type`] / [`archetype_parts`] —
//!   the decomposition granularity, delegated to the BMM-generated RM model
//!   (`structure`).
//! - [`PROMOTED_LEAVES`] — the promoted-leaf registry (`promoted`): the shared
//!   `(rm_type, path) → node column` mapping the write codec and the AQL read
//!   lowering both consult, so a hot leaf reads an indexed column.
//! - [`node_repo`] — `node`-table writes + the single node→canonical reload.
//! - [`version_repo`] — `vo_version`/`audit`/`contribution`/`vo_attestation`
//!   row I/O, the folder-membership and event-outbox writes, and the version
//!   read shape ([`version_repo::StoredVersion`]).
//! - [`ehr_repo`] — `ehr`-table + `ehr_folder`-membership reads/writes (EHR
//!   root row, subject lookup, folder-hierarchy resolution, the
//!   `is_modifiable` guard read).
//!
//! The seam with the versioning layer (register 01) is a value contract, not
//! shared SQL: versioning owns the *semantics* (classify, tree placement,
//! lifecycle, sign, attest, import policy) and calls these functions with plain
//! inputs, consuming [`version_repo::StoredVersion`] on read.

mod codec;
mod error;
mod row;
mod structure;

pub mod ehr_repo;
pub mod node_repo;
pub mod promoted;
pub mod tag_repo;
pub mod version_repo;

pub use codec::{decompose, reassemble};
pub use error::StorageError;
use error::classify_sqlx;
pub use promoted::{PROMOTED_LEAVES, PromotedKind, PromotedLeaf};
pub use row::{NodeContent, NodeRow, ReadRow};
pub use structure::{archetype_parts, is_structure_type, is_versioned_root_type};
