//! Shared real-world corpus plumbing for the `openehr-serde` acceptance
//! suite (`real_world_round_trip.rs`, `class_coverage.rs`, `gap_fixtures.rs`).
//!
//! This module is a test-support helper (it lives in a `tests/` subdirectory
//! so Cargo does **not** compile it as its own test binary — the standard
//! `tests/<name>/mod.rs` shared-code convention). It owns:
//!
//! - the vendored ITS-JSON schema handle ([`RM_SCHEMA`]),
//! - the round-trippable corpus enumeration ([`round_trippable`]), which
//!   walks the vendored `ehrbase/openEHR_SDK` `canonical_json` corpus plus
//!   the four in-repo EHRbase test resources, skipping the documented
//!   [`EXCLUSIONS`],
//! - per-class JSON-Schema validation ([`schema_errors`]), the same draft-07
//!   `$ref`-into-`definitions` technique the deleted `full_rm_canonical_json.rs`
//!   used, and
//! - `_type` collection helpers for the coverage test.
//!
//! Everything here is READ-ONLY over external ground truth — the vendored
//! files are never edited (see `tests/vendor/PROVENANCE.md`).
//!
//! This is a shared test-support module: each integration-test binary that
//! `mod corpus;`-includes it uses only a subset of these helpers, so
//! `dead_code` is expected and silenced crate-locally for this file.
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};

/// The vendored ITS-JSON RM 1.1.0 all-schema (draft-07), pinned at commit
/// `5acae056248e917a4b4c56f7e712f4fcfeb616a6` (see `docs/VERSIONS.md`).
pub const RM_SCHEMA: &str = include_str!("../../schemas/openehr_rm_1.1.0_all.json");

/// A single round-trippable corpus entry.
pub struct CorpusFile {
    /// Short, stable human label used in test output (e.g.
    /// `vendor/composition/nested.json`, `in-repo/config/ehr_status.json`).
    pub id: String,
    /// Absolute path to the JSON file on disk.
    pub path: PathBuf,
    /// The RM class the top-level object is dispatched as, keyed off its
    /// `_type` (or an explicit override for the one legal `_type`-less file).
    pub class: String,
}

