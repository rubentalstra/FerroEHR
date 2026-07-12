//! The `_`-prefixed optional-RM-attribute family, both directions.
//!
//! master02/master04 §"RM Attributes prefix": attributes defined by the openEHR
//! Reference Model that are optional and not surfaced by the template are
//! addressed with a leading underscore (`_attributeName`). This module is the
//! single place that emits (RM→FLAT) and rebuilds (FLAT→RM) that family, per the
//! per-class tables in master05:
//!
//! * `LOCATABLE` metadata — `_uid`, `_link:i` (LINK), `_feeder_audit`
//!   (FEEDER_AUDIT) — on COMPOSITION / the ENTRY types / CLUSTER / ELEMENT;
//! * `ELEMENT` — `_null_flavour` (DV_CODED_TEXT), `_null_reason` (DV_TEXT);
//! * the ENTRY types — `_guideline_id` / `_work_flow_id` (OBJECT_REF),
//!   `_other_participation:i` (PARTICIPATION), `_provider` (PARTY_IDENTIFIED);
//! * `PARTY_IDENTIFIED` — `_identifier:i` (DV_IDENTIFIER);
//! * the `DV_ORDERED` family — `_normal_range` (DV_INTERVAL<T>),
//!   `_other_reference_ranges:i` (REFERENCE_RANGE<T>);
//! * `DV_TEXT` / `DV_CODED_TEXT` — `_language` / `_encoding` (CODE_PHRASE),
//!   `_mapping:i` (TERM_MAPPING);
//! * the temporal `DV_*` — `_accuracy` (DV_DURATION);
//! * `DV_MULTIMEDIA` — `_thumbnail` (recursive DV_MULTIMEDIA), `_charset`,
//!   `_language`; `DV_PARSABLE` — `_charset`, `_language`.
//!
//! No openEHR spec governs the internal dispatch shape — our own design; the wire
//! shape (`<path>/_attr…`) is fixed by master02/master05.

use serde_json::{Map, Value, json};

use super::graph::code_phrase;
use super::mappers::{self, FlatMap};
use super::sub::{Entry, FlatView};

/// The `LOCATABLE` metadata carried on an `ELEMENT` (as opposed to the
/// `DV_ORDERED`/`DV_TEXT`-level attributes that live on the value itself). Used
/// to route a leaf's `_`-attribute entries to the ELEMENT wrapper vs the value.
pub(crate) const ELEMENT_LEVEL: &[&str] =
    &["uid", "link", "feeder_audit", "null_flavour", "null_reason"];

/// Whether a FLAT key segment id (e.g. `_uid`, `_link:0`) names an ELEMENT-level
/// (`LOCATABLE`) attribute rather than a value-level (`DV_*`) one.
pub(crate) fn is_element_level(seg_id: &str) -> bool {
    let name = seg_id.trim_start_matches('_');
    ELEMENT_LEVEL.contains(&name)
}

/// Whether `seg_id` names any `_`-prefixed RM attribute.
pub(crate) fn is_rm_attr(seg_id: &str) -> bool {
    seg_id.starts_with('_') && seg_id.len() > 1
}

// ── RM → FLAT ────────────────────────────────────────────────────────────────

#[allow(dead_code)] // RM→FLAT emit tree: implemented + unit-tested, not yet wired into `to_flat` (see G-1 read-side TODO)
const DV_ORDERED: &[&str] = &[
    "DV_QUANTITY",
    "DV_COUNT",
    "DV_PROPORTION",
    "DV_DURATION",
    "DV_DATE",
    "DV_DATE_TIME",
    "DV_TIME",
    "DV_ORDINAL",
    "DV_SCALE",
];

