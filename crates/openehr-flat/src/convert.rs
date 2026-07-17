//! The public conversion entry points: canonical-JSON COMPOSITION ⇄ the
//! FLAT and STRUCTURED wire forms, and the pure FLAT ⇄ STRUCTURED
//! transforms.
//!
//! Wire semantics: ITS-REST `simplified_formats/master04-basic_concepts.adoc`
//! (§Format variants, §Conversion Between Formats); media types
//! (`master02 §MIME Types`): `application/openehr.wt.flat+json`,
//! `application/openehr.wt.structured+json`. Every conversion runs through
//! the shared simplified tree ([`crate::sim`]), so the two wire variants
//! cannot drift from each other.
//!
//! `now` on the building directions supplies the `ctx/time` default
//! (`master04 §Context`: "defaults to the current server time (now())") —
//! injected by the caller so conversions stay deterministic under test.

use serde_json::{Map, Value};

use crate::build;
use crate::error::FlatError;
use crate::flatten;
use crate::sim::flat;
use crate::sim::structured;
use crate::webtemplate::WebTemplate;

/// FLAT document → canonical-JSON COMPOSITION.
///
/// # Errors
/// Path-syntax, unknown-identifier, datum, and context errors per
/// [`FlatError`].
pub fn composition_from_flat(
    doc: &Map<String, Value>,
    wt: &WebTemplate,
    now: &str,
) -> Result<Value, FlatError> {
    let tree = flat::parse_flat(doc)?;
    build::build_composition(&tree, wt, now)
}

/// Canonical-JSON COMPOSITION → FLAT document.
///
/// # Errors
/// [`FlatError::Conversion`] when `composition` is not a JSON object.
pub fn composition_to_flat(
    composition: &Value,
    wt: &WebTemplate,
) -> Result<Map<String, Value>, FlatError> {
    let tree = flatten::flatten_composition(composition, wt)?;
    Ok(flat::emit_flat(&tree))
}

/// STRUCTURED document → canonical-JSON COMPOSITION.
///
/// # Errors
/// Structure, unknown-identifier, datum, and context errors per
/// [`FlatError`].
pub fn composition_from_structured(
    doc: &Value,
    wt: &WebTemplate,
    now: &str,
) -> Result<Value, FlatError> {
    let tree = structured::parse_structured(doc)?;
    build::build_composition(&tree, wt, now)
}

/// Canonical-JSON COMPOSITION → STRUCTURED document.
///
/// # Errors
/// [`FlatError::Conversion`] when `composition` is not a JSON object.
pub fn composition_to_structured(
    composition: &Value,
    wt: &WebTemplate,
) -> Result<Value, FlatError> {
    let tree = flatten::flatten_composition(composition, wt)?;
    Ok(structured::emit_structured(&tree))
}

/// FLAT → STRUCTURED, the pure template-free transform
/// (`master04 §Flat to Structured`).
///
/// # Errors
/// [`FlatError::MalformedPath`] on an invalid key.
pub fn flat_to_structured(doc: &Map<String, Value>) -> Result<Value, FlatError> {
    Ok(structured::emit_structured(&flat::parse_flat(doc)?))
}

/// STRUCTURED → FLAT, the pure template-free transform
/// (`master04 §Structured to Flat`).
///
/// # Errors
/// [`FlatError::Conversion`]/[`FlatError::MalformedPath`] on an invalid
/// document.
pub fn structured_to_flat(doc: &Value) -> Result<Map<String, Value>, FlatError> {
    Ok(flat::emit_flat(&structured::parse_structured(doc)?))
}
