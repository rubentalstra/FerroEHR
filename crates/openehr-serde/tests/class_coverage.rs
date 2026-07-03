//! Transparent RM-class coverage of the real-world corpus.
//!
//! Recursively collects every distinct `_type` reachable across ALL
//! RM-canonical real files (`corpus::coverage_corpus` — the round-trippable
//! set plus any `corpus::ROUND_TRIP_IGNORED` files, which are still valid RM
//! data; that list is presently empty, so coverage currently coincides with
//! the round-trippable set) and pins coverage two ways so it can never
//! silently drift:
//!
//! - [`COVERED_FLOOR`] — the classes the corpus reaches today. The test
//!   asserts every one is STILL reached, so coverage cannot silently shrink.
//! - [`DOCUMENTED_UNCOVERED`] — `schema_classes − covered`. The test asserts
//!   the uncovered set is EXACTLY this list, so a newly-uncovered class (or a
//!   newly-covered one) fails the test until the constant is consciously
//!   updated. Every uncovered class is therefore visible and intentional.
//!
//! Together these two constants partition all 134 schema definitions, so the
//! pair pins the covered set exactly.

mod corpus;

use std::collections::BTreeSet;

/// Classes reached by the real corpus today (derived from a first run, then
/// pinned as a regression floor). 57 classes.
const COVERED_FLOOR: &[&str] = &[
    "ACTION",
    "ACTIVITY",
    "ADMIN_ENTRY",
    "ARCHETYPED",
    "ARCHETYPE_ID",
    "CLUSTER",
    "CODE_PHRASE",
    "COMPOSITION",
    "DV_BOOLEAN",
    "DV_CODED_TEXT",
    "DV_COUNT",
    "DV_DATE",
    "DV_DATE_TIME",
    "DV_DURATION",
    "DV_EHR_URI",
    "DV_IDENTIFIER",
    "DV_INTERVAL",
    "DV_MULTIMEDIA",
    "DV_ORDINAL",
    "DV_PARSABLE",
    "DV_PROPORTION",
    "DV_QUANTITY",
    "DV_TEXT",
    "DV_TIME",
    "DV_URI",
    "EHR_STATUS",
    "ELEMENT",
    "EVALUATION",
    "EVENT_CONTEXT",
    "FEEDER_AUDIT",
    "FEEDER_AUDIT_DETAILS",
    "FOLDER",
    "GENERIC_ID",
    "HIER_OBJECT_ID",
    "HISTORY",
    "INSTRUCTION",
    "INSTRUCTION_DETAILS",
    "INTERVAL_EVENT",
    "ISM_TRANSITION",
    "ITEM_LIST",
    "ITEM_SINGLE",
    "ITEM_TREE",
    "LINK",
    "LOCATABLE_REF",
    "OBJECT_REF",
    "OBJECT_VERSION_ID",
    "OBSERVATION",
    "PARTICIPATION",
    "PARTY_IDENTIFIED",
    "PARTY_REF",
    "PARTY_RELATED",
    "PARTY_SELF",
    "POINT_EVENT",
    "SECTION",
    "TEMPLATE_ID",
    "TERMINOLOGY_ID",
    "TERM_MAPPING",
];

