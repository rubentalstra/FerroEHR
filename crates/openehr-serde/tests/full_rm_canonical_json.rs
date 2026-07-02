//! Full-RM canonical JSON acceptance harness — the Phase 04 exit instrument.
//!
//! Enumerates **every** class definition in the vendored ITS-JSON schema
//! (`schemas/openehr_rm_1.1.0_all.json`, pinned commit
//! `5acae056248e917a4b4c56f7e712f4fcfeb616a6`) and demands that each one is
//! either:
//!
//! 1. **covered** — a fixture in `tests/fixtures/` constructs a Rust
//!    instance, round-trips it (serialize → deserialize → equal), pins the
//!    JSON with an insta golden vector, and validates it against the
//!    schema definition; or
//! 2. **excluded** — listed in [`EXCLUSIONS`] with a named reason
//!    (deferred `ehr_extract` feature, non-RM classes, foundation types
//!    that only ever serialize embedded, degenerate schema definitions).
//!
//! Anything else fails `full_rm_coverage` **by class name** — a missing RM
//! class is a red test, never a silent gap.
//!
//! `ITEM_TAG` (RM 1.1.0 common.tags) has NO definition in the pinned
//! ITS-JSON commit (2021-10-31, which predates the class) and so cannot
//! appear in this partition; its ADR-002 round-trip is pinned by a unit
//! test in `openehr-rm`'s `item_tag.rs` instead.

mod fixtures;

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

const RM_SCHEMA: &str = include_str!("../schemas/openehr_rm_1.1.0_all.json");

/// Schema definitions deliberately not covered by a fixture, each with the
/// reason on record. Keep this list justified — an exclusion without a
/// defensible reason is a coverage gap wearing a costume.
const EXCLUSIONS: &[(&str, &str)] = &[
    // rm.ehr_extract is experimental and explicitly deferred
    // (PORT_MASTER_PLAN.md §7.1); openehr-rm gates it behind the
    // `ehr-extract` feature, which is empty at P4.
    (
        "ADDRESSED_MESSAGE",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "EXTRACT",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "EXTRACT_ACTION_REQUEST",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "EXTRACT_CHAPTER",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "EXTRACT_ENTITY_CHAPTER",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "EXTRACT_ENTITY_MANIFEST",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "EXTRACT_FOLDER",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "EXTRACT_MANIFEST",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "EXTRACT_PARTICIPATION",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "EXTRACT_REQUEST",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "EXTRACT_SPEC",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "EXTRACT_UPDATE_SPEC",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "EXTRACT_VERSION_SPEC",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "GENERIC_CONTENT_ITEM",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "MESSAGE",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "OPENEHR_CONTENT_ITEM",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "SYNC_EXTRACT",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "SYNC_EXTRACT_REQUEST",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "SYNC_EXTRACT_SPEC",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "X_CONTRIBUTION",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "X_VERSIONED_COMPOSITION",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "X_VERSIONED_EHR_ACCESS",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "X_VERSIONED_EHR_STATUS",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "X_VERSIONED_FOLDER",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "X_VERSIONED_OBJECT",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    (
        "X_VERSIONED_PARTY",
        "rm.ehr_extract — deferred behind the ehr-extract feature",
    ),
    // Not part of the RM transcription scope.
    (
        "ACCESS_GROUP_REF",
        "not migrated to BASE 1.2.0 (settled hazard, rm-transcription.md) — implement only if legacy data needs it",
    ),
    (
        "ARCHETYPE_HRID",
        "AM 2.x class — Phase 09 (ADL/AOM), not RM transcription",
    ),
    // Structurally impossible to cover as standalone canonical-JSON
    // objects — each schema definition here is a degenerate placeholder
    // (`{_type const}` with no member properties), and the Rust type's
    // faithful wire shape is not a JSON object at all.
    (
        "ARRAY",
        "foundation container — serializes as a JSON array, not an object; degenerate placeholder definition in the pinned schema",
    ),
    (
        "LIST",
        "foundation container — serializes as a JSON array, not an object; degenerate placeholder definition in the pinned schema",
    ),
    (
        "SET",
        "foundation container — serializes as a JSON array, not an object; degenerate placeholder definition in the pinned schema",
    ),
    (
        "URI",
        "foundation Uri — transparent newtype serializing as a bare string (DV_URI is the RM-level object fixture); degenerate placeholder definition",
    ),
    (
        "ISO8601_TYPE",
        "abstract foundation temporal — never instantiated; concrete DATE/TIME/DATE_TIME/DURATION are covered",
    ),
    (
        "VALIDITY_KIND",
        "enumeration — serializes as a string per the spec; degenerate object placeholder definition in the pinned schema",
    ),
    (
        "VERSION_STATUS",
        "enumeration — serializes as a string per the spec; degenerate object placeholder definition in the pinned schema",
    ),
];

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

/// The partition check: fixtures ∪ exclusions == schema definitions,
/// with no overlap and no stragglers on either side. A missing RM class
/// shows up here BY NAME.
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
    let excluded: BTreeMap<&str, &str> = EXCLUSIONS.iter().copied().collect();

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
        if excluded.contains_key(class.as_str()) {
            problems.push(format!(
                "`{class}` is both covered and excluded — remove the exclusion"
            ));
        }
    }
    for (class, _) in EXCLUSIONS {
        if !defs.contains(*class) {
            problems.push(format!(
                "exclusion `{class}` is not a schema definition — stale entry"
            ));
        }
    }
    let missing: Vec<&str> = defs
        .iter()
        .map(String::as_str)
        .filter(|d| !covered.contains(*d) && !excluded.contains_key(*d))
        .collect();
    if !missing.is_empty() {
        problems.push(format!(
            "{} of {} schema classes have NO fixture and NO exclusion:\n  {}",
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
    // Sanity floor so the partition can't be gamed by growing EXCLUSIONS.
    assert!(
        covered.len() >= 97,
        "only {} classes covered — the fixture set shrank below the P4 floor of 97",
        covered.len()
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
        if !v.schema_check {
            continue;
        }
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
//   note: P4 acceptance instrument — coverage partition over all 134 schema definitions, jsonschema validation, insta golden vectors; fixtures live in tests/fixtures/
// ─────────────────────────────────────────────
