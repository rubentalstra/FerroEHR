//! RM-path parsing + navigation shared by the RM ⇄ simplified walkers and the
//! composition validator.
//!
//! This is a thin, FLAT-local layer over the canonical single implementation in
//! [`openehr_rm::v1_2::paths`] (the BASE `master11-paths` parser + `PATHABLE`
//! navigation over canonical-JSON RM trees). It adds only the two conveniences
//! the SDT pipeline needs and the RM primitive does not carry: taking the
//! *relative* path between a parent and child [`WebTemplateNode`](crate::flat::webtemplate::model::WebTemplateNode)
//! `aqlPath`, and multi-root navigation over a slice of parsed segments.
//!
//! A `WebTemplateNode`'s `aqlPath` is the full RM path from the versioned-object
//! root (compacted intermediates kept), so the path between a parent web-template
//! node and one of its children is the sequence of RM attribute steps
//! (`/attr[predicate]`) that locates the child's RM value inside the parent's,
//! including the structural nodes (`HISTORY`, `ITEM_TREE`, a single `EVENT`, the
//! `ELEMENT` wrapper) the web-template compacted away.
#![allow(
    dead_code,
    reason = "consumed by the RM ⇄ sim walkers landing in this same rewrite; drop this allow with their arrival"
)]
#![expect(
    clippy::disallowed_types,
    reason = "canonical JSON / Simplified Formats operate on the wire value by definition \
              (ITS-JSON; ITS-REST Simplified Formats) — serde_json::Value IS the subject matter \
              (#1694)"
)]

use openehr_rm::v1_2::paths::{PathSegment, RmPath, select_children};
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
/// order. Each step is [`openehr_rm::v1_2::paths::select_children`], so array
/// attributes branch (filtered by the segment predicate) and single-valued
/// attributes descend (predicate re-checked, per BASE `master11-paths`).
///
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

/// One template-path step with a conditional name fallback.
///
/// NOTE (template-name predicates): the paths fed here are
/// **template-derived** (`WebTemplateNode.aqlPath`), so a `[atNNNN,'name']`
/// name conjunct carries the *template's* term text. RM `LOCATABLE.name` is a
/// runtime attribute an instance may legitimately redefine when the archetype
/// does not constrain it (RM common, `LOCATABLE.name`), so a strict
/// name-conjunct match (BASE `master11-paths` semantics — correct for
/// instance-authored AQL/RM paths) can silently locate nothing and skip the
/// downstream check. The fallback (retry by `archetype_node_id` alone when the
/// strict match finds nothing) is only sound when the name is **redundant** —
/// i.e. the archetype id is unique among the template siblings being matched
/// (`allow_name_fallback`, computed by the caller from its sibling set). Where
/// a template distinguishes same-id siblings *by name* (the corona-corpus
/// shape), the fallback must stay off or it would claim the wrong siblings.
pub(crate) fn select_children_matched<'a>(
    container: &'a Value,
    seg: &PathSegment,
    allow_name_fallback: bool,
) -> Vec<&'a Value> {
    let strict = select_children(container, seg);
    if strict.is_empty() && allow_name_fallback && seg.predicate.name_value.is_some() {
        let mut id_only = seg.clone();
        id_only.predicate.name_value = None;
        return select_children(container, &id_only);
    }
    strict
}

/// Select the children matching an **unqualified** identity segment (one with an
/// `archetype_node_id` but no `name/value` conjunct) that shares its
/// `archetype_node_id` with one or more *name-qualified* sibling constraints,
/// excluding the instances those siblings claim.
///
/// This is the residual/catch-all arm of name-based sibling differentiation
/// (RM common `master03-archetyped_package.adoc` §"The `LOCATABLE` class": a
/// runtime `name` distinguishes sibling nodes that share an `archetype_node_id`;
/// AOM 1.4 `master04-constraint_model_package.adoc` §`node_id` — node ids
/// "guarantee sibling node unique identification", which templates realise for
/// repeated same-archetype fills via a fixed `name/value` `C_STRING` on all but
/// one sibling). The unqualified sibling carries no name constraint, so its
/// `LOCATABLE.name` is unconstrained (redefinable at runtime, master03 §"The
/// `LOCATABLE` class" L35); it therefore admits every instance of the shared
/// `archetype_node_id` **except** those whose `name/value` matches a
/// name-qualified sibling — those belong to that sibling, never here.
pub(crate) fn select_children_excluding_names<'a>(
    container: &'a Value,
    seg: &PathSegment,
    excluded_names: &[String],
) -> Vec<&'a Value> {
    select_children(container, seg)
        .into_iter()
        .filter(|node| {
            node.get("name")
                .and_then(|n| n.get("value"))
                .and_then(Value::as_str)
                .is_none_or(|name| !excluded_names.iter().any(|e| e == name))
        })
        .collect()
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
    fn matched_step_falls_back_to_id_only_for_renamed_instances() {
        // Template step names the node 'Template Name'; the instance renamed it
        // (LOCATABLE.name is runtime-redefinable when unconstrained) — with the
        // fallback allowed (id unambiguous among siblings) the step still
        // locates the node by its archetype_node_id.
        let rm = serde_json::json!({
            "content": [
                {"archetype_node_id": "openEHR-EHR-EVALUATION.a.v1",
                 "name": {"value": "Runtime Name"},
                 "data": {"archetype_node_id": "at0001", "_type": "ITEM_LIST"}}
            ]
        });
        let segs = parse("/content[openEHR-EHR-EVALUATION.a.v1,'Template Name']/data[at0001]");
        let step = select_children_matched(&rm, &segs[0], true);
        assert_eq!(step.len(), 1);
        assert_eq!(step[0]["name"]["value"], "Runtime Name");
        // Fallback disallowed (same-id siblings distinguished by name): strict
        // only, nothing matches.
        assert!(select_children_matched(&rm, &segs[0], false).is_empty());
    }

    #[test]
    fn matched_step_still_disambiguates_matching_siblings() {
        // Two same-id siblings with distinct (template-constrained) names: the
        // strict match succeeds and must NOT widen to both, fallback or not.
        let rm = serde_json::json!({
            "content": [
                {"archetype_node_id": "openEHR-EHR-OBSERVATION.x.v1",
                 "name": {"value": "First"}, "tag": 1},
                {"archetype_node_id": "openEHR-EHR-OBSERVATION.x.v1",
                 "name": {"value": "Second"}, "tag": 2}
            ]
        });
        let segs = parse("/content[openEHR-EHR-OBSERVATION.x.v1,'Second']");
        let found = select_children_matched(&rm, &segs[0], true);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0]["tag"], 2);
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
