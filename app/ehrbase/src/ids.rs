//! Strongly-typed object identities.
//!
//! An EHR id and a versioned-object id are both UUIDs on the wire and in the
//! store, but confusing them is a *clinical* fault: a swapped pair reads or
//! writes another record's data and still type-checks. These newtypes make
//! that mixup uncompilable (the API-guidelines C-NEWTYPE discipline: newtypes
//! provide static distinctions). The wire shapes are unchanged — both
//! serialize as the bare UUID (`#[serde(transparent)]`), and both bind to
//! PostgreSQL `uuid` columns directly (`#[sqlx(transparent)]`).
//!
//! Spec identities: the EHR id is RM ehr §EHR `ehr_id` (a `HIER_OBJECT_ID`);
//! the versioned-object id is RM common master06 §Version Identification —
//! the `object_id` part of an `OBJECT_VERSION_ID` / the
//! `VERSIONED_OBJECT.uid`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The identity of one EHR (RM ehr §EHR `ehr_id`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, sqlx::Type,
)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct EhrId(pub Uuid);

/// The identity of one versioned object — COMPOSITION / `EHR_STATUS` /
/// FOLDER / demographic party (RM common master06 §Version Identification:
/// the `object_id` of an `OBJECT_VERSION_ID`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, sqlx::Type,
)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct VoId(pub Uuid);

impl EhrId {
    /// A fresh time-ordered id for a new EHR.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for EhrId {
    fn default() -> Self {
        Self::new()
    }
}

impl VoId {
    /// A fresh time-ordered id for a new versioned object.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for VoId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EhrId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Display for VoId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for EhrId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::from_str(s)?))
    }
}

impl std::str::FromStr for VoId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::from_str(s)?))
    }
}

impl From<EhrId> for Uuid {
    fn from(id: EhrId) -> Self {
        id.0
    }
}

impl From<VoId> for Uuid {
    fn from(id: VoId) -> Self {
        id.0
    }
}
