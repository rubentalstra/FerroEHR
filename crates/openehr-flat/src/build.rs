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

    // The template's hoisted-wrapper type narrowings (`ITEM_STRUCTURE`/`EVENT`
    // families the compactor folded away), keyed by absolute archetype path, so
    // the structural re-materialisation below stamps the *constrained* concrete
    // type (`ITEM_LIST`/`ITEM_SINGLE`/`INTERVAL_EVENT`) rather than the abstract
    // family default (AOM 1.4 type conformance; master04 §Level Removal — the
    // collapsed wrapper's type is not carried on the FLAT wire, so it is resolved
    // from the template here).
    let mut slot_types: Vec<(Vec<PathSegment>, String)> = Vec::new();
    collect_slot_types(&wt.tree, &mut slot_types);

    let mut comp = match build(&wt.tree, data_root, root_id, &slot_types)? {
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

/// Collect the whole template's hoisted-wrapper type narrowings as
/// `(parsed absolute archetype path, constrained RM type)` pairs, so the builder
/// can look a re-materialised structural node's constrained type up by its
/// absolute path (see [`build_composition`]).
fn collect_slot_types(node: &WebTemplateNode, out: &mut Vec<(Vec<PathSegment>, String)>) {
    for slot in &node.slots {
        out.push((rmpath::parse(&slot.path), slot.rm_type.clone()));
    }
    for child in &node.children {
        collect_slot_types(child, out);
    }
}

/// Whether two archetype-path segment slices name the same path (attribute +
/// archetype node id per step; the runtime `name` conjunct is ignored — the
/// wrapper identity is its node id).
fn segments_match(a: &[PathSegment], b: &[PathSegment]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            x.attribute == y.attribute
                && x.predicate.archetype_node_id == y.predicate.archetype_node_id
        })
}

/// Resolve, per relative segment of `child`, the template-constrained concrete RM
/// type of the structural wrapper at that absolute path (`None` when the template
/// records no narrowing there — the caller then falls back to
/// [`infer_type`]). Index-aligned with the relative segment list.
fn wrapper_types(
    child_aql: &str,
    rel_len: usize,
    slots: &[(Vec<PathSegment>, String)],
) -> Vec<Option<String>> {
    let child_abs = rmpath::parse(child_aql);
    if child_abs.len() < rel_len {
        return vec![None; rel_len];
    }
    let base = child_abs.len() - rel_len;
    (0..rel_len)
        .map(|i| {
            let abs = &child_abs[..=base + i];
            slots
                .iter()
                .find(|(segs, _)| segments_match(segs, abs))
                .map(|(_, ty)| ty.clone())
        })
        .collect()
}

/// Build the RM value for `node` from the simplified occurrence `sim`.
/// `path` is the printed simplified path (diagnostics); `slots` is the template's
/// hoisted-wrapper type map (see [`build_composition`]).
fn build(
    node: &WebTemplateNode,
    sim: &SimNode,
    path: &str,
    slots: &[(Vec<PathSegment>, String)],
) -> Result<Value, FlatError> {
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
    // `OBSERVATION/history_origin` (master05 §OBSERVATION `/history_origin`) is
    // an alias for `data.origin`; captured here and applied after the mandatory
    // HISTORY is materialised (see below).
    let mut history_origin: Option<String> = None;

    for child in &node.children {
        // Standard EVENT_CONTEXT fields come from ctx/ (master06); only the
        // archetyped other_context content is tree data.
        if node.rm_type == "EVENT_CONTEXT" && !child.aql_path.contains("other_context") {
            continue;
        }
        // Composition-level in-context attributes arrive via ctx resolution
        // (`ctx::resolve` also reads their path spellings). The `context`
        // child IS built when path keys address it — archetyped
        // `other_context` content and the `_`-attribute families
        // (`_health_care_facility`, `_participation:i`, `_end_time`,
        // `_location` — master05 §EVENT_CONTEXT) are tree data; the standard
        // leaf fields (start_time/setting) still come from ctx/ (the
        // EVENT_CONTEXT rule above) via `apply_composition_ctx`.
        // `category` is real tree data (master05 §COMPOSITION) and builds.
        if node.rm_type == "COMPOSITION"
            && matches!(child.id.as_str(), "language" | "territory" | "composer")
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
            let child_val = build(child, occ, &child_path, slots)?;
            place(&mut obj, &rel, child_val, child, occ, &child_path, slots)?;
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
        let known = node
            .children
            .iter()
            .any(|c| c.id == *seg || c.alt_json_id.as_deref() == Some(seg));
        let ctx_covered = node.rm_type == "COMPOSITION"
            && matches!(
                seg.as_str(),
                "language" | "territory" | "composer" | "category"
            );
        if known || ctx_covered {
            continue;
        }
        // A direct RM-attribute path the master05 per-type mapping tables
        // declare addressable on this node even when the OPT leaves it
        // unconstrained (so the compacted web-template carries no child for
        // it) — e.g. `ACTION/time`, `ACTION/ism_transition` (master05
        // §§ACTION, ISM_TRANSITION). Built here from the datum sub-tree.
        if place_direct_rm_path(
            &mut obj,
            &node.rm_type,
            seg,
            &child.occurrences,
            path,
            &mut history_origin,
        )? {
            continue;
        }
        // Otherwise the identifier matches no template child and no
        // spec-listed RM path (master04 §Validation: field identifiers match
        // WT metadata structure).
        return Err(FlatError::UnknownPath(format!("{path}/{seg}")));
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
        // `OBSERVATION/history_origin` sets `data.origin` (master05 §OBSERVATION
        // `/history_origin`), applied after the mandatory HISTORY is
        // materialised so the datum-supplied origin wins the structural default.
        if let Some(ts) = &history_origin
            && let Some(Value::Object(hist)) = m.get_mut("data")
        {
            hist.insert("origin".to_owned(), dv_date_time(ts));
        }
    }
    Ok(value)
}

