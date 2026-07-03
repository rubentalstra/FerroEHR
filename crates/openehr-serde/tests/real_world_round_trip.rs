//! Real-world canonical JSON acceptance harness.
//!
//! Complements `full_rm_canonical_json.rs` (synthetic per-class fixtures)
//! with REAL EHRbase test-data files — the acceptance oracles named in
//! `PORT_MASTER_PLAN.md` §15 — moved verbatim into
//! `crates/openehr-server/tests/resources/` at Phase 0 and treated here as
//! READ-ONLY inputs. Each file is:
//!
//! 1. parsed to a raw [`serde_json::Value`],
//! 2. deserialized into the corresponding `openehr_rm` type (with
//!    `serde_path_to_error` pinpointing the exact failing path on error),
//! 3. re-serialized,
//! 4. validated — BOTH the original and our re-serialized output — against
//!    the vendored ITS-JSON schema
//!    (`schemas/openehr_rm_1.1.0_all.json`, pinned commit
//!    `5acae056248e917a4b4c56f7e712f4fcfeb616a6`), and
//! 5. compared for value equality against the original after the
//!    normalization pass documented at [`strip_redundant_type_tags`] and
//!    [`collect_diffs`] below.
//!
//! ## Fixture inventory
//!
//! - `service/org/ehrbase/repository/conformance_ehrbase.de.v0_max.json`
//!   — 91 KB max-conformance `COMPOSITION` exercising most RM data types.
//! - `aql/org/ehrbase/openehr/aqlengine/testdata/composition.json`
//!   — 43 KB Corona_Anamnese `COMPOSITION`.
//! - `config/composition.json` — minimal-evaluation `COMPOSITION`.
//! - `config/ehr_status.json` — `EHR_STATUS`.
//! - `config/contribution.json` — EXCLUDED: although its `_type` is
//!   `CONTRIBUTION` (an RM class we have), the file is a Java
//!   `String.format` template, not JSON — its `versions[0].data` slot is a
//!   bare `%s` placeholder, so it cannot be parsed by any JSON parser.
//!   There is nothing to round-trip until a composition is spliced in,
//!   which would no longer be the real fixture.
//!
//! ## Normalization rules (each one ITS-JSON-legal, see the fn docs)
//!
//! - **R1** — strip a `_type` key from OUR output wherever the original
//!   omits it at the same position (redundant-but-legal explicit tagging).
//! - **R2** — compare JSON numbers by numeric value (`as_f64`), not by
//!   serde_json's internal integer/float variant.
//!
//! No rule ever touches a position where the ORIGINAL carries information
//! we dropped — that stays a hard failure.

use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use openehr_rm::ehr::composition::Composition;
use openehr_rm::ehr::ehr_status::EhrStatus;

/// Vendored ITS-JSON schema (see `docs/VERSIONS.md` for the commit pin).
const RM_SCHEMA: &str = include_str!("../schemas/openehr_rm_1.1.0_all.json");

/// Maximum number of mismatched paths reported before truncating — the
/// fixtures are up to 91 KB, so dumping both documents on failure would be
/// useless noise.
const MAX_REPORTED_DIFFS: usize = 25;

/// Loads one real fixture from the EHRbase test resources that were moved
/// into `crates/openehr-server/tests/resources/` at Phase 0.
fn load_fixture(rel: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../openehr-server/tests/resources")
        .join(rel);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read fixture {}: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("fixture {} is not valid JSON: {e}", path.display()))
}

/// Deserializes the raw fixture `Value` into the target RM type, using
/// `serde_path_to_error` so a failure names the exact JSON path (e.g.
/// `content[3].data.events[0].data.items[2].value.magnitude`) instead of
/// serde's bare "data did not match any variant".
fn deserialize_with_path<T: DeserializeOwned>(class: &str, rel: &str, original: &Value) -> T {
    match serde_path_to_error::deserialize::<_, T>(original) {
        Ok(v) => v,
        Err(e) => panic!(
            "{class} fixture `{rel}`: deserialization failed\n  at path: {}\n  error: {}",
            e.path(),
            e.inner()
        ),
    }
}

