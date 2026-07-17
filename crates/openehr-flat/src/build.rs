//! Simplified tree → RM (canonical JSON) — the composition builder.
//!
//! The reverse of [`crate::flatten`]: the [`WebTemplate`] tree is walked
//! against the parsed [`SimDocument`], each leaf's datum parts rebuild its
//! DATA_VALUE per the `master05-rm_mapping.adoc` tables ([`crate::map`]),
//! and the RM structural nodes the simplified form omits (ITS-REST
//! `simplified_formats/master04` §Level Removal: `HISTORY`, a collapsed
//! `EVENT`, the `ITEM_STRUCTURE` family, the `ELEMENT` wrapper) are
//! re-materialised from the relative RM path between a node and its
//! parent. The `ctx/` vocabulary ([`crate::ctx`],
//! `master06-context_information.adoc`) supplies the composition context
//! and per-entry defaults. A final structural pass fills the RM-mandatory
//! fields the simplified form never surfaces, so the result deserialises
//! as an `openehr-rm` Composition.
//!
//! Unknown field identifiers are rejected, not ignored
//! (`master04 §Validation`: "Field identifiers match WT metadata
//! structure").

use openehr_rm::paths::PathSegment;
use serde_json::{Map, Value, json};

use crate::ctx;
use crate::error::FlatError;
use crate::map;
use crate::rmpath;
use crate::sim::{SimDocument, SimNode};
use crate::webtemplate::{CodedName, WebTemplate, WebTemplateNode};

/// openEHR reference-model release stamped into a rebuilt
/// `ARCHETYPED.rm_version` (RM common `archetyped.adoc`: the "version of
/// the openEHR reference model used to create this object" — this system
/// creates data against the pinned RM 1.2.0).
pub(crate) const RM_VERSION: &str = "1.2.0";

/// Deterministic fill for RM-mandatory temporal fields the simplified form
/// never surfaces as data (`HISTORY.origin` of a synthesised history,
/// `EVENT.time` with no derivable origin). Never present on the simplified
/// wire, so a fixed value keeps the round-trip stable — a fresh `now()`
/// here would make two successive flattenings disagree. (The `ctx/time`
/// `now()` default of `master04 §Context` applies to the EVENT_CONTEXT via
/// [`crate::ctx`], not to these structural fills.)
pub(crate) const DEFAULT_TIME: &str = "1970-01-01T00:00:00Z";

