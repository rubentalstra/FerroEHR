//! Party / reference / participation / identifier codecs.
//!
//! Covers the ITS-REST `simplified_formats/master05-rm_mapping.adoc` tables for
//! `PARTY_SELF`, `PARTY_IDENTIFIED`, `PARTY_RELATED` (the `PARTY_PROXY` subtypes),
//! `OBJECT_REF`, `PARTICIPATION`, and `DV_IDENTIFIER`. These shapes are shared by
//! the ENTRY / EVENT_CONTEXT `_`-attribute families in [`super::structures`] and
//! by the `_identifier:i` / performer inlining rules.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use super::data_values;
use crate::sim::SimNode;

// ── DV_IDENTIFIER (master05 §DV_IDENTIFIER) ───────────────────────────────────

/// DV_IDENTIFIER → `|id`/`|issuer`/`|assigner`/`|type` attrs on `out`.
pub(super) fn emit_identifier(rm: &Value, out: &mut SimNode) {
    for (field, suffix) in [
        ("id", "id"),
        ("type", "type"),
        ("issuer", "issuer"),
        ("assigner", "assigner"),
    ] {
        if let Some(v) = rm.get(field).filter(|v| !v.is_null()) {
            out.attrs.insert(suffix.to_owned(), v.clone());
        }
    }
}

/// A DV_IDENTIFIER from `|id` (or, per master05 §DV_IDENTIFIER "for the input
/// |id might be left out", the bare value) plus the optional `|issuer` /
/// `|assigner` / `|type`.
pub(super) fn build_identifier(node: &SimNode) -> Option<Value> {
    let id = node
        .attrs
        .get("id")
        .or_else(|| node.bare())
        .cloned()
        .unwrap_or_else(|| json!(""));
    let mut o = Map::new();
    o.insert("_type".to_owned(), json!("DV_IDENTIFIER"));
    o.insert("id".to_owned(), id);
    // `issuer`/`assigner`/`type` are optional; only set when present so an empty
    // string is never fabricated (which would break the round-trip).
    for field in ["type", "issuer", "assigner"] {
        if let Some(v) = node.attrs.get(field) {
            o.insert(field.to_owned(), v.clone());
        }
    }
    Some(Value::Object(o))
}

// ── OBJECT_REF (master05 §OBJECT_REF) ─────────────────────────────────────────

/// OBJECT_REF → `|type`/`|id`/`|id_scheme`/`|namespace` attrs on `out`.
///
/// PORT NOTE: master05 §OBJECT_REF's table row is `|scheme` (→ `id.scheme`), but
/// every spec example carries `|id_scheme` (e.g. INSTRUCTION `_guideline_id`,
/// master05 §INSTRUCTION). The example blocks are the wire authority, so this
/// emits `|id_scheme`; [`build_object_ref`] accepts either.
pub(super) fn emit_object_ref(oref: &Value, out: &mut SimNode) {
    if let Some(v) = oref.get("type").filter(|v| !v.is_null()) {
        out.attrs.insert("type".to_owned(), v.clone());
    }
    if let Some(v) = oref.pointer("/id/value") {
        out.attrs.insert("id".to_owned(), v.clone());
    }
    if let Some(v) = oref.pointer("/id/scheme") {
        out.attrs.insert("id_scheme".to_owned(), v.clone());
    }
    if let Some(v) = oref.get("namespace").filter(|v| !v.is_null()) {
        out.attrs.insert("namespace".to_owned(), v.clone());
    }
}

/// An OBJECT_REF from `|id` (+ `|type`/`|id_scheme`(or `|scheme`)/`|namespace`).
/// `None` when no `|id` is present. master05 §OBJECT_REF: `type`, `id.value`,
/// `id.scheme`, `namespace` are all required, so absent parts default.
pub(super) fn build_object_ref(node: &SimNode) -> Option<Value> {
    let id = node.attrs.get("id").and_then(Value::as_str)?;
    let ty = node
        .attrs
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("ANY");
    let scheme = node
        .attrs
        .get("id_scheme")
        .or_else(|| node.attrs.get("scheme"))
        .and_then(Value::as_str)
        .unwrap_or("id_scheme");
    let ns = node
        .attrs
        .get("namespace")
        .and_then(Value::as_str)
        .unwrap_or("EHR");
    Some(json!({
        "_type": "OBJECT_REF",
        "namespace": ns,
        "type": ty,
        "id": {"_type": "GENERIC_ID", "value": id, "scheme": scheme},
    }))
}