/// Validates one `Value` against a single class definition of the vendored
/// ITS-JSON schema, exactly as `full_rm_canonical_json.rs` does (draft-07,
/// `$ref` into the shared `definitions` map). Returns the error list.
fn schema_errors(class: &str, value: &Value) -> Vec<String> {
    let root: Value = serde_json::from_str(RM_SCHEMA).expect("vendored schema must parse");
    let class_schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema",
        "definitions": root["definitions"],
        "$ref": format!("#/definitions/{class}"),
    });
    let validator = jsonschema::validator_for(&class_schema)
        .unwrap_or_else(|e| panic!("{class}: schema compile failed: {e}"));
    validator
        .iter_errors(value)
        .map(|e| format!("    at {}: {e}", e.instance_path()))
        .collect()
}

/// Normalization rule **R1**: remove `_type` from OUR re-serialized output
/// wherever the ORIGINAL omits it at the same structural position.
///
/// Why this is ITS-JSON-legal: the ITS-JSON convention (and the vendored
/// schema, where every `_type` property is an optional `const`, never in
/// `required`) mandates `_type` only where the statically declared type of
/// the containing slot is ABSTRACT; on a concretely-declared slot the tag
/// is optional. Our serializer mirrors stock EHRbase *output* and emits
/// `_type` on every object (ADR-002 / `serde_support.rs`), while these
/// hand-written EHRbase test files omit it on concrete slots — both spell
/// the same instance. Stripping only OUR redundant tags (never the
/// original's) means a `_type` that WE fail to emit where the original has
/// one still fails equality, and the schema-validation step independently
/// checks every tag we do emit against its `const` value.
///
/// The walk is strictly parallel (objects by key, arrays by index); a key
/// present on only one side is left alone so it surfaces as a difference.
fn strip_redundant_type_tags(ours: &mut Value, original: &Value) {
    match (ours, original) {
        (Value::Object(ours), Value::Object(original)) => {
            if ours.contains_key("_type") && !original.contains_key("_type") {
                ours.shift_remove("_type");
            }
            for (key, ours_child) in ours.iter_mut() {
                if let Some(original_child) = original.get(key) {
                    strip_redundant_type_tags(ours_child, original_child);
                }
            }
        }
        (Value::Array(ours), Value::Array(original)) => {
            for (ours_child, original_child) in ours.iter_mut().zip(original) {
                strip_redundant_type_tags(ours_child, original_child);
            }
        }
        _ => {}
    }
}

/// Structural comparison collecting the paths of every mismatch (bounded
/// by [`MAX_REPORTED_DIFFS`]). Identical to `Value` equality — objects are
/// key-order-insensitive maps — except for normalization rule **R2**:
/// numbers compare by numeric value (`as_f64`) rather than by serde_json's
/// internal u64/i64/f64 variant.
///
/// Why R2 is ITS-JSON-legal: JSON itself has a single number type; the
/// schema types RM `Real`/`Double` fields as JSON `number`, which admits
/// integer literals. An original writing `"magnitude": 78` for a Real slot
/// and our re-serialization of the parsed `f64` as `78.0` denote the same
/// value. (`as_f64` is exact for every integer below 2^53; RM integer
/// fields — counts, version numbers, ordinals — are nowhere near it.)
fn collect_diffs(path: &str, ours: &Value, original: &Value, out: &mut Vec<String>) {
    if out.len() >= MAX_REPORTED_DIFFS {
        return;
    }
    match (ours, original) {
        (Value::Object(ours), Value::Object(original)) => {
            for (key, ours_child) in ours {
                match original.get(key) {
                    Some(original_child) => {
                        collect_diffs(&format!("{path}.{key}"), ours_child, original_child, out);
                    }
                    None => out.push(format!(
                        "{path}.{key}: present in OUR output, absent in original (= {ours_child})"
                    )),
                }
            }
            for (key, original_child) in original {
                if !ours.contains_key(key) {
                    out.push(format!(
                        "{path}.{key}: present in original, MISSING from our output (= {original_child})"
                    ));
                }
            }
        }
        (Value::Array(ours), Value::Array(original)) => {
            if ours.len() != original.len() {
                out.push(format!(
                    "{path}: array length {} (ours) vs {} (original)",
                    ours.len(),
                    original.len()
                ));
            }
            for (i, (ours_child, original_child)) in ours.iter().zip(original).enumerate() {
                collect_diffs(&format!("{path}[{i}]"), ours_child, original_child, out);
            }
        }
        // Normalization rule R2: numeric-value comparison.
        (Value::Number(a), Value::Number(b)) => {
            if a != b && a.as_f64() != b.as_f64() {
                out.push(format!("{path}: number {a} (ours) vs {b} (original)"));
            }
        }
        (a, b) => {
            if a != b {
                out.push(format!("{path}: {a} (ours) vs {b} (original)"));
            }
        }
    }
}