/// Convert a parsed simplified document to a canonical-JSON composition,
/// driven by `wt`. `now` is the caller's current timestamp, used only for
/// the `ctx/time` default (`master04 §Context`).
///
/// # Errors
/// [`FlatError::UnknownPath`] for a field identifier matching no
/// web-template node; the [`crate::map`]/[`crate::ctx`] errors for datum
/// and context violations; [`FlatError::Conversion`] when the document
/// carries nothing buildable.
pub fn build_composition(
    doc: &SimDocument,
    wt: &WebTemplate,
    now: &str,
) -> Result<Value, FlatError> {
    let root_id = &wt.tree.id;
    let mut data_root = None;
    let mut ctx_node = None;
    for (name, child) in &doc.children {
        match name.as_str() {
            "ctx" => ctx_node = child.occurrences.first(),
            n if n == root_id => data_root = child.occurrences.first(),
            other => return Err(FlatError::UnknownPath(other.to_owned())),
        }
    }
    if data_root.is_none() && ctx_node.is_none() {
        return Err(FlatError::Conversion(
            "empty simplified document: no data root and no ctx".to_owned(),
        ));
    }

    let empty = SimNode::default();
    let data_root = data_root.unwrap_or(&empty);
    // Root-level path spellings of the in-context composition attributes
    // (`<root>/language|code`, `<root>/composer|name`, …) are the path form
    // of the `ctx/` shortcuts (`master05 §COMPOSITION` rows); an explicit
    // `ctx/…` key wins.
    let merged_ctx = merge_root_ctx_spellings(ctx_node, data_root);
    let explicit_event_context = has_explicit_event_context(merged_ctx.as_ref());
    let defaults = ctx::resolve(merged_ctx.as_ref(), now)?;

    let mut comp = match build(&wt.tree, data_root, root_id)? {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    finish_identity(&mut comp, &wt.tree, true, &wt.template_id);
    ensure_template_id(&mut comp, &wt.tree, &wt.template_id);
    // Resolve `category` BEFORE the context is applied: persistent
    // compositions may optionally omit the Event context (RM ehr master05
    // §"Persistent Compositions may optionally have an Event context").
    ensure_category(&mut comp);
    apply_composition_ctx(&mut comp, &defaults, explicit_event_context);
    let mut value = Value::Object(comp);
    apply_entry_defaults(&mut value, &defaults);
    complete_tree(&mut value);
    Ok(value)
}

/// Merge the root-level path spellings of the in-context composition
/// attributes into the `ctx` tree (`master05 §COMPOSITION` rows spell the
/// same data as paths; `master06` gives the `ctx/` shortcut). An explicit
/// `ctx/…` key always wins; the path spellings only fill gaps.
fn merge_root_ctx_spellings(ctx_node: Option<&SimNode>, data_root: &SimNode) -> Option<SimNode> {
    let mut merged = ctx_node.cloned().unwrap_or_default();
    let mut fill = |key: &str, value: Option<&Value>| {
        if let Some(v) = value
            && !merged.children.contains_key(key)
        {
            merged
                .occurrence_mut(key, None)
                .attrs
                .insert(String::new(), v.clone());
        }
    };
    let attr = |name: &str, suffix: &str| data_root.child(name).and_then(|n| n.attrs.get(suffix));
    fill("language", attr("language", "code"));
    fill("territory", attr("territory", "code"));
    fill("composer_name", attr("composer", "name"));
    fill("composer_id", attr("composer", "id"));
    fill("id_namespace", attr("composer", "id_namespace"));
    fill("id_scheme", attr("composer", "id_scheme"));
    if merged.is_empty() {
        None
    } else {
        Some(merged)
    }
}

/// Whether the client supplied any explicit EVENT_CONTEXT content — a
/// persistent composition only carries a context when it did (RM ehr
/// master05 §"Persistent Compositions may optionally have an Event
/// context").
fn has_explicit_event_context(ctx_node: Option<&SimNode>) -> bool {
    let Some(ctx) = ctx_node else { return false };
    ctx.children.keys().any(|key| {
        matches!(
            key.as_str(),
            "time" | "end_time" | "setting" | "location" | "health_care_facility"
        ) || key.starts_with("participation_")
    })
}

/// The `|raw` bypass on a container node: the payload must be a
/// canonical-JSON RM object carrying `_type`
/// (`master04 §Raw canonical JSON`).
fn build_raw(raw: &Value, path: &str) -> Result<Value, FlatError> {
    if raw.get("_type").and_then(Value::as_str).is_none() {
        return Err(FlatError::InvalidRaw {
            path: path.to_owned(),
            reason: "the |raw value must be a canonical-JSON object with _type".to_owned(),
        });
    }
    Ok(raw.clone())
}

// ── the template walk ──────────────────────────────────────────────────────

/// Segment names of the ELEMENT-level `_` RM attribute family — attributes
/// of the wrapping ELEMENT, not of the DATA_VALUE itself
/// (`master05 §ELEMENT`).
fn is_element_level(seg: &str) -> bool {
    matches!(
        seg,
        "_uid" | "_null_flavour" | "_null_reason" | "_link" | "_feeder_audit"
    )
}

/// Build the RM value for `node` from the simplified occurrence `sim`.
/// `path` is the printed simplified path (diagnostics).
fn build(node: &WebTemplateNode, sim: &SimNode, path: &str) -> Result<Value, FlatError> {
    if node.has_input() {
        let mut dv = map::build_leaf(sim, base_type(&node.rm_type), Some(node), path)?;
        // Value-level `_` attributes (`_normal_range`, `_mapping`,
        // `_accuracy`, `_language`, …) attach to the DV value itself;
        // ELEMENT-level ones are routed onto the wrapping ELEMENT by
        // `place` (master05 per-type tables vs §ELEMENT).
        if let Value::Object(m) = &mut dv {
            for (seg, child) in &sim.children {
                if seg.starts_with('_') && !is_element_level(seg) {
                    apply_rm_attr(m, seg, &child.occurrences, base_type(&node.rm_type), path)?;
                }
            }
        }
        return Ok(dv);
    }

    let mut obj = Map::new();
    obj.insert("_type".into(), json!(concrete_type(&node.rm_type)));

    for child in &node.children {
        // Standard EVENT_CONTEXT fields come from ctx/ (master06); only the
        // archetyped other_context content is tree data.
        if node.rm_type == "EVENT_CONTEXT" && !child.aql_path.contains("other_context") {
            continue;
        }
        // Composition-level in-context attributes arrive via ctx resolution
        // (`ctx::resolve` also reads their path spellings); `context` comes from
        // `apply_composition_ctx` (master06 §Context). `category` is real tree
        // data (master05 §COMPOSITION) and still builds.
        if node.rm_type == "COMPOSITION"
            && matches!(
                child.id.as_str(),
                "language" | "territory" | "composer" | "context"
            )
        {
            continue;
        }
        // ENTRY-level `language`/`encoding` default from the composition context
        // (master06 §"Language and Territory"); they are filled by
        // `apply_entry_defaults`, never rebuilt from a per-entry path key (the
        // synthesized CODE_PHRASE in-context nodes carry no leaf inputs). `subject`
        // is NOT skipped — a non-`PARTY_SELF` subject is real data
        // (master05 §OBSERVATION `/subject`) and builds as a leaf.
        if is_entry_family(&node.rm_type) && matches!(child.id.as_str(), "language" | "encoding") {
            continue;
        }
        let Some(sim_child) = sim.children.get(&child.id).or_else(|| {
            child
                .alt_json_id
                .as_deref()
                .and_then(|a| sim.children.get(a))
        }) else {
            continue;
        };
        let rel = rmpath::relative(&node.aql_path, &child.aql_path);
        for (i, occ) in sim_child.occurrences.iter().enumerate() {
            if occ.is_empty() {
                continue; // a preserved index hole
            }
            let child_path = format!("{path}/{}:{i}", child.id);
            let child_val = build(child, occ, &child_path)?;
            place(&mut obj, &rel, child_val, child, occ, &child_path)?;
        }
    }

    // Container-level `_` RM attribute families addressed to this node.
    for (seg, child) in &sim.children {
        if seg.starts_with('_') {
            apply_rm_attr(
                &mut obj,
                seg,
                &child.occurrences,
                base_type(&node.rm_type),
                path,
            )?;
            continue;
        }
        // Reject identifiers that matched no template child (master04
        // §Validation: field identifiers match WT metadata structure).
        let known = node
            .children
            .iter()
            .any(|c| c.id == *seg || c.alt_json_id.as_deref() == Some(seg));
        let ctx_covered = node.rm_type == "COMPOSITION"
            && matches!(
                seg.as_str(),
                "language" | "territory" | "composer" | "category"
            );
        if !known && !ctx_covered {
            return Err(FlatError::UnknownPath(format!("{path}/{seg}")));
        }
    }
    // Datum parts on a container node are only legal via |raw
    // (master04 §Raw canonical JSON); anything else is an unknown suffix.
    if let Some(raw) = sim.attrs.get("raw") {
        return build_raw(raw, path);
    }
    if let Some((chain, _)) = sim.attrs.iter().find(|(c, _)| !c.is_empty()) {
        return Err(FlatError::UnknownSuffix {
            rm_type: node.rm_type.clone(),
            suffix: chain.clone(),
            path: path.to_owned(),
        });
    }

    let mut value = Value::Object(obj);
    if let Value::Object(m) = &mut value {
        finish_identity(m, node, false, "");
    }
    Ok(value)
}

/// Apply one `_`-segment RM attribute family onto the RM object under
/// construction.
fn apply_rm_attr(
    obj: &mut Map<String, Value>,
    seg: &str,
    occurrences: &[SimNode],
    host_rm_type: &str,
    path: &str,
) -> Result<(), FlatError> {
    if let Some((attr, value)) = map::build_rm_attr(seg, occurrences, host_rm_type, path)? {
        obj.insert(attr, value);
    }
    Ok(())
}

// ── structural re-materialisation ──────────────────────────────────────────

/// Insert `child_value` into `parent` at the relative RM path `rel`,
/// re-materialising the collapsed structural nodes it passes through
/// (`master04 §Level Removal`).
fn place(
    parent: &mut Map<String, Value>,
    rel: &[PathSegment],
    child_value: Value,
    child: &WebTemplateNode,
    occ: &SimNode,
    path: &str,
) -> Result<(), FlatError> {
    if rel.is_empty() {
        return Ok(());
    }
    let id_idx = rel.iter().rposition(|s| is_multiple(&s.attribute));
    place_rec(parent, rel, 0, id_idx, child_value, child, occ, path)
}

#[allow(clippy::too_many_arguments)] // a recursive placement cursor, not an API
fn place_rec(
    cur: &mut Map<String, Value>,
    rel: &[PathSegment],
    i: usize,
    id_idx: Option<usize>,
    child_value: Value,
    child: &WebTemplateNode,
    occ: &SimNode,
    path: &str,
) -> Result<(), FlatError> {
    let seg = &rel[i];
    let node_id = seg.predicate.archetype_node_id.as_deref();
    let last = i + 1 == rel.len();

    if Some(i) == id_idx {
        // The child's own (repeating) array level.
        let arr = cur
            .entry(seg.attribute.clone())
            .or_insert_with(|| json!([]))
            .as_array_mut();
        let Some(arr) = arr else { return Ok(()) };
        if last {
            // The child value is itself the array element (a container child).
            let mut el = child_value;
            set_node_id(&mut el, node_id);
            arr.push(el);
        } else {
            // Wrap: the remaining path (e.g. `/value`) lives inside a new
            // ELEMENT (or deeper structural) node.
            let mut el = new_struct(
                seg,
                rel.get(i + 1),
                child.name.as_deref(),
                child.name_coded.as_ref(),
            );
            if let Value::Object(m) = &mut el {
                place(m, &rel[i + 1..], child_value, child, occ, path)?;
                // The ELEMENT wrapper's own `_` attribute family
                // (master05 §ELEMENT).
                apply_element_attrs(m, occ, path)?;
            }
            arr.push(el);
        }
        return Ok(());
    }

    if is_multiple(&seg.attribute) {
        // A structural (single-occurrence) array level: find-or-create by
        // node id.
        let arr = cur
            .entry(seg.attribute.clone())
            .or_insert_with(|| json!([]))
            .as_array_mut();
        let Some(arr) = arr else { return Ok(()) };
        let pos = arr
            .iter()
            .position(|e| e.get("archetype_node_id").and_then(Value::as_str) == node_id);
        let idx = if let Some(p) = pos {
            p
        } else {
            arr.push(new_struct(seg, rel.get(i + 1), None, None));
            arr.len() - 1
        };
        if let Some(Value::Object(m)) = arr.get_mut(idx) {
            place_rec(m, rel, i + 1, id_idx, child_value, child, occ, path)?;
        }
        return Ok(());
    }

    // A single-valued (object) attribute.
    if last {
        let wraps_element = seg.attribute == "value";
        cur.insert(seg.attribute.clone(), child_value);
        if wraps_element {
            // The map receiving `value` is the compacted-away ELEMENT
            // wrapper; apply its `_` attribute family (master05 §ELEMENT).
            apply_element_attrs(cur, occ, path)?;
        }
        return Ok(());
    }
    if !cur.contains_key(&seg.attribute) {
        cur.insert(
            seg.attribute.clone(),
            new_struct(seg, rel.get(i + 1), None, None),
        );
    }
    if let Some(Value::Object(m)) = cur.get_mut(&seg.attribute) {
        place_rec(m, rel, i + 1, id_idx, child_value, child, occ, path)?;
    }
    Ok(())
}

/// Apply the ELEMENT-level `_` attributes of a leaf occurrence onto its
/// re-materialised ELEMENT wrapper (`master05 §ELEMENT`).
fn apply_element_attrs(
    element: &mut Map<String, Value>,
    occ: &SimNode,
    path: &str,
) -> Result<(), FlatError> {
    for (seg, child) in &occ.children {
        if is_element_level(seg) {
            apply_rm_attr(element, seg, &child.occurrences, "ELEMENT", path)?;
        }
    }
    Ok(())
}

/// RM attributes that are arrays — needed to re-materialise collapsed
/// structure. Driven from the generated BMM RM attribute model (the same
/// static model the AQL path analysis uses), via the shared derivation in
/// [`crate::tdd::is_multiple_attr`].
fn is_multiple(attr: &str) -> bool {
    crate::tdd::is_multiple_attr(attr)
}

/// Map a (possibly abstract/generic) web-template rm type to the concrete
/// RM type to instantiate. `EVENT` is abstract → `POINT_EVENT`.
fn concrete_type(rm_type: &str) -> &str {
    match base_type(rm_type) {
        "EVENT" => "POINT_EVENT",
        other => other,
    }
}

fn base_type(rm_type: &str) -> &str {
    rm_type.split('<').next().unwrap_or(rm_type)
}

/// The ENTRY family (`RM/docs/UML/classes/org.openehr.rm.composition.*`): the
/// concrete ENTRY subtypes that carry the RM-mandatory
/// `language`/`encoding`/`subject` in-context attributes.
fn is_entry_family(rm_type: &str) -> bool {
    matches!(
        rm_type,
        "OBSERVATION" | "EVALUATION" | "INSTRUCTION" | "ACTION" | "ADMIN_ENTRY" | "GENERIC_ENTRY"
    )
}

/// The `name` value for a locatable node: a DV_CODED_TEXT carrying the
/// template's constrained `defining_code` when the name is coded (RM common
/// `master03-archetyped_package.adoc` §"The LOCATABLE class"), else a plain
/// DV_TEXT.
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

/// Create a collapsed structural RM node for `seg` (its `_type` inferred
/// from the attribute it sits under and the next step), with mandatory
/// fields filled.
fn new_struct(
    seg: &PathSegment,
    next: Option<&PathSegment>,
    name: Option<&str>,
    coded_name: Option<&CodedName>,
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
            o.insert("origin".into(), dv_date_time(DEFAULT_TIME));
            o.insert("events".into(), json!([]));
        }
        "POINT_EVENT" | "EVENT" | "INTERVAL_EVENT" => {
            o.insert("time".into(), dv_date_time(DEFAULT_TIME));
        }
        "ITEM_TREE" | "ITEM_LIST" | "ITEM_SINGLE" | "ITEM_TABLE" | "CLUSTER" => {
            o.insert("items".into(), json!([]));
        }
        _ => {}
    }
    Value::Object(o)
}

