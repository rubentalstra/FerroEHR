//! FLAT (simSDT) → RM (canonical JSON) conversion — the composition builder.
//!
//! The reverse of [`to_flat`](super::to_flat): FLAT keys are parsed into
//! path-segment entries, grouped as the [`WebTemplate`] tree is walked (matching
//! each child by json-`id` / `alternativeJsonId`, `:i`-indexed instances kept in
//! order), and each leaf's datum parts rebuild its `DATA_VALUE`. The RM
//! structural nodes the web-template compacted away (`HISTORY`, a single
//! `EVENT`, `ITEM_TREE`, the `ELEMENT` wrapper, `INSTRUCTION.activities` /
//! `ACTIVITY`, `ACTION.ism_transition`, `SECTION`, archetyped `other_context`)
//! are re-materialised from the relative AQL path between a node and its parent;
//! a final [`graph::fill_structural_mandatory`] pass fills the RM-mandatory
//! fields FLAT never surfaces (event `width`/`math_function`, activity
//! `action_archetype_id`, ism `current_state`, …), so the result deserialises as
//! an `openehr-rm` `Composition`. `ctx/…` keys rebuild the composition context
//! (see [`context`](super::context)).

use openehr_rm::paths::PathSegment;
use serde_json::{Map, Value, json};

use super::defaults::{DEFAULT_TIME, RM_VERSION};
use super::mappers::{self};
use super::sub::{Entry, FlatView, parse_key};
use super::{context, graph, rmattr};
use crate::FlatError;
use crate::path;
use crate::webtemplate::{CodedName, WebTemplate, WebTemplateNode};

/// Map a (possibly abstract/generic) web-template rm type to the concrete RM
/// type to instantiate. `EVENT` is abstract → default `POINT_EVENT`.
fn concrete_type(rm_type: &str) -> &str {
    match strip_generic(rm_type) {
        "EVENT" => "POINT_EVENT",
        other => other,
    }
}

/// RM attributes that are arrays (needed to re-materialise compacted structure).
///
/// Multiplicity is driven from the generated BMM RM attribute model (the static
/// `openehr_rm::model`, the same model AQL path analysis uses) rather than a
/// hard-coded list: the shared derivation lives in [`crate::tdd::is_multiple_attr`]
/// (walk from the versioned-object roots, count class-typed `List`/`Set`/`Hash`
/// attributes, exclude primitive byte arrays such as `DV_MULTIMEDIA.data`). This
/// now covers every genuinely multi-valued structural attribute a template can
/// reach (`other_participations`, `participations`, nested cluster/section
/// variants, …), not only the COMPOSITION/HISTORY/ITEM/ENTRY common path.
fn is_multiple(attr: &str) -> bool {
    crate::tdd::is_multiple_attr(attr)
}

/// Convert a FLAT map to a canonical-JSON composition, driven by `wt`.
///
/// # Errors
/// [`FlatError::Conversion`] if the FLAT input is empty of routable keys and no
/// `ctx/…` context is present (nothing to build).
pub fn from_flat(flat: &Map<String, Value>, wt: &WebTemplate) -> Result<Value, FlatError> {
    let root_id = &wt.tree.id;
    // Root-level composition attributes addressed by path (`<root>/territory|
    // code`, `<root>/language|code`, `<root>/composer|name`, `<root>/category|
    // code`, …) are the path-form spelling of the `ctx/…` shortcuts (SDT /
    // Better accept either). The compacted tree carries no node for these
    // in-context attributes, so map them onto the corresponding ctx key (an
    // explicit `ctx/…` key wins) and let `apply_ctx` build the RM values.
    let flat = &synthesize_root_ctx(flat, root_id);
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
    // Resolve `category` BEFORE the context is built: `apply_ctx` inspects it to
    // decide whether a persistent Composition should carry a synthesised Event
    // context (RM ehr master05 §"Persistent Compositions may optionally have an
    // Event context").
    if let Some(cat) = root_category(flat, root_id) {
        comp.entry("category".to_owned()).or_insert(cat);
    }
    ensure_category(&mut comp);
    context::apply_ctx(flat, &mut comp);
    let mut value = Value::Object(comp);
    // Final structural pass: fill the RM-mandatory fields FLAT never surfaces
    // (INTERVAL_EVENT width/math_function, ACTIVITY action_archetype_id, event
    // data, item items, ism current_state) on every node — synthesised
    // intermediates and web-template nodes alike — so the result deserialises as
    // an `openehr-rm` Composition. `or_insert_with` never overwrites a
    // datum-driven value, so the round-trip stays stable.
    complete_tree(&mut value);
    Ok(value)
}

