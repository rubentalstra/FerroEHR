//! RM (canonical JSON) → FLAT (simSDT) conversion.
//!
//! Walks the [`WebTemplate`] tree in parallel with the canonical-JSON
//! composition: each web-template child's RM value(s) are located by the
//! relative AQL path from its parent (`path::resolve`), the flat path segment is
//! the child's json-`id` (`:i`-indexed when the node is repeating, Better's
//! `isRepeating` rule: `max == -1 || max > 1`), and each populated leaf emits
//! its `path|suffix` datum parts via [`mappers::leaf_to_flat`]. The composition
//! context (language, territory, composer, start-time, setting, participations)
//! is emitted as `ctx/…` keys (see [`context`](super::context)).

use serde_json::Value;

use super::context;
use super::mappers::{self, FlatMap};
use super::rmattr;
use crate::FlatError;
use crate::path;
use crate::webtemplate::{WebTemplate, WebTemplateNode};

/// Convert a canonical-JSON composition to a FLAT map, driven by `wt`.
///
/// # Errors
/// [`FlatError::Conversion`] if `composition` is not a JSON object.
pub fn to_flat(composition: &Value, wt: &WebTemplate) -> Result<FlatMap, FlatError> {
    if !composition.is_object() {
        return Err(FlatError::Conversion(
            "composition must be a JSON object".to_owned(),
        ));
    }
    let mut out = FlatMap::new();
    context::emit_ctx(composition, &mut out);
    walk(&wt.tree, composition, &wt.tree.id, &mut out);
    Ok(out)
}

/// Whether an RM value's `_type` matches a web-template node's (possibly
/// generic) rm type (base name comparison, e.g. `DV_INTERVAL<DV_QUANTITY>`).
fn type_matches(rm: &Value, rm_type: &str) -> bool {
    let want = rm_type.split('<').next().unwrap_or(rm_type);
    match rm.get("_type").and_then(Value::as_str) {
        Some(actual) => actual == want,
        None => true,
    }
}

fn walk(node: &WebTemplateNode, rm: &Value, prefix: &str, out: &mut FlatMap) {
    if node.has_input() {
        let list_open = node.inputs.iter().find_map(|i| i.list_open);
        mappers::leaf_to_flat(rm, &node.rm_type, prefix, list_open, out);
        // The `_`-prefixed optional RM attributes on RM→FLAT (master05
        // per-type tables; master02 §"RM Attributes prefix").
        rmattr::emit_rm_attrs(rm, prefix, out);
        return;
    }
    // Container nodes (COMPOSITION / ENTRY types / CLUSTER …) carry their own
    // `_`-attribute families (master05 per-class tables).
    rmattr::emit_rm_attrs(rm, prefix, out);
    for child in &node.children {
        // Inside EVENT_CONTEXT only the archetyped `other_context` items are tree
        // leaves; the standard context fields (start_time / setting /
        // participations / …) are surfaced via ctx/ (see `context`).
        if node.rm_type == "EVENT_CONTEXT" && !child.aql_path.contains("other_context") {
            continue;
        }
        let rel = path::relative(&node.aql_path, &child.aql_path);
        let matches = path::resolve(rm, &rel);
        let repeating = child.max == -1 || child.max > 1;
        // Polymorphic choice alternatives share one aqlPath; emit only under the
        // alternative whose rm type matches this value's `_type`.
        let matches: Vec<&Value> = if child.alt_json_id.is_some() {
            matches
                .into_iter()
                .filter(|m| type_matches(m, &child.rm_type))
                .collect()
        } else {
            matches
        };
        for (i, m) in matches.iter().enumerate() {
            let seg = if repeating {
                format!("{}:{}", child.id, i)
            } else {
                child.id.clone()
            };
            let child_prefix = format!("{prefix}/{seg}");
            walk(child, m, &child_prefix, out);
        }
    }
}