// ── PARTY_PROXY family (master05 §§PARTY_SELF, PARTY_IDENTIFIED, PARTY_RELATED) ─

/// PARTY_PROXY (`PARTY_SELF`/`PARTY_IDENTIFIED`/`PARTY_RELATED`) → inlined
/// `|name`/`|id`/`|id_scheme`/`|id_namespace` attrs on `out`, plus `_identifier:i`
/// (PARTY_IDENTIFIED) and a `/relationship` DV_CODED_TEXT child (PARTY_RELATED).
pub(super) fn emit_party(p: &Value, out: &mut SimNode) {
    if let Some(name) = p.get("name").filter(|v| !v.is_null()) {
        out.attrs.insert("name".to_owned(), name.clone());
    }
    if let Some(id) = p.pointer("/external_ref/id/value") {
        out.attrs.insert("id".to_owned(), id.clone());
    }
    if let Some(s) = p.pointer("/external_ref/id/scheme") {
        out.attrs.insert("id_scheme".to_owned(), s.clone());
    }
    if let Some(ns) = p.pointer("/external_ref/namespace") {
        out.attrs.insert("id_namespace".to_owned(), ns.clone());
    }
    if let Some(ids) = p.get("identifiers").and_then(Value::as_array) {
        for (i, id) in ids.iter().enumerate() {
            emit_identifier(id, out.occurrence_mut("_identifier", Some(i as u32)));
        }
    }
    if let Some(rel) = p.get("relationship").filter(|v| !v.is_null()) {
        data_values::emit_leaf(
            rel,
            "DV_CODED_TEXT",
            None,
            out.occurrence_mut("relationship", None),
        );
    }
}

/// Build a PARTY_PROXY from a party sim node. `PARTY_SELF` when `|_type` is
/// `"PARTY_SELF"` (master05 §FEEDER_AUDIT_DETAILS note: "add /subject|_type:
/// PARTY_SELF"); `PARTY_RELATED` when a `/relationship` child is present
/// (master05 §PARTY_RELATED); otherwise `PARTY_IDENTIFIED` (master05
/// §PARTY_IDENTIFIED). `default_party_type` is the `PARTY_REF.type` used when an
/// `|id` is present but no explicit ref type (`PERSON`/`ORGANISATION`).
pub(super) fn build_party(node: &SimNode, default_party_type: &str) -> Value {
    let explicit_self = node
        .attrs
        .get("_type")
        .and_then(Value::as_str)
        .is_some_and(|t| t == "PARTY_SELF");
    let relationship = single(node, "relationship");
    let identifiers = build_party_identifiers(node);

    let mut p = Map::new();
    if explicit_self {
        p.insert("_type".to_owned(), json!("PARTY_SELF"));
        if let Some(ext) = external_ref(node, default_party_type) {
            p.insert("external_ref".to_owned(), ext);
        }
        return Value::Object(p);
    }

    let ty = if relationship.is_some() {
        "PARTY_RELATED"
    } else {
        "PARTY_IDENTIFIED"
    };
    p.insert("_type".to_owned(), json!(ty));
    if let Some(name) = node.attrs.get("name") {
        p.insert("name".to_owned(), name.clone());
    }
    if let Some(ext) = external_ref(node, default_party_type) {
        p.insert("external_ref".to_owned(), ext);
    }
    if !identifiers.is_empty() {
        p.insert("identifiers".to_owned(), Value::Array(identifiers));
    }
    if let Some(rel) = relationship.and_then(|r| data_values::build_leaf(r, "DV_CODED_TEXT", None))
    {
        p.insert("relationship".to_owned(), rel);
    }
    Value::Object(p)
}

