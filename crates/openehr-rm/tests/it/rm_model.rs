// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-FileCopyrightText: openEHR Foundation
// SPDX-License-Identifier: BUSL-1.1 AND Apache-2.0

#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop,
    reason = "integration-test assertions, diagnostics and fixture plumbing outside #[test] fns, which the clippy.toml allow-*-in-tests scoping does not reach"
)]
//! Tests for the generated static RM attribute/type model (`openehr_rm::v1_2::model`,
//! the AQL planner's oracle). These assert behaviour the
//! planner relies on: inheritance-aware attribute resolution, descendant/ancestor
//! sets, container multiplicity, and the node-codec structure classification.

use openehr_rm::v1_2::model::{
    Container, ancestors, attribute, attributes, class, descendants, is_a, is_structure_root,
};

#[test]
fn inherited_attribute_resolves_through_the_hierarchy() {
    // LOCATABLE.name resolves for OBSERVATION (ancestor-flattened attributes).
    let name = attribute("OBSERVATION", "name").expect("OBSERVATION.name");
    assert_eq!(name.declared_type, "DV_TEXT");
    assert_eq!(name.container, Container::None);
    assert!(name.is_mandatory);

    // OBSERVATION.data : HISTORY (single, mandatory).
    let data = attribute("OBSERVATION", "data").expect("OBSERVATION.data");
    assert_eq!(data.declared_type, "HISTORY");
    assert_eq!(data.container, Container::None);

    // The flattened attribute set includes both own and inherited attributes.
    let names: Vec<&str> = attributes("OBSERVATION").map(|a| a.name).collect();
    assert!(names.contains(&"name")); // inherited from LOCATABLE
    assert!(names.contains(&"data")); // own
}

#[test]
fn event_context_is_pathable_not_locatable() {
    // EVENT_CONTEXT inherits PATHABLE, not LOCATABLE, so `name` does NOT resolve.
    assert!(class("EVENT_CONTEXT").is_some());
    assert!(
        attribute("EVENT_CONTEXT", "name").is_none(),
        "EVENT_CONTEXT must not inherit LOCATABLE.name"
    );
    assert!(is_a("EVENT_CONTEXT", "PATHABLE"));
    assert!(!is_a("EVENT_CONTEXT", "LOCATABLE"));
}

#[test]
fn entry_descendants_are_the_concrete_entry_subtypes() {
    let d = descendants("ENTRY");
    for c in [
        "OBSERVATION",
        "EVALUATION",
        "INSTRUCTION",
        "ACTION",
        "ADMIN_ENTRY",
    ] {
        assert!(d.contains(&c), "descendants(ENTRY) missing {c}: {d:?}");
    }
    // ENTRY and CARE_ENTRY are abstract → excluded from the concrete descendant set.
    assert!(!d.contains(&"ENTRY"));
    assert!(!d.contains(&"CARE_ENTRY"));
}

#[test]
fn is_a_walks_the_ancestor_chain() {
    assert!(is_a("DV_CODED_TEXT", "DATA_VALUE"));
    assert!(is_a("DV_CODED_TEXT", "DV_TEXT"));
    assert!(is_a("DV_CODED_TEXT", "DV_CODED_TEXT")); // reflexive
    assert!(!is_a("DV_TEXT", "DV_CODED_TEXT")); // not the other way

    // ancestors() exposes the transitive chain.
    let anc = ancestors("OBSERVATION");
    for a in [
        "CARE_ENTRY",
        "ENTRY",
        "CONTENT_ITEM",
        "LOCATABLE",
        "PATHABLE",
    ] {
        assert!(
            anc.contains(&a),
            "ancestors(OBSERVATION) missing {a}: {anc:?}"
        );
    }
}

#[test]
fn container_multiplicity_is_recorded() {
    let content = attribute("COMPOSITION", "content").expect("COMPOSITION.content");
    assert_eq!(content.container, Container::List);
    assert_eq!(content.declared_type, "CONTENT_ITEM");

    // A generic list attribute reduces to its item root, tagged List.
    let events = attribute("HISTORY", "events").expect("HISTORY.events");
    assert_eq!(events.container, Container::List);
    assert_eq!(events.declared_type, "EVENT");
}

#[test]
fn generic_parameters_resolve_to_their_bound() {
    // DV_INTERVAL<T: DV_ORDERED>: `lower`/`upper` are the bare param T → DV_ORDERED.
    let lower = attribute("DV_INTERVAL", "lower").expect("DV_INTERVAL.lower");
    assert_eq!(lower.declared_type, "DV_ORDERED");
    let upper = attribute("DV_INTERVAL", "upper").expect("DV_INTERVAL.upper");
    assert_eq!(upper.declared_type, "DV_ORDERED");
}

#[test]
fn is_structure_root_matches_the_node_codec() {
    // Mirrors ferroehr::storage::codec::STRUCTURE_TYPES.
    for t in [
        "COMPOSITION",
        "EHR_STATUS",
        "FOLDER",
        "EVENT_CONTEXT",
        "SECTION",
        "OBSERVATION",
        "EVALUATION",
        "INSTRUCTION",
        "ACTION",
        "ACTIVITY",
        "HISTORY",
        "POINT_EVENT",
        "INTERVAL_EVENT",
        "ITEM_TREE",
        "CLUSTER",
        "ELEMENT",
        "FEEDER_AUDIT",
    ] {
        assert!(is_structure_root(t), "{t} should be a structure root");
    }
    // Data values, references, demographics, and EHR itself are NOT node roots.
    for t in [
        "DV_TEXT",
        "DV_CODED_TEXT",
        "CODE_PHRASE",
        "PARTY_IDENTIFIED",
        "EHR",
        "DV_QUANTITY",
    ] {
        assert!(!is_structure_root(t), "{t} should NOT be a structure root");
    }
}
