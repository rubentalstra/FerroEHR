#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
//! The two published ITS-XML wire lineages over ONE generated codec.
//!
//! `docs/specs/openehr/ITS-XML/README.adoc` §"Releases and IM Versions" states
//! the whole delta: the `Release-2.0.0` restructure is "a major version" whose
//! change to the documents is that "the internal namespace used in the schemas
//! is also changed to `http://schemas.openehr.org/v2`", with the older STABLE
//! bundle still published at `Release-1.0.2`. This gate pins exactly that: the
//! default entry point emits v1, the namespace-parameterized entry point emits
//! whichever lineage it is handed, the two documents are byte-identical apart
//! from the root `xmlns`, and the reader accepts both.

use openehr_its::xml::{Namespace, from_canonical_xml, to_canonical_xml, to_canonical_xml_ns};
use openehr_rm::prelude::Composition;

const V1_NS: &str = "xmlns=\"http://schemas.openehr.org/v1\"";
const V2_NS: &str = "xmlns=\"http://schemas.openehr.org/v2\"";

fn fixture() -> Composition {
    let json =
        include_str!("vendor/openehr_sdk/composition/canonical_json/minimal_evaluation.json");
    openehr_its::json::from_canonical_json(json).expect("deserialize composition JSON")
}

/// The default entry point is unchanged by the namespace parameterization:
/// v1 stays what `to_canonical_xml` emits.
#[test]
fn default_entry_point_still_emits_the_v1_lineage() {
    let xml = to_canonical_xml(&fixture(), "composition").expect("serialize default");
    assert!(xml.contains(V1_NS), "default lineage is v1: {xml:.200}");
    assert!(!xml.contains(V2_NS), "default lineage is not v2");
    assert_eq!(
        xml,
        to_canonical_xml_ns(&fixture(), "composition", Namespace::V1).expect("serialize v1"),
        "to_canonical_xml is to_canonical_xml_ns(.., V1)"
    );
}

/// The v2 selection changes the root namespace and NOTHING else — the
/// README's own statement of the 2.0.0 delta.
#[test]
fn v2_selection_changes_only_the_root_namespace() {
    let v1 = to_canonical_xml_ns(&fixture(), "composition", Namespace::V1).expect("serialize v1");
    let v2 = to_canonical_xml_ns(&fixture(), "composition", Namespace::V2).expect("serialize v2");

    assert!(v2.starts_with("<composition"), "root element: {v2:.80}");
    assert!(v2.contains(V2_NS), "v2 namespace declared");
    assert!(!v2.contains(V1_NS), "v1 namespace not declared");
    assert!(
        v2.contains("xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\""),
        "the xsi namespace is declared in both lineages"
    );
    assert_eq!(
        v1.replace(V1_NS, V2_NS),
        v2,
        "the two lineages differ by the root xmlns alone"
    );
}

/// The reader is namespace-agnostic: a v2 document round-trips through the
/// same generated codec and re-serializes to the same v2 bytes.
#[test]
fn v2_document_round_trips() {
    let v2 = to_canonical_xml_ns(&fixture(), "composition", Namespace::V2).expect("serialize v2");
    let back: Composition = from_canonical_xml(&v2).expect("parse the v2 document");
    assert_eq!(
        to_canonical_xml_ns(&back, "composition", Namespace::V2).expect("re-serialize v2"),
        v2,
        "v2 → parse → v2 is lossless"
    );
    // …and the SAME parsed value re-serializes into the v1 lineage, which is
    // what makes namespace selection a pure presentation choice.
    assert!(
        to_canonical_xml_ns(&back, "composition", Namespace::V1)
            .expect("re-serialize v1")
            .contains(V1_NS),
        "a v2-parsed value can be emitted as v1"
    );
}
