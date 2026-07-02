//! Full-RM canonical JSON acceptance harness — the Phase 04 exit instrument.
//!
//! Enumerates **every** class definition in the vendored ITS-JSON schema
//! (`schemas/openehr_rm_1.1.0_all.json`, pinned commit
//! `5acae056248e917a4b4c56f7e712f4fcfeb616a6`) and demands that each one is
//! covered by a fixture in `tests/fixtures/`, which constructs a Rust
//! instance, round-trips it (serialize → deserialize → equal), pins the
//! JSON with an insta golden vector, and validates it against the schema
//! definition.
//!
//! Anything else fails `full_rm_coverage` **by class name** — a missing RM
//! class is a red test, never a silent gap.
//!
//! `ITEM_TAG` (RM 1.1.0 common.tags) has NO definition in the pinned
//! ITS-JSON commit (2021-10-31, which predates the class) and so cannot
//! appear in this partition; its ADR-002 round-trip is pinned by a unit
//! test in `openehr-rm`'s `item_tag.rs` instead.

mod fixtures;

use std::collections::BTreeSet;

use serde_json::{Value, json};

const RM_SCHEMA: &str = include_str!("../schemas/openehr_rm_1.1.0_all.json");

fn schema_root() -> Value {
    serde_json::from_str(RM_SCHEMA).expect("vendored schema must parse")
}

fn definition_names(root: &Value) -> BTreeSet<String> {
    root["definitions"]
        .as_object()
        .expect("schema has a definitions map")
        .keys()
        .cloned()
        .collect()
}

/// The coverage check: every schema definition must have a fixture. A
/// missing RM class shows up here BY NAME.
#[test]
fn full_rm_coverage() {
    let root = schema_root();
    let defs = definition_names(&root);

    let vectors = fixtures::all();
    let mut covered = BTreeSet::new();
    let mut duplicates = Vec::new();
    for v in &vectors {
        if !covered.insert(v.class.to_string()) {
            duplicates.push(v.class);
        }
    }

    let mut problems = Vec::new();
    if !duplicates.is_empty() {
        problems.push(format!("duplicate fixtures: {duplicates:?}"));
    }
    for class in covered.iter() {
        if !defs.contains(class.as_str()) {
            problems.push(format!(
                "fixture `{class}` names a class that is NOT in the schema — typo or wrong TypeName"
            ));
        }
    }
    let missing: Vec<&str> = defs
        .iter()
        .map(String::as_str)
        .filter(|d| !covered.contains(*d))
        .collect();
    if !missing.is_empty() {
        problems.push(format!(
            "{} of {} schema classes have NO fixture:\n  {}",
            missing.len(),
            defs.len(),
            missing.join("\n  ")
        ));
    }

    assert!(
        problems.is_empty(),
        "full-RM coverage failed:\n{}",
        problems.join("\n")
    );
    assert!(
        covered.len() == defs.len(),
        "{} of {} schema classes covered",
        covered.len(),
        defs.len()
    );
}

/// Every fixture's serialized JSON must validate against its own schema
/// definition (draft-07; most definitions carry
/// `additionalProperties: false`, so this catches wrong/extra keys, not
/// just missing ones).
#[test]
fn fixtures_validate_against_its_json_schema() {
    let root = schema_root();
    let definitions = root["definitions"].clone();

    let mut failures = Vec::new();
    for v in fixtures::all() {
        let class_schema = json!({
            "$schema": "http://json-schema.org/draft-07/schema",
            "definitions": definitions,
            "$ref": format!("#/definitions/{}", v.class),
        });
        let validator = jsonschema::validator_for(&class_schema)
            .unwrap_or_else(|e| panic!("{}: schema compile failed: {e}", v.class));
        let errors: Vec<String> = validator
            .iter_errors(&v.value)
            .map(|e| format!("    at {}: {e}", e.instance_path()))
            .collect();
        if !errors.is_empty() {
            failures.push(format!(
                "  {} failed validation:\n{}\n    json: {}",
                v.class,
                errors.join("\n"),
                v.value
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} fixture(s) do not validate against the pinned ITS-JSON schema:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Golden vectors: pin every fixture's canonical JSON with insta. Any
/// wire-format change must be consciously reviewed via `cargo insta
/// review`, never silently absorbed.
#[test]
fn golden_vectors() {
    for v in fixtures::all() {
        insta::assert_json_snapshot!(v.class, v.value);
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: ITS-JSON (pinned commit 5acae056248e917a4b4c56f7e712f4fcfeb616a6) openehr_rm_1.1.0_all.json + phase-04 exit criteria
//   source_loc: n/a
//   confidence: high
//   todos: 0
//   note: P4 acceptance instrument — full fixture coverage over all 134 schema definitions, jsonschema validation, insta golden vectors; fixtures live in tests/fixtures/
// ─────────────────────────────────────────────
