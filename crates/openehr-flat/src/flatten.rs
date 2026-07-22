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
    // Composition-level in-context shortcuts (master06). The `context`
    // child is ALWAYS walked: its standard EVENT_CONTEXT leaf fields
    // surface as ctx/ scalars (the inner EVENT_CONTEXT rule below keeps
    // them out of the tree), while its archetyped `other_context` content
    // and the lossless `_`-attribute families (`_health_care_facility`,
    // `_participation:i` — master05 §EVENT_CONTEXT) emit as path keys.
    if node.rm_type == "COMPOSITION"
        && matches!(child.id.as_str(), "language" | "territory" | "composer")
    {
        return true;
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
        // A stored value whose `_type` does not conform to the template's
        // declared leaf type cannot be decomposed faithfully into that
        // type's suffixes — embed it verbatim as `|raw` instead
        // (master04 §Raw canonical JSON: pre-existing canonical data /
        // shapes the simplified form cannot express). Lossless both ways.
        if !leaf_type_conforms(rm, &node.rm_type) {
            out.attrs.insert("raw".to_owned(), rm.clone());
            return;
        }
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
        // the alternative whose rm type matches this value's `_type`. The
        // filter must apply to EVERY member of a choice group — the first
        // alternative carries no `alt_json_id`, so sharing an aqlPath with a
        // sibling is the group marker.
        let in_choice = child.alt_json_id.is_some()
            || node
                .children
                .iter()
                .any(|c| c.id != child.id && c.aql_path == child.aql_path);
        let matches: Vec<&Value> = if in_choice {
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
    emit_direct_rm_paths(node, rm, out);
}

/// Whether the template-child walk already emitted a child that realizes RM
/// attribute `attr` on `node` — i.e. some template child whose relative RM path
/// (master04 §Level Removal) begins with `attr` produced an entry in `out`.
///
/// This is the correct suppression signal for the direct-RM-path fallback: the
/// web template may constrain an attribute under a node-id-specialized child
/// whose id is NOT the attribute name (e.g. the CKM ACTION templates model
/// `ism_transition` as the careflow-state children `ism_transition[at0109,…]`,
/// id `intended`/`completed`/…), so an id-string check on the attribute name
/// misses it. Conversely, when the template models the attribute but no such
/// child matched this instance (e.g. the RM value carries no careflow node id),
/// the walk emits nothing — and the direct path MUST still emit the generic
/// spelling so the datum is not lost. Hence: suppress iff a realizing child was
/// actually emitted; the WT-child realization then wins entirely (it carries
/// the constrained node identity/name the direct path cannot reconstruct).
fn attr_emitted(node: &WebTemplateNode, out: &SimNode, attr: &str) -> bool {
    node.children.iter().any(|c| {
        out.children.contains_key(&c.id)
            && rmpath::relative(&node.aql_path, &c.aql_path)
                .first()
                .is_some_and(|s| s.attribute == attr)
    })
}

/// Emit the direct RM-attribute paths the master05 per-type mapping tables
/// declare on this node but that the OPT left unconstrained (so the compacted
/// web-template carries no child for them) — the mirror of
/// [`crate::build`]'s direct-path handling, keeping the RM⇄FLAT round-trip
/// lossless. Each attribute is emitted ONLY when the template-child walk did
/// not already realize that RM attribute ([`attr_emitted`]); whenever a
/// WT-child realized it, that realization wins entirely.
///
/// `master05-rm_mapping.adoc` §§ACTION (`/time`, `/ism_transition`),
/// INSTRUCTION (`/narrative`), OBSERVATION (`/history_origin`), ACTIVITY
/// (`/timing`, `/action_archetype_id`), POINT_EVENT/INTERVAL_EVENT (`/time`),
/// INTERVAL_EVENT (`/width`, `/math_function`). EVENT_CONTEXT `start_time`/
/// `setting` are NOT emitted here — they surface through the `ctx/` vocabulary
/// (master06; [`crate::ctx`]) to avoid a duplicate encoding of the same datum.
fn emit_direct_rm_paths(node: &WebTemplateNode, rm: &Value, out: &mut SimNode) {
    let mut leaf = |name: &str, rm_type: &str, value: Option<&Value>| {
        if attr_emitted(node, out, name) {
            return;
        }
        if let Some(v) = value.filter(|v| !v.is_null()) {
            map::emit_leaf(v, rm_type, None, out.occurrence_mut(name, None));
        }
    };
    match base_type(&node.rm_type) {
        "ACTION" => {
            leaf("time", "DV_DATE_TIME", rm.get("time"));
            if !attr_emitted(node, out, "ism_transition")
                && let Some(ism) = rm.get("ism_transition").filter(|v| !v.is_null())
            {
                emit_ism_transition(ism, out.occurrence_mut("ism_transition", None));
            }
        }
        "INSTRUCTION" => leaf("narrative", "DV_TEXT", rm.get("narrative")),
        "OBSERVATION" => {
            // `history_origin` maps to the nested `data.origin` (master05
            // §OBSERVATION); the HISTORY is compacted away, so `origin` is
            // never a template leaf child — emit it unless the walk already did.
            if !out.children.contains_key("history_origin")
                && let Some(origin) = rm.pointer("/data/origin/value")
            {
                out.occurrence_mut("history_origin", None)
                    .attrs
                    .insert(String::new(), origin.clone());
            }
        }
        "ACTIVITY" => {
            leaf("timing", "DV_PARSABLE", rm.get("timing"));
            // `action_archetype_id` is the match-all `/.*/` when unset
            // (master05 §ACTIVITY: "Will be set to /.*/ if not set explicit.");
            // that default is re-synthesised on build, so emitting it would
            // desync the round-trip — emit only an explicit non-default value.
            if !attr_emitted(node, out, "action_archetype_id")
                && let Some(aid) = rm.get("action_archetype_id").and_then(Value::as_str)
                && aid != "/.*/"
            {
                out.occurrence_mut("action_archetype_id", None)
                    .attrs
                    .insert(String::new(), Value::String(aid.to_owned()));
            }
        }
        "POINT_EVENT" | "EVENT" => leaf("time", "DV_DATE_TIME", rm.get("time")),
        "INTERVAL_EVENT" => {
            leaf("time", "DV_DATE_TIME", rm.get("time"));
            leaf("width", "DV_DURATION", rm.get("width"));
            leaf("math_function", "DV_CODED_TEXT", rm.get("math_function"));
        }
        _ => {}
    }
}

/// Emit an ISM_TRANSITION as its master05 sub-paths — `/current_state`,
/// `/transition`, `/careflow_step` (DV_CODED_TEXT) and the `/_reason:i`
/// DV_TEXT list (master05 §ISM_TRANSITION). The mirror of
/// [`crate::build`]'s `build_ism_transition`.
fn emit_ism_transition(ism: &Value, out: &mut SimNode) {
    for attr in ["current_state", "transition", "careflow_step"] {
        if let Some(cs) = ism.get(attr).filter(|v| !v.is_null()) {
            map::emit_leaf(cs, "DV_CODED_TEXT", None, out.occurrence_mut(attr, None));
        }
    }
    map::emit_rm_attrs(ism, "ISM_TRANSITION", out);
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

/// Whether an RM leaf value's `_type` conforms to the template's declared
/// leaf type, including the specialisations the simplified mappings handle
/// natively: DV_TEXT ⇄ DV_CODED_TEXT (the `|other` open-value-set pair,
/// master04 §Open Value-Sets) and the PARTY_PROXY family (master05
/// §PARTY_PROXY dispatches on the concrete party type). Anything else is a
/// non-conforming stored value and is embedded as `|raw`.
fn leaf_type_conforms(rm: &Value, declared: &str) -> bool {
    let Some(actual) = rm.get("_type").and_then(Value::as_str) else {
        return true;
    };
    let declared = base_type(declared);
    if actual == declared {
        return true;
    }
    matches!(
        (declared, actual),
        ("DV_TEXT", "DV_CODED_TEXT")
            | ("DV_CODED_TEXT", "DV_TEXT")
            | (
                "PARTY_PROXY",
                "PARTY_SELF" | "PARTY_IDENTIFIED" | "PARTY_RELATED"
            )
    )
}