/// The `PARTY_REF` external reference for a party sim node, when it carries an
/// `|id` (master05 §PARTY_IDENTIFIED: `|id_namespace` is required when `|id` is
/// set). A `|id_scheme` makes the id a `GENERIC_ID`; without one it is a
/// `HIER_OBJECT_ID` (so a scheme is never fabricated).
fn external_ref(node: &SimNode, default_party_type: &str) -> Option<Value> {
    let id = node.attrs.get("id").and_then(Value::as_str)?;
    let namespace = node
        .attrs
        .get("id_namespace")
        .and_then(Value::as_str)
        .unwrap_or("EHR");
    let id_obj = match node.attrs.get("id_scheme").and_then(Value::as_str) {
        Some(scheme) => json!({"_type": "GENERIC_ID", "value": id, "scheme": scheme}),
        None => json!({"_type": "HIER_OBJECT_ID", "value": id}),
    };
    Some(json!({
        "_type": "PARTY_REF",
        "id": id_obj,
        "namespace": namespace,
        "type": default_party_type,
    }))
}

/// The `_identifier:i` DV_IDENTIFIER array on a PARTY node (master05
/// §PARTY_IDENTIFIED `/_identifier:i`).
fn build_party_identifiers(node: &SimNode) -> Vec<Value> {
    node.children
        .get("_identifier")
        .map(|c| {
            c.occurrences
                .iter()
                .filter(|o| !o.is_empty())
                .filter_map(build_identifier)
                .collect()
        })
        .unwrap_or_default()
}

// ── PARTICIPATION (master05 §PARTICIPATION) ───────────────────────────────────

/// PARTICIPATION → `|function`/`|mode`, the inlined performer
/// (`|name`/`|id`/`|id_scheme`/`|id_namespace`, `|identifiers_*:i`), and a
/// PARTY_RELATED `/relationship` child.
pub(super) fn emit_participation(p: &Value, out: &mut SimNode) {
    if let Some(f) = p.pointer("/function/value") {
        out.attrs.insert("function".to_owned(), f.clone());
    }
    if let Some(m) = p.pointer("/mode/value") {
        out.attrs.insert("mode".to_owned(), m.clone());
    }
    let Some(performer) = p.get("performer").filter(|v| !v.is_null()) else {
        return;
    };
    if let Some(name) = performer.get("name").filter(|v| !v.is_null()) {
        out.attrs.insert("name".to_owned(), name.clone());
    }
    if let Some(id) = performer.pointer("/external_ref/id/value") {
        out.attrs.insert("id".to_owned(), id.clone());
    }
    if let Some(s) = performer.pointer("/external_ref/id/scheme") {
        out.attrs.insert("id_scheme".to_owned(), s.clone());
    }
    if let Some(ns) = performer.pointer("/external_ref/namespace") {
        out.attrs.insert("id_namespace".to_owned(), ns.clone());
    }
    // Performer identifiers are inlined as `|identifiers_<field>:i` suffixes
    // (master05 §PARTICIPATION), not the `_identifier:i` sub-path form.
    if let Some(ids) = performer.get("identifiers").and_then(Value::as_array) {
        for (i, id) in ids.iter().enumerate() {
            for (field, suffix) in [
                ("id", "identifiers_id"),
                ("issuer", "identifiers_issuer"),
                ("assigner", "identifiers_assigner"),
                ("type", "identifiers_type"),
            ] {
                if let Some(v) = id.get(field).filter(|v| !v.is_null()) {
                    out.attrs.insert(format!("{suffix}:{i}"), v.clone());
                }
            }
        }
    }
    if let Some(rel) = performer.get("relationship").filter(|v| !v.is_null()) {
        data_values::emit_leaf(
            rel,
            "DV_CODED_TEXT",
            None,
            out.occurrence_mut("relationship", None),
        );
    }
}