/// Emit the `_`-prefixed optional RM attributes present on the RM value `rm` at
/// `base`. Field presence + `_type` gate which family applies, so one entry point
/// serves every node kind (ELEMENT, the ENTRY types, COMPOSITION, CLUSTER, the
/// `DV_*` values, PARTY).
#[allow(dead_code)]
pub(crate) fn emit_rm_attrs(rm: &Value, base: &str, out: &mut FlatMap) {
    let ty = rm.get("_type").and_then(Value::as_str).unwrap_or("");
    if is_locatable(ty) {
        emit_uid(rm, base, out);
        emit_links(rm, base, out);
        emit_feeder_audit(rm, base, out);
    }
    match ty {
        "ELEMENT" => {
            if let Some(nf) = rm.get("null_flavour").filter(|v| !v.is_null()) {
                mappers::leaf_to_flat(
                    nf,
                    "DV_CODED_TEXT",
                    &subpath(base, "_null_flavour"),
                    None,
                    out,
                );
            }
            if let Some(nr) = rm.get("null_reason").filter(|v| !v.is_null()) {
                mappers::leaf_to_flat(nr, "DV_TEXT", &subpath(base, "_null_reason"), None, out);
            }
        }
        "OBSERVATION" | "EVALUATION" | "INSTRUCTION" | "ACTION" | "ADMIN_ENTRY" => {
            // TODO(w3e-formats): G-1 remaining — the INSTRUCTION/ACTION-specific
            // `_wf_definition` (DV_PARSABLE), `_expiry_time` (DV_DATE_TIME) and
            // `_instruction_details` (INSTRUCTION_DETAILS) attributes (master05
            // §§INSTRUCTION, ACTION) are not yet surfaced/parsed here.
            emit_object_ref(rm.get("guideline_id"), &subpath(base, "_guideline_id"), out);
            emit_object_ref(rm.get("workflow_id"), &subpath(base, "_work_flow_id"), out);
            if let Some(p) = rm.get("provider").filter(|v| !v.is_null()) {
                emit_party(p, &subpath(base, "_provider"), out);
            }
            if let Some(parts) = rm.get("other_participations").and_then(Value::as_array) {
                for (i, p) in parts.iter().enumerate() {
                    emit_participation(p, &format!("{base}/_other_participation:{i}"), out);
                }
            }
        }
        "PARTY_IDENTIFIED" | "PARTY_RELATED" => emit_identifiers(rm, base, out),
        "DV_TEXT" | "DV_PARAGRAPH" | "DV_CODED_TEXT" | "DV_STATE" => emit_text_meta(rm, base, out),
        "DV_PARSABLE" => {
            emit_code_phrase_sub(rm.get("charset"), &subpath(base, "_charset"), out);
            emit_code_phrase_sub(rm.get("language"), &subpath(base, "_language"), out);
        }
        "DV_MULTIMEDIA" => {
            if let Some(t) = rm.get("thumbnail").filter(|v| !v.is_null()) {
                mappers::leaf_to_flat(t, "DV_MULTIMEDIA", &subpath(base, "_thumbnail"), None, out);
            }
            emit_code_phrase_sub(rm.get("charset"), &subpath(base, "_charset"), out);
            emit_code_phrase_sub(rm.get("language"), &subpath(base, "_language"), out);
        }
        _ => {}
    }
    if DV_ORDERED.contains(&ty) {
        emit_reference_ranges(rm, base, ty, out);
        if matches!(ty, "DV_DATE" | "DV_DATE_TIME" | "DV_TIME") {
            // `accuracy` is a DV_DURATION carried as the bare `/_accuracy` sub-path.
            if let Some(acc) = rm.pointer("/accuracy/value") {
                out.insert(format!("{base}/_accuracy"), acc.clone());
            }
        }
    }
}

#[allow(dead_code)]
fn is_locatable(ty: &str) -> bool {
    matches!(
        ty,
        "COMPOSITION"
            | "SECTION"
            | "OBSERVATION"
            | "EVALUATION"
            | "INSTRUCTION"
            | "ACTION"
            | "ADMIN_ENTRY"
            | "GENERIC_ENTRY"
            | "CLUSTER"
            | "ELEMENT"
    )
}

#[allow(dead_code)]
fn emit_uid(rm: &Value, base: &str, out: &mut FlatMap) {
    if let Some(uid) = rm.pointer("/uid/value") {
        out.insert(format!("{base}/_uid"), uid.clone());
    }
}

#[allow(dead_code)]
fn emit_links(rm: &Value, base: &str, out: &mut FlatMap) {
    let Some(links) = rm.get("links").and_then(Value::as_array) else {
        return;
    };
    for (i, link) in links.iter().enumerate() {
        let b = format!("{base}/_link:{i}");
        if let Some(v) = link.pointer("/type/value") {
            out.insert(format!("{b}|type"), v.clone());
        }
        if let Some(v) = link.pointer("/meaning/value") {
            out.insert(format!("{b}|meaning"), v.clone());
        }
        if let Some(v) = link.pointer("/target/value") {
            out.insert(format!("{b}|target"), v.clone());
        }
    }
}

/// FEEDER_AUDIT (master05 §FEEDER_AUDIT). A pragmatic subset covering the
/// common `originating_system_audit` / `feeder_system_audit` (system_id,
/// version_id, time) plus `original_content` (DV_PARSABLE); the full audit-detail
/// PARTY sub-trees are TODO(w3e-formats): _feeder_audit deep PARTY_IDENTIFIED.
#[allow(dead_code)]
fn emit_feeder_audit(rm: &Value, base: &str, out: &mut FlatMap) {
    let Some(fa) = rm.get("feeder_audit").filter(|v| !v.is_null()) else {
        return;
    };
    let b = format!("{base}/_feeder_audit");
    emit_audit_details(
        fa.get("originating_system_audit"),
        &format!("{b}/originating_system_audit"),
        out,
    );
    emit_audit_details(
        fa.get("feeder_system_audit"),
        &format!("{b}/feeder_system_audit"),
        out,
    );
    if let Some(oc) = fa.get("original_content").filter(|v| !v.is_null()) {
        // DV_PARSABLE (`original_content`) or DV_MULTIMEDIA (`original_content`).
        let ty = oc
            .get("_type")
            .and_then(Value::as_str)
            .unwrap_or("DV_PARSABLE");
        let key = if ty == "DV_MULTIMEDIA" {
            "original_content_multimedia"
        } else {
            "original_content"
        };
        mappers::leaf_to_flat(oc, ty, &format!("{b}/{key}"), None, out);
    }
    for (arr_key, seg) in [
        ("originating_system_item_ids", "originating_system_item_id"),
        ("feeder_system_item_ids", "feeder_system_item_id"),
    ] {
        if let Some(ids) = fa.get(arr_key).and_then(Value::as_array) {
            for (i, id) in ids.iter().enumerate() {
                mappers::leaf_to_flat(id, "DV_IDENTIFIER", &format!("{b}/{seg}:{i}"), None, out);
            }
        }
    }
}

