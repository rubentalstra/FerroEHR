//! Real-world canonical-JSON round-trip harness — the PRIMARY acceptance
//! oracle for `openehr-serde`.
//!
//! This suite replaced the old circular fixture suite (hand-built Rust
//! objects serialized against our own schema). It instead drives REAL
//! openEHR canonical-JSON produced by `ehrbase/openEHR_SDK` (the serializer
//! EHRbase itself uses) plus four in-repo EHRbase test resources, so it
//! tests genuine interoperability rather than our own assumptions.
//!
//! For every round-trippable file (see `corpus::round_trippable`) the harness:
//!
//!   a. reads and parses it to a [`serde_json::Value`] (the parse gate);
//!   b. deserializes it into the dispatched RM type using
//!      [`serde_path_to_error`], so a failure names the exact JSON path;
//!   c. re-serializes the object back to a [`serde_json::Value`];
//!   d. asserts SEMANTIC equality input ≈ output under the minimal
//!      normalizer documented below, reporting a path-located diff on
//!      mismatch; and
//!   e. validates the re-serialized OUTPUT against the vendored ITS-JSON
//!      1.1.0 schema definition for the top class.
//!
//! It is data-driven: `real_world_corpus_round_trips` runs every file and
//! reports ALL failures at once (with file names and paths), so a single run
//! shows the whole picture instead of stopping at the first red file.
//!
//! ## The normalizer (minimal, and each rule ITS-JSON-justified)
//!
//! Input and output are compared as [`serde_json::Value`]s with these — and
//! ONLY these — tolerances:
//!
//! - **R1 (redundant `_type`)** — if an INPUT object node has no `_type` but
//!   the corresponding OUTPUT node does, that output `_type` is ignored.
//!   ITS-JSON requires `_type` only where the declared slot type is abstract;
//!   emitting it on a concrete slot is legal-but-redundant, and our
//!   serializer (ADR-002) always emits it while these files often omit it on
//!   concrete slots. If BOTH sides carry `_type`, they must be EQUAL — a
//!   wrong tag is a hard failure. A `_type` the INPUT supplied is never
//!   ignored (a tag WE fail to emit where the original has one still fails).
//! - **R2 (numbers)** — JSON numbers compare by `f64` value, so an original
//!   `5` and our `5.0` for a `Real`/`Double` slot are equal. JSON has a
//!   single number type; the schema types RM real fields as `number`, which
//!   admits integer literals. (`as_f64` is exact for every integer below
//!   2^53; RM integer fields are nowhere near it.)
//!
//! Everything else must match exactly: same keys, same array order, same
//! scalar values, nothing dropped or added. No rule ever masks a position
//! where the ORIGINAL carries information we lost.

mod corpus;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use openehr_rm::common::change_control::contribution::Contribution;
use openehr_rm::common::directory::folder::Folder;
use openehr_rm::data_structures::item_structure::item_tree::ItemTree;
use openehr_rm::ehr::composition::Composition;
use openehr_rm::ehr::ehr_status::EhrStatus;

/// Maximum number of mismatched paths reported per file before truncating —
/// fixtures reach ~91 KB, so dumping whole documents would be useless noise.
const MAX_REPORTED_DIFFS: usize = 25;

/// Deserializes `input` into `T` (path-diagnosed) and re-serializes it back
/// to a `Value`. Returns the re-serialized output or a human error string.
fn round_trip<T: DeserializeOwned + Serialize>(input: &Value) -> Result<Value, String> {
    let parsed: T = serde_path_to_error::deserialize(input).map_err(|e| {
        format!(
            "deserialization failed\n    at path: {}\n    error: {}",
            e.path(),
            e.inner()
        )
    })?;
    serde_json::to_value(&parsed).map_err(|e| format!("re-serialization failed: {e}"))
}

/// Dispatches the round-trip on the top-level class string.
fn round_trip_as(class: &str, input: &Value) -> Result<Value, String> {
    match class {
        "COMPOSITION" => round_trip::<Composition>(input),
        "EHR_STATUS" => round_trip::<EhrStatus>(input),
        "CONTRIBUTION" => round_trip::<Contribution>(input),
        "FOLDER" => round_trip::<Folder>(input),
        "ITEM_TREE" => round_trip::<ItemTree>(input),
        other => Err(format!(
            "no round-trip dispatch for top-level class `{other}`"
        )),
    }
}

/// Normalizer rule **R1**: remove `_type` from OUR output wherever the
/// ORIGINAL omits it at the same structural position. The walk is strictly
/// parallel (objects by key, arrays by index); a key present on only one
/// side is left alone so it surfaces as a difference in [`collect_diffs`].
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

/// Structural comparison collecting the paths of every mismatch (bounded by
/// [`MAX_REPORTED_DIFFS`]). Identical to `Value` equality except for
/// normalizer rule **R2**: numbers compare by `as_f64` rather than by
/// serde_json's internal u64/i64/f64 variant.
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
        // Normalizer rule R2: numeric-value comparison.
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

/// Runs the full per-file pipeline (steps b–e), returning `Ok(())` on a clean
/// round-trip or a formatted failure describing the first failing step.
fn check_file(class: &str, input: &Value) -> Result<(), String> {
    // b + c: deserialize (path-diagnosed) then re-serialize.
    let mut ours = round_trip_as(class, input)?;

    // e: OUR output must validate against the pinned ITS-JSON schema.
    let output_errors = corpus::schema_errors(class, &ours);
    if !output_errors.is_empty() {
        return Err(format!(
            "re-serialized output does NOT validate against the ITS-JSON {class} schema:\n{}",
            output_errors.join("\n")
        ));
    }

    // d: normalized (R1 then R2-aware) structural equality with the original.
    strip_redundant_type_tags(&mut ours, input);
    let mut diffs = Vec::new();
    collect_diffs("$", &ours, input, &mut diffs);
    if !diffs.is_empty() {
        return Err(format!(
            "round-trip is not value-identical to the original ({}{} difference(s)):\n  {}",
            diffs.len(),
            if diffs.len() >= MAX_REPORTED_DIFFS {
                "+ (truncated)"
            } else {
                ""
            },
            diffs.join("\n  ")
        ));
    }
    Ok(())
}