/// The full per-fixture harness: load → schema-validate original →
/// deserialize (path-diagnosed) → re-serialize → schema-validate ours →
/// normalized value equality.
fn run_round_trip<T: DeserializeOwned + Serialize>(class: &str, rel: &str) {
    let original = load_fixture(rel);

    // The REAL file must itself validate against the vendored schema —
    // otherwise it is not a legitimate oracle for step 5.
    let original_errors = schema_errors(class, &original);
    assert!(
        original_errors.is_empty(),
        "{class} fixture `{rel}`: the ORIGINAL file does not validate against the pinned ITS-JSON schema:\n{}",
        original_errors.join("\n")
    );

    let parsed: T = deserialize_with_path(class, rel, &original);

    let mut ours = serde_json::to_value(&parsed)
        .unwrap_or_else(|e| panic!("{class} fixture `{rel}`: re-serialization failed: {e}"));

    let ours_errors = schema_errors(class, &ours);
    assert!(
        ours_errors.is_empty(),
        "{class} fixture `{rel}`: OUR re-serialized output does not validate against the pinned ITS-JSON schema:\n{}",
        ours_errors.join("\n")
    );

    // Normalization rule R1, then bounded structural diff (rule R2 lives
    // inside collect_diffs).
    strip_redundant_type_tags(&mut ours, &original);
    let mut diffs = Vec::new();
    collect_diffs("$", &ours, &original, &mut diffs);
    assert!(
        diffs.is_empty(),
        "{class} fixture `{rel}`: round-trip is not value-identical to the original ({}{} difference(s)):\n  {}",
        diffs.len(),
        if diffs.len() >= MAX_REPORTED_DIFFS {
            "+ (truncated)"
        } else {
            ""
        },
        diffs.join("\n  ")
    );
}

/// The crown jewel: EHRbase's max-conformance composition (91 KB),
/// exercising most RM data types in one document.
#[test]
fn conformance_max_composition_round_trips() {
    run_round_trip::<Composition>(
        "COMPOSITION",
        "service/org/ehrbase/repository/conformance_ehrbase.de.v0_max.json",
    );
}

/// The Corona_Anamnese composition used by the EHRbase AQL-engine tests
/// (43 KB).
#[test]
fn aql_corona_anamnese_composition_round_trips() {
    run_round_trip::<Composition>(
        "COMPOSITION",
        "aql/org/ehrbase/openehr/aqlengine/testdata/composition.json",
    );
}

/// The minimal-evaluation composition from the EHRbase config test set.
#[test]
fn config_composition_round_trips() {
    run_round_trip::<Composition>("COMPOSITION", "config/composition.json");
}

/// The EHR_STATUS from the EHRbase config test set. (Its
/// `subject.external_ref.id.value` is the literal string `"%s"` — an
/// EHRbase substitution slot, but valid JSON, so it round-trips as-is.)
#[test]
fn config_ehr_status_round_trips() {
    run_round_trip::<EhrStatus>("EHR_STATUS", "config/ehr_status.json");
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: EHRbase test resources (crates/openehr-server/tests/resources) + ITS-JSON pinned commit 5acae056248e917a4b4c56f7e712f4fcfeb616a6
//   source_loc: n/a
//   confidence: high
//   todos: 0
//   note: real-world acceptance harness over EHRbase fixture files — deserialize → re-serialize → normalized Value equality (rules R1/R2 documented in-file) + schema validation of both sides; contribution.json excluded (Java %s format template, not JSON)
// ─────────────────────────────────────────────