#[allow(dead_code)]
fn emit_audit_details(details: Option<&Value>, base: &str, out: &mut FlatMap) {
    let Some(d) = details.filter(|v| !v.is_null()) else {
        return;
    };
    if let Some(v) = d.get("system_id") {
        out.insert(format!("{base}|system_id"), v.clone());
    }
    // `version_id` is a plain String on FEEDER_AUDIT_DETAILS (RM common).
    if let Some(v) = d.get("version_id") {
        out.insert(format!("{base}|version_id"), v.clone());
    }
    if let Some(v) = d.pointer("/time/value") {
        out.insert(format!("{base}|time"), v.clone());
    }
    for party in ["location", "subject", "provider"] {
        if let Some(p) = d.get(party).filter(|v| !v.is_null()) {
            emit_party(p, &format!("{base}/{party}"), out);
        }
    }
}

/// PARTY_IDENTIFIED / PARTY_RELATED inlined (`|name`, `|id`, `|id_scheme`,
/// `|id_namespace`), plus `_identifier:i` and (PARTY_RELATED) `/relationship`.
#[allow(dead_code)]
fn emit_party(p: &Value, base: &str, out: &mut FlatMap) {
    if let Some(name) = p.get("name").filter(|v| !v.is_null()) {
        out.insert(format!("{base}|name"), name.clone());
    }
    if let Some(id) = p.pointer("/external_ref/id/value") {
        out.insert(format!("{base}|id"), id.clone());
    }
    if let Some(s) = p.pointer("/external_ref/id/scheme") {
        out.insert(format!("{base}|id_scheme"), s.clone());
    }
    if let Some(ns) = p.pointer("/external_ref/namespace") {
        out.insert(format!("{base}|id_namespace"), ns.clone());
    }
    emit_identifiers(p, base, out);
    if let Some(rel) = p.get("relationship").filter(|v| !v.is_null()) {
        mappers::leaf_to_flat(
            rel,
            "DV_CODED_TEXT",
            &format!("{base}/relationship"),
            None,
            out,
        );
    }
}

#[allow(dead_code)]
fn emit_identifiers(p: &Value, base: &str, out: &mut FlatMap) {
    let Some(ids) = p.get("identifiers").and_then(Value::as_array) else {
        return;
    };
    for (i, id) in ids.iter().enumerate() {
        mappers::leaf_to_flat(
            id,
            "DV_IDENTIFIER",
            &format!("{base}/_identifier:{i}"),
            None,
            out,
        );
    }
}

#[allow(dead_code)]
fn emit_participation(p: &Value, base: &str, out: &mut FlatMap) {
    if let Some(f) = p.pointer("/function/value") {
        out.insert(format!("{base}|function"), f.clone());
    }
    if let Some(m) = p.pointer("/mode/value") {
        out.insert(format!("{base}|mode"), m.clone());
    }
    if let Some(name) = p.pointer("/performer/name") {
        out.insert(format!("{base}|name"), name.clone());
    }
    if let Some(id) = p.pointer("/performer/external_ref/id/value") {
        out.insert(format!("{base}|id"), id.clone());
    }
    if let Some(s) = p.pointer("/performer/external_ref/id/scheme") {
        out.insert(format!("{base}|id_scheme"), s.clone());
    }
    if let Some(ns) = p.pointer("/performer/external_ref/namespace") {
        out.insert(format!("{base}|id_namespace"), ns.clone());
    }
}

/// OBJECT_REF (`|type`, `|id`, `|id_scheme`, `|namespace`) — master05 §OBJECT_REF.
#[allow(dead_code)]
fn emit_object_ref(oref: Option<&Value>, base: &str, out: &mut FlatMap) {
    let Some(o) = oref.filter(|v| !v.is_null()) else {
        return;
    };
    if let Some(v) = o.get("type") {
        out.insert(format!("{base}|type"), v.clone());
    }
    if let Some(v) = o.pointer("/id/value") {
        out.insert(format!("{base}|id"), v.clone());
    }
    if let Some(v) = o.pointer("/id/scheme") {
        out.insert(format!("{base}|id_scheme"), v.clone());
    }
    if let Some(v) = o.get("namespace") {
        out.insert(format!("{base}|namespace"), v.clone());
    }
}