/// A direct RM-attribute path (non-`_`, non-`|`) that the master05 per-type
/// mapping tables declare addressable on a node of `host` even when the OPT
/// leaves it unconstrained — so the compacted web-template carries no child
/// for it and the datum-driven walk must build it here
/// (`master05-rm_mapping.adoc`).
enum DirectPath {
    /// A single `DATA_VALUE` leaf attribute of the named RM type.
    Leaf {
        /// The RM attribute name the value is inserted under.
        attr: &'static str,
        /// The concrete leaf RM type to build.
        rm_type: &'static str,
    },
    /// `ACTION.ism_transition` — the ISM_TRANSITION object built from its
    /// `current_state`/`transition`/`careflow_step` + `_reason:i` sub-tree
    /// (master05 §ISM_TRANSITION).
    Ism,
    /// `ACTIVITY.action_archetype_id` — a plain-String RM field
    /// (master05 §ACTIVITY).
    ActionArchetypeId,
    /// `OBSERVATION.history_origin` — the alias for `data.origin`
    /// (master05 §OBSERVATION `/history_origin`).
    HistoryOrigin,
}

/// The direct RM-attribute path (if any) named by `seg` on a node of base RM
/// type `host`, per the master05 per-type mapping tables.
fn direct_rm_path(host: &str, seg: &str) -> Option<DirectPath> {
    use DirectPath::{ActionArchetypeId, HistoryOrigin, Ism, Leaf};
    Some(match (host, seg) {
        // master05 §§ACTION, POINT_EVENT, INTERVAL_EVENT: `/time` (DV_DATE_TIME)
        // is addressable on the ACTION and on every EVENT concrete type.
        ("ACTION" | "POINT_EVENT" | "INTERVAL_EVENT" | "EVENT", "time") => Leaf {
            attr: "time",
            rm_type: "DV_DATE_TIME",
        },
        // master05 §ACTION: `/ism_transition`.
        ("ACTION", "ism_transition") => Ism,
        // master05 §INSTRUCTION: `/narrative` (DV_TEXT).
        ("INSTRUCTION", "narrative") => Leaf {
            attr: "narrative",
            rm_type: "DV_TEXT",
        },
        // master05 §OBSERVATION: `/history_origin` → `history.origin`.
        ("OBSERVATION", "history_origin") => HistoryOrigin,
        // master05 §ACTIVITY: `/timing` (DV_PARSABLE), `/action_archetype_id`.
        ("ACTIVITY", "timing") => Leaf {
            attr: "timing",
            rm_type: "DV_PARSABLE",
        },
        ("ACTIVITY", "action_archetype_id") => ActionArchetypeId,
        // master05 §INTERVAL_EVENT: `/width` (DV_DURATION), `/math_function`.
        ("INTERVAL_EVENT", "width") => Leaf {
            attr: "width",
            rm_type: "DV_DURATION",
        },
        ("INTERVAL_EVENT", "math_function") => Leaf {
            attr: "math_function",
            rm_type: "DV_CODED_TEXT",
        },
        // master05 §EVENT_CONTEXT: `/start_time` (DV_DATE_TIME), `/setting`
        // (DV_CODED_TEXT). Normally supplied via the `ctx/` vocabulary
        // (master06); accepted as path keys here for the master05 path form.
        ("EVENT_CONTEXT", "start_time") => Leaf {
            attr: "start_time",
            rm_type: "DV_DATE_TIME",
        },
        ("EVENT_CONTEXT", "setting") => Leaf {
            attr: "setting",
            rm_type: "DV_CODED_TEXT",
        },
        _ => return None,
    })
}