/// Infer the RM type of a collapsed structural step (`master04 §Level
/// Removal`: the elided container attributes name their owner types).
fn infer_type(attr: &str, next: Option<&str>) -> &'static str {
    match (attr, next) {
        ("data", Some("events")) => "HISTORY",
        ("events", _) => "POINT_EVENT",
        ("items", _) => "ELEMENT",
        ("activities", _) => "ACTIVITY",
        _ => "ITEM_TREE",
    }
}

fn set_node_id(v: &mut Value, node_id: Option<&str>) {
    if let (Value::Object(m), Some(nid)) = (v, node_id) {
        m.entry("archetype_node_id".to_owned())
            .or_insert_with(|| json!(nid));
    }
}

// ── identity + mandatory-field completion ──────────────────────────────────

pub(crate) fn dv_date_time(time: &str) -> Value {
    json!({"_type": "DV_DATE_TIME", "value": time})
}

pub(crate) fn code_phrase(terminology: &str, code: &str) -> Value {
    json!({
        "_type": "CODE_PHRASE",
        "terminology_id": {"_type": "TERMINOLOGY_ID", "value": terminology},
        "code_string": code,
    })
}

pub(crate) fn dv_coded_text(value: &str, terminology: &str, code: &str) -> Value {
    json!({
        "_type": "DV_CODED_TEXT",
        "value": value,
        "defining_code": code_phrase(terminology, code),
    })
}