/// `_language` / `_encoding` (CODE_PHRASE) + `_mapping:i` (TERM_MAPPING) on a
/// DV_TEXT-family value (master05 §§DV_TEXT, DV_CODED_TEXT, TERM_MAPPING).
#[allow(dead_code)]
fn emit_text_meta(rm: &Value, base: &str, out: &mut FlatMap) {
    emit_code_phrase_sub(rm.get("language"), &subpath(base, "_language"), out);
    emit_code_phrase_sub(rm.get("encoding"), &subpath(base, "_encoding"), out);
    if let Some(maps) = rm.get("mappings").and_then(Value::as_array) {
        for (i, m) in maps.iter().enumerate() {
            let b = format!("{base}/_mapping:{i}");
            if let Some(v) = m.get("match") {
                out.insert(format!("{b}|match"), v.clone());
            }
            emit_code_phrase_sub(m.get("target"), &format!("{b}/target"), out);
            if let Some(purpose) = m.get("purpose").filter(|v| !v.is_null()) {
                mappers::leaf_to_flat(purpose, "DV_CODED_TEXT", &format!("{b}/purpose"), None, out);
            }
        }
    }
}

#[allow(dead_code)]
fn emit_code_phrase_sub(cp: Option<&Value>, base: &str, out: &mut FlatMap) {
    let Some(cp) = cp.filter(|v| !v.is_null()) else {
        return;
    };
    if let Some(code) = cp.get("code_string") {
        out.insert(format!("{base}|code"), code.clone());
    }
    if let Some(term) = cp.pointer("/terminology_id/value") {
        out.insert(format!("{base}|terminology"), term.clone());
    }
    if let Some(pt) = cp.get("preferred_term").filter(|v| !v.is_null()) {
        out.insert(format!("{base}|preferred_term"), pt.clone());
    }
}

/// `_normal_range` (DV_INTERVAL<T>) + `_other_reference_ranges:i`
/// (REFERENCE_RANGE<T>), the endpoints emitted via the leaf mapper for `T`.
#[allow(dead_code)]
fn emit_reference_ranges(rm: &Value, base: &str, t: &str, out: &mut FlatMap) {
    if let Some(nr) = rm.get("normal_range").filter(|v| !v.is_null()) {
        emit_interval(nr, &subpath(base, "_normal_range"), t, out);
    }
    if let Some(ranges) = rm.get("other_reference_ranges").and_then(Value::as_array) {
        for (i, r) in ranges.iter().enumerate() {
            let b = format!("{base}/_other_reference_ranges:{i}");
            if let Some(range) = r.get("range") {
                emit_interval(range, &b, t, out);
            }
            if let Some(meaning) = r.get("meaning").filter(|v| !v.is_null()) {
                let mty = meaning
                    .get("_type")
                    .and_then(Value::as_str)
                    .unwrap_or("DV_TEXT");
                mappers::leaf_to_flat(meaning, mty, &format!("{b}/meaning"), None, out);
            }
        }
    }
}

#[allow(dead_code)]
fn emit_interval(iv: &Value, base: &str, t: &str, out: &mut FlatMap) {
    if let Some(lower) = iv.get("lower").filter(|v| !v.is_null()) {
        mappers::leaf_to_flat(lower, t, &format!("{base}/lower"), None, out);
    }
    if let Some(upper) = iv.get("upper").filter(|v| !v.is_null()) {
        mappers::leaf_to_flat(upper, t, &format!("{base}/upper"), None, out);
    }
    // Boundary flags — spec default is emit-only-if-non-default.
    for (field, suffix, default) in [
        ("lower_unbounded", "lower_unbounded", false),
        ("upper_unbounded", "upper_unbounded", false),
        ("lower_included", "lower_included", true),
        ("upper_included", "upper_included", true),
    ] {
        if let Some(b) = iv.get(field).and_then(Value::as_bool)
            && b != default
        {
            out.insert(format!("{base}|{suffix}"), json!(b));
        }
    }
}

#[allow(dead_code)]
fn subpath(base: &str, seg: &str) -> String {
    format!("{base}/{seg}")
}

// ── FLAT → RM ────────────────────────────────────────────────────────────────

