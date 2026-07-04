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
pub const RM_SCHEMA_JSON: &str = include_str!("../schemas/json/openehr_rm_1.1.0_all.json");

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