fn empty_item_tree() -> Value {
    json!({"_type": "ITEM_TREE", "name": {"_type": "DV_TEXT", "value": "Tree"}, "items": []})
}

/// Fill the mandatory identity/occurrence fields for a built locatable node.
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
        return; // PATHABLE, not LOCATABLE; fields come from ctx/
    }
    obj.entry("name".to_owned()).or_insert_with(|| {
        let text = node
            .name
            .as_deref()
            .or(node.node_id.as_deref())
            .unwrap_or(rm_type);
        name_value(text, node.name_coded.as_ref())
    });
    match rm_type {
        "POINT_EVENT" | "INTERVAL_EVENT" => {
            obj.entry("time".to_owned())
                .or_insert_with(|| dv_date_time(DEFAULT_TIME));
        }
        "HISTORY" => {
            obj.entry("origin".to_owned())
                .or_insert_with(|| dv_date_time(DEFAULT_TIME));
        }
        _ => {}
    }
    if let Some(nid) = &node.node_id {
        obj.entry("archetype_node_id".to_owned())
            .or_insert_with(|| json!(nid));
    }
    let is_root_arch = node
        .node_id
        .as_deref()
        .is_some_and(|n| n.starts_with("openEHR-"));
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
    if matches!(
        rm_type,
        "OBSERVATION" | "EVALUATION" | "INSTRUCTION" | "ACTION" | "ADMIN_ENTRY" | "GENERIC_ENTRY"
    ) {
        obj.entry("subject".to_owned())
            .or_insert_with(|| json!({"_type": "PARTY_SELF"}));
    }
    // RM-mandatory structural attributes the simplified form carried no
    // content under. When the template constrains the attribute to a
    // node-identified structural child, the recorded structural stub's
    // identity is stamped — a value the closed-archetype walk admits
    // (AOM 1.4 `master04-constraint_model_package.adoc` §Valid_value).
    // Unconstrained attributes get the spec-legal `at0001` "Any"
    // placeholder (ADL 1.4 `master05-cadl.adoc` §"Any" Constraints).
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
                .or_insert_with(|| dv_date_time(DEFAULT_TIME));
            obj.entry("description".to_owned())
                .or_insert_with(|| structural_from_stub(node, "description", "ITEM_TREE", "Tree"));
            obj.entry("ism_transition".to_owned()).or_insert_with(|| {
                json!({"_type": "ISM_TRANSITION",
                       "current_state": dv_coded_text("initial", "openehr", "524")})
            });
        }
        _ => {}
    }
}

