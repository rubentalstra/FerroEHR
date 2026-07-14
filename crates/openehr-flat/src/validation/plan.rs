//! Pre-parsed archetype-conformance walk plan (P20 item 31).
//!
//! The archetype-conformance walk ([`super::Validator::walk`]) navigates the
//! instance guided by the constraint paths a [`WebTemplateNode`] carries
//! (`closed_attributes`, `existence`, `card_all`, `slots`, and the children's
//! `aqlPath`s). Those paths are **template-static**, but the pre-item-31 walk
//! re-parsed every one of them ([`crate::path::parse`], an `RmPath` string
//! parse allocating a `Vec<PathSegment>` + per-segment `String`s) and rebuilt
//! the child/slot `IndexMap` grouping **on every instance-node visit** — for a
//! ~1.5k-node IPS commit that is thousands of redundant parses per commit.
//!
//! This module parses each constraint path **once** (at [`crate::build_web_template`]
//! time, via [`prepare_walk`]) and caches the parsed form + the sibling groups
//! on the node as [`WebTemplateNode::walk`]. A hand-built node that never went
//! through the builder has no cached plan; the walk then builds a [`NodeWalk`]
//! on the fly once per visit (identical result, no cache), so every consumer of
//! the walk reads exactly one code path.
//!
//! No openEHR spec governs the WebTemplate model or this plan — our own
//! design/extension (the walk *semantics* it serves cite AOM 1.4 / RM common in
//! [`super`]).

use openehr_rm::paths::PathSegment;

use super::SiblingNameIndex;
use crate::path;
use crate::webtemplate::WebTemplateNode;

/// A pre-parsed archetype-conformance walk plan for one [`WebTemplateNode`].
///
/// Every `Vec` is index-aligned with the corresponding node constraint vector so
/// the walk can zip them; `None` marks a constraint path that is not an
/// extension of the node's own `aqlPath` (the pre-item-31 walk's early
/// `strip_prefix … else continue`).
#[derive(Debug, Clone)]
pub(crate) struct NodeWalk {
    /// Child sibling groups, in first-seen order (mirrors the `IndexMap` the
    /// pre-item-31 walk built by iterating `children` and grouping on `aqlPath`).
    pub(crate) child_groups: Vec<ChildGroup>,
    /// Name-based sibling routing index over the child groups.
    pub(crate) child_names: SiblingNameIndex,
    /// Parsed relative segments per `existence` constraint (index-aligned with
    /// [`WebTemplateNode::existence`]).
    pub(crate) existence: Vec<Option<Vec<PathSegment>>>,
    /// Parsed relative segments per `card_all` constraint (index-aligned with
    /// [`WebTemplateNode::card_all`]).
    pub(crate) cardinalities: Vec<Option<Vec<PathSegment>>>,
    /// Parsed relative segments per `closed_attributes` constraint (index-aligned
    /// with [`WebTemplateNode::closed_attributes`]).
    pub(crate) closed: Vec<Option<Vec<PathSegment>>>,
    /// Slot groups (grouped by slot path, first-seen order).
    pub(crate) slot_groups: Vec<SlotGroup>,
    /// Name-based sibling routing index over the slot groups.
    pub(crate) slot_names: SiblingNameIndex,
}

/// A sibling group of `WebTemplate` children sharing one `aqlPath` (a single
/// node, or a set of polymorphic type alternatives).
#[derive(Debug, Clone)]
pub(crate) struct ChildGroup {
    /// Indices into [`WebTemplateNode::children`] of the group's members.
    pub(crate) members: Vec<usize>,
    /// The group's `aqlPath` parsed relative to the parent node's `aqlPath`; an
    /// empty vec means the child path is not a strict extension of the parent's
    /// (or equals it) — the walk skips such a group (the pre-item-31
    /// `strip_prefix … else return` / `segments.is_empty()` early-outs).
    pub(crate) segments: Vec<PathSegment>,
}

/// A group of hoisted-wrapper slot constraints sharing one absolute path.
#[derive(Debug, Clone)]
pub(crate) struct SlotGroup {
    /// The allowed RM types (first-seen order — the `join(", ")` message order).
    pub(crate) allowed: Vec<String>,
    /// The absolute slot path (the violation path the walk pushes).
    pub(crate) path: String,
    /// The slot path parsed relative to the node's `aqlPath` (empty = not an
    /// extension; the walk skips it).
    pub(crate) segments: Vec<PathSegment>,
}

