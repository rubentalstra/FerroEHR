//! FLAT (simSDT) → RM (canonical JSON) conversion — the composition builder.
//!
//! The reverse of [`to_flat`](super::to_flat): FLAT keys are parsed into
//! path-segment entries, grouped as the [`WebTemplate`] tree is walked (matching
//! each child by json-`id` / `alternativeJsonId`, `:i`-indexed instances kept in
//! order), and each leaf's datum parts rebuild its `DATA_VALUE`. The RM
//! structural nodes the web-template compacted away (`HISTORY`, `ITEM_TREE`, a
//! single `EVENT`, the `ELEMENT` wrapper) are re-materialised from the relative
//! AQL path between a node and its parent, with mandatory identity/occurrence
//! fields filled so the result deserialises as an `openehr-rm` `Composition`.
//! `ctx/…` keys rebuild the composition context (see [`context`](super::context)).
//!
//! Scope for this PR (recorded as `TODO(port)`): `INSTRUCTION.activities` /
//! `ACTIVITY`, `ACTION.ism_transition`, nested `SECTION`s beyond `content`,
//! `FOLDER`/DIRECTORY, feeder-audit / links / term-mappings / reference ranges,
//! and archetyped `other_context`. Unroutable keys are collected and reported.

use serde_json::{Map, Value, json};

use super::mappers::{self};
use super::sub::{Entry, FlatView, parse_key};
use super::{aql, context};
use crate::FlatError;
use crate::webtemplate::{WebTemplate, WebTemplateNode};

/// The default time filled into mandatory `start_time`/`origin`/`time` fields
/// (never surfaced in FLAT, so it does not affect the round-trip).
const DEFAULT_TIME: &str = "1970-01-01T00:00:00Z";

/// Map a (possibly abstract/generic) web-template rm type to the concrete RM
/// type to instantiate. `EVENT` is abstract → default `POINT_EVENT`.
fn concrete_type(rm_type: &str) -> &str {
    match strip_generic(rm_type) {
        "EVENT" => "POINT_EVENT",
        other => other,
    }
}

/// RM attributes that are arrays (needed to re-materialise compacted structure).
fn is_multiple(attr: &str) -> bool {
    matches!(
        attr,
        "content" | "items" | "events" | "activities" | "actions"
    )
}