/// Synthesize an empty structural node for a missing mandatory ENTRY
/// attribute (see [`finish_identity`]).
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

/// An abstract `ITEM_STRUCTURE` constraint materialises as `ITEM_TREE`; a
/// concrete type passes through.
fn concrete_structural(rm_type: &str) -> &str {
    match rm_type {
        "ITEM_STRUCTURE" => "ITEM_TREE",
        other => other,
    }
}

fn structural_container(rm_type: &str, node_id: &str, name: &str) -> Value {
    let mut o = Map::new();
    o.insert("_type".into(), json!(rm_type));
    o.insert("archetype_node_id".into(), json!(node_id));
    o.insert("name".into(), json!({"_type": "DV_TEXT", "value": name}));
    match rm_type {
        "HISTORY" => {
            o.insert("origin".into(), dv_date_time(DEFAULT_TIME));
            o.insert("events".into(), json!([]));
        }
        _ => {
            o.insert("items".into(), json!([]));
        }
    }
    Value::Object(o)
}

/// COMPOSITION.category is mandatory; default `event` (openEHR 433) when
/// the tree carried none.
fn ensure_category(comp: &mut Map<String, Value>) {
    comp.entry("category".to_owned())
        .or_insert_with(|| dv_coded_text("event", "openehr", "433"));
}