/// Vendored files that are deliberately **excluded** from the RM round-trip
/// corpus, each paired with the precise reason it is not an RM-canonical
/// instance. Kept as data (not just skipped silently) so `real_world_round_trip.rs`
/// can assert every one still exists on disk — the exclusion stays auditable,
/// and a vanished vendored file becomes a red test rather than a silent gap.
///
/// The first block is the set named by the task; the second block
/// (`contribution-*.json`) is an evidence-based addition — see the note.
pub const EXCLUSIONS: &[(&str, &str)] = &[
    // Legacy Jackson `@class` discriminator, not the ITS-JSON `_type` form.
    (
        "composition/canonical_json/full_composition.json",
        "legacy Jackson @class discriminator, not ITS-JSON _type",
    ),
    // EHRbase decomposed row-per-locatable DB format (`/$CLASS$`, `/name`
    // keys), not canonical JSON.
    (
        "composition/canonical_json/rawdb_composition.json",
        "EHRbase decomposed DB row format, not canonical JSON",
    ),
    (
        "composition/canonical_json/rawdb_composition_history.json",
        "EHRbase decomposed DB row format, not canonical JSON",
    ),
    (
        "composition/canonical_json/rawdb_composition_observation_event.json",
        "EHRbase decomposed DB row format, not canonical JSON",
    ),
    (
        "composition/canonical_json/rawdb_composition_observation_event_item.json",
        "EHRbase decomposed DB row format, not canonical JSON",
    ),
    (
        "composition/canonical_json/rawdb_returning_array.json",
        "EHRbase decomposed DB row format, not canonical JSON",
    ),
    // Deliberate RM-invalid negatives (parse/validation negatives, not
    // round-trip oracles).
    (
        "composition/canonical_json/invalid.json",
        "deliberate RM-invalid negative",
    ),
    (
        "composition/canonical_json/ips_invalid.json",
        "deliberate RM-invalid negative",
    ),
    // EHRbase CONTRIBUTION *request* DTOs (`{versions, audit}` with the
    // full ORIGINAL_VERSIONs to be committed), not the RM CONTRIBUTION object
    // (whose `versions` are OBJECT_REFs). These lack the top-level `_type`.
    (
        "contribution/canonical_json/latest-contribution-one_entry-composition.json",
        "EHRbase {versions, audit} commit-request DTO, not RM CONTRIBUTION",
    ),
    (
        "contribution/canonical_json/latest-contribution-one_entry-composition-deletion.json",
        "EHRbase {versions, audit} commit-request DTO, not RM CONTRIBUTION",
    ),
    (
        "contribution/canonical_json/latest-contribution-one_entry-composition-modification.json",
        "EHRbase {versions, audit} commit-request DTO, not RM CONTRIBUTION",
    ),
    (
        "contribution/canonical_json/status.contribution.modification.json",
        "EHRbase {versions, audit} commit-request DTO, not RM CONTRIBUTION",
    ),
    // Evidence-based addition (beyond the task's named list): these two carry
    // a top-level `"_type": "CONTRIBUTION"` label, but structurally they are
    // the SAME EHRbase commit-request DTO as the four above — their `versions`
    // array holds full ORIGINAL_VERSION objects (keys `commit_audit`, `data`,
    // `lifecycle_state`) and they have NO `uid`. The ITS-JSON schema's
    // CONTRIBUTION definition requires `uid` and types `versions` as an array
    // of OBJECT_REF (enum: LOCATABLE_REF/PARTY_REF/ACCESS_GROUP_REF/OBJECT_REF),
    // so both files are schema-INVALID as RM CONTRIBUTION and cannot deserialize
    // into `openehr_rm::…::Contribution` (`versions: Vec<ObjectRef>`). The RM
    // type and schema agree (RM 1.1.0 CONTRIBUTION.versions: Set<OBJECT_REF>);
    // the files are mislabelled DTOs, not a bug in our serde. There is thus no
    // real-world RM CONTRIBUTION oracle in the corpus — CONTRIBUTION is covered
    // by a minimal synthetic instance in `gap_fixtures.rs` instead.
    (
        "contribution/canonical_json/contribution-one_entry-composition.json",
        "top _type says CONTRIBUTION but shape is the EHRbase commit-request DTO \
         (versions = full ORIGINAL_VERSIONs, no uid); schema-invalid as RM CONTRIBUTION",
    ),
    (
        "contribution/canonical_json/contribution-two_entries-composition.json",
        "top _type says CONTRIBUTION but shape is the EHRbase commit-request DTO \
         (versions = full ORIGINAL_VERSIONs, no uid); schema-invalid as RM CONTRIBUTION",
    ),
    // Evidence-based additions (non-canonical / schema-invalid source files
    // surfaced by the round-trip run; each is skipped for the stated reason,
    // none is an RM-canonical instance):
    //
    // `content` is a FLAT path-keyed object (`{"/content[...]": …}`), i.e. the
    // web-template/flat encoding, not the canonical `content: [CONTENT_ITEM]`
    // array. Belongs to openehr-flat, not canonical JSON.
    (
        "composition/canonical_json/composition_with_dvinterval_composite.json",
        "FLAT path-keyed `content` object, not canonical JSON (belongs to openehr-flat)",
    ),
    // `name` is a DV_TEXT carrying `mappings: []`; the ITS-JSON schema types
    // TERM_MAPPING arrays with minItems=1 and the RM `Mappings_valid`
    // invariant forbids an empty mappings list, so `[]` is non-canonical
    // (canonical form omits the key).
    (
        "folder/canonical_json/flat_folder_insert.json",
        "name.mappings is [] — schema minItems=1 / RM Mappings_valid forbids an empty mappings list",
    ),
    // `folders[*].items[*].id` is serialized with `_type: OBJECT_REF_ID`, an
    // EHRbase-internal type name absent from the ITS-JSON OBJECT_REF.id enum
    // (which allows only HIER_OBJECT_ID/OBJECT_VERSION_ID/ARCHETYPE_ID/
    // TEMPLATE_ID/TERMINOLOGY_ID/GENERIC_ID). Non-canonical id typing.
    (
        "folder/canonical_json/folder_with_items.json",
        "OBJECT_REF.id typed `_type: OBJECT_REF_ID` — not an ITS-JSON OBJECT_ID subtype (EHRbase quirk)",
    ),
    // `folders[1].name` is `{\"_type\":\"DV_TEXT\",\"name\":\"…\"}` — a DV_TEXT
    // with a bogus `name` key and no required `value`. Malformed DV_TEXT.
    (
        "folder/canonical_json/folder_without_duplicates.json",
        "folders[1].name is a DV_TEXT with a `name` key and no required `value` (malformed)",
    ),
    // Omits the schema-required `archetype_node_id`, carries empty
    // `links`/`items`/`folders` arrays (schema minItems=1), and an extra
    // non-schema `path` key. Non-canonical folder.
    (
        "folder/canonical_json/simple_empty_folder.json",
        "missing required archetype_node_id; empty links/items/folders arrays; extra non-schema `path` key",
    ),
    // Standalone ITEM_TREE fragment (an EHR_STATUS.other_details slice) that
    // omits the schema-required `name` and `archetype_node_id`. ITEM_TREE is
    // otherwise well covered inside the composition corpus.
    (
        "item_structure/canonical_json/ehr_other_details.json",
        "standalone ITEM_TREE fragment missing required name + archetype_node_id",
    ),
    // The `all_types` systematic composition, feeder-audit variant. Beyond the
    // naked-`DV_INTERVAL` default flags that normalizer R3 tolerates (see
    // `real_world_round_trip.rs`), this file places a bare
    // `feeder_system_audit` (a FEEDER_AUDIT_DETAILS) DIRECTLY on an INSTRUCTION
    // (`content[2].items[0].items[0].items[0]`, system_id `SECTION-FA`) and on
    // an ADMIN_ENTRY (`content[2].items[0].items[1]`, system_id
    // `ADMIN-ENTRY-FA`) — i.e. as a stray key on the content item itself, NOT
    // nested inside the RM `feeder_audit: FEEDER_AUDIT` attribute.
    // `feeder_system_audit` is a member of FEEDER_AUDIT, never of
    // LOCATABLE/CONTENT_ITEM/ENTRY, and the ITS-JSON schema types INSTRUCTION
    // and ADMIN_ENTRY with `additionalProperties: false` (neither declares a
    // `feeder_system_audit` property), so both keys are schema-INVALID on those
    // classes. A faithful RM deserializer correctly drops them, so the file
    // cannot byte-round-trip — and re-emitting them would fail this harness's
    // own ITS-JSON output-validation gate. This is a non-canonical source, not
    // a serde bug: verified independently that a WELL-FORMED `feeder_audit` on
    // these same nested-in-SECTION content items round-trips cleanly (5→5), and
    // the file's three well-formed `feeder_audit`s (on COMPOSITION,
    // content[0] OBSERVATION, content[1] EVALUATION) DO survive intact — only
    // the two stray keys are lost. FEEDER_AUDIT / FEEDER_AUDIT_DETAILS coverage
    // is retained via `compo_feeder_audit_details.json`.
    (
        "composition/canonical_json/all_types_systematic_tests_feeder_audit.json",
        "places a bare `feeder_system_audit` directly on an INSTRUCTION and an ADMIN_ENTRY \
         (not inside the RM `feeder_audit: FEEDER_AUDIT`); ITS-JSON forbids it on those classes \
         (additionalProperties:false), so a faithful deserializer drops it and it cannot round-trip",
    ),
];

