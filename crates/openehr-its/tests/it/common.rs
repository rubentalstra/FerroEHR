//! Shared corpus plumbing for the canonical-JSON gates (`fidelity.rs`,
//! `canonical_contract.rs`): the corpus walker and the single documented
//! exclusion list, so the gates can never drift apart on what counts as a
//! canonical RM root.

use std::fs;
use std::path::{Path, PathBuf};

/// Every `.json` under `tests/vendor/`, sorted for determinism.
pub(crate) fn corpus_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vendor");
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "json") {
                    out.push(p);
                }
            }
        }
    }
    out.sort();
    out
}

/// Corpus files that are **not** canonical RM 1.2 objects, with the reason each
/// is out of scope for the canonical gates. They are excluded, not silently
/// skipped — the gates still fail if a file *not* listed here fails to read.
/// (Same discipline as the AQL example-corpus exclusions.) Keyed by the
/// trailing path.
///
/// None of these reflect a defect in the generated types: they are either a
/// different serialization (raw-DB / ITS-REST request), deliberately invalid,
/// malformed, or RM-1.1-era data that omits fields RM 1.2 made mandatory.
pub(crate) fn excluded(name: &str) -> Option<&'static str> {
    // Normalize path separators for matching on any platform, and drop the
    // corpus-root prefix so keys are relative to the openEHR_SDK tree.
    let n = name.replace('\\', "/");
    let n = n.strip_prefix("openehr_sdk/").unwrap_or(&n);
    let reason = |r| Some(r);
    match n {
        // Path-keyed `content` map (ethercis raw-DB / flat form), not canonical.
        "composition/canonical_json/rawdb_composition.json"
        | "composition/canonical_json/composition_with_dvinterval_composite.json" => {
            reason("not canonical JSON: `content` is a path-keyed map (raw-DB/flat form)")
        }
        // Deliberately invalid fixture (`EVENT_CONTEXT_WRONG`) — a negative test.
        "composition/canonical_json/invalid.json" => {
            reason("deliberately invalid fixture (a wrong `_type`), a negative test")
        }
        // ITS-REST contribution *request* bodies: `versions` holds full
        // `ORIGINAL_VERSION`s, whereas RM `CONTRIBUTION.versions` is `Set<OBJECT_REF>`.
        // These belong to the REST DTO layer, not the RM canonical gate.
        "contribution/canonical_json/contribution-one_entry-composition.json"
        | "contribution/canonical_json/contribution-two_entries-composition.json" => reason(
            "ITS-REST contribution request shape (versions = ORIGINAL_VERSION, not OBJECT_REF)",
        ),
        // Malformed: a `DV_TEXT` whose value is under a `name` key instead of `value`.
        "folder/canonical_json/folder_without_duplicates.json" => {
            reason("malformed fixture: a DV_TEXT carries its text under `name` instead of `value`")
        }
        // RM-1.1-era EHRbase output that omits fields RM 1.2 makes mandatory on
        // LOCATABLE. Deserialization is strict here; leniency is a validation-layer
        // concern. Tracked as the RM 1.1↔1.2 divergence (docs/VERSIONS.md).
        "folder/canonical_json/simple_empty_folder.json" => {
            reason("RM 1.1-era: FOLDER omits mandatory LOCATABLE.archetype_node_id (RM 1.2)")
        }
        "item_structure/canonical_json/ehr_other_details.json" => {
            reason("RM 1.1-era: ITEM_TREE omits mandatory LOCATABLE.name (RM 1.2)")
        }
        _ => None,
    }
}