/// Ensure the composition's `archetype_details` carries `archetype_id` +
/// `template_id` (the composition must be self-describing for its
/// template; the `openehr-template-id` REST header exists exactly because
/// the simplified payload cannot carry it — ITS-REST
/// `Requests_and_responses.md §openehr-template-id`).
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

// ── context application ────────────────────────────────────────────────────

/// Apply the composition-level context (`master06`: language, territory,
/// composer, and the EVENT_CONTEXT when warranted).
fn apply_composition_ctx(
    comp: &mut Map<String, Value>,
    defaults: &ctx::CtxDefaults,
    explicit_event_context: bool,
) {
    if let Some(language) = defaults.language_code_phrase() {
        comp.entry("language".to_owned()).or_insert(language);
    }
    if let Some(territory) = defaults.territory_code_phrase() {
        comp.entry("territory".to_owned()).or_insert(territory);
    }
    if let Some(composer) = &defaults.composer {
        comp.entry("composer".to_owned())
            .or_insert_with(|| composer.clone());
    }
    // An event composition carries an EVENT_CONTEXT; a persistent one gets
    // it only when explicit context content was supplied (RM ehr master05
    // §"Persistent Compositions may optionally have an Event context").
    let is_event = comp
        .get("category")
        .and_then(|c| c.get("defining_code"))
        .and_then(|d| d.get("code_string"))
        .and_then(Value::as_str)
        == Some("433");
    if !is_event && !explicit_event_context && !comp.contains_key("context") {
        return;
    }
    let context = comp
        .entry("context".to_owned())
        .or_insert_with(|| json!({"_type": "EVENT_CONTEXT"}));
    let Value::Object(context) = context else {
        return;
    };
    context
        .entry("start_time".to_owned())
        .or_insert_with(|| dv_date_time(&defaults.time));
    context
        .entry("setting".to_owned())
        .or_insert_with(|| defaults.setting.clone());
    if let Some(end_time) = &defaults.end_time {
        context
            .entry("end_time".to_owned())
            .or_insert_with(|| dv_date_time(end_time));
    }
    if let Some(location) = &defaults.location {
        context
            .entry("location".to_owned())
            .or_insert_with(|| json!(location));
    }
    if let Some(facility) = &defaults.health_care_facility {
        context
            .entry("health_care_facility".to_owned())
            .or_insert_with(|| facility.clone());
    }
    if !defaults.participations.is_empty() {
        context
            .entry("participations".to_owned())
            .or_insert_with(|| json!(defaults.participations));
    }
}

