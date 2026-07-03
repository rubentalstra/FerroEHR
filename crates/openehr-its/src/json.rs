//! **ITS-JSON** — canonical JSON serialization.
//!
//! The canonical conventions (`_type` first on every object, snake_case keys,
//! omitted nulls, `{_type, value}` UIDs, inline-base64 `DV_MULTIMEDIA.data`)
//! are carried by the RM types' own `#[derive(OpenEhrType)]` impls
//! (`openehr-derive`). These functions are the single named entry points so
//! call sites read as canonical-JSON, not raw `serde_json`.

use serde::Serialize;
use serde::de::DeserializeOwned;

/// The vendored ITS-JSON RM all-schema (draft-07), used to validate output in
/// the fidelity-gate tests. Pinned commit recorded in `docs/VERSIONS.md`.
pub const RM_SCHEMA_JSON: &str = include_str!("../schemas/openehr_rm_1.1.0_all.json");

/// Serialize an RM value to canonical JSON (compact).
///
/// # Errors
/// Propagates any `serde_json` serialization error.
pub fn to_canonical_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}

/// Serialize an RM value to canonical JSON, pretty-printed.
///
/// # Errors
/// Propagates any `serde_json` serialization error.
pub fn to_canonical_json_pretty<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(value)
}

/// Deserialize an RM value from canonical JSON. A present-but-wrong `_type` is
/// rejected by the target type's `OpenEhrType` impl; a missing `_type` in a
/// concrete slot is tolerated (per ITS-JSON).
///
/// # Errors
/// Propagates any `serde_json` deserialization error.
pub fn from_canonical_json<T: DeserializeOwned>(json: &str) -> Result<T, serde_json::Error> {
    serde_json::from_str(json)
}
