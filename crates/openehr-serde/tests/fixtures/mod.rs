//! Fixture registry for the full-RM canonical JSON harness.
//!
//! One fixture per concrete ITS-JSON class. Every fixture is built through
//! [`vector`], which enforces the ADR-002 invariants at construction time:
//! serialize → deserialize → equal (round-trip), the JSON is an object,
//! and `_type` is the FIRST key with exactly the class name as its value.
//!
//! Fixture values must be fully deterministic — fixed strings, fixed
//! numbers, no generated UUIDs, no clock reads — because they are pinned
//! as insta golden vectors.

use std::fmt::Debug;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub mod base_identification;
pub mod base_resource;
pub mod common;
pub mod data_structures;
pub mod data_types;
pub mod demographic;
pub mod ehr;
pub mod foundation;
pub mod helpers;

/// One golden vector: a schema class name plus its canonical JSON,
/// already round-trip-verified by [`vector`].
pub struct Vector {
    /// Canonical class name — must match a `definitions` key in the
    /// pinned schema exactly.
    pub class: &'static str,
    /// The serialized canonical JSON.
    pub value: Value,
}

/// Build a vector from a live RM instance, enforcing the ADR-002
/// invariants. Panics (failing the calling test) with the class name on
/// any violation.
pub fn vector<T>(class: &'static str, instance: &T) -> Vector
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let value =
        serde_json::to_value(instance).unwrap_or_else(|e| panic!("{class}: serialize failed: {e}"));

    let obj = value
        .as_object()
        .unwrap_or_else(|| panic!("{class}: canonical JSON must be an object, got: {value}"));
    match obj.keys().next() {
        Some(first) if first == "_type" => {}
        other => panic!("{class}: first key must be \"_type\", got {other:?}: {value}"),
    }
    assert_eq!(
        obj["_type"],
        Value::String(class.to_string()),
        "{class}: _type value mismatch"
    );

    let back: T = serde_json::from_value(value.clone())
        .unwrap_or_else(|e| panic!("{class}: deserialize failed: {e}\n  json: {value}"));
    assert!(
        &back == instance,
        "{class}: round-trip mismatch\n  json: {value}\n  back: {back:?}"
    );

    Vector { class, value }
}

/// Like [`vector`], but for foundation classes that serialize **without**
/// a `_type` discriminator (they only ever appear embedded inside RM
/// classes; no schema definition marks `_type` required). Round-trip and
/// schema validation still apply in full.
pub fn vector_tagless<T>(class: &'static str, instance: &T) -> Vector
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let value =
        serde_json::to_value(instance).unwrap_or_else(|e| panic!("{class}: serialize failed: {e}"));
    assert!(
        value.is_object(),
        "{class}: canonical JSON must be an object, got: {value}"
    );
    let back: T = serde_json::from_value(value.clone())
        .unwrap_or_else(|e| panic!("{class}: deserialize failed: {e}\n  json: {value}"));
    assert!(
        &back == instance,
        "{class}: round-trip mismatch\n  json: {value}\n  back: {back:?}"
    );
    Vector { class, value }
}

/// The full registry the harness iterates.
pub fn all() -> Vec<Vector> {
    let mut vectors = Vec::new();
    vectors.extend(base_identification::fixtures());
    vectors.extend(base_resource::fixtures());
    vectors.extend(foundation::fixtures());
    vectors.extend(data_types::fixtures());
    vectors.extend(data_structures::fixtures());
    vectors.extend(common::fixtures());
    vectors.extend(ehr::fixtures());
    vectors.extend(demographic::fixtures());
    vectors
}
