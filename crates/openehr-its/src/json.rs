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

/// Refuse the first `_type`-tagged node of `value` that carries a wire key its
/// class does not declare — the **wire door's** half of the strict reader.
///
/// The protocol adapter parses a canonical-JSON body into an untyped
/// `serde_json::Value` (so a committed document keeps the client's bytes
/// verbatim) and therefore never runs a typed decode at the door. This walk
/// gives that door the reader's undeclared-key refusal WITHOUT a decode, using
/// the generated `declared_fields` table — the same field view the typed reader
/// refuses from, so the two can never disagree.
///
/// Only `_type`-tagged nodes are judged: canonical JSON requires `_type` only
/// where the declared attribute type is abstract, so an untagged node's class
/// is not knowable from the value alone here. An undeclared key on such a node
/// is still refused, one tier later, by the typed dispatch
/// (`crate::wire_validate`) — this door narrows *which status* a refusal
/// carries, never *whether* one happens.
///
/// # Why the door and not validation
/// A document the reader cannot READ never converts, so it cannot reach the
/// convertible-but-semantically-invalid branch: ITS-REST overview
/// `Requests_and_responses.md` §HTTP status codes defines `400` as content that
/// "could not be parsed or is invalid" and `422` as content that is
/// "well-formed but was unable to be followed due to semantic errors".
///
/// # Errors
/// A [`JsonParseError`] naming the offending key, the class that does not
/// declare it, and the path to the node.
pub fn reject_undeclared_keys(value: &serde_json::Value) -> Result<(), JsonParseError> {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(ty) = map.get("_type").and_then(serde_json::Value::as_str)
                && let Some(declared) =
                    crate::json_codec::generated::structural::declared_fields(ty)
                && let Some(key) = map
                    .keys()
                    .find(|k| k.as_str() != "_type" && declared.binary_search(&k.as_str()).is_err())
            {
                return Err(JsonParseError::unknown_field(key, ty, declared));
            }
            for (key, child) in map {
                reject_undeclared_keys(child).map_err(|e| e.in_field(key))?;
            }
            Ok(())
        }
        serde_json::Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                reject_undeclared_keys(child).map_err(|e| e.in_index(index))?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

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
