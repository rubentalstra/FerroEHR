//! RM (canonical JSON) → simplified tree — the template-driven flattener.
//!
//! Walks the [`WebTemplate`] tree in parallel with the canonical-JSON
//! composition. Level removal is inherent to the walk: a web-template
//! child's RM value(s) are located by the relative RM path between the
//! parent's and child's `aqlPath`, so the container attributes and
//! collapsed wrappers of ITS-REST `simplified_formats/master04` §Level
//! Removal never appear as segments. Each populated leaf emits its datum
//! parts per the `master05-rm_mapping.adoc` tables ([`crate::map`]); the
//! composition context emits as `ctx/…` ([`crate::ctx`]).
//!
//! Output scope: `in-context` metadata whose FLAT surface is the `ctx/`
//! vocabulary (composition language/territory/composer, the EVENT_CONTEXT
//! fields, per-entry language/encoding/subject when they carry the
//! defaults) is emitted once as `ctx/…`, not duplicated as path keys —
//! matching the `master04 §Flat format` example. `category` has no `ctx/`
//! key (`master06` defines none), so it emits as a path key. A per-entry
//! `subject` that is not the PARTY_SELF default emits as a path key
//! (`master05 §OBSERVATION` `/subject` row).

use serde_json::Value;

use crate::ctx;
use crate::error::FlatError;
use crate::map;
use crate::rmpath;
use crate::sim::{SimDocument, SimNode};
use crate::webtemplate::{WebTemplate, WebTemplateNode};

/// Flatten a canonical-JSON composition into the simplified tree, driven by
/// `wt`. The result serializes to either wire variant via [`crate::sim`].
///
/// # Errors
/// [`FlatError::Conversion`] if `composition` is not a JSON object.
pub fn flatten_composition(
    composition: &Value,
    wt: &WebTemplate,
) -> Result<SimDocument, FlatError> {
    if !composition.is_object() {
        return Err(FlatError::Conversion(
            "composition must be a JSON object".to_owned(),
        ));
    }
    let mut doc = SimNode::default();
    ctx::emit(composition, doc.occurrence_mut("ctx", None));
    let root = doc.occurrence_mut(&wt.tree.id, None);
    walk(&wt.tree, composition, root);
    doc.prune_empty();
    Ok(doc)
}

/// Whether an RM value's `_type` matches a web-template node's (possibly
/// generic) rm type (base-name comparison, e.g. `DV_INTERVAL<DV_QUANTITY>`).
fn type_matches(rm: &Value, rm_type: &str) -> bool {
    let want = rm_type.split('<').next().unwrap_or(rm_type);
    match rm.get("_type").and_then(Value::as_str) {
        Some(actual) => actual == want,
        None => true,
    }
}

/// A composition-level in-context child whose FLAT surface is the `ctx/`
/// vocabulary (`master06-context_information.adoc`) rather than a path key.
/// `category` is deliberately absent — `master06` defines no `ctx/category`,
/// so it stays a path key (`master05 §COMPOSITION` `/category` row).
fn covered_by_ctx(node: &WebTemplateNode, child: &WebTemplateNode) -> bool {
    if node.rm_type == "COMPOSITION" {
        return matches!(child.id.as_str(), "language" | "territory" | "composer");
    }
    // Standard EVENT_CONTEXT fields (start_time, setting, participations, …)
    // surface via ctx/; only archetyped other_context content is tree data.
    if node.rm_type == "EVENT_CONTEXT" {
        return !child.aql_path.contains("other_context");
    }
    false
}