/// Walk the built tree applying the per-entry `ctx/` defaults
/// (`master06`: ENTRY language/encoding/subject/provider/workflow_id/
/// other_participations, OBSERVATION history origin + event times, ACTION
/// time/ism state, INSTRUCTION narrative, ACTIVITY timing, LOCATABLE
/// links on the root).
fn apply_entry_defaults(value: &mut Value, defaults: &ctx::CtxDefaults) {
    if !defaults.links.is_empty()
        && let Value::Object(root) = value
    {
        root.entry("links".to_owned())
            .or_insert_with(|| json!(defaults.links));
    }
    walk_entry_defaults(value, defaults);
}

#[allow(clippy::too_many_lines)] // one recursive walk over the ENTRY-default families
fn walk_entry_defaults(value: &mut Value, defaults: &ctx::CtxDefaults) {
    match value {
        Value::Array(items) => {
            for item in items {
                walk_entry_defaults(item, defaults);
            }
        }
        Value::Object(obj) => {
            let rm_type = obj
                .get("_type")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            if matches!(
                rm_type.as_str(),
                "OBSERVATION"
                    | "EVALUATION"
                    | "INSTRUCTION"
                    | "ACTION"
                    | "ADMIN_ENTRY"
                    | "GENERIC_ENTRY"
            ) {
                if let Some(language) = defaults.language_code_phrase() {
                    obj.entry("language".to_owned()).or_insert(language);
                }
                obj.entry("encoding".to_owned())
                    .or_insert_with(|| code_phrase("IANA_character-sets", "UTF-8"));
                if let Some(provider) = &defaults.provider {
                    obj.entry("provider".to_owned())
                        .or_insert_with(|| provider.clone());
                }
                if let Some(wf) = &defaults.work_flow_id {
                    obj.entry("workflow_id".to_owned())
                        .or_insert_with(|| wf.clone());
                }
                if !defaults.participations.is_empty() {
                    obj.entry("other_participations".to_owned())
                        .or_insert_with(|| json!(defaults.participations));
                }
            }
            match rm_type.as_str() {
                "OBSERVATION" => {
                    let origin = defaults
                        .history_origin
                        .clone()
                        .unwrap_or_else(|| defaults.time.clone());
                    if let Some(Value::Object(history)) = obj.get_mut("data") {
                        let origin_value = history
                            .entry("origin".to_owned())
                            .or_insert_with(|| dv_date_time(&origin));
                        if origin_value.get("value").and_then(Value::as_str) == Some(DEFAULT_TIME) {
                            *origin_value = dv_date_time(&origin);
                        }
                        let origin_now = origin_value.clone();
                        // EVENT.time defaults to the history origin
                        // (master06 §time).
                        if let Some(Value::Array(events)) = history.get_mut("events") {
                            for event in events {
                                if let Value::Object(ev) = event {
                                    let time = ev
                                        .entry("time".to_owned())
                                        .or_insert_with(|| origin_now.clone());
                                    if time.get("value").and_then(Value::as_str)
                                        == Some(DEFAULT_TIME)
                                    {
                                        *time = origin_now.clone();
                                    }
                                }
                            }
                        }
                    }
                }
                "ACTION" => {
                    let time = defaults
                        .action_time
                        .clone()
                        .unwrap_or_else(|| defaults.time.clone());
                    let entry = obj
                        .entry("time".to_owned())
                        .or_insert_with(|| dv_date_time(&time));
                    if entry.get("value").and_then(Value::as_str) == Some(DEFAULT_TIME) {
                        *entry = dv_date_time(&time);
                    }
                    if let Some(state) = &defaults.action_ism_current_state {
                        let ism = obj
                            .entry("ism_transition".to_owned())
                            .or_insert_with(|| json!({"_type": "ISM_TRANSITION"}));
                        if let Value::Object(ism) = ism {
                            ism.entry("current_state".to_owned())
                                .or_insert_with(|| state.clone());
                        }
                    }
                }
                "INSTRUCTION" => {
                    if let Some(narrative) = &defaults.instruction_narrative {
                        obj.entry("narrative".to_owned())
                            .or_insert_with(|| json!({"_type": "DV_TEXT", "value": narrative}));
                    }
                }
                "ACTIVITY" => {
                    if let Some(timing) = &defaults.activity_timing {
                        obj.entry("timing".to_owned()).or_insert_with(|| {
                            json!({"_type": "DV_PARSABLE", "value": timing, "formalism": "timing"})
                        });
                    }
                }
                _ => {}
            }
            for child in obj.values_mut() {
                walk_entry_defaults(child, defaults);
            }
        }
        _ => {}
    }
}