/// Map root-level in-context attribute keys (`<root>/territory|code`, …) onto
/// their `ctx/…` equivalents so [`context::apply_ctx`] builds the RM values.
/// An explicit `ctx/…` key always wins; the root keys are left in place (they
/// route to no tree node and are ignored by `build`).
fn synthesize_root_ctx(flat: &Map<String, Value>, root_id: &str) -> Map<String, Value> {
    const MAP: [(&str, &str); 6] = [
        ("language|code", "ctx/language"),
        ("territory|code", "ctx/territory"),
        ("composer|name", "ctx/composer_name"),
        ("composer|id", "ctx/composer_id"),
        ("composer|id_namespace", "ctx/id_namespace"),
        ("composer|id_scheme", "ctx/id_scheme"),
    ];
    let mut out = flat.clone();
    for (root_key, ctx_key) in MAP {
        if let Some(v) = flat.get(&format!("{root_id}/{root_key}"))
            && !out.contains_key(ctx_key)
        {
            out.insert(ctx_key.to_owned(), v.clone());
        }
    }
    out
}

/// `COMPOSITION.category` from the root-level `<root>/category|code` /
/// `|value` / `|terminology` keys, when present.
fn root_category(flat: &Map<String, Value>, root_id: &str) -> Option<Value> {
    let code = flat
        .get(&format!("{root_id}/category|code"))?
        .as_str()?
        .to_owned();
    let value = flat
        .get(&format!("{root_id}/category|value"))
        .and_then(Value::as_str)
        .unwrap_or("event")
        .to_owned();
    let term = flat
        .get(&format!("{root_id}/category|terminology"))
        .and_then(Value::as_str)
        .unwrap_or("openehr")
        .to_owned();
    Some(json!({
        "_type": "DV_CODED_TEXT",
        "value": value,
        "defining_code": {
            "_type": "CODE_PHRASE",
            "terminology_id": {"_type": "TERMINOLOGY_ID", "value": term},
            "code_string": code,
        },
    }))
}

/// Recursively fill each node's RM-mandatory structural fields (see
/// [`graph::fill_structural_mandatory`]).
fn complete_tree(v: &mut Value) {
    match v {
        Value::Object(m) => {
            if let Some(ty) = m.get("_type").and_then(Value::as_str).map(str::to_owned) {
                graph::fill_structural_mandatory(m, &ty, DEFAULT_TIME);
            }
            for child in m.values_mut() {
                complete_tree(child);
            }
        }
        Value::Array(a) => {
            for e in a.iter_mut() {
                complete_tree(e);
            }
        }
        _ => {}
    }
}

