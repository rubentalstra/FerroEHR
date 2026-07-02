//! openEHR canonical serialization: canonical JSON (ITS-JSON, `_type`
//! discriminated) and canonical XML (ITS-XML 1.0.2 + 2.0.0, Phase 05).
//!
//! ## Where the serde impls actually live (ADR-002)
//!
//! Rust's orphan rule forbids this crate from implementing `Serialize`/
//! `Deserialize` for `openehr-rm`/`openehr-base` types, so the canonical-
//! JSON derives and the `TypeTag` self-tagging mechanism live **on the RM
//! types themselves** (infrastructure in
//! `openehr_foundation::serde_support`). What this crate owns is:
//!
//! - the vendored ITS-JSON schema ([`RM_SCHEMA_JSON`], pinned commit
//!   `5acae056248e917a4b4c56f7e712f4fcfeb616a6`),
//! - the canonical entry points ([`to_canonical_json`],
//!   [`to_canonical_json_pretty`], [`from_canonical_json`]) the server
//!   crates use so no caller reaches for raw `serde_json` conventions,
//! - the Phase 04 acceptance instrument
//!   (`tests/full_rm_canonical_json.rs`): a coverage partition over every
//!   schema class definition, per-class round-trip + jsonschema
//!   validation, and insta golden vectors.

use serde::Serialize;
use serde::de::DeserializeOwned;

/// The vendored ITS-JSON RM 1.1.0 all-schema (draft-07), pinned at commit
/// `5acae056248e917a4b4c56f7e712f4fcfeb616a6` (see `docs/VERSIONS.md`).
pub const RM_SCHEMA_JSON: &str = include_str!("../schemas/openehr_rm_1.1.0_all.json");

/// Serialize an RM value to canonical JSON (compact form).
///
/// The canonical conventions (`_type` first on every object, snake_case
/// keys, omitted nulls, `{_type, value}` UIDs, inline-base64
/// `DV_MULTIMEDIA.data`) are carried by the RM types' own serde impls per
/// ADR-002; this function is the single named entry point so intent is
/// explicit at call sites.
///
/// # Errors
/// Returns any underlying `serde_json` serialization error unchanged.
pub fn to_canonical_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}

/// Serialize an RM value to canonical JSON, pretty-printed (the form the
/// insta golden vectors pin).
///
/// # Errors
/// Returns any underlying `serde_json` serialization error unchanged.
pub fn to_canonical_json_pretty<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(value)
}

/// Deserialize an RM value from canonical JSON. A present-but-wrong
/// `_type` is rejected by the target type's `TypeTag`; a missing `_type`
/// in a concrete-declared slot is tolerated per ITS-JSON.
///
/// # Errors
/// Returns any underlying `serde_json` deserialization error unchanged.
pub fn from_canonical_json<T: DeserializeOwned>(json: &str) -> Result<T, serde_json::Error> {
    serde_json::from_str(json)
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: ITS-JSON (pinned commit 5acae056248e917a4b4c56f7e712f4fcfeb616a6) + ADR-002
//   source_loc: n/a
//   confidence: high
//   todos: 0
//   note: canonical-JSON entry points + vendored schema; canonical XML lands here at P5; the serde impls themselves live on the RM types (orphan rule, ADR-002)
// ─────────────────────────────────────────────