/// Recursively fill each node's remaining RM-mandatory structural fields.
fn complete_tree(v: &mut Value) {
    match v {
        Value::Object(m) => {
            if let Some(ty) = m.get("_type").and_then(Value::as_str).map(str::to_owned) {
                fill_structural_mandatory(m, &ty);
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

/// Fill the RM-mandatory fields of an RM node that are never surfaced on
/// the simplified wire and are therefore absent after the datum-driven
/// build. Only missing fields are added, so a datum-driven value is never
/// overwritten and the round-trip stays stable.
fn fill_structural_mandatory(obj: &mut Map<String, Value>, rm_type: &str) {
    match rm_type {
        "HISTORY" => {
            obj.entry("origin".to_owned())
                .or_insert_with(|| dv_date_time(DEFAULT_TIME));
            obj.entry("events".to_owned()).or_insert_with(|| json!([]));
        }
        "POINT_EVENT" | "EVENT" => {
            obj.entry("time".to_owned())
                .or_insert_with(|| dv_date_time(DEFAULT_TIME));
            obj.entry("data".to_owned()).or_insert_with(empty_item_tree);
        }
        "INTERVAL_EVENT" => {
            obj.entry("time".to_owned())
                .or_insert_with(|| dv_date_time(DEFAULT_TIME));
            obj.entry("data".to_owned()).or_insert_with(empty_item_tree);
            // width + math_function are RM-mandatory on INTERVAL_EVENT but
            // never data-entry leaves (openEHR `146` = "mean").
            obj.entry("width".to_owned())
                .or_insert_with(|| json!({"_type": "DV_DURATION", "value": "P0D"}));
            obj.entry("math_function".to_owned())
                .or_insert_with(|| dv_coded_text("mean", "openehr", "146"));
        }
        "ITEM_TREE" | "ITEM_LIST" | "ITEM_SINGLE" | "ITEM_TABLE" | "CLUSTER" => {
            obj.entry("items".to_owned()).or_insert_with(|| json!([]));
        }
        "ACTIVITY" => {
            // action_archetype_id defaults to the match-all form
            // (master05 §ACTIVITY: "Will be set to /.*/ if not set
            // explicit.").
            obj.entry("action_archetype_id".to_owned())
                .or_insert_with(|| json!("/.*/"));
            obj.entry("description".to_owned())
                .or_insert_with(empty_item_tree);
        }
        "ISM_TRANSITION" => {
            obj.entry("current_state".to_owned())
                .or_insert_with(|| dv_coded_text("initial", "openehr", "524"));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_value_stamps_coded_and_plain_names() {
        let coded = CodedName {
            terminology: "local".to_owned(),
            code: "at0007".to_owned(),
            incoherent: false,
        };
        let v = name_value("Exclusion", Some(&coded));
        assert_eq!(v["_type"], "DV_CODED_TEXT");
        assert_eq!(v["defining_code"]["code_string"], "at0007");
        assert_eq!(
            name_value("Systolic", None),
            json!({"_type": "DV_TEXT", "value": "Systolic"})
        );
    }

    #[test]
    fn infer_type_matches_level_removal_owners() {
        // master04 §Level Removal: the elided container attributes.
        assert_eq!(infer_type("data", Some("events")), "HISTORY");
        assert_eq!(infer_type("events", None), "POINT_EVENT");
        assert_eq!(infer_type("items", None), "ELEMENT");
        assert_eq!(infer_type("activities", None), "ACTIVITY");
        assert_eq!(infer_type("state", Some("items")), "ITEM_TREE");
    }
}