/// Valid RM-canonical files that deserialize and re-serialize cleanly and
/// whose OUTPUT is ITS-JSON 1.1.0 conformant, but which cannot be BYTE-round-
/// tripped against their source because of a documented archie/SDK ↔ ITS-JSON
/// 1.1.0 wire difference that is NOT expressible as a one-directional
/// normalizer rule. Files here are kept OUT of the data-driven round-trip set
/// while still counting toward `coverage_corpus`.
///
/// This list is **currently empty**. It formerly held the four
/// `all_types`/`datetime` `DV_INTERVAL` stress fixtures, which omit the
/// default-valued `Interval.lower_included`/`upper_included` flags (archie
/// declares `boolean lowerIncluded = true` and serializes with default-value
/// omission, while the ITS-JSON 1.1.0 schema lists all four `required`, so our
/// serializer re-emits them). That difference is one-directional and
/// value-preserving, so it is now handled by normalizer rule **R3** in
/// `real_world_round_trip.rs` and those files round-trip in the data-driven
/// test. The feeder-audit variant of that fixture additionally carries
/// schema-invalid stray `feeder_system_audit` keys and moved to [`EXCLUSIONS`]
/// instead (a non-canonical source, not a wire difference). The constant is
/// retained as the extension point for any future file that genuinely cannot
/// round-trip for a reason a normalizer rule must not paper over.
pub const ROUND_TRIP_IGNORED: &[(&str, &str)] = &[];