/// Apply every `_`-prefixed RM-attribute entry in `entries` onto `target` (an RM
/// node object). `entries` are relative to that node (their first segment is the
/// `_attr[:i]`). `value_type` is the node's `_type` — used to type the endpoints
/// of `_normal_range` / `_other_reference_ranges` (`T`).
pub(crate) fn apply_rm_attrs(target: &mut Map<String, Value>, entries: &[Entry], value_type: &str) {
    // Group by the leading `_attr` name and (for `:i` families) index.
    let mut links: Vec<(usize, Vec<Entry>)> = Vec::new();
    let mut identifiers: Vec<(usize, Vec<Entry>)> = Vec::new();
    let mut participations: Vec<(usize, Vec<Entry>)> = Vec::new();
    let mut mappings: Vec<(usize, Vec<Entry>)> = Vec::new();
    let mut ranges: Vec<(usize, Vec<Entry>)> = Vec::new();
    let mut normal_range: Vec<Entry> = Vec::new();
    let mut named: std::collections::BTreeMap<String, Vec<Entry>> =
        std::collections::BTreeMap::new();

    for e in entries {
        let Some(first) = e.segs.first() else {
            continue;
        };
        if !is_rm_attr(&first.id) {
            continue;
        }
        let name = first.id.trim_start_matches('_').to_owned();
        let rest = strip_first(e);
        let idx = first.index.unwrap_or(0);
        match name.as_str() {
            "link" => push_indexed(&mut links, idx, rest),
            "identifier" => push_indexed(&mut identifiers, idx, rest),
            "other_participation" => push_indexed(&mut participations, idx, rest),
            "mapping" => push_indexed(&mut mappings, idx, rest),
            "other_reference_ranges" => push_indexed(&mut ranges, idx, rest),
            "normal_range" => normal_range.push(rest),
            other => named.entry(other.to_owned()).or_default().push(rest),
        }
    }

    // Single-valued named attributes.
    for (name, group) in &named {
        let view = FlatView::new(group);
        match name.as_str() {
            "uid" => {
                if let Some(v) = view.bare().and_then(Value::as_str) {
                    target.insert("uid".into(), uid_value(v));
                }
            }
            "null_flavour" => {
                if let Some(dv) = mappers::leaf_from_flat("DV_CODED_TEXT", &view) {
                    target.insert("null_flavour".into(), dv);
                }
            }
            "null_reason" => {
                if let Some(dv) = mappers::leaf_from_flat("DV_TEXT", &view) {
                    target.insert("null_reason".into(), dv);
                }
            }
            "guideline_id" => {
                if let Some(o) = object_ref_from(&view) {
                    target.insert("guideline_id".into(), o);
                }
            }
            "work_flow_id" => {
                if let Some(o) = object_ref_from(&view) {
                    target.insert("workflow_id".into(), o);
                }
            }
            "provider" => {
                target.insert("provider".into(), party_from(group));
            }
            "language" => {
                if let Some(cp) = code_phrase_from(&view) {
                    target.insert("language".into(), cp);
                }
            }
            "encoding" => {
                if let Some(cp) = code_phrase_from(&view) {
                    target.insert("encoding".into(), cp);
                }
            }
            "charset" => {
                if let Some(cp) = code_phrase_from(&view) {
                    target.insert("charset".into(), cp);
                }
            }
            "accuracy" => {
                if let Some(v) = view.bare() {
                    target.insert(
                        "accuracy".into(),
                        json!({"_type": "DV_DURATION", "value": v.clone()}),
                    );
                }
            }
            "thumbnail" => {
                if let Some(dv) = mappers::leaf_from_flat("DV_MULTIMEDIA", &view) {
                    target.insert("thumbnail".into(), dv);
                }
            }
            "feeder_audit" => {
                target.insert("feeder_audit".into(), feeder_audit_from(group));
            }
            _ => {}
        }
    }

    if !links.is_empty() {
        target.insert(
            "links".into(),
            Value::Array(build_indexed(links, link_from)),
        );
    }
    if !identifiers.is_empty() {
        target.insert(
            "identifiers".into(),
            Value::Array(build_indexed(identifiers, |g| {
                mappers::leaf_from_flat("DV_IDENTIFIER", &FlatView::new(g))
                    .unwrap_or_else(|| json!({"_type": "DV_IDENTIFIER"}))
            })),
        );
    }
    if !participations.is_empty() {
        target.insert(
            "other_participations".into(),
            Value::Array(build_indexed(participations, participation_from)),
        );
    }
    if !mappings.is_empty() {
        target.insert(
            "mappings".into(),
            Value::Array(build_indexed(mappings, mapping_from)),
        );
    }
    if !normal_range.is_empty() {
        target.insert(
            "normal_range".into(),
            interval_from(&normal_range, value_type),
        );
    }
    if !ranges.is_empty() {
        let vt = value_type.to_owned();
        target.insert(
            "other_reference_ranges".into(),
            Value::Array(build_indexed(ranges, move |g| reference_range_from(g, &vt))),
        );
    }
}

fn push_indexed(v: &mut Vec<(usize, Vec<Entry>)>, idx: usize, e: Entry) {
    match v.iter_mut().find(|(i, _)| *i == idx) {
        Some((_, g)) => g.push(e),
        None => v.push((idx, vec![e])),
    }
}

fn build_indexed<F>(mut groups: Vec<(usize, Vec<Entry>)>, f: F) -> Vec<Value>
where
    F: Fn(&[Entry]) -> Value,
{
    groups.sort_by_key(|(i, _)| *i);
    groups.iter().map(|(_, g)| f(g.as_slice())).collect()
}

