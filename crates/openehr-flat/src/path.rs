//! RM-path parsing + navigation shared across the FLAT converters and the
//! composition validator.
//!
//! This is a thin, FLAT-local layer over the canonical single implementation in
//! [`openehr_rm::paths`] (the BASE `master11-paths` parser + `PATHABLE`
//! navigation over canonical-JSON RM trees). It adds only the two conveniences
//! the SDT pipeline needs and the RM primitive does not carry: taking the
//! *relative* path between a parent and child [`WebTemplateNode`](crate::webtemplate::WebTemplateNode)
//! `aqlPath`, and multi-root navigation over a slice of parsed segments.
//!
//! A `WebTemplateNode`'s `aqlPath` is the full RM path from the versioned-object
//! root (compacted intermediates kept), so the path between a parent web-template
//! node and one of its children is the sequence of RM attribute steps
//! (`/attr[predicate]`) that locates the child's RM value inside the parent's,
//! including the structural nodes (`HISTORY`, `ITEM_TREE`, a single `EVENT`, the
//! `ELEMENT` wrapper) the web-template compacted away.

use openehr_rm::paths::{PathSegment, RmPath, select_children};
use serde_json::Value;

/// Parse a relative RM path (`/attr[pred]/attr2/…`) into its segments.
///
/// The `aqlPath`s fed here are template-derived and always well-formed; a parse
/// error (an unterminated predicate, or a general-comparison predicate that
/// belongs to AQL rather than a `PATHABLE` path) resolves to no segments — the
/// caller then treats the path as locating nothing, never panicking.
pub(crate) fn parse(rel: &str) -> Vec<PathSegment> {
    rel.parse::<RmPath>()
        .map(|p| p.segments)
        .unwrap_or_default()
}

/// The segments from `parent_aql` to `child_aql` (the child path extends the
/// parent). When `child_aql` is not a suffix-extension of `parent_aql` the whole
/// child path is parsed (matching the prior FLAT behaviour).
pub(crate) fn relative(parent_aql: &str, child_aql: &str) -> Vec<PathSegment> {
    parse(child_aql.strip_prefix(parent_aql).unwrap_or(child_aql))
}

/// Follow `segs` from each of `roots`, returning the reached nodes in document
/// order. Each step is [`openehr_rm::paths::select_children`], so array
/// attributes branch (filtered by the segment predicate) and single-valued
/// attributes descend (predicate re-checked, per BASE `master11-paths`).
pub(crate) fn navigate<'a>(roots: &[&'a Value], segs: &[PathSegment]) -> Vec<&'a Value> {
    let mut current: Vec<&Value> = roots.to_vec();
    for seg in segs {
        current = current
            .iter()
            .flat_map(|n| select_children(n, seg))
            .collect();
    }
    current
}

/// Resolve the RM value(s) a full relative path reaches from `rm` (an empty
/// segment list resolves to `rm` itself).
pub(crate) fn resolve<'a>(rm: &'a Value, segs: &[PathSegment]) -> Vec<&'a Value> {
    navigate(&[rm], segs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_segments() {
        let segs = parse(
            "/content[openEHR-EHR-SECTION.ispek_dialog.v1,'Vitals']/items[openEHR-EHR-OBSERVATION.lab_test-hba1c.v1]/data[at0001]/value",
        );
        assert_eq!(segs.len(), 4);
        assert_eq!(segs[0].attribute, "content");
        assert_eq!(
            segs[0].predicate.archetype_node_id.as_deref(),
            Some("openEHR-EHR-SECTION.ispek_dialog.v1")
        );
        assert_eq!(segs[0].predicate.name_value.as_deref(), Some("Vitals"));
        assert_eq!(segs[2].attribute, "data");
        assert_eq!(
            segs[2].predicate.archetype_node_id.as_deref(),
            Some("at0001")
        );
        assert_eq!(segs[3].attribute, "value");
        assert!(segs[3].predicate.is_empty());
    }

    #[test]
    fn relative_strips_parent() {
        let rel = relative(
            "/content[openEHR-EHR-OBSERVATION.x.v1]",
            "/content[openEHR-EHR-OBSERVATION.x.v1]/data[at0001]/events[at0002]",
        );
        assert_eq!(rel.len(), 2);
        assert_eq!(rel[0].attribute, "data");
        assert_eq!(rel[1].attribute, "events");
    }

    #[test]
    fn resolves_through_arrays_and_objects() {
        let rm = serde_json::json!({
            "data": {
                "events": [
                    {"archetype_node_id": "at0002", "value": {"_type": "DV_TEXT", "value": "a"}},
                    {"archetype_node_id": "at0009", "value": {"_type": "DV_TEXT", "value": "b"}}
                ]
            }
        });
        let segs = parse("/data/events[at0002]/value");
        let found = resolve(&rm, &segs);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0]["value"], "a");
    }
}