/// Absolute path to this crate's manifest directory.
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Root of the vendored `ehrbase/openEHR_SDK` canonical-JSON corpus.
fn vendor_root() -> PathBuf {
    manifest_dir().join("tests/vendor/openehr_sdk")
}

/// The four in-repo EHRbase test resources referenced relative to this
/// crate's manifest dir (they live in the sibling `openehr-server` crate).
/// Each is paired with its dispatch class.
const IN_REPO: &[(&str, &str)] = &[
    (
        "../openehr-server/tests/resources/service/org/ehrbase/repository/conformance_ehrbase.de.v0_max.json",
        "COMPOSITION",
    ),
    (
        "../openehr-server/tests/resources/aql/org/ehrbase/openehr/aqlengine/testdata/composition.json",
        "COMPOSITION",
    ),
    (
        "../openehr-server/tests/resources/config/composition.json",
        "COMPOSITION",
    ),
    (
        "../openehr-server/tests/resources/config/ehr_status.json",
        "EHR_STATUS",
    ),
];

/// Files whose top-level object legally OMITS `_type` (ITS-JSON: the tag is
/// required only where the declared slot type is abstract; a top-level FOLDER
/// slot is concrete). Maps the vendor-relative path to its dispatch class.
const TYPELESS_OVERRIDE: &[(&str, &str)] =
    &[("folder/canonical_json/folder_with_items.json", "FOLDER")];

/// Reads and parses a JSON file, panicking with a clear message on failure.
pub fn read_json(path: &PathBuf) -> Value {
    let text =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{} is not valid JSON: {e}", path.display()))
}

/// Recursively lists every `*.json` file under `dir`, returned as
/// vendor-root-relative slash paths, sorted for deterministic test output.
fn list_vendor_json() -> Vec<String> {
    fn walk(dir: &PathBuf, root: &PathBuf, out: &mut Vec<String>) {
        let entries =
            fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read dir {}: {e}", dir.display()));
        for entry in entries {
            let entry = entry.expect("dir entry readable");
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out);
            } else if path.extension().is_some_and(|e| e == "json") {
                let rel = path
                    .strip_prefix(root)
                    .expect("path under vendor root")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push(rel);
            }
        }
    }
    let root = vendor_root();
    let mut out = Vec::new();
    walk(&root, &root, &mut out);
    out.sort();
    out
}