/// Build the RM value for `node` from its (root-relative) `entries`.
fn build(node: &WebTemplateNode, entries: &[Entry]) -> Value {
    if node.has_input() {
        let view = FlatView::new(entries);
        let mut dv = mappers::leaf_from_flat(&node.rm_type, &view)
            .unwrap_or_else(|| json!({"_type": strip_generic(&node.rm_type)}));
        // Value-level `_`-attributes (`_normal_range`, `_language`, `_mapping`,
        // `_accuracy`, `_charset`, `_thumbnail`) attach to the DV value itself;
        // ELEMENT-level ones (`_uid`, `_null_flavour`, …) are applied to the
        // ELEMENT wrapper by `place` (master05 per-type tables).
        if let Value::Object(m) = &mut dv {
            let dv_attrs: Vec<Entry> = entries
                .iter()
                .filter(|e| {
                    e.segs.first().is_some_and(|s| {
                        rmattr::is_rm_attr(&s.id) && !rmattr::is_element_level(&s.id)
                    })
                })
                .cloned()
                .collect();
            if !dv_attrs.is_empty() {
                rmattr::apply_rm_attrs(m, &dv_attrs, strip_generic(&node.rm_type));
            }
        }
        return dv;
    }

    let mut obj = Map::new();
    obj.insert("_type".into(), json!(concrete_type(&node.rm_type)));

    // Container-level `_`-attributes addressed to this node (first segment is an
    // `_attr`, matching no child) — applied after the children are built.
    let own_attrs: Vec<Entry> = entries
        .iter()
        .filter(|e| e.segs.first().is_some_and(|s| rmattr::is_rm_attr(&s.id)))
        .cloned()
        .collect();

    let ctx_time = json!(DEFAULT_TIME);
    for child in &node.children {
        // Inside EVENT_CONTEXT only the archetyped `other_context` items are
        // rebuilt from the tree; the mandatory context fields come from ctx/
        // (merged by `context::apply_ctx`).
        if node.rm_type == "EVENT_CONTEXT" && !child.aql_path.contains("other_context") {
            continue;
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

        let rel = path::relative(&node.aql_path, &child.aql_path);
        for (_, sub) in groups {
            let child_val = build(child, &sub);
            // A leaf child sits inside an ELEMENT wrapper `place` creates; route
            // its ELEMENT-level `_`-attributes there (`_uid`, `_null_flavour`, …).
            let elem_attrs: Vec<Entry> = if child.has_input() {
                sub.iter()
                    .filter(|e| {
                        e.segs
                            .first()
                            .is_some_and(|s| rmattr::is_element_level(&s.id))
                    })
                    .cloned()
                    .collect()
            } else {
                Vec::new()
            };
            place(&mut obj, &rel, child_val, child, &ctx_time, &elem_attrs);
        }
    }

    let mut value = Value::Object(obj);
    if let Value::Object(m) = &mut value {
        rmattr::apply_rm_attrs(m, &own_attrs, &node.rm_type);
        finish_identity(m, node, false, "");
    }
    value
}

/// Insert `child_value` into `parent` at the relative AQL path `rel`,
/// re-materialising the compacted RM structural nodes it passes through.
fn place(
    parent: &mut Map<String, Value>,
    rel: &[PathSegment],
    child_value: Value,
    child: &WebTemplateNode,
    time: &Value,
    elem_attrs: &[Entry],
) {
    if rel.is_empty() {
        return;
    }
    let id_idx = rel.iter().rposition(|s| is_multiple(&s.attribute));
    place_rec(parent, rel, 0, id_idx, child_value, child, time, elem_attrs);
}

#[allow(clippy::too_many_arguments)]
fn place_rec(
    cur: &mut Map<String, Value>,
    rel: &[PathSegment],
    i: usize,
    id_idx: Option<usize>,
    child_value: Value,
    child: &WebTemplateNode,
    time: &Value,
    elem_attrs: &[Entry],
) {
    let seg = &rel[i];
    let node_id = seg.predicate.archetype_node_id.as_deref();
    let last = i + 1 == rel.len();

    if Some(i) == id_idx {
        // The child's own (repeating) array level.
        let arr = cur
            .entry(seg.attribute.clone())
            .or_insert_with(|| json!([]))
            .as_array_mut();
        let Some(arr) = arr else { return };
        if last {
            // The child value is itself the array element (a container child).
            let mut el = child_value;
            set_node_id(&mut el, node_id);
            arr.push(el);
        } else {
            // Wrap: the remaining path (e.g. `/value`) lives inside a new element.
            let mut el = new_struct(
                seg,
                rel.get(i + 1),
                child.name.as_deref(),
                child.name_coded.as_ref(),
                time,
            );
            if let Value::Object(m) = &mut el {
                place(m, &rel[i + 1..], child_value, child, time, elem_attrs);
            }
            arr.push(el);
        }
        return;
    }

    if is_multiple(&seg.attribute) {
        // A structural (single-occurrence) array level: find-or-create by node id.
        let arr = cur
            .entry(seg.attribute.clone())
            .or_insert_with(|| json!([]))
            .as_array_mut();
        let Some(arr) = arr else { return };
        let pos = arr
            .iter()
            .position(|e| e.get("archetype_node_id").and_then(Value::as_str) == node_id);
        let idx = if let Some(p) = pos {
            p
        } else {
            arr.push(new_struct(seg, rel.get(i + 1), None, None, time));
            arr.len() - 1
        };
        if let Some(Value::Object(m)) = arr.get_mut(idx) {
            place_rec(m, rel, i + 1, id_idx, child_value, child, time, elem_attrs);
        }
        return;
    }

    // A single-valued (object) attribute.
    if last {
        cur.insert(seg.attribute.clone(), child_value);
        // The map that receives the `value` attribute is the ELEMENT wrapper the
        // web-template compacted away; apply its ELEMENT-level `_`-attributes here
        // (master05 §ELEMENT). Guarded on `value` so a bare leaf (e.g. EVENT.time)
        // is never misattributed.
        if seg.attribute == "value" && !elem_attrs.is_empty() {
            rmattr::apply_rm_attrs(cur, elem_attrs, "");
        }
        return;
    }
    if !cur.contains_key(&seg.attribute) {
        cur.insert(
            seg.attribute.clone(),
            new_struct(seg, rel.get(i + 1), None, None, time),
        );
    }
    if let Some(Value::Object(m)) = cur.get_mut(&seg.attribute) {
        place_rec(m, rel, i + 1, id_idx, child_value, child, time, elem_attrs);
    }
}

/// The `name` value for a locatable node: a `DV_CODED_TEXT` carrying the
/// template's constrained `defining_code` when the name is coded (RM common
/// `master03-archetyped_package.adoc` §"The `LOCATABLE` class" — a
/// `LOCATABLE.name` is `DV_TEXT` or `DV_CODED_TEXT`), else a plain `DV_TEXT`.
fn name_value(display: &str, coded: Option<&CodedName>) -> Value {
    match coded {
        Some(CodedName {
            terminology, code, ..
        }) => json!({
            "_type": "DV_CODED_TEXT",
            "value": display,
            "defining_code": {
                "_type": "CODE_PHRASE",
                "terminology_id": {"_type": "TERMINOLOGY_ID", "value": terminology},
                "code_string": code,
            },
        }),
        None => json!({"_type": "DV_TEXT", "value": display}),
    }
}

/// Create a compacted structural RM node for `seg` (its `_type` inferred from
/// the attribute it sits under and the next step), with mandatory fields filled.
fn new_struct(
    seg: &PathSegment,
    next: Option<&PathSegment>,
    name: Option<&str>,
    coded_name: Option<&CodedName>,
    time: &Value,
) -> Value {
    let rm_type = infer_type(&seg.attribute, next.map(|s| s.attribute.as_str()));
    let node_id = seg.predicate.archetype_node_id.as_deref();
    let mut o = Map::new();
    o.insert("_type".into(), json!(rm_type));
    let display = name.or(node_id).unwrap_or(rm_type);
    o.insert("name".into(), name_value(display, coded_name));
    if let Some(nid) = node_id {
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
    if rm_type == "EVENT_CONTEXT" {
        return; // PATHABLE, not LOCATABLE: no name/archetype; fields come from ctx/
    }
    // name
    obj.entry("name".to_owned()).or_insert_with(|| {
        let text = node
            .name
            .as_deref()
            .or(node.node_id.as_deref())
            .unwrap_or(rm_type);
        name_value(text, node.name_coded.as_ref())
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
                a.insert("rm_version".into(), json!(RM_VERSION));
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
    // The synthesized node stands in for a structural attribute the simplified
    // form carried no content under. Two spec cases:
    //
    // * The template **constrains** the attribute to a node-identified structural
    //   child (e.g. `ACTION.description` → `ITEM_TREE[at0017]`): that constraint
    //   must be satisfied by a conforming value (AOM 1.4
    //   `AM/docs/AOM1.4/master04-constraint_model_package.adoc` §`Valid_value`).
    //   The web-template records the constrained identity as a structural stub
    //   (dropped from the tree because it had no leaf content), so we stamp the
    //   *constrained* `archetype_node_id`/type/name — an `at0001` placeholder here
    //   is rejected by the closed-archetype walk (`unexpected node 'at0001'`).
    // * The template leaves the attribute **unconstrained**: no constraint stated
    //   means any RM-valid value is permitted (ADL 1.4
    //   `AM/docs/ADL1.4/master05-cadl.adoc` §"Any" Constraints; CNF
    //   `master15-content_tc_composition.adoc` L38 — "When there is no constraint
    //   defined for an attribute … anything is allowed on that attribute"). No
    //   faithful source id exists, so the `at0001` placeholder is stamped — it
    //   only needs to be a non-empty archetype-relative id for the rebuilt object
    //   to be a valid `LOCATABLE` (`RM/.../common/locatable.adoc` invariants).
    match rm_type {
        "OBSERVATION" => {
            obj.entry("data".to_owned())
                .or_insert_with(|| structural_from_stub(node, "data", "HISTORY", "History"));
        }
        "EVALUATION" | "ADMIN_ENTRY" => {
            obj.entry("data".to_owned())
                .or_insert_with(|| structural_from_stub(node, "data", "ITEM_TREE", "Tree"));
        }
        "INSTRUCTION" => {
            obj.entry("narrative".to_owned())
                .or_insert_with(|| json!({"_type": "DV_TEXT", "value": "<narrative>"}));
        }
        "ACTION" => {
            obj.entry("time".to_owned())
                .or_insert_with(|| json!({"_type": "DV_DATE_TIME", "value": DEFAULT_TIME}));
            obj.entry("description".to_owned())
                .or_insert_with(|| structural_from_stub(node, "description", "ITEM_TREE", "Tree"));
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

/// Synthesize an empty structural node for a missing mandatory ENTRY attribute.
///
/// If the resolved web-template `node` records a structural stub for `attr` (the
/// template constrained the attribute to a node-identified structural child that
/// carried no content and so was dropped from the compacted tree), the stub's
/// constrained RM type / `archetype_node_id` / rubric name are stamped — a value
/// the closed-archetype walk admits (AOM 1.4
/// `AM/docs/AOM1.4/master04-constraint_model_package.adoc` §`Valid_value`).
/// Otherwise the attribute is unconstrained, so the spec-legal `at0001` "Any"
/// placeholder (ADL 1.4 `master05-cadl.adoc` §"Any" Constraints; CNF
/// `master15-content_tc_composition.adoc` L38) is used with `default_rm_type` /
/// `default_name`.
fn structural_from_stub(
    node: &WebTemplateNode,
    attr: &str,
    default_rm_type: &str,
    default_name: &str,
) -> Value {
    match node.structural_stubs.iter().find(|s| s.attr == attr) {
        Some(stub) => {
            let name = stub.name.as_deref().unwrap_or(default_name);
            structural_container(concrete_structural(&stub.rm_type), &stub.node_id, name)
        }
        None => structural_container(default_rm_type, "at0001", default_name),
    }
}

/// A concrete instantiable structural RM type: an abstract `ITEM_STRUCTURE`
/// constraint (the archetype constrains the family, not a concrete member)
/// materialises as an `ITEM_TREE`; a concrete type passes through.
fn concrete_structural(rm_type: &str) -> &str {
    match rm_type {
        "ITEM_STRUCTURE" => "ITEM_TREE",
        other => other,
    }
}

/// Build an empty structural container node of `rm_type` with the given
/// `node_id`/`name` and its RM-mandatory empty child containers (`HISTORY` →
/// `origin` + `events`; `ITEM_*`/`CLUSTER` → `items`). The final
/// [`complete_tree`] pass fills any remaining RM-mandatory fields.
fn structural_container(rm_type: &str, node_id: &str, name: &str) -> Value {
    let mut o = Map::new();
    o.insert("_type".into(), json!(rm_type));
    o.insert("archetype_node_id".into(), json!(node_id));
    o.insert("name".into(), json!({"_type": "DV_TEXT", "value": name}));
    match rm_type {
        "HISTORY" => {
            o.insert(
                "origin".into(),
                json!({"_type": "DV_DATE_TIME", "value": DEFAULT_TIME}),
            );
            o.insert("events".into(), json!([]));
        }
        _ => {
            o.insert("items".into(), json!([]));
        }
    }
    Value::Object(o)
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
                   "rm_version": RM_VERSION})
        });
    if let Value::Object(ad) = ad {
        ad.entry("template_id".to_owned())
            .or_insert_with(|| json!({"_type": "TEMPLATE_ID", "value": template_id}));
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::{CodedName, name_value};
    use serde_json::json;

    #[test]
    fn name_value_stamps_dv_coded_text_when_coded() {
        let coded = CodedName {
            terminology: "local".to_owned(),
            code: "at0007".to_owned(),
            incoherent: false,
        };
        assert_eq!(
            name_value("Global exclusion of adverse reactions", Some(&coded)),
            json!({
                "_type": "DV_CODED_TEXT",
                "value": "Global exclusion of adverse reactions",
                "defining_code": {
                    "_type": "CODE_PHRASE",
                    "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "local"},
                    "code_string": "at0007",
                },
            })
        );
    }

    #[test]
    fn name_value_stamps_plain_dv_text_when_uncoded() {
        assert_eq!(
            name_value("Systolic", None),
            json!({"_type": "DV_TEXT", "value": "Systolic"})
        );
    }
}