/// A per-entry in-context leaf that emits only when it differs from the
/// value the `ctx/` defaults would rebuild (round-trip stability; the input
/// side accepts the path form regardless, per the `master05` per-entry
/// table rows).
fn is_default_entry_context(child: &WebTemplateNode, rm: &Value) -> bool {
    if child.in_context != Some(true) {
        return false;
    }
    match child.id.as_str() {
        // PARTY_SELF with no external_ref is the ENTRY.subject default
        // (`master05 §OBSERVATION` — "set to PARTY_SELF if not explicitly
        // set").
        "subject" => {
            rm.get("_type").and_then(Value::as_str) == Some("PARTY_SELF")
                && rm.get("external_ref").is_none()
        }
        // ENTRY.language / encoding default from ctx (`master06 §Language
        // and Territory`); their non-default values still round-trip via
        // the ctx emission of the composition language, so per-entry copies
        // are only emitted when they diverge — handled by ctx::emit
        // comparing entries. Here the walk keeps them out of the tree data.
        "language" | "encoding" => true,
        // EVENT time / history origin surface via ctx/history_origin +
        // ctx/time only when they equal the derived defaults; a distinct
        // per-event time is real data and must stay a path key.
        _ => false,
    }
}

fn walk(node: &WebTemplateNode, rm: &Value, out: &mut SimNode) {
    if node.has_input() {
        let list_open = node.inputs.iter().find_map(|i| i.list_open);
        map::emit_leaf(rm, &node.rm_type, list_open, out);
        // Value-level `_` RM attributes of the leaf value itself
        // (`master05` per-type tables: `_normal_range`, `_mapping`, …).
        map::emit_rm_attrs(rm, base_type(&node.rm_type), out);
        return;
    }
    // Container-level `_` RM attribute families (`master05` per-class
    // tables: `_uid`, `_link:i`, `_feeder_audit`, `_work_flow_id`, …).
    map::emit_rm_attrs(rm, base_type(&node.rm_type), out);

    for child in &node.children {
        if covered_by_ctx(node, child) {
            continue;
        }
        let rel = rmpath::relative(&node.aql_path, &child.aql_path);
        let matches = rmpath::resolve(rm, &rel);
        // Polymorphic choice alternatives share one aqlPath; emit only under
        // the alternative whose rm type matches this value's `_type`.
        let matches: Vec<&Value> = if child.alt_json_id.is_some() {
            matches
                .into_iter()
                .filter(|m| type_matches(m, &child.rm_type))
                .collect()
        } else {
            matches
        };
        if matches.is_empty() {
            continue;
        }
        let repeating = child.max == -1 || child.max > 1;
        if repeating {
            out.children.entry(child.id.clone()).or_default().indexed = true;
        }
        for (i, m) in matches.iter().enumerate() {
            if is_default_entry_context(child, m) {
                continue;
            }
            let slot = out.place_mut(&child.id, u32::try_from(i).unwrap_or(u32::MAX));
            walk(child, m, slot);
            // The leaf's wrapping ELEMENT carries its own `_` attribute
            // family (`master05 §ELEMENT`: `_uid`, `_null_flavour`,
            // `_null_reason`, `_link:i`, `_feeder_audit`); the leaf walk
            // above saw only the DV value, so surface the wrapper's here.
            if child.has_input()
                && let Some(element) = element_of(rm, &rel, i)
            {
                map::emit_rm_attrs(element, "ELEMENT", slot);
            }
        }
    }
}

/// The ELEMENT wrapper that owns the `i`-th leaf value reached by `rel`
/// (`rel` ends in the `value` attribute step for ELEMENT-wrapped leaves;
/// non-wrapped leaves — e.g. `EVENT.time` — have none).
fn element_of<'a>(
    rm: &'a Value,
    rel: &[openehr_rm::paths::PathSegment],
    i: usize,
) -> Option<&'a Value> {
    let (last, parents) = rel.split_last()?;
    if last.attribute != "value" {
        return None;
    }
    let wrappers = rmpath::resolve(rm, parents);
    let element = *wrappers.get(i)?;
    (element.get("_type").and_then(Value::as_str) == Some("ELEMENT")).then_some(element)
}

/// The base (generic-stripped) RM type name.
fn base_type(rm_type: &str) -> &str {
    rm_type.split('<').next().unwrap_or(rm_type)
}