/// Resolves the dispatch class of a top-level canonical-JSON object: its
/// `_type` if present, otherwise the explicit typeless override, else panic
/// (so a newly-added `_type`-less file is never silently misclassified).
fn class_of(rel: &str, value: &Value) -> String {
    if let Some(Value::String(t)) = value.get("_type") {
        return t.clone();
    }
    for (path, class) in TYPELESS_OVERRIDE {
        if rel == *path {
            return (*class).to_string();
        }
    }
    panic!(
        "vendored file `{rel}` has no top-level `_type` and no explicit \
         TYPELESS_OVERRIDE entry — classify it before adding it to the corpus"
    );
}

/// The coverage corpus: every RM-canonical file — every vendored
/// `canonical_json` file not in [`EXCLUSIONS`], plus the four in-repo EHRbase
/// resources. Any [`ROUND_TRIP_IGNORED`] files are also included here (they
/// are valid RM data that reaches real classes, blocked only from the byte
/// round-trip); that list is presently empty, so coverage currently coincides
/// with [`round_trippable`]. Used by `class_coverage.rs`.
pub fn coverage_corpus() -> Vec<CorpusFile> {
    let excluded: BTreeSet<&str> = EXCLUSIONS.iter().map(|(f, _)| *f).collect();
    let root = vendor_root();
    let mut files = Vec::new();

    for rel in list_vendor_json() {
        if excluded.contains(rel.as_str()) {
            continue;
        }
        let path = root.join(&rel);
        let value = read_json(&path);
        let class = class_of(&rel, &value);
        files.push(CorpusFile {
            id: format!("vendor/{rel}"),
            path,
            class,
        });
    }

    for (rel, class) in IN_REPO {
        let path = manifest_dir().join(rel);
        // Strip the `../openehr-server/tests/resources/` prefix for a tidy id.
        let short = rel.rsplit("resources/").next().unwrap_or(rel).to_string();
        files.push(CorpusFile {
            id: format!("in-repo/{short}"),
            path,
            class: (*class).to_string(),
        });
    }

    files
}

/// The data-driven round-trip corpus: [`coverage_corpus`] minus any
/// [`ROUND_TRIP_IGNORED`] files. Every file here is expected to round-trip to
/// value-equality with its source under the `real_world_round_trip.rs`
/// normalizer. [`ROUND_TRIP_IGNORED`] is presently empty, so this currently
/// equals [`coverage_corpus`].
pub fn round_trippable() -> Vec<CorpusFile> {
    let ignored: BTreeSet<&str> = ROUND_TRIP_IGNORED.iter().map(|(f, _)| *f).collect();
    coverage_corpus()
        .into_iter()
        .filter(|f| {
            // vendored ids are `vendor/<rel>`; keep any non-vendored file.
            !f.id
                .strip_prefix("vendor/")
                .is_some_and(|rel| ignored.contains(rel))
        })
        .collect()
}

/// Parses the vendored schema root once.
pub fn schema_root() -> Value {
    serde_json::from_str(RM_SCHEMA).expect("vendored schema must parse")
}

/// The set of every class definition name in the vendored schema.
pub fn schema_definition_names() -> BTreeSet<String> {
    schema_root()["definitions"]
        .as_object()
        .expect("schema has a definitions map")
        .keys()
        .cloned()
        .collect()
}

/// Validates `value` against one class definition of the vendored ITS-JSON
/// schema — draft-07, `$ref` into the shared `definitions` map — exactly the
/// technique the deleted `full_rm_canonical_json.rs` used. Returns the list
/// of validation error strings (empty = valid). Most definitions carry
/// `additionalProperties: false`, so this catches wrong/extra keys too.
pub fn schema_errors(class: &str, value: &Value) -> Vec<String> {
    let root = schema_root();
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

/// Recursively collects every distinct `_type` string reachable in `value`
/// into `out`.
pub fn collect_types(value: &Value, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(t)) = map.get("_type") {
                out.insert(t.clone());
            }
            for child in map.values() {
                collect_types(child, out);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_types(child, out);
            }
        }
        _ => {}
    }
}