impl NodeWalk {
    /// Build the walk plan for `node` from its (final, post-compaction) constraint
    /// vectors and children. Parses each constraint path once.
    pub(crate) fn build(node: &WebTemplateNode) -> Self {
        let aql = node.aql_path.as_str();

        // Child groups: iterate children in order, grouping by `aqlPath`; the
        // first-seen order matches the `IndexMap` the pre-item-31 walk built.
        let mut child_groups: Vec<ChildGroup> = Vec::new();
        let mut child_idx: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for (i, child) in node.children.iter().enumerate() {
            let key = child.aql_path.as_str();
            let next = child_groups.len();
            let gi = *child_idx.entry(key).or_insert(next);
            if gi == next {
                let segments = key.strip_prefix(aql).map(path::parse).unwrap_or_default();
                child_groups.push(ChildGroup {
                    members: Vec::new(),
                    segments,
                });
            }
            child_groups[gi].members.push(i);
        }
        let child_names =
            sibling_index_from_segments(child_groups.iter().map(|g| g.segments.as_slice()));

        // Slot groups: grouped by absolute slot path, first-seen order.
        let mut slot_groups: Vec<SlotGroup> = Vec::new();
        let mut slot_idx: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for slot in &node.slots {
            let key = slot.path.as_str();
            let next = slot_groups.len();
            let gi = *slot_idx.entry(key).or_insert(next);
            if gi == next {
                let segments = key.strip_prefix(aql).map(path::parse).unwrap_or_default();
                slot_groups.push(SlotGroup {
                    allowed: Vec::new(),
                    path: slot.path.clone(),
                    segments,
                });
            }
            slot_groups[gi].allowed.push(slot.rm_type.clone());
        }
        let slot_names =
            sibling_index_from_segments(slot_groups.iter().map(|g| g.segments.as_slice()));

        let rel = |p: &str| p.strip_prefix(aql).map(path::parse);

        Self {
            child_groups,
            child_names,
            existence: node.existence.iter().map(|e| rel(&e.path)).collect(),
            cardinalities: node.card_all.iter().map(|c| rel(&c.path)).collect(),
            closed: node
                .closed_attributes
                .iter()
                .map(|c| rel(&c.path))
                .collect(),
            slot_groups,
            slot_names,
        }
    }
}

/// Build the name-based sibling routing index from a set of already-parsed
/// relative sibling paths (one per distinct child/slot `aqlPath`).
///
/// Identical to the pre-item-31 `sibling_name_index` (which parsed each path
/// string here): per identity `(attribute, archetype_node_id)`, count the
/// distinct sibling paths carrying it and collect the explicit `name/value`
/// constraints; keep only identities carried by more than one sibling. A path
/// with no predicate-bearing segment, or no `archetype_node_id`, is skipped
/// (the pre-item-31 `rfind … else continue`).
fn sibling_index_from_segments<'a>(
    groups: impl Iterator<Item = &'a [PathSegment]>,
) -> SiblingNameIndex {
    let mut acc: std::collections::HashMap<(String, String), (u32, Vec<String>)> =
        std::collections::HashMap::new();
    for segs in groups {
        let Some(id_seg) = segs.iter().rfind(|s| !s.predicate.is_empty()) else {
            continue;
        };
        let Some(id) = &id_seg.predicate.archetype_node_id else {
            continue;
        };
        let entry = acc
            .entry((id_seg.attribute.clone(), id.clone()))
            .or_default();
        entry.0 += 1;
        if let Some(name) = &id_seg.predicate.name_value
            && !entry.1.contains(name)
        {
            entry.1.push(name.clone());
        }
    }
    acc.into_iter()
        .filter(|(_, (count, _))| *count > 1)
        .map(|(k, (_, names))| (k, names))
        .collect()
}

/// Populate the archetype-conformance walk plan on `node` and, recursively, every
/// descendant — called once by [`crate::build_web_template`] on the finalized
/// tree so the validation walk never re-parses a template-static constraint path.
pub(crate) fn prepare_walk(node: &mut WebTemplateNode) {
    let plan = NodeWalk::build(node);
    node.walk = Some(Box::new(plan));
    for child in &mut node.children {
        prepare_walk(child);
    }
}