/// Schema classes the real corpus does NOT reach, each intentionally so.
/// `schema_classes − covered` must equal exactly this set. 77 classes,
/// grouped by WHY no real-world canonical-JSON oracle reaches them. The
/// demographic classes and the rare data-value types (marked ★) are the ones
/// `gap_fixtures.rs` covers with minimal synthetic instances.
const DOCUMENTED_UNCOVERED: &[&str] = &[
    // rm.demographic — openEHR deployments keep demographics in a separate
    // repository and rarely serialize these; archie / specifications-RM /
    // openEHR_SDK ship no demographic canonical-JSON corpus (★ gap fixtures).
    "ADDRESS",
    "AGENT",
    "CAPABILITY",
    "CONTACT",
    "GROUP",
    "ORGANISATION",
    "PARTY_IDENTITY",
    "PARTY_RELATIONSHIP",
    "PERSON",
    "ROLE",
    // Rare data-value types absent from the composition corpus (★ gap
    // fixtures).
    "DV_GENERAL_TIME_SPECIFICATION",
    "DV_PARAGRAPH",
    "DV_PERIODIC_TIME_SPECIFICATION",
    "DV_SCALE",
    "DV_STATE",
    // rm.ehr top-level containers not serialized by the composition/folder/
    // ehr_status corpus (EHR is served as separate resources; EHR_ACCESS is
    // effectively unused).
    "EHR",
    "EHR_ACCESS",
    // rm.common.change_control + versioning: the corpus is bare RM objects,
    // never wrapped in versions/contributions (the SDK contribution files are
    // EHRbase commit DTOs, excluded — see corpus::EXCLUSIONS).
    "ATTESTATION",
    "AUDIT_DETAILS",
    "CONTRIBUTION",
    "IMPORTED_VERSION",
    "ORIGINAL_VERSION",
    "REVISION_HISTORY",
    "REVISION_HISTORY_ITEM",
    "VERSIONED_OBJECT",
    // EHRbase REST versioned-object wrapper DTOs (the `X_`-prefixed schema
    // entries) — server-surface shapes, not RM instances in this corpus.
    "X_CONTRIBUTION",
    "X_VERSIONED_COMPOSITION",
    "X_VERSIONED_EHR_ACCESS",
    "X_VERSIONED_EHR_STATUS",
    "X_VERSIONED_FOLDER",
    "X_VERSIONED_OBJECT",
    "X_VERSIONED_PARTY",
    // rm.ehr_extract (EHR EXTRACT) — experimental package, no corpus.
    "ADDRESSED_MESSAGE",
    "EXTRACT",
    "EXTRACT_ACTION_REQUEST",
    "EXTRACT_CHAPTER",
    "EXTRACT_ENTITY_CHAPTER",
    "EXTRACT_ENTITY_MANIFEST",
    "EXTRACT_FOLDER",
    "EXTRACT_MANIFEST",
    "EXTRACT_PARTICIPATION",
    "EXTRACT_REQUEST",
    "EXTRACT_SPEC",
    "EXTRACT_UPDATE_SPEC",
    "EXTRACT_VERSION_SPEC",
    "MESSAGE",
    "SYNC_EXTRACT",
    "SYNC_EXTRACT_REQUEST",
    "SYNC_EXTRACT_SPEC",
    // rm.ehr content variants specific to GENERIC_ENTRY / integration; not in
    // the archetyped composition corpus.
    "GENERIC_CONTENT_ITEM",
    "GENERIC_ENTRY",
    "OPENEHR_CONTENT_ITEM",
    // rm.data_structures: ITEM_TABLE is transcribed but no corpus file uses
    // the tabular structure.
    "ITEM_TABLE",
    // rm.data_types.quantity: REFERENCE_RANGE appears only inside DV_ORDERED
    // normal_range/other_reference_ranges, which the corpus never populates.
    "REFERENCE_RANGE",
    // base.resource (AUTHORED_RESOURCE description) — archetype/template
    // resource metadata, not RM data instances.
    "RESOURCE_DESCRIPTION",
    "RESOURCE_DESCRIPTION_ITEM",
    "TRANSLATION_DETAILS",
    // base.identification leaves not used by the corpus's ids.
    "ARCHETYPE_HRID",
    "INTERNET_ID",
    "ISO_OID",
    "VERSION_TREE_ID",
    "ACCESS_GROUP_REF",
    // base.foundation_types wrappers that only ever appear FLATTENED inside a
    // concrete RM class on the wire, never as a standalone tagged object:
    // the bare container/primitive/interval/time schema entries.
    "ARRAY",
    "LIST",
    "SET",
    "INTERVAL",
    "ISO8601_TYPE",
    "DATE",
    "DATE_TIME",
    "DURATION",
    "TIME",
    "URI",
    "UUID",
    "TERMINOLOGY_CODE",
    "TERMINOLOGY_TERM",
    // base.base_types enumerations serialized as bare strings, never as tagged
    // objects.
    "VALIDITY_KIND",
    "VERSION_STATUS",
];