/// Build and place a direct RM-attribute path onto `obj` (see [`direct_rm_path`]).
/// Returns `true` when `seg` was a recognised direct RM path (built or captured),
/// `false` when it is not — the caller then rejects it as an unknown path.
fn place_direct_rm_path(
    obj: &mut Map<String, Value>,
    host_rm_type: &str,
    seg: &str,
    occurrences: &[SimNode],
    path: &str,
    history_origin: &mut Option<String>,
) -> Result<bool, FlatError> {
    let Some(dp) = direct_rm_path(base_type(host_rm_type), seg) else {
        return Ok(false);
    };
    let occ = occurrences.iter().find(|o| !o.is_empty());
    let sub_path = format!("{path}/{seg}");
    match dp {
        DirectPath::Leaf { attr, rm_type } => {
            if let Some(node) = occ {
                obj.insert(
                    attr.to_owned(),
                    map::build_leaf(node, rm_type, None, &sub_path)?,
                );
            }
        }
        DirectPath::Ism => {
            if let Some(node) = occ {
                obj.insert(
                    "ism_transition".to_owned(),
                    build_ism_transition(node, &sub_path)?,
                );
            }
        }
        DirectPath::ActionArchetypeId => {
            if let Some(v) = occ.and_then(SimNode::bare) {
                obj.insert("action_archetype_id".to_owned(), v.clone());
            }
        }
        DirectPath::HistoryOrigin => {
            *history_origin = occ
                .and_then(SimNode::bare)
                .and_then(Value::as_str)
                .map(str::to_owned);
        }
    }
    Ok(true)
}

/// Build an `ACTION.ism_transition` (ISM_TRANSITION) from its simplified
/// sub-tree: `/current_state` (DV_CODED_TEXT, required), `/transition`,
/// `/careflow_step` (DV_CODED_TEXT), and the `/_reason:i` DV_TEXT list
/// (master05 §ISM_TRANSITION). `current_state` is 1..1 DV_CODED_TEXT, the other
/// two 0..1 DV_CODED_TEXT (RM ehr `ism_transition.adoc`).
fn build_ism_transition(node: &SimNode, path: &str) -> Result<Value, FlatError> {
    let mut o = Map::new();
    o.insert("_type".into(), json!("ISM_TRANSITION"));
    for (seg, group) in [
        ("current_state", Some("instruction_states")),
        ("transition", Some("instruction_transitions")),
        ("careflow_step", None),
    ] {
        if let Some(cs) = node.child(seg).filter(|n| !n.is_empty()) {
            o.insert(
                seg.to_owned(),
                build_ism_coded(cs, group, &format!("{path}/{seg}"))?,
            );
        }
    }
    for (seg, child) in &node.children {
        if seg.starts_with('_') {
            apply_rm_attr(&mut o, seg, &child.occurrences, "ISM_TRANSITION", path)?;
        } else if !matches!(
            seg.as_str(),
            "current_state" | "transition" | "careflow_step"
        ) {
            return Err(FlatError::UnknownPath(format!("{path}/{seg}")));
        }
    }
    Ok(Value::Object(o))
}