/// Strip the leading segment of `e`, returning the remainder (segs after the
/// first, same suffix/value).
fn strip_first(e: &Entry) -> Entry {
    Entry {
        segs: e.segs.iter().skip(1).cloned().collect(),
        suffix: e.suffix.clone(),
        value: e.value.clone(),
    }
}

/// A view over the sub-entries addressed to `seg` (a path segment id, no leading
/// underscore), with that segment stripped.
fn scoped(entries: &[Entry], seg: &str) -> Vec<Entry> {
    entries
        .iter()
        .filter(|e| e.segs.first().is_some_and(|s| s.id == seg))
        .map(strip_first)
        .collect()
}

fn uid_value(v: &str) -> Value {
    // A `::`-bearing id is an OBJECT_VERSION_ID; otherwise a HIER_OBJECT_ID
    // (RM support identifiers — both are valid `LOCATABLE.uid` UID_BASED_IDs).
    let ty = if v.contains("::") {
        "OBJECT_VERSION_ID"
    } else {
        "HIER_OBJECT_ID"
    };
    json!({"_type": ty, "value": v})
}

fn link_from(group: &[Entry]) -> Value {
    let view = FlatView::new(group);
    let text = |s: &str| view.suffix(s).and_then(Value::as_str).unwrap_or("");
    json!({
        "_type": "LINK",
        "type": {"_type": "DV_TEXT", "value": text("type")},
        "meaning": {"_type": "DV_TEXT", "value": text("meaning")},
        "target": {"_type": "DV_EHR_URI", "value": text("target")},
    })
}

fn mapping_from(group: &[Entry]) -> Value {
    let view = FlatView::new(group);
    let mut o = Map::new();
    o.insert("_type".into(), json!("TERM_MAPPING"));
    o.insert(
        "match".into(),
        json!(view.suffix("match").and_then(Value::as_str).unwrap_or("=")),
    );
    if let Some(target) = code_phrase_from(&FlatView::new(&scoped(group, "target"))) {
        o.insert("target".into(), target);
    }
    let purpose = scoped(group, "purpose");
    if !purpose.is_empty()
        && let Some(dv) = mappers::leaf_from_flat("DV_CODED_TEXT", &FlatView::new(&purpose))
    {
        o.insert("purpose".into(), dv);
    }
    Value::Object(o)
}

fn code_phrase_from(view: &FlatView) -> Option<Value> {
    let code = view.suffix("code").and_then(Value::as_str)?;
    let term = view
        .suffix("terminology")
        .and_then(Value::as_str)
        .unwrap_or("local");
    let mut cp = code_phrase(term, code);
    if let (Value::Object(m), Some(pt)) = (&mut cp, view.suffix("preferred_term")) {
        m.insert("preferred_term".into(), pt.clone());
    }
    Some(cp)
}

fn object_ref_from(view: &FlatView) -> Option<Value> {
    let id = view.suffix("id").and_then(Value::as_str)?;
    let ty = view.suffix("type").and_then(Value::as_str).unwrap_or("ANY");
    let scheme = view
        .suffix("id_scheme")
        .and_then(Value::as_str)
        .unwrap_or("id_scheme");
    let ns = view
        .suffix("namespace")
        .and_then(Value::as_str)
        .unwrap_or("EHR");
    Some(json!({
        "_type": "OBJECT_REF",
        "namespace": ns,
        "type": ty,
        "id": {"_type": "GENERIC_ID", "value": id, "scheme": scheme},
    }))
}

/// A PARTY_IDENTIFIED (with optional `_identifier:i`) or PARTY_RELATED (when a
/// `/relationship` sub-path is present) from inlined `|name`/`|id`/… entries.
fn party_from(group: &[Entry]) -> Value {
    let view = FlatView::new(group);
    let relationship = scoped(group, "relationship");
    let mut p = Map::new();
    let ty = if relationship.is_empty() {
        "PARTY_IDENTIFIED"
    } else {
        "PARTY_RELATED"
    };
    p.insert("_type".into(), json!(ty));
    if let Some(name) = view.suffix("name") {
        p.insert("name".into(), name.clone());
    }
    if let Some(id) = view.suffix("id").and_then(Value::as_str) {
        p.insert(
            "external_ref".into(),
            json!({
                "_type": "PARTY_REF",
                "namespace": view.suffix("id_namespace").and_then(Value::as_str).unwrap_or("EHR"),
                "type": "PERSON",
                "id": {"_type": "GENERIC_ID", "value": id, "scheme": view.suffix("id_scheme").and_then(Value::as_str).unwrap_or("id_scheme")},
            }),
        );
    }
    // `_identifier:i` inside the party.
    let mut ids: Vec<(usize, Vec<Entry>)> = Vec::new();
    for e in group {
        if let Some(first) = e.segs.first()
            && first.id.trim_start_matches('_') == "identifier"
        {
            push_indexed(&mut ids, first.index.unwrap_or(0), strip_first(e));
        }
    }
    if !ids.is_empty() {
        p.insert(
            "identifiers".into(),
            Value::Array(build_indexed(ids, |g| {
                mappers::leaf_from_flat("DV_IDENTIFIER", &FlatView::new(g))
                    .unwrap_or_else(|| json!({"_type": "DV_IDENTIFIER"}))
            })),
        );
    }
    if !relationship.is_empty()
        && let Some(dv) = mappers::leaf_from_flat("DV_CODED_TEXT", &FlatView::new(&relationship))
    {
        p.insert("relationship".into(), dv);
    }
    Value::Object(p)
}

