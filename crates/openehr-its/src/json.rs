//! **ITS-JSON** — canonical JSON serialization: the named entry points.
//!
//! The canonical conventions (`_type` first on every object, snake_case keys,
//! omitted nulls/empties, integer-vs-real number typing, `{_type, value}` UIDs,
//! inline-base64 `DV_MULTIMEDIA.data`) are carried by the native canonical-JSON
//! codec ([`crate::json_codec`] — the emitted `ToJson`/`FromJson` impls over a
//! hand-written runtime), NOT by a serde derive. These functions are the single
//! named entry points so call sites read as canonical-JSON, not raw
//! `serde_json`, and they hide the codec's `runtime` module behind stable names.

use crate::json_codec::runtime::{
    FromJson, JsonParseError, ToJson, from_json_str, from_json_value,
};

/// The vendored ITS-JSON RM all-schema (draft-07), used to validate output in
/// the fidelity-gate tests. Pinned commit recorded in `docs/VERSIONS.md`.
pub const RM_SCHEMA_JSON: &str = include_str!("../schemas/json/openehr_rm_1.1.0_all.json");

/// Serialize an RM value to canonical JSON (compact) through the native codec.
///
/// Serialization is infallible (the codec cannot fail), so this returns a
/// `String` directly.
#[must_use]
pub fn to_canonical_json<T: ToJson + ?Sized>(value: &T) -> String {
    crate::json_codec::runtime::to_json_string(value)
}

/// Serialize an RM value to a canonical-JSON `serde_json::Value` through the
/// native codec (for the boundary where the caller needs an in-memory tree).
///
/// A non-object codec output (never produced by a spec type) degrades to
/// `Value::Null` rather than panicking in library code.
#[must_use]
pub fn to_canonical_value<T: ToJson + ?Sized>(value: &T) -> serde_json::Value {
    let json = crate::json_codec::runtime::to_json_string(value);
    serde_json::from_str(&json).unwrap_or(serde_json::Value::Null)
}

/// Deserialize an RM value from canonical JSON (`&str`) through the native codec.
/// A present-but-wrong `_type` is rejected; a missing `_type` in a concrete slot
/// is tolerated (per ITS-JSON); unknown keys are ignored (RM-version skew).
///
/// # Errors
/// Returns a [`JsonParseError`] on a syntax error or an invalid encoding.
pub fn from_canonical_json<T: FromJson>(json: &str) -> Result<T, JsonParseError> {
    from_json_str(json)
}

/// Deserialize an RM value from an already-parsed canonical-JSON
/// `serde_json::Value` through the native codec (no re-stringifying — the
/// codec's `Value` reader backend). Same tolerance rules as
/// [`from_canonical_json`].
///
/// # Errors
/// Returns a [`JsonParseError`] on an invalid encoding.
pub fn from_canonical_value<T: FromJson>(value: &serde_json::Value) -> Result<T, JsonParseError> {
    from_json_value(value)
}

/// The compiled ITS-JSON validator, built once from [`RM_SCHEMA_JSON`]. Stored
/// as a `Result` (not `expect`ed) so a schema-compile failure surfaces as a
/// validation error rather than a panic in library code.
static RM_VALIDATOR: std::sync::LazyLock<Result<jsonschema::Validator, String>> =
    std::sync::LazyLock::new(|| {
        let schema: serde_json::Value =
            serde_json::from_str(RM_SCHEMA_JSON).map_err(|e| format!("parse RM schema: {e}"))?;
        jsonschema::validator_for(&schema).map_err(|e| format!("compile RM schema: {e}"))
    });

/// Validate a canonical-JSON value against the vendored ITS-JSON RM schema
/// (`openehr_rm_1.1.0_all.json`). The schema's root dispatches on the top-level
/// `_type` (draft-07 `if`/`then`) to the matching class definition, so any RM
/// object with a `_type` is validated against its own definition.
///
/// # Errors
/// Returns every schema violation (path + message), or a single-element error if
/// the schema itself failed to compile.
pub fn validate_canonical(value: &serde_json::Value) -> Result<(), Vec<String>> {
    match &*RM_VALIDATOR {
        Ok(validator) => {
            let errors: Vec<String> = validator
                .iter_errors(value)
                .map(|e| format!("{} (at {})", e, e.instance_path()))
                .collect();
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        }
        Err(e) => Err(vec![e.clone()]),
    }
}