/// Convert a FLAT map to a canonical-JSON composition, driven by `wt`.
///
/// # Errors
/// [`FlatError::Conversion`] if the FLAT input is empty of routable keys and no
/// `ctx/…` context is present (nothing to build).
pub fn from_flat(flat: &Map<String, Value>, wt: &WebTemplate) -> Result<Value, FlatError> {
    let root_id = &wt.tree.id;
    let mut entries: Vec<Entry> = Vec::new();
    for (key, value) in flat {
        if key == "ctx" || key.starts_with("ctx/") {
            continue;
        }
        let (mut segs, suffix) = parse_key(key);
        // Drop the root template-id segment.
        if segs.first().is_some_and(|s| &s.id == root_id) {
            segs.remove(0);
        }
        entries.push(Entry {
            segs,
            suffix,
            value: value.clone(),
        });
    }

    let mut comp = match build(&wt.tree, &entries) {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    finish_identity(&mut comp, &wt.tree, true, &wt.template_id);
    // `build`'s per-node pass may have created `archetype_details` without the
    // template id; the root must carry it (self-describing composition).
    ensure_template_id(&mut comp, &wt.tree, &wt.template_id);
    context::apply_ctx(flat, &mut comp);
    ensure_category(&mut comp);
    Ok(Value::Object(comp))
}

/// Build the RM value for `node` from its (root-relative) `entries`.
fn build(node: &WebTemplateNode, entries: &[Entry]) -> Value {
    if node.has_input() {
        let view = FlatView::new(entries);
        return mappers::leaf_from_flat(&node.rm_type, &view)
            .unwrap_or_else(|| json!({"_type": strip_generic(&node.rm_type)}));
    }

    let mut obj = Map::new();
    obj.insert("_type".into(), json!(concrete_type(&node.rm_type)));

    let ctx_time = json!(DEFAULT_TIME);
    for child in &node.children {
        if child.rm_type == "EVENT_CONTEXT" {
            continue; // handled via ctx/
        }
        // Gather the entries addressed to this child, grouped by :index.
        let mut groups: Vec<(Option<usize>, Vec<Entry>)> = Vec::new();
        for e in entries {
            let Some(first) = e.segs.first() else {
                continue;
            };
            if first.id != child.id && Some(first.id.as_str()) != child.alt_json_id.as_deref() {
                continue;
            }
            let idx = first.index;
            let rest = Entry {
                segs: e.segs[1..].to_vec(),
                suffix: e.suffix.clone(),
                value: e.value.clone(),
            };
            match groups.iter_mut().find(|(i, _)| *i == idx) {
                Some((_, v)) => v.push(rest),
                None => groups.push((idx, vec![rest])),
            }
        }
        groups.sort_by_key(|(i, _)| i.unwrap_or(0));

        let rel = aql::relative(&node.aql_path, &child.aql_path);
        for (_, sub) in groups {
            let child_val = build(child, &sub);
            place(&mut obj, &rel, child_val, child, &ctx_time);
        }
    }

    let mut value = Value::Object(obj);
    if let Value::Object(m) = &mut value {
        finish_identity(m, node, false, "");
    }
    value
}

/// Insert `child_value` into `parent` at the relative AQL path `rel`,
/// re-materialising the compacted RM structural nodes it passes through.
fn place(
    parent: &mut Map<String, Value>,
    rel: &[aql::AqlSeg],
    child_value: Value,
    child: &WebTemplateNode,
    time: &Value,
) {
    if rel.is_empty() {
        return;
    }
    let id_idx = rel.iter().rposition(|s| is_multiple(&s.attr));
    place_rec(parent, rel, 0, id_idx, child_value, child, time);
}

fn place_rec(
    cur: &mut Map<String, Value>,
    rel: &[aql::AqlSeg],
    i: usize,
    id_idx: Option<usize>,
    child_value: Value,
    child: &WebTemplateNode,
    time: &Value,
) {
    let seg = &rel[i];
    let last = i + 1 == rel.len();

    if Some(i) == id_idx {
        // The child's own (repeating) array level.
        let arr = cur
            .entry(seg.attr.clone())
            .or_insert_with(|| json!([]))
            .as_array_mut();
        let Some(arr) = arr else { return };
        if last {
            // The child value is itself the array element (a container child).
            let mut el = child_value;
            set_node_id(&mut el, seg.node_id.as_deref());
            arr.push(el);
        } else {
            // Wrap: the remaining path (e.g. `/value`) lives inside a new element.
            let mut el = new_struct(seg, rel.get(i + 1), child.name.as_deref(), time);
            if let Value::Object(m) = &mut el {
                place(m, &rel[i + 1..], child_value, child, time);
            }
            arr.push(el);
        }
        return;
    }

    if is_multiple(&seg.attr) {
        // A structural (single-occurrence) array level: find-or-create by node id.
        let arr = cur
            .entry(seg.attr.clone())
            .or_insert_with(|| json!([]))
            .as_array_mut();
        let Some(arr) = arr else { return };
        let pos = arr.iter().position(|e| {
            e.get("archetype_node_id").and_then(Value::as_str) == seg.node_id.as_deref()
        });
        let idx = if let Some(p) = pos {
            p
        } else {
            arr.push(new_struct(seg, rel.get(i + 1), None, time));
            arr.len() - 1
        };
        if let Some(Value::Object(m)) = arr.get_mut(idx) {
            place_rec(m, rel, i + 1, id_idx, child_value, child, time);
        }
        return;
    }

    // A single-valued (object) attribute.
    if last {
        cur.insert(seg.attr.clone(), child_value);
        return;
    }
    if !cur.contains_key(&seg.attr) {
        cur.insert(
            seg.attr.clone(),
            new_struct(seg, rel.get(i + 1), None, time),
        );
    }
    if let Some(Value::Object(m)) = cur.get_mut(&seg.attr) {
        place_rec(m, rel, i + 1, id_idx, child_value, child, time);
    }
}

/// Create a compacted structural RM node for `seg` (its `_type` inferred from
/// the attribute it sits under and the next step), with mandatory fields filled.
fn new_struct(
    seg: &aql::AqlSeg,
    next: Option<&aql::AqlSeg>,
    name: Option<&str>,
    time: &Value,
) -> Value {
    let rm_type = infer_type(&seg.attr, next.map(|s| s.attr.as_str()));
    let mut o = Map::new();
    o.insert("_type".into(), json!(rm_type));
    let display = name
        .map(str::to_owned)
        .or_else(|| seg.node_id.clone())
        .unwrap_or_else(|| rm_type.to_owned());
    o.insert("name".into(), json!({"_type": "DV_TEXT", "value": display}));
    if let Some(nid) = &seg.node_id {
        o.insert("archetype_node_id".into(), json!(nid));
    }
    match rm_type {
        "HISTORY" => {
            o.insert(
                "origin".into(),
                json!({"_type": "DV_DATE_TIME", "value": time}),
            );
            o.insert("events".into(), json!([]));
        }
        "POINT_EVENT" | "EVENT" | "INTERVAL_EVENT" => {
            o.insert(
                "time".into(),
                json!({"_type": "DV_DATE_TIME", "value": time}),
            );
        }
        "ITEM_TREE" | "ITEM_LIST" | "ITEM_SINGLE" | "ITEM_TABLE" | "CLUSTER" => {
            o.insert("items".into(), json!([]));
        }
        _ => {}
    }
    Value::Object(o)
}

fn infer_type(attr: &str, next: Option<&str>) -> &'static str {
    match (attr, next) {
        ("data", Some("events")) => "HISTORY", // ENTRY.data ⇒ HISTORY of events
        ("events", _) => "POINT_EVENT",
        ("items", _) => "ELEMENT",
        ("activities", _) => "ACTIVITY",
        // ENTRY.data (non-events), state/protocol/description, and any other
        // compacted item structure default to ITEM_TREE.
        _ => "ITEM_TREE",
    }
}