/// Runs `f` on a worker thread with a large stack and propagates any panic.
///
/// PORT NOTE: serde's `#[serde(untagged)]` deserialization buffers and
/// recursively re-probes each candidate variant, so the deeply-nested
/// conformance compositions (~91 KB) can exhaust the default 2 MiB test-
/// thread stack. Running on a 64 MiB stack removes the dependency on a
/// `RUST_MIN_STACK` env var. (A `PERF(port)` note: hand-written deserializers
/// or a tagged dispatch would avoid the re-probing entirely — deferred, this
/// is only the acceptance harness, not a hot path.)
fn on_big_stack<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(f)
        .expect("spawn big-stack worker thread")
        .join()
        .expect("round-trip worker thread panicked");
}

/// The primary oracle: every round-trippable real file, all failures
/// reported together.
#[test]
fn real_world_corpus_round_trips() {
    on_big_stack(|| {
        let files = corpus::round_trippable();
        let total = files.len();
        let mut failures = Vec::new();

        for file in &files {
            let input = corpus::read_json(&file.path);
            if let Err(msg) = check_file(&file.class, &input) {
                failures.push(format!("[{}] ({})\n  {}", file.id, file.class, msg));
            }
        }

        let passed = total - failures.len();
        // Printed even on success (visible under `--nocapture`) so the pass
        // count is easy to confirm.
        eprintln!("real-world round-trip: {passed}/{total} files clean");

        assert!(
            failures.is_empty(),
            "{}/{} real-world files failed round-trip:\n\n{}",
            failures.len(),
            total,
            failures.join("\n\n")
        );
    });
}

/// Every excluded AND every round-trip-ignored vendored file must still exist
/// on disk, so `corpus::EXCLUSIONS` / `corpus::ROUND_TRIP_IGNORED` stay
/// auditable: a vanished (or renamed) vendored file becomes a red test rather
/// than a silently missing oracle.
#[test]
fn excluded_and_ignored_files_still_exist() {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/vendor/openehr_sdk");
    let mut missing = Vec::new();
    for (rel, reason) in corpus::EXCLUSIONS {
        if !root.join(rel).exists() {
            missing.push(format!("  {rel}  (excluded: {reason})"));
        }
    }
    for (rel, reason) in corpus::ROUND_TRIP_IGNORED {
        if !root.join(rel).exists() {
            missing.push(format!("  {rel}  (round-trip-ignored: {reason})"));
        }
    }
    assert!(
        missing.is_empty(),
        "{} excluded/ignored vendored file(s) are missing — re-vendor or update the lists:\n{}",
        missing.len(),
        missing.join("\n")
    );
}

/// Drives one [`corpus::ROUND_TRIP_IGNORED`] file through the same pipeline as
/// the data-driven test. These are `#[ignore]`d because they cannot byte-
/// round-trip against archie/SDK output (see each `#[ignore]` reason); run
/// `cargo test -- --ignored` to observe each failing for exactly that reason
/// (the `lower_included`/`upper_included` our output re-adds).
fn run_ignored(rel: &str) {
    let rel = rel.to_string();
    on_big_stack(move || {
        let file = corpus::ignored_file(&rel);
        let input = corpus::read_json(&file.path);
        if let Err(msg) = check_file(&file.class, &input) {
            panic!("[{}] ({})\n  {}", file.id, file.class, msg);
        }
    });
}

#[test]
#[ignore = "RM 1.0.4/archie omits default-valued Interval.lower_included/upper_included; ITS-JSON 1.1.0 requires them, so our output re-adds them"]
fn ignored_all_types_no_multimedia() {
    run_ignored("composition/canonical_json/all_types_no_multimedia.json");
}

#[test]
#[ignore = "RM 1.0.4/archie omits default-valued Interval.lower_included/upper_included; ITS-JSON 1.1.0 requires them, so our output re-adds them"]
fn ignored_all_types_systematic_tests() {
    run_ignored("composition/canonical_json/all_types_systematic_tests.json");
}

#[test]
#[ignore = "RM 1.0.4/archie omits default-valued Interval.lower_included/upper_included; ITS-JSON 1.1.0 requires them, so our output re-adds them"]
fn ignored_all_types_systematic_tests_feeder_audit() {
    run_ignored("composition/canonical_json/all_types_systematic_tests_feeder_audit.json");
}

#[test]
#[ignore = "RM 1.0.4/archie omits default-valued Interval.lower_included/upper_included; ITS-JSON 1.1.0 requires them, so our output re-adds them"]
fn ignored_datetime_tests() {
    run_ignored("composition/canonical_json/datetime_tests.json");
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: ehrbase/openEHR_SDK canonical_json corpus @ 22b01e0c (tests/vendor) + in-repo EHRbase resources + ITS-JSON pinned commit 5acae056248e917a4b4c56f7e712f4fcfeb616a6
//   source_loc: n/a
//   confidence: high
//   note: PRIMARY acceptance oracle — data-driven real-world deserialize → re-serialize → normalized (R1/R2) Value equality + ITS-JSON schema validation of OUR output; all failures reported at once; exclusions + round-trip-ignored files audited by excluded_and_ignored_files_still_exist; 4 naked-DV_INTERVAL files carried as #[ignore]d tests (archie omits default-valued included flags)
// ─────────────────────────────────────────────