/// The distinct set of `_type` values reachable across the whole coverage
/// corpus.
fn covered_types() -> BTreeSet<String> {
    let mut covered = BTreeSet::new();
    for file in corpus::coverage_corpus() {
        let value = corpus::read_json(&file.path);
        corpus::collect_types(&value, &mut covered);
    }
    covered
}

/// Every `_type` in the real corpus must be a real schema class — this
/// catches typos and non-canonical `_type` quirks (e.g. `OBJECT_REF_ID`)
/// leaking into the corpus.
#[test]
fn every_covered_type_is_a_schema_class() {
    let covered = covered_types();
    let schema = corpus::schema_definition_names();
    let unknown: Vec<&String> = covered.difference(&schema).collect();
    assert!(
        unknown.is_empty(),
        "corpus contains {} `_type` value(s) that are NOT schema classes: {unknown:?}",
        unknown.len()
    );
}

/// Coverage cannot silently shrink: every class in [`COVERED_FLOOR`] must
/// still be reached by the corpus.
#[test]
fn covered_floor_is_still_reached() {
    let covered = covered_types();
    let floor: BTreeSet<String> = COVERED_FLOOR.iter().map(|s| (*s).to_string()).collect();
    let regressed: Vec<&String> = floor.difference(&covered).collect();
    assert!(
        regressed.is_empty(),
        "{} floor class(es) are no longer reached by the corpus (coverage shrank): {regressed:?}",
        regressed.len()
    );
    // The floor is the covered set, pinned — guard against a stale floor too.
    let grew: Vec<&String> = covered.difference(&floor).collect();
    assert!(
        grew.is_empty(),
        "{} class(es) are now covered but missing from COVERED_FLOOR — add them: {grew:?}",
        grew.len()
    );
}

/// The uncovered set is EXACTLY [`DOCUMENTED_UNCOVERED`]: a newly-uncovered
/// class (or a newly-covered one) fails until the constant is updated, so
/// every gap stays visible and intentional.
#[test]
fn uncovered_set_matches_documented_constant() {
    let covered = covered_types();
    let schema = corpus::schema_definition_names();
    let uncovered: BTreeSet<String> = schema.difference(&covered).cloned().collect();
    let documented: BTreeSet<String> = DOCUMENTED_UNCOVERED
        .iter()
        .map(|s| (*s).to_string())
        .collect();

    let newly_uncovered: Vec<&String> = uncovered.difference(&documented).collect();
    let newly_covered: Vec<&String> = documented.difference(&uncovered).collect();
    assert!(
        newly_uncovered.is_empty() && newly_covered.is_empty(),
        "uncovered set drifted from DOCUMENTED_UNCOVERED:\n  newly UNCOVERED (add to constant or add corpus): {newly_uncovered:?}\n  now COVERED (remove from constant): {newly_covered:?}"
    );

    // Consistency: the two constants must partition all 134 schema classes.
    assert_eq!(
        COVERED_FLOOR.len() + DOCUMENTED_UNCOVERED.len(),
        schema.len(),
        "COVERED_FLOOR ({}) + DOCUMENTED_UNCOVERED ({}) must equal the {} schema classes",
        COVERED_FLOOR.len(),
        DOCUMENTED_UNCOVERED.len(),
        schema.len()
    );
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: ehrbase/openEHR_SDK canonical_json corpus @ 22b01e0c + in-repo EHRbase resources + ITS-JSON pinned commit 5acae056248e917a4b4c56f7e712f4fcfeb616a6
//   source_loc: n/a
//   confidence: high
//   note: transparent coverage — 57 classes reached by the real corpus (pinned floor), 77 documented-uncovered (partitioning all 134 schema defs); demographic + rare DV types (★) are filled synthetically in gap_fixtures.rs
// ─────────────────────────────────────────────