/// Build a PARTICIPATION: `function` (DV_TEXT), the inlined `performer`, and an
/// optional coded `mode` resolved against the openEHR `participation mode` group
/// (master05 §PARTICIPATION note "ValueSet openEHR participation mode group").
pub(super) fn build_participation(node: &SimNode) -> Value {
    let mut p = Map::new();
    p.insert("_type".to_owned(), json!("PARTICIPATION"));
    p.insert(
        "function".to_owned(),
        json!({
            "_type": "DV_TEXT",
            "value": node.attrs.get("function").and_then(Value::as_str).unwrap_or(""),
        }),
    );
    p.insert("performer".to_owned(), build_performer(node));
    if let Some(mode) = node.attrs.get("mode").and_then(Value::as_str) {
        p.insert(
            "mode".to_owned(),
            super::coded_from_group("participation_mode", mode),
        );
    }
    Value::Object(p)
}

/// The inlined performer PARTY of a PARTICIPATION (`PARTY_IDENTIFIED`, or
/// `PARTY_RELATED` when a `/relationship` child is present), including the
/// `|identifiers_*:i` performer identifiers.
fn build_performer(node: &SimNode) -> Value {
    let relationship = single(node, "relationship");
    let identifiers = build_inline_identifiers(node);
    let mut p = Map::new();
    let ty = if relationship.is_some() {
        "PARTY_RELATED"
    } else {
        "PARTY_IDENTIFIED"
    };
    p.insert("_type".to_owned(), json!(ty));
    if let Some(name) = node.attrs.get("name") {
        p.insert("name".to_owned(), name.clone());
    }
    if let Some(ext) = external_ref(node, "PERSON") {
        p.insert("external_ref".to_owned(), ext);
    }
    if !identifiers.is_empty() {
        p.insert("identifiers".to_owned(), Value::Array(identifiers));
    }
    if let Some(rel) = relationship.and_then(|r| data_values::build_leaf(r, "DV_CODED_TEXT", None))
    {
        p.insert("relationship".to_owned(), rel);
    }
    Value::Object(p)
}

/// Rebuild the performer `identifiers` from the inlined `|identifiers_<field>:i`
/// attrs, ordered by `i` (master05 §PARTICIPATION).
fn build_inline_identifiers(node: &SimNode) -> Vec<Value> {
    let mut by_index: BTreeMap<u32, Map<String, Value>> = BTreeMap::new();
    for (key, value) in &node.attrs {
        let Some(rest) = key.strip_prefix("identifiers_") else {
            continue;
        };
        let Some((field, idx)) = rest.split_once(':') else {
            continue;
        };
        let Ok(idx) = idx.parse::<u32>() else {
            continue;
        };
        if !matches!(field, "id" | "issuer" | "assigner" | "type") {
            continue;
        }
        by_index
            .entry(idx)
            .or_default()
            .insert(field.to_owned(), value.clone());
    }
    by_index
        .into_values()
        .map(|mut fields| {
            fields.insert("_type".to_owned(), json!("DV_IDENTIFIER"));
            fields.entry("id".to_owned()).or_insert_with(|| json!(""));
            Value::Object(fields)
        })
        .collect()
}