fn participation_from(group: &[Entry]) -> Value {
    let view = FlatView::new(group);
    let mut p = Map::new();
    p.insert("_type".into(), json!("PARTICIPATION"));
    p.insert(
        "function".into(),
        json!({"_type": "DV_TEXT", "value": view.suffix("function").and_then(Value::as_str).unwrap_or("")}),
    );
    // The performer is inlined at the participation level (master05 §PARTICIPATION).
    p.insert("performer".into(), party_from(group));
    if let Some(m) = view.suffix("mode") {
        p.insert(
            "mode".into(),
            json!({"_type": "DV_CODED_TEXT", "value": m, "defining_code": code_phrase("openehr", "193")}),
        );
    }
    Value::Object(p)
}

/// FEEDER_AUDIT (subset mirroring [`emit_feeder_audit`]).
fn feeder_audit_from(group: &[Entry]) -> Value {
    let mut fa = Map::new();
    fa.insert("_type".into(), json!("FEEDER_AUDIT"));
    let osa = scoped(group, "originating_system_audit");
    fa.insert("originating_system_audit".into(), audit_details_from(&osa));
    let fsa = scoped(group, "feeder_system_audit");
    if !fsa.is_empty() {
        fa.insert("feeder_system_audit".into(), audit_details_from(&fsa));
    }
    let oc = scoped(group, "original_content");
    if !oc.is_empty()
        && let Some(dv) = mappers::leaf_from_flat("DV_PARSABLE", &FlatView::new(&oc))
    {
        fa.insert("original_content".into(), dv);
    }
    Value::Object(fa)
}