/// Build one ISM_TRANSITION state field as a DV_CODED_TEXT. With an explicit
/// `|value` the datum parts stand as given; with only a `|code` in the openEHR
/// terminology, the rubric is resolved from the state's openEHR group
/// (`group`) — the same idiom the `ctx/action_ism_transition_current_state`
/// shortcut uses ([`crate::ctx`]; master05 §DV_CODED_TEXT).
fn build_ism_coded(node: &SimNode, group: Option<&str>, path: &str) -> Result<Value, FlatError> {
    let has_value = node.attrs.contains_key("value");
    let terminology = node.attrs.get("terminology").and_then(Value::as_str);
    if !has_value
        && let Some(group) = group
        && let Some(code) = node.attrs.get("code").and_then(Value::as_str)
        && matches!(terminology, None | Some("openehr"))
    {
        return Ok(map::coded_from_group(group, code));
    }
    map::build_leaf(node, "DV_CODED_TEXT", None, path)
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
    slots: &[(Vec<PathSegment>, String)],
) -> Result<(), FlatError> {
    if rel.is_empty() {
        return Ok(());
    }
    let id_idx = rel.iter().rposition(|s| is_multiple(&s.attribute));
    // Per relative segment, the template-constrained concrete type of the
    // structural wrapper at that absolute path (empty ⇒ fall back to `infer_type`).
    let types = wrapper_types(&child.aql_path, rel.len(), slots);
    place_rec(
        parent,
        rel,
        0,
        id_idx,
        child_value,
        child,
        occ,
        path,
        &types,
        slots,
    )
}