/// The single (index-0) occurrence of a sub-path child of a party node.
fn single<'a>(node: &'a SimNode, name: &str) -> Option<&'a SimNode> {
    node.children.get(name).and_then(|c| c.occurrences.first())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node_of(attrs: &[(&str, Value)]) -> SimNode {
        let mut n = SimNode::default();
        for (k, v) in attrs {
            n.attrs.insert((*k).to_owned(), v.clone());
        }
        n
    }

    // master05 §DV_IDENTIFIER: `|id` (+ optional issuer/assigner/type) round-trip,
    // and the bare-input tolerance.
    #[test]
    fn identifier_roundtrip_and_bare_input() {
        let rm = json!({"_type": "DV_IDENTIFIER", "id": "A123", "issuer": "Issuer", "type": "Prescription"});
        let mut out = SimNode::default();
        emit_identifier(&rm, &mut out);
        assert_eq!(out.attrs.get("id"), Some(&json!("A123")));
        assert_eq!(out.attrs.get("type"), Some(&json!("Prescription")));

        let bare = node_of(&[("", json!("A123"))]);
        let dv = build_identifier(&bare).unwrap();
        assert_eq!(dv["id"], json!("A123"));
    }

    // master05 §PARTY_IDENTIFIED: `|name`/`|id`/`|id_scheme`/`|id_namespace` +
    // `_identifier:i` round-trip.
    #[test]
    fn party_identified_roundtrip() {
        let rm = json!({
            "_type": "PARTY_IDENTIFIED", "name": "Silvia Blake",
            "external_ref": {"_type": "PARTY_REF",
                "namespace": "EHR.NETWORK", "type": "PERSON",
                "id": {"_type": "GENERIC_ID", "value": "1234-5678", "scheme": "UUID"}},
            "identifiers": [{"_type": "DV_IDENTIFIER", "id": "122", "issuer": "issuer"}]
        });
        let mut out = SimNode::default();
        emit_party(&rm, &mut out);
        assert_eq!(out.attrs.get("name"), Some(&json!("Silvia Blake")));
        assert_eq!(out.attrs.get("id"), Some(&json!("1234-5678")));
        assert_eq!(out.attrs.get("id_scheme"), Some(&json!("UUID")));

        let built = build_party(&out, "PERSON");
        assert_eq!(built["_type"], json!("PARTY_IDENTIFIED"));
        assert_eq!(built["name"], json!("Silvia Blake"));
        assert_eq!(built["external_ref"]["id"]["scheme"], json!("UUID"));
        assert_eq!(built["identifiers"][0]["id"], json!("122"));
    }

    // master05 §PARTY_SELF: `/subject|_type: PARTY_SELF` marks a PARTY_SELF.
    #[test]
    fn party_self_via_type_marker() {
        let node = node_of(&[
            ("_type", json!("PARTY_SELF")),
            ("id", json!("x")),
            ("id_namespace", json!("EHR")),
        ]);
        let built = build_party(&node, "PERSON");
        assert_eq!(built["_type"], json!("PARTY_SELF"));
        assert_eq!(built["external_ref"]["id"]["value"], json!("x"));
    }

    // master05 §PARTICIPATION: inlined performer + `|identifiers_*:i`; the mode is
    // coded against the openEHR participation-mode group.
    #[test]
    fn participation_inlined_performer_and_identifiers() {
        let node = node_of(&[
            ("function", json!("requester")),
            ("mode", json!("face-to-face communication")),
            ("name", json!("Dr. Marcus Johnson")),
            ("id", json!("199")),
            ("id_namespace", json!("HOSPITAL-NS")),
            ("identifiers_id:0", json!("122")),
            ("identifiers_issuer:0", json!("issuer")),
        ]);
        let p = build_participation(&node);
        assert_eq!(p["function"]["value"], json!("requester"));
        assert_eq!(p["performer"]["name"], json!("Dr. Marcus Johnson"));
        assert_eq!(p["performer"]["identifiers"][0]["id"], json!("122"));
        assert_eq!(p["mode"]["_type"], json!("DV_CODED_TEXT"));
        assert_eq!(p["mode"]["value"], json!("face-to-face communication"));
        // resolved to a real openEHR participation-mode code (not fabricated).
        assert!(p["mode"]["defining_code"]["code_string"].is_string());
    }

    // master05 §OBJECT_REF: `|id_scheme` accepted (spec example form).
    #[test]
    fn object_ref_builds_from_id_scheme() {
        let node = node_of(&[
            ("type", json!("GUIDELINE")),
            ("id", json!("3445")),
            ("id_scheme", json!("HOSPITAL-NS")),
            ("namespace", json!("HOSPITAL-NS")),
        ]);
        let o = build_object_ref(&node).unwrap();
        assert_eq!(o["type"], json!("GUIDELINE"));
        assert_eq!(o["id"]["value"], json!("3445"));
        assert_eq!(o["id"]["scheme"], json!("HOSPITAL-NS"));
        assert_eq!(o["namespace"], json!("HOSPITAL-NS"));
    }
}