fn audit_details_from(group: &[Entry]) -> Value {
    let view = FlatView::new(group);
    let mut d = Map::new();
    d.insert("_type".into(), json!("FEEDER_AUDIT_DETAILS"));
    d.insert(
        "system_id".into(),
        json!(
            view.suffix("system_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ),
    );
    if let Some(v) = view.suffix("version_id") {
        // Plain String on FEEDER_AUDIT_DETAILS (RM common).
        d.insert("version_id".into(), v.clone());
    }
    if let Some(v) = view.suffix("time") {
        d.insert(
            "time".into(),
            json!({"_type": "DV_DATE_TIME", "value": v.clone()}),
        );
    }
    for party in ["location", "subject", "provider"] {
        let scoped_party = scoped(group, party);
        if !scoped_party.is_empty() {
            d.insert(party.into(), party_from(&scoped_party));
        }
    }
    Value::Object(d)
}

/// DV_INTERVAL<T> from `/lower`, `/upper` and the `|*_unbounded`/`|*_included`
/// boundary flags (master05 §DV_INTERVAL).
fn interval_from(group: &[Entry], t: &str) -> Value {
    let view = FlatView::new(group);
    let mut iv = Map::new();
    iv.insert("_type".into(), json!("DV_INTERVAL"));
    let lower = scoped(group, "lower");
    if !lower.is_empty()
        && let Some(dv) = mappers::leaf_from_flat(t, &FlatView::new(&lower))
    {
        iv.insert("lower".into(), dv);
    }
    let upper = scoped(group, "upper");
    if !upper.is_empty()
        && let Some(dv) = mappers::leaf_from_flat(t, &FlatView::new(&upper))
    {
        iv.insert("upper".into(), dv);
    }
    let flag = |s: &str, default: bool| view.suffix(s).and_then(Value::as_bool).unwrap_or(default);
    iv.insert(
        "lower_unbounded".into(),
        json!(flag("lower_unbounded", false)),
    );
    iv.insert(
        "upper_unbounded".into(),
        json!(flag("upper_unbounded", false)),
    );
    iv.insert("lower_included".into(), json!(flag("lower_included", true)));
    iv.insert("upper_included".into(), json!(flag("upper_included", true)));
    Value::Object(iv)
}

fn reference_range_from(group: &[Entry], t: &str) -> Value {
    let meaning = scoped(group, "meaning");
    let meaning_val = if meaning.is_empty() {
        json!({"_type": "DV_TEXT", "value": ""})
    } else {
        // meaning may be DV_TEXT (bare) or DV_CODED_TEXT (coded suffixes).
        let mty = if FlatView::new(&meaning).suffix("code").is_some() {
            "DV_CODED_TEXT"
        } else {
            "DV_TEXT"
        };
        mappers::leaf_from_flat(mty, &FlatView::new(&meaning))
            .unwrap_or_else(|| json!({"_type": "DV_TEXT", "value": ""}))
    };
    json!({
        "_type": "REFERENCE_RANGE",
        "meaning": meaning_val,
        "range": interval_from(group, t),
    })
}

#[cfg(test)]
mod tests {
    use super::super::sub::parse_key;
    use super::*;

    fn entries(keys: &[(&str, Value)]) -> Vec<Entry> {
        keys.iter()
            .map(|(k, v)| {
                let (segs, suffix) = parse_key(k);
                Entry {
                    segs,
                    suffix,
                    value: v.clone(),
                }
            })
            .collect()
    }

    #[test]
    fn uid_link_roundtrip() {
        let rm = json!({
            "_type": "OBSERVATION",
            "uid": {"_type": "HIER_OBJECT_ID", "value": "9fcc1c70"},
            "links": [{"_type": "LINK",
                "type": {"_type": "DV_TEXT", "value": "problem"},
                "meaning": {"_type": "DV_TEXT", "value": "related"},
                "target": {"_type": "DV_EHR_URI", "value": "ehr://x"}}]
        });
        let mut out = FlatMap::new();
        emit_rm_attrs(&rm, "obs", &mut out);
        assert_eq!(out.get("obs/_uid"), Some(&json!("9fcc1c70")));
        assert_eq!(out.get("obs/_link:0|type"), Some(&json!("problem")));
        assert_eq!(out.get("obs/_link:0|target"), Some(&json!("ehr://x")));

        // reverse
        let es = entries(&[
            ("_uid", json!("9fcc1c70")),
            ("_link:0|type", json!("problem")),
            ("_link:0|meaning", json!("related")),
            ("_link:0|target", json!("ehr://x")),
        ]);
        let mut target = Map::new();
        apply_rm_attrs(&mut target, &es, "OBSERVATION");
        assert_eq!(target["uid"]["_type"], json!("HIER_OBJECT_ID"));
        assert_eq!(target["links"][0]["type"]["value"], json!("problem"));
        assert_eq!(target["links"][0]["_type"], json!("LINK"));
    }

    #[test]
    fn null_flavour_element() {
        let es = entries(&[
            ("_null_flavour|code", json!("253")),
            ("_null_flavour|value", json!("unknown")),
            ("_null_flavour|terminology", json!("openehr")),
            ("_null_reason", json!("sample reason")),
        ]);
        let mut target = Map::new();
        apply_rm_attrs(&mut target, &es, "");
        assert_eq!(
            target["null_flavour"]["defining_code"]["code_string"],
            json!("253")
        );
        assert_eq!(target["null_reason"]["value"], json!("sample reason"));
    }

    #[test]
    fn quantity_normal_range_roundtrip() {
        let rm = json!({
            "_type": "DV_QUANTITY",
            "magnitude": 65.9, "units": "unit",
            "normal_range": {"_type": "DV_INTERVAL",
                "lower": {"_type": "DV_QUANTITY", "magnitude": 20.5, "units": "unit"},
                "upper": {"_type": "DV_QUANTITY", "magnitude": 66.6, "units": "unit"},
                "lower_included": true, "upper_included": true,
                "lower_unbounded": false, "upper_unbounded": false}
        });
        let mut out = FlatMap::new();
        emit_rm_attrs(&rm, "q", &mut out);
        assert_eq!(
            out.get("q/_normal_range/lower|magnitude"),
            Some(&json!(20.5))
        );
        assert_eq!(out.get("q/_normal_range/upper|unit"), Some(&json!("unit")));

        let es = entries(&[
            ("_normal_range/lower|magnitude", json!(20.5)),
            ("_normal_range/lower|unit", json!("unit")),
            ("_normal_range/upper|magnitude", json!(66.6)),
            ("_normal_range/upper|unit", json!("unit")),
        ]);
        let mut target = Map::new();
        apply_rm_attrs(&mut target, &es, "DV_QUANTITY");
        assert_eq!(target["normal_range"]["lower"]["magnitude"], json!(20.5));
        assert_eq!(target["normal_range"]["upper"]["units"], json!("unit"));
    }

    #[test]
    fn text_language_mapping_roundtrip() {
        let es = entries(&[
            ("_language|code", json!("en")),
            ("_language|terminology", json!("ISO_639-1")),
            ("_mapping:0|match", json!("=")),
            ("_mapping:0/target|code", json!("21794005")),
            ("_mapping:0/target|terminology", json!("SNOMED-CT")),
        ]);
        let mut target = Map::new();
        apply_rm_attrs(&mut target, &es, "DV_TEXT");
        assert_eq!(target["language"]["code_string"], json!("en"));
        assert_eq!(target["mappings"][0]["match"], json!("="));
        assert_eq!(
            target["mappings"][0]["target"]["code_string"],
            json!("21794005")
        );
    }
}