fn set_node_id(v: &mut Value, node_id: Option<&str>) {
    if let (Value::Object(m), Some(nid)) = (v, node_id) {
        m.entry("archetype_node_id".to_owned())
            .or_insert_with(|| json!(nid));
    }
}

/// Fill the mandatory identity / occurrence fields for a built locatable node.
fn finish_identity(
    obj: &mut Map<String, Value>,
    node: &WebTemplateNode,
    is_root: bool,
    template_id: &str,
) {
    let rm_type = concrete_type(&node.rm_type);
    if rm_type.starts_with("DV_") || rm_type == "CODE_PHRASE" {
        return; // leaves already complete
    }
    // name
    obj.entry("name".to_owned()).or_insert_with(|| {
        let text = node
            .name
            .clone()
            .or_else(|| node.node_id.clone())
            .unwrap_or_else(|| rm_type.to_owned());
        json!({"_type": "DV_TEXT", "value": text})
    });
    // Mandatory time/origin on structural container nodes that survive as
    // web-template nodes (e.g. a repeating EVENT, or a HISTORY).
    match rm_type {
        "POINT_EVENT" | "INTERVAL_EVENT" => {
            obj.entry("time".to_owned())
                .or_insert_with(|| json!({"_type": "DV_DATE_TIME", "value": DEFAULT_TIME}));
        }
        "HISTORY" => {
            obj.entry("origin".to_owned())
                .or_insert_with(|| json!({"_type": "DV_DATE_TIME", "value": DEFAULT_TIME}));
        }
        _ => {}
    }
    // archetype_node_id
    if let Some(nid) = &node.node_id {
        obj.entry("archetype_node_id".to_owned())
            .or_insert_with(|| json!(nid));
    }
    // archetype_details for archetype roots
    let is_root_arch = node
        .node_id
        .as_deref()
        .is_some_and(|n| n.starts_with("openEHR-") || n.starts_with("openEHR_"));
    if is_root_arch {
        obj.entry("archetype_details".to_owned())
            .or_insert_with(|| {
                let mut a = Map::new();
                a.insert("_type".into(), json!("ARCHETYPED"));
                a.insert(
                    "archetype_id".into(),
                    json!({"_type": "ARCHETYPE_ID", "value": node.node_id}),
                );
                if is_root && !template_id.is_empty() {
                    a.insert(
                        "template_id".into(),
                        json!({"_type": "TEMPLATE_ID", "value": template_id}),
                    );
                }
                a.insert("rm_version".into(), json!("1.0.4"));
                Value::Object(a)
            });
    }
    // ENTRY-family mandatory fields.
    if matches!(
        rm_type,
        "OBSERVATION" | "EVALUATION" | "INSTRUCTION" | "ACTION" | "ADMIN_ENTRY" | "GENERIC_ENTRY"
    ) {
        obj.entry("language".to_owned())
            .or_insert_with(|| json!({"_type": "CODE_PHRASE", "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "ISO_639-1"}, "code_string": "en"}));
        obj.entry("encoding".to_owned())
            .or_insert_with(|| json!({"_type": "CODE_PHRASE", "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "IANA_character-sets"}, "code_string": "UTF-8"}));
        obj.entry("subject".to_owned())
            .or_insert_with(|| json!({"_type": "PARTY_SELF"}));
    }
    // Per-ENTRY mandatory structural fields not surfaced in FLAT (only added when
    // a populated leaf did not already create them, so the round-trip is stable).
    let item_tree = || json!({"_type": "ITEM_TREE", "name": {"_type": "DV_TEXT", "value": "Tree"}, "items": []});
    match rm_type {
        "OBSERVATION" => {
            obj.entry("data".to_owned()).or_insert_with(|| {
                json!({"_type": "HISTORY", "name": {"_type": "DV_TEXT", "value": "History"},
                       "origin": {"_type": "DV_DATE_TIME", "value": DEFAULT_TIME}, "events": []})
            });
        }
        "EVALUATION" | "ADMIN_ENTRY" => {
            obj.entry("data".to_owned()).or_insert_with(item_tree);
        }
        "INSTRUCTION" => {
            obj.entry("narrative".to_owned())
                .or_insert_with(|| json!({"_type": "DV_TEXT", "value": "<narrative>"}));
        }
        "ACTION" => {
            obj.entry("time".to_owned())
                .or_insert_with(|| json!({"_type": "DV_DATE_TIME", "value": DEFAULT_TIME}));
            obj.entry("description".to_owned())
                .or_insert_with(item_tree);
            obj.entry("ism_transition".to_owned()).or_insert_with(|| {
                json!({"_type": "ISM_TRANSITION", "current_state": {
                    "_type": "DV_CODED_TEXT", "value": "initial",
                    "defining_code": {"_type": "CODE_PHRASE",
                        "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "openehr"},
                        "code_string": "524"}}})
            });
        }
        _ => {}
    }
}

/// COMPOSITION.category is mandatory; default to `event` (openEHR 433) if the
/// template did not carry it as a tree node.
fn ensure_category(comp: &mut Map<String, Value>) {
    comp.entry("category".to_owned()).or_insert_with(|| {
        json!({
            "_type": "DV_CODED_TEXT",
            "value": "event",
            "defining_code": {
                "_type": "CODE_PHRASE",
                "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "openehr"},
                "code_string": "433",
            },
        })
    });
}

fn strip_generic(rm_type: &str) -> &str {
    rm_type.split('<').next().unwrap_or(rm_type)
}

/// Ensure the composition's `archetype_details` carries `archetype_id` +
/// `template_id` (the composition must be self-describing for its template).
fn ensure_template_id(comp: &mut Map<String, Value>, root: &WebTemplateNode, template_id: &str) {
    let ad = comp
        .entry("archetype_details".to_owned())
        .or_insert_with(|| {
            json!({"_type": "ARCHETYPED",
                   "archetype_id": {"_type": "ARCHETYPE_ID", "value": root.node_id},
                   "rm_version": "1.0.4"})
        });
    if let Value::Object(ad) = ad {
        ad.entry("template_id".to_owned())
            .or_insert_with(|| json!({"_type": "TEMPLATE_ID", "value": template_id}));
    }
}
