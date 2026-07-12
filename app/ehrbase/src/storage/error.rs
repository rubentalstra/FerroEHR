//! Storage-layer error surface.
//!
//! No openEHR spec governs the physical storage — these are our own
//! codec/repository errors. Semantic outcomes (version conflict, not-found,
//! validation) belong to the versioning/service layer and are NOT modelled
//! here; storage reports only what the codec and the SQL row I/O can observe.

use uuid::Uuid;

/// Errors produced by the node-storage codec and the row repositories.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// The root value handed to [`crate::storage::decompose`] is not a
    /// decomposable versioned-object tree (its `_type` is not a versioned root).
    #[error("root object has no structure _type (found {0:?})")]
    NotAStructureRoot(Option<String>),

    /// A canonical-JSON array mixed structure and non-structure elements, which
    /// canonical RM JSON never does.
    #[error("array {attribute:?} mixes structure and non-structure elements")]
    MixedArray { attribute: String },

    /// Reassembly received rows that do not form one tree rooted at `num = 0`.
    #[error("invalid node rows: {0}")]
    InvalidRows(String),

    /// A client-supplied CONTRIBUTION uid is already in use (the insert hit
    /// `ON CONFLICT (id) DO NOTHING`). ITS-REST `contribution_create`: "if the
    /// `uid` is supplied it must not already be in use" — a duplicate is a
    /// conflict, never an overwrite. Mapped to 409 by the versioning layer.
    #[error("CONTRIBUTION uid {0:?} is already in use")]
    ContributionUidInUse(Option<Uuid>),

    /// A driver/pool/query error from `sqlx`.
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
}