// a recursive placement cursor, not an API: many arguments thread the cursor
// state, and the structural-case body runs just over the line limit.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn place_rec(
    cur: &mut Map<String, Value>,
    rel: &[PathSegment],
    i: usize,
    id_idx: Option<usize>,
    child_value: Value,
    child: &WebTemplateNode,
    occ: &SimNode,
    path: &str,
    types: &[Option<String>],
    slots: &[(Vec<PathSegment>, String)],
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
                types.get(i).and_then(Option::as_deref),
            );
            if let Value::Object(m) = &mut el {
                place(m, &rel[i + 1..], child_value, child, occ, path, slots)?;
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
            arr.push(new_struct(
                seg,
                rel.get(i + 1),
                None,
                None,
                types.get(i).and_then(Option::as_deref),
            ));
            arr.len() - 1
        };
        if let Some(Value::Object(m)) = arr.get_mut(idx) {
            place_rec(
                m,
                rel,
                i + 1,
                id_idx,
                child_value,
                child,
                occ,
                path,
                types,
                slots,
            )?;
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
            new_struct(
                seg,
                rel.get(i + 1),
                None,
                None,
                types.get(i).and_then(Option::as_deref),
            ),
        );
    }
    if let Some(Value::Object(m)) = cur.get_mut(&seg.attribute) {
        place_rec(
            m,
            rel,
            i + 1,
            id_idx,
            child_value,
            child,
            occ,
            path,
            types,
            slots,
        )?;
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

/// Concretise an abstract structural narrowing to the default concrete subtype
/// (`EVENT` → `POINT_EVENT`, `ITEM_STRUCTURE` → `ITEM_TREE`); a concrete type
/// passes through. A structural `_type` must never be abstract.
fn concretize_structural(rm_type: &str) -> &str {
    match rm_type {
        "EVENT" => "POINT_EVENT",
        "ITEM_STRUCTURE" => "ITEM_TREE",
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

/// Create a collapsed structural RM node for `seg`. Its `_type` is the
/// template-constrained concrete type (`constrained`, when the template narrowed
/// the hoisted wrapper — `ITEM_LIST`/`ITEM_SINGLE`/`INTERVAL_EVENT` — per AOM 1.4
/// type conformance), else inferred from the attribute it sits under and the next
/// step (`master04 §Level Removal`). Mandatory fields are filled per the concrete
/// type.
fn new_struct(
    seg: &PathSegment,
    next: Option<&PathSegment>,
    name: Option<&str>,
    coded_name: Option<&CodedName>,
    constrained: Option<&str>,
) -> Value {
    // An abstract narrowing (`EVENT`, `ITEM_STRUCTURE`) is concretised to the
    // default concrete subtype — so it never mis-stamps an abstract `_type` —
    // while a concrete narrowing (`INTERVAL_EVENT`, `ITEM_LIST`, `ITEM_SINGLE`)
    // passes through.
    let rm_type = constrained.map_or_else(
        || infer_type(&seg.attribute, next.map(|s| s.attribute.as_str())),
        concretize_structural,
    );
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
        // ITEM_SINGLE carries a single `item: ELEMENT` (1..1), ITEM_TABLE a
        // `rows: List<CLUSTER>`; the other ITEM_STRUCTUREs + CLUSTER carry
        // `items` (RM data_structures §ITEM_STRUCTURE). The mandatory member is
        // otherwise filled from content (or by `fill_structural_mandatory`).
        "ITEM_TREE" | "ITEM_LIST" | "CLUSTER" => {
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
        // ELEMENT under either the multi-valued `items` (ITEM_TREE/ITEM_LIST/
        // CLUSTER) or the single `item` (ITEM_SINGLE) attribute — both hold an
        // ELEMENT (RM data_structures §ITEM_STRUCTURE).
        ("items" | "item", _) => "ELEMENT",
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
    // The `at0001` "Any" placeholder keeps the synthesized filler a valid
    // LOCATABLE (`archetype_node_id` is mandatory; ADL 1.4 master05-cadl
    // §"Any" Constraints — an unconstrained attribute admits any RM-valid
    // value).
    json!({
        "_type": "ITEM_TREE",
        "archetype_node_id": "at0001",
        "name": {"_type": "DV_TEXT", "value": "Tree"},
        "items": []
    })
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
    // PARTY_PROXY subtypes are not LOCATABLE (RM common
    // `master04-generic_package.adoc` §PARTY_PROXY): they carry NO locatable
    // `name`/`archetype_node_id`/`archetype_details`. `PARTY_IDENTIFIED.name` is
    // a plain `String` (RM common
    // `UML/classes/org.openehr.rm.common.party_identified.adoc` §name — "Optional
    // human-readable name (in String form)"), never a `DV_TEXT`; stamping the
    // locatable coded/plain name object below would mis-type it. `Basic_validity`
    // (same class) requires at least one of `name`/`identifiers`/`external_ref`,
    // and `Name_valid` requires a present `name` to be non-empty — so synthesise a
    // plain-String `name` when the built party carries none of the three.
    if matches!(rm_type, "PARTY_SELF" | "PARTY_IDENTIFIED" | "PARTY_RELATED") {
        if rm_type != "PARTY_SELF"
            && !obj.contains_key("name")
            && !obj.contains_key("identifiers")
            && !obj.contains_key("external_ref")
        {
            obj.insert("name".to_owned(), json!("Example party"));
        }
        return;
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
    // An EVENT_CONTEXT is built only when the client expressed event-context
    // content (explicit ctx/ keys per master06, or archetyped other_context
    // tree data) — never fabricated from the category alone. NOTE: no openEHR
    // spec mandates synthesizing a context; the master06 time/setting
    // defaults apply when a context is being expressed, and round-trip
    // stability requires not inventing one (RM ehr master05 §"Persistent
    // Compositions may optionally have an Event context").
    if !explicit_event_context && !comp.contains_key("context") {
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
                // NOTE: ctx participations land on the EVENT_CONTEXT only.
                // master06 §Participation names ENTRY.other_participations as
                // a second landing site, but the entry-level path form
                // (`_other_participation:i`, master05 per-entry tables) is the
                // explicit spelling for entry participations — defaulting the
                // ctx list onto every entry would duplicate the data on
                // round-trip, so the path form takes precedence.
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
        "ITEM_TREE" | "ITEM_LIST" | "CLUSTER" => {
            obj.entry("items".to_owned()).or_insert_with(|| json!([]));
        }
        // ITEM_SINGLE carries one mandatory `item: ELEMENT` (1..1, RM
        // data_structures §ITEM_SINGLE); when the wrapper carried no content, a
        // null-flavoured ELEMENT is the minimal conforming filler
        // (`ELEMENT.Is_null`: value xor null_flavour — openEHR `253` = "unknown").
        "ITEM_SINGLE" => {
            obj.entry("item".to_owned()).or_insert_with(|| {
                json!({
                    "_type": "ELEMENT",
                    "archetype_node_id": "at0001",
                    "name": {"_type": "DV_TEXT", "value": "Element"},
                    "null_flavour": dv_coded_text("unknown", "openehr", "253"),
                })
            });
        }
        // ITEM_TABLE carries `rows: List<CLUSTER>` (RM data_structures §ITEM_TABLE).
        "ITEM_TABLE" => {
            obj.entry("rows".to_owned()).or_insert_with(|| json!([]));
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
