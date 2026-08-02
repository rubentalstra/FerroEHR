//! Shared corpus plumbing for the canonical-JSON gates (`fidelity.rs`,
//! `canonical_contract.rs`): the corpus walker and the single documented
//! exclusion list, so the gates can never drift apart on what counts as a
//! canonical RM root.

use std::fs;
use std::path::{Path, PathBuf};

/// Every `.json` under `tests/vendor/` **and** `tests/fixtures/twins/`, sorted
/// for determinism.
///
/// The twins directory carries the repo-authored VALID half of each adjudicated
/// defective vendored fixture (`fixture_twins.rs`). It joins the corpus walk on
/// purpose: excluding a defective vendored document without admitting its
/// corrected twin would silently narrow what these gates cover, and the twins
/// rule exists precisely so a spec-correct refusal costs no coverage.
pub(crate) fn corpus_files() -> Vec<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    let mut stack = vec![
        manifest.join("tests/vendor"),
        manifest.join("tests/fixtures/twins"),
    ];
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

/// The corpus-relative key of `path`: the path with its corpus root
/// (`tests/vendor/` or `tests/fixtures/`) stripped and separators normalized.
///
/// One derivation for every gate, so an absolute, machine-dependent path can
/// never reach a snapshot manifest or an exclusion lookup.
pub(crate) fn corpus_rel(path: &Path) -> String {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rel = [manifest.join("tests/vendor"), manifest.join("tests/fixtures")]
        .iter()
        .find_map(|root| path.strip_prefix(root).ok())
        .unwrap_or(path);
    rel.display().to_string().replace('\\', "/")
}

/// The file a gate should actually read for the vendored fixture at `path`:
/// its repo-authored VALID TWIN when one exists, else `path` itself.
///
/// A gate that walks a vendored directory DIRECTLY (rather than through
/// [`corpus_files`] + [`excluded`]) still has to honour the twins adjudication,
/// or it re-reads a document the corpus gates already refused. Routing through
/// here keeps one substitution rule for every gate.
pub(crate) fn twinned(path: &Path) -> PathBuf {
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return path.to_path_buf();
    };
    let twin = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/twins")
        .join(format!("{stem}.valid.json"));
    if twin.is_file() {
        twin
    } else {
        path.to_path_buf()
    }
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
        // DEFECTIVE upstream fixture, adjudicated: two ENTRY nodes hoist
        // `feeder_system_audit` onto the ENTRY itself, while three sibling
        // nodes in the SAME document nest it correctly. That member belongs to
        // FEEDER_AUDIT
        // (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.feeder_audit.adoc`
        // §Attributes), reachable from a LOCATABLE only through
        // `LOCATABLE.feeder_audit` (`…common.locatable.adoc` §Attributes). Kept
        // vendored verbatim as the INVALID twin; the corrected VALID twin and
        // the asserted refusal live in `fixture_twins.rs`.
        "composition/canonical_json/all_types_systematic_tests_feeder_audit.json" => reason(
            "defective fixture: two ENTRY nodes hoist FEEDER_AUDIT.feeder_system_audit onto \
             the ENTRY (valid twin in tests/fixtures/twins/, refusal in fixture_twins.rs)",
        ),
        // DEFECTIVE upstream fixture class, adjudicated (14 documents): a
        // PLACEHOLDER `OBJECT_VERSION_ID`
        // (`__THIS_SHOULD_BE_MODIFIED_BY_THE_TEST_::ehrbase.org::1`) whose
        // `object_id` matches none of the three `uid` productions of BASE
        // `base_types/master05-identification_package.adoc` §Syntaxes
        // (`uid = iso_oid | uuid | internet_id`; an `internet_id` label must
        // begin with a letter). Each is kept vendored verbatim as the INVALID
        // twin, with its corrected VALID twin in `tests/fixtures/twins/` — so
        // the coverage these documents provide is preserved, not lost.
        // Asserted refusals: `fixture_twins.rs`.
        "composition/canonical_json/alternative_types.json"
        | "composition/canonical_json/duration_tests.json"
        | "composition/canonical_json/laboratory_report.json"
        | "composition/canonical_json/laboratory_report_no_content.json"
        | "composition/canonical_json/minimal_admin.json"
        | "composition/canonical_json/minimal_evaluation_item_tree_name.json"
        | "composition/canonical_json/minimal_observation.json"
        | "composition/canonical_json/minimal_persistent.json"
        | "composition/canonical_json/nested.json"
        | "composition/canonical_json/obs_admin.json"
        | "composition/canonical_json/obs_admin_null_flavour.json"
        | "composition/canonical_json/obs_eva.json"
        | "composition/canonical_json/obs_inst.json"
        | "composition/canonical_json/time_series.json" => reason(
            "defective fixture: a placeholder OBJECT_VERSION_ID whose object_id is not a \
             legal uid (valid twin in tests/fixtures/twins/, refusal in fixture_twins.rs)",
        ),
        // DEFECTIVE upstream fixture, adjudicated: `composer.external_ref.id` is a
        // FHIR-style reference (`PractitionerRole/12345-mock`). `PARTY_REF.id` is
        // an `OBJECT_ID`; as a `HIER_OBJECT_ID` its root must be a `uid`, and no
        // `uid` production admits `/`
        // (`docs/specs/openehr/BASE/docs/base_types/master05-identification_package.adoc`
        // §Syntaxes). Valid twin in `tests/fixtures/twins/`.
        "composition/canonical_json/simple_composition_dvinterval.json" => reason(
            "defective fixture: a FHIR-style HIER_OBJECT_ID root that is not a uid \
             (valid twin in tests/fixtures/twins/, refusal in fixture_twins.rs)",
        ),
        // DEFECTIVE upstream fixture, adjudicated: three `uid` slots are tagged
        // `_type: OBJECT_VERSION_ID` while carrying a BARE UUID. §Syntaxes gives
        // `object_version_id = object_id, '::', creating_system_id, '::',
        // version_tree_id` — exactly three parts — so a one-part value is a
        // `HIER_OBJECT_ID`, the other subtype of the declared `UID_BASED_ID`
        // slot. Valid twin in `tests/fixtures/twins/` (re-tagged, values kept).
        "folder/canonical_json/folder_with_items.json" => reason(
            "defective fixture: bare-UUID values tagged OBJECT_VERSION_ID (valid twin in \
             tests/fixtures/twins/, refusal in fixture_twins.rs)",
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
