//! `LINK`, `FEEDER_AUDIT`, `FEEDER_AUDIT_DETAILS`, `ISM_TRANSITION`,
//! `INSTRUCTION_DETAILS`, and the `_`-prefixed optional RM-attribute families.
//!
//! Covers ITS-REST `simplified_formats/master05-rm_mapping.adoc` §§LINK,
//! FEEDER_AUDIT, FEEDER_AUDIT_DETAILS, ISM_TRANSITION, INSTRUCTION_DETAILS, plus
//! the `_`-rows of the LOCATABLE / ENTRY / ELEMENT / EVENT_CONTEXT / PARTY /
//! DV_ORDERED / DV_TEXT tables (master04 §"RM Attributes prefix"). The
//! value-internal families (`_normal_range`, `_mapping`, …) reuse the builders in
//! [`super::data_values`]; the party / participation / reference shapes reuse
//! [`super::parties`].

use serde_json::{Map, Value, json};

use super::{data_values, parties};
use crate::error::FlatError;
use crate::sim::SimNode;

// ── RM → sim ──────────────────────────────────────────────────────────────────

/// Emit the `_`-prefixed families present on `rm` (see [`super::emit_rm_attrs`]).
#[allow(clippy::too_many_lines)] // one dispatch over the `_`-attribute families
pub(super) fn emit_rm_attrs(rm: &Value, rm_type: &str, out: &mut SimNode) {
    let ty = rm
        .get("_type")
        .and_then(Value::as_str)
        .unwrap_or_else(|| super::base_type(rm_type));

    if is_locatable(ty) {
        if let Some(uid) = rm.pointer("/uid/value") {
            out.occurrence_mut("_uid", None)
                .attrs
                .insert(String::new(), uid.clone());
        }
        if let Some(links) = rm.get("links").and_then(Value::as_array) {
            for (i, link) in links.iter().enumerate() {
                emit_link(link, out.occurrence_mut("_link", Some(i as u32)));
            }
        }
        if let Some(fa) = rm.get("feeder_audit").filter(|v| !v.is_null()) {
            emit_feeder_audit(fa, out.occurrence_mut("_feeder_audit", None));
        }
    }

    match ty {
        "ELEMENT" => {
            if let Some(nf) = rm.get("null_flavour").filter(|v| !v.is_null()) {
                data_values::emit_leaf(
                    nf,
                    "DV_CODED_TEXT",
                    None,
                    out.occurrence_mut("_null_flavour", None),
                );
            }
            if let Some(nr) = rm.get("null_reason").filter(|v| !v.is_null()) {
                data_values::emit_leaf(
                    nr,
                    "DV_TEXT",
                    None,
                    out.occurrence_mut("_null_reason", None),
                );
            }
        }
        "OBSERVATION" | "EVALUATION" | "INSTRUCTION" | "ACTION" | "ADMIN_ENTRY" => {
            if let Some(o) = rm.get("guideline_id").filter(|v| !v.is_null()) {
                parties::emit_object_ref(o, out.occurrence_mut("_guideline_id", None));
            }
            if let Some(o) = rm.get("workflow_id").filter(|v| !v.is_null()) {
                parties::emit_object_ref(o, out.occurrence_mut("_work_flow_id", None));
            }
            if let Some(p) = rm.get("provider").filter(|v| !v.is_null()) {
                parties::emit_party(p, out.occurrence_mut("_provider", None));
            }
            if let Some(parts) = rm.get("other_participations").and_then(Value::as_array) {
                for (i, p) in parts.iter().enumerate() {
                    parties::emit_participation(
                        p,
                        out.occurrence_mut("_other_participation", Some(i as u32)),
                    );
                }
            }
            if ty == "INSTRUCTION" {
                if let Some(et) = rm.get("expiry_time").filter(|v| !v.is_null()) {
                    data_values::emit_leaf(
                        et,
                        "DV_DATE_TIME",
                        None,
                        out.occurrence_mut("_expiry_time", None),
                    );
                }
                if let Some(wf) = rm.get("wf_definition").filter(|v| !v.is_null()) {
                    data_values::emit_leaf(
                        wf,
                        "DV_PARSABLE",
                        None,
                        out.occurrence_mut("_wf_definition", None),
                    );
                }
            }
            if ty == "ACTION"
                && let Some(det) = rm.get("instruction_details").filter(|v| !v.is_null())
            {
                emit_instruction_details(det, out.occurrence_mut("_instruction_details", None));
            }
        }
        "PARTY_IDENTIFIED" | "PARTY_RELATED" => {
            if let Some(ids) = rm.get("identifiers").and_then(Value::as_array) {
                for (i, id) in ids.iter().enumerate() {
                    parties::emit_identifier(id, out.occurrence_mut("_identifier", Some(i as u32)));
                }
            }
        }
        "ISM_TRANSITION" => {
            if let Some(reasons) = rm.get("reason").and_then(Value::as_array) {
                for (i, r) in reasons.iter().enumerate() {
                    data_values::emit_leaf(
                        r,
                        "DV_TEXT",
                        None,
                        out.occurrence_mut("_reason", Some(i as u32)),
                    );
                }
            }
        }
        // EVENT_CONTEXT `_`-families (master05 §EVENT_CONTEXT). A composition's
        // context is normally surfaced through the `ctx/` vocabulary (see
        // `crate::ctx`); these emit only when a walker renders an EVENT_CONTEXT
        // node through the tree instead.
        "EVENT_CONTEXT" => {
            if let Some(et) = rm.get("end_time").filter(|v| !v.is_null()) {
                data_values::emit_leaf(
                    et,
                    "DV_DATE_TIME",
                    None,
                    out.occurrence_mut("_end_time", None),
                );
            }
            if let Some(loc) = rm.get("location").filter(|v| !v.is_null()) {
                out.occurrence_mut("_location", None)
                    .attrs
                    .insert(String::new(), loc.clone());
            }
            if let Some(hcf) = rm.get("health_care_facility").filter(|v| !v.is_null()) {
                parties::emit_party(hcf, out.occurrence_mut("_health_care_facility", None));
            }
            if let Some(parts) = rm.get("participations").and_then(Value::as_array) {
                for (i, p) in parts.iter().enumerate() {
                    parties::emit_participation(
                        p,
                        out.occurrence_mut("_participation", Some(i as u32)),
                    );
                }
            }
        }
        _ => {}
    }

    // The value-internal families of a `DV_*` leaf (ranges / accuracy / text
    // meta / multimedia thumbnail+charset+language).
    data_values::emit_value_internal(rm, ty, out);
}

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

/// LINK → `|type`/`|meaning`/`|target` (master05 §LINK).
fn emit_link(link: &Value, out: &mut SimNode) {
    if let Some(v) = link.pointer("/type/value") {
        out.attrs.insert("type".to_owned(), v.clone());
    }
    if let Some(v) = link.pointer("/meaning/value") {
        out.attrs.insert("meaning".to_owned(), v.clone());
    }
    if let Some(v) = link.pointer("/target/value") {
        out.attrs.insert("target".to_owned(), v.clone());
    }
}

/// FEEDER_AUDIT (master05 §FEEDER_AUDIT): the two `*_system_audit`
/// FEEDER_AUDIT_DETAILS, `original_content`(`_multimedia`), and the
/// `*_system_item_id:i` DV_IDENTIFIER lists.
fn emit_feeder_audit(fa: &Value, out: &mut SimNode) {
    if let Some(d) = fa.get("originating_system_audit").filter(|v| !v.is_null()) {
        emit_audit_details(d, out.occurrence_mut("originating_system_audit", None));
    }
    if let Some(d) = fa.get("feeder_system_audit").filter(|v| !v.is_null()) {
        emit_audit_details(d, out.occurrence_mut("feeder_system_audit", None));
    }
    if let Some(oc) = fa.get("original_content").filter(|v| !v.is_null()) {
        let ty = oc
            .get("_type")
            .and_then(Value::as_str)
            .unwrap_or("DV_PARSABLE");
        let name = if ty == "DV_MULTIMEDIA" {
            "original_content_multimedia"
        } else {
            "original_content"
        };
        data_values::emit_leaf(oc, ty, None, out.occurrence_mut(name, None));
    }
    for (arr_key, seg) in [
        ("originating_system_item_ids", "originating_system_item_id"),
        ("feeder_system_item_ids", "feeder_system_item_id"),
    ] {
        if let Some(ids) = fa.get(arr_key).and_then(Value::as_array) {
            for (i, id) in ids.iter().enumerate() {
                parties::emit_identifier(id, out.occurrence_mut(seg, Some(i as u32)));
            }
        }
    }
}

/// FEEDER_AUDIT_DETAILS (master05 §FEEDER_AUDIT_DETAILS): `|system_id`,
/// `|version_id`, `|time`, and the `location`/`subject`/`provider` parties.
fn emit_audit_details(d: &Value, out: &mut SimNode) {
    if let Some(v) = d.get("system_id").filter(|v| !v.is_null()) {
        out.attrs.insert("system_id".to_owned(), v.clone());
    }
    if let Some(v) = d.get("version_id").filter(|v| !v.is_null()) {
        out.attrs.insert("version_id".to_owned(), v.clone());
    }
    if let Some(v) = d.pointer("/time/value") {
        out.attrs.insert("time".to_owned(), v.clone());
    }
    for party in ["location", "subject", "provider"] {
        if let Some(p) = d.get(party).filter(|v| !v.is_null()) {
            parties::emit_party(p, out.occurrence_mut(party, None));
        }
    }
}

/// INSTRUCTION_DETAILS (master05 §INSTRUCTION_DETAILS): the `instruction_id`
/// (LOCATABLE_REF: `|id`/`|type`/`|namespace`/`|path`) + `|activity_id`.
fn emit_instruction_details(det: &Value, out: &mut SimNode) {
    if let Some(iid) = det.get("instruction_id").filter(|v| !v.is_null()) {
        let child = out.occurrence_mut("instruction_id", None);
        if let Some(v) = iid.pointer("/id/value") {
            child.attrs.insert("id".to_owned(), v.clone());
        }
        if let Some(v) = iid.get("type").filter(|v| !v.is_null()) {
            child.attrs.insert("type".to_owned(), v.clone());
        }
        if let Some(v) = iid.get("namespace").filter(|v| !v.is_null()) {
            child.attrs.insert("namespace".to_owned(), v.clone());
        }
        if let Some(v) = iid.get("path").filter(|v| !v.is_null()) {
            child.attrs.insert("path".to_owned(), v.clone());
        }
    }
    if let Some(aid) = det.get("activity_id").filter(|v| !v.is_null()) {
        out.attrs.insert("activity_id".to_owned(), aid.clone());
    }
}

// ── sim → RM ──────────────────────────────────────────────────────────────────

/// One `_`-segment family → `(rm_attribute, value)` (see [`super::build_rm_attr`]).
#[allow(clippy::too_many_lines)] // one dispatch over the `_`-attribute families
pub(super) fn build_rm_attr(
    seg: &str,
    occurrences: &[SimNode],
    host_base: &str,
    path: &str,
) -> Result<Option<(String, Value)>, FlatError> {
    let name = seg.trim_start_matches('_');
    let first = || occurrences.iter().find(|o| !o.is_empty());
    let out = match name {
        "uid" => first()
            .and_then(SimNode::bare)
            .and_then(Value::as_str)
            .map(|v| ("uid".to_owned(), uid_value(v))),
        "link" => Some((
            "links".to_owned(),
            Value::Array(build_each(occurrences, build_link)),
        )),
        "feeder_audit" => first().map(|o| ("feeder_audit".to_owned(), build_feeder_audit(o))),
        "null_flavour" => first()
            .and_then(|o| data_values::build_leaf(o, "DV_CODED_TEXT", None))
            .map(|v| ("null_flavour".to_owned(), v)),
        "null_reason" => first()
            .and_then(|o| data_values::build_leaf(o, "DV_TEXT", None))
            .map(|v| ("null_reason".to_owned(), v)),
        "guideline_id" => first()
            .and_then(parties::build_object_ref)
            .map(|v| ("guideline_id".to_owned(), v)),
        "work_flow_id" => first()
            .and_then(parties::build_object_ref)
            .map(|v| ("workflow_id".to_owned(), v)),
        "provider" => first().map(|o| ("provider".to_owned(), parties::build_party(o, "PERSON"))),
        "other_participation" => Some((
            "other_participations".to_owned(),
            Value::Array(build_each(occurrences, parties::build_participation)),
        )),
        "participation" => Some((
            "participations".to_owned(),
            Value::Array(build_each(occurrences, parties::build_participation)),
        )),
        "instruction_details" => first().map(|o| {
            (
                "instruction_details".to_owned(),
                build_instruction_details(o),
            )
        }),
        "expiry_time" => first()
            .and_then(|o| data_values::build_leaf(o, "DV_DATE_TIME", None))
            .map(|v| ("expiry_time".to_owned(), v)),
        "wf_definition" => first()
            .and_then(|o| data_values::build_leaf(o, "DV_PARSABLE", None))
            .map(|v| ("wf_definition".to_owned(), v)),
        "identifier" => Some((
            "identifiers".to_owned(),
            Value::Array(build_each(occurrences, |o| {
                parties::build_identifier(o)
                    .unwrap_or_else(|| json!({"_type": "DV_IDENTIFIER", "id": ""}))
            })),
        )),
        "reason" => Some((
            "reason".to_owned(),
            Value::Array(build_each(occurrences, |o| {
                data_values::build_leaf(o, "DV_TEXT", None)
                    .unwrap_or_else(|| json!({"_type": "DV_TEXT", "value": ""}))
            })),
        )),
        "end_time" => first()
            .and_then(|o| data_values::build_leaf(o, "DV_DATE_TIME", None))
            .map(|v| ("end_time".to_owned(), v)),
        // EVENT_CONTEXT.location is a plain String (master05 §EVENT_CONTEXT note).
        "location" => first()
            .and_then(SimNode::bare)
            .map(|v| ("location".to_owned(), v.clone())),
        "health_care_facility" => first().map(|o| {
            (
                "health_care_facility".to_owned(),
                parties::build_party(o, "ORGANISATION"),
            )
        }),
        // value-internal families (`T` = the host leaf type).
        "normal_range" => first().map(|o| {
            (
                "normal_range".to_owned(),
                data_values::build_interval(o, host_base),
            )
        }),
        "other_reference_ranges" => Some((
            "other_reference_ranges".to_owned(),
            Value::Array(build_each(occurrences, |o| {
                data_values::build_reference_range(o, host_base)
            })),
        )),
        "accuracy" => first().and_then(SimNode::bare).map(|v| {
            (
                "accuracy".to_owned(),
                json!({"_type": "DV_DURATION", "value": v.clone()}),
            )
        }),
        "thumbnail" => first()
            .and_then(|o| data_values::build_leaf(o, "DV_MULTIMEDIA", None))
            .map(|v| ("thumbnail".to_owned(), v)),
        "charset" => first()
            .and_then(data_values::build_code_phrase)
            .map(|v| ("charset".to_owned(), v)),
        "language" => first()
            .and_then(data_values::build_code_phrase)
            .map(|v| ("language".to_owned(), v)),
        "encoding" => first()
            .and_then(data_values::build_code_phrase)
            .map(|v| ("encoding".to_owned(), v)),
        "mapping" => Some((
            "mappings".to_owned(),
            Value::Array(build_each(occurrences, data_values::build_term_mapping)),
        )),
        _ => {
            return Err(FlatError::UnknownSuffix {
                rm_type: host_base.to_owned(),
                suffix: seg.to_owned(),
                path: path.to_owned(),
            });
        }
    };
    Ok(out)
}

/// Apply `f` to each non-empty occurrence, in order.
fn build_each<F>(occurrences: &[SimNode], f: F) -> Vec<Value>
where
    F: Fn(&SimNode) -> Value,
{
    occurrences
        .iter()
        .filter(|o| !o.is_empty())
        .map(f)
        .collect()
}

/// A `::`-bearing id is an OBJECT_VERSION_ID; otherwise a HIER_OBJECT_ID (RM
/// support identifiers — both are valid `LOCATABLE.uid` UID_BASED_IDs).
fn uid_value(v: &str) -> Value {
    let ty = if v.contains("::") {
        "OBJECT_VERSION_ID"
    } else {
        "HIER_OBJECT_ID"
    };
    json!({"_type": ty, "value": v})
}

/// LINK (master05 §LINK): `type`/`meaning` are DV_TEXT, `target` is DV_EHR_URI.
fn build_link(node: &SimNode) -> Value {
    let text = |s: &str| node.attrs.get(s).and_then(Value::as_str).unwrap_or("");
    json!({
        "_type": "LINK",
        "type": {"_type": "DV_TEXT", "value": text("type")},
        "meaning": {"_type": "DV_TEXT", "value": text("meaning")},
        "target": {"_type": "DV_EHR_URI", "value": text("target")},
    })
}

/// FEEDER_AUDIT (master05 §FEEDER_AUDIT).
fn build_feeder_audit(node: &SimNode) -> Value {
    let mut fa = Map::new();
    fa.insert("_type".to_owned(), json!("FEEDER_AUDIT"));
    if let Some(o) = single(node, "originating_system_audit") {
        fa.insert(
            "originating_system_audit".to_owned(),
            build_audit_details(o),
        );
    }
    if let Some(o) = single(node, "feeder_system_audit") {
        fa.insert("feeder_system_audit".to_owned(), build_audit_details(o));
    }
    // master05: only one of `original_content` / `original_content_multimedia`.
    if let Some(o) = single(node, "original_content")
        && let Some(dv) = data_values::build_leaf(o, "DV_PARSABLE", None)
    {
        fa.insert("original_content".to_owned(), dv);
    }
    if let Some(o) = single(node, "original_content_multimedia")
        && let Some(dv) = data_values::build_leaf(o, "DV_MULTIMEDIA", None)
    {
        fa.insert("original_content".to_owned(), dv);
    }
    for (seg, key) in [
        ("originating_system_item_id", "originating_system_item_ids"),
        ("feeder_system_item_id", "feeder_system_item_ids"),
    ] {
        if let Some(child) = node.children.get(seg) {
            let ids = build_each(&child.occurrences, |o| {
                parties::build_identifier(o)
                    .unwrap_or_else(|| json!({"_type": "DV_IDENTIFIER", "id": ""}))
            });
            if !ids.is_empty() {
                fa.insert(key.to_owned(), Value::Array(ids));
            }
        }
    }
    Value::Object(fa)
}

fn build_audit_details(node: &SimNode) -> Value {
    let mut d = Map::new();
    d.insert("_type".to_owned(), json!("FEEDER_AUDIT_DETAILS"));
    d.insert(
        "system_id".to_owned(),
        json!(
            node.attrs
                .get("system_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ),
    );
    if let Some(v) = node.attrs.get("version_id") {
        d.insert("version_id".to_owned(), v.clone());
    }
    if let Some(v) = node.attrs.get("time") {
        d.insert(
            "time".to_owned(),
            json!({"_type": "DV_DATE_TIME", "value": v.clone()}),
        );
    }
    for party in ["location", "subject", "provider"] {
        if let Some(p) = single(node, party) {
            // master05 §FEEDER_AUDIT_DETAILS: `subject` is a PARTY_PROXY (may be
            // PARTY_SELF); `location`/`provider` are PARTY_IDENTIFIED.
            d.insert(party.to_owned(), parties::build_party(p, "PERSON"));
        }
    }
    Value::Object(d)
}

/// INSTRUCTION_DETAILS (master05 §INSTRUCTION_DETAILS).
fn build_instruction_details(node: &SimNode) -> Value {
    let mut o = Map::new();
    o.insert("_type".to_owned(), json!("INSTRUCTION_DETAILS"));
    if let Some(iid) = single(node, "instruction_id") {
        let id = iid.attrs.get("id").and_then(Value::as_str).unwrap_or("");
        let ty = iid
            .attrs
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("VERSIONED_COMPOSITION");
        let ns = iid
            .attrs
            .get("namespace")
            .and_then(Value::as_str)
            .unwrap_or("EHR");
        let id_type = if id.contains("::") {
            "OBJECT_VERSION_ID"
        } else {
            "HIER_OBJECT_ID"
        };
        let mut lref = json!({
            "_type": "LOCATABLE_REF",
            "namespace": ns,
            "type": ty,
            "id": {"_type": id_type, "value": id},
        });
        if let Some(p) = iid.attrs.get("path").and_then(Value::as_str) {
            lref["path"] = json!(p);
        }
        o.insert("instruction_id".to_owned(), lref);
    }
    if let Some(aid) = node.attrs.get("activity_id") {
        o.insert("activity_id".to_owned(), aid.clone());
    }
    Value::Object(o)
}

fn single<'a>(node: &'a SimNode, name: &str) -> Option<&'a SimNode> {
    node.children.get(name).and_then(|c| c.occurrences.first())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // master05 §LINK + §COMPOSITION `_uid`: round-trip through the RM-attr layer.
    #[test]
    fn uid_and_link_roundtrip() {
        let rm = json!({
            "_type": "OBSERVATION",
            "uid": {"_type": "HIER_OBJECT_ID", "value": "9fcc1c70"},
            "links": [{"_type": "LINK",
                "type": {"_type": "DV_TEXT", "value": "problem"},
                "meaning": {"_type": "DV_TEXT", "value": "related"},
                "target": {"_type": "DV_EHR_URI", "value": "ehr://x"}}]
        });
        let mut out = SimNode::default();
        emit_rm_attrs(&rm, "OBSERVATION", &mut out);
        assert_eq!(
            out.child("_uid").and_then(SimNode::bare),
            Some(&json!("9fcc1c70"))
        );
        let link = out.child("_link").unwrap();
        assert_eq!(link.attrs.get("type"), Some(&json!("problem")));
        assert_eq!(link.attrs.get("target"), Some(&json!("ehr://x")));

        let uid_occ = out.children["_uid"].occurrences.clone();
        let (attr, v) = build_rm_attr("_uid", &uid_occ, "OBSERVATION", "p")
            .unwrap()
            .unwrap();
        assert_eq!(attr, "uid");
        assert_eq!(v["_type"], json!("HIER_OBJECT_ID"));
        let link_occ = out.children["_link"].occurrences.clone();
        let (attr, v) = build_rm_attr("_link", &link_occ, "OBSERVATION", "p")
            .unwrap()
            .unwrap();
        assert_eq!(attr, "links");
        assert_eq!(v[0]["type"]["value"], json!("problem"));
        assert_eq!(v[0]["_type"], json!("LINK"));
    }

    // master05 §ELEMENT: `_null_flavour` (DV_CODED_TEXT) + `_null_reason` (DV_TEXT).
    #[test]
    fn null_flavour_and_reason() {
        let mut nf = SimNode::default();
        nf.attrs.insert("code".to_owned(), json!("253"));
        nf.attrs.insert("value".to_owned(), json!("unknown"));
        nf.attrs.insert("terminology".to_owned(), json!("openehr"));
        let (attr, v) = build_rm_attr("_null_flavour", std::slice::from_ref(&nf), "", "p")
            .unwrap()
            .unwrap();
        assert_eq!(attr, "null_flavour");
        assert_eq!(v["defining_code"]["code_string"], json!("253"));
    }

    // master05 §ISM_TRANSITION `/_reason:i` (DV_TEXT list).
    #[test]
    fn ism_reason_list() {
        let mut r0 = SimNode::default();
        r0.attrs.insert(String::new(), json!("reason 1"));
        let (attr, v) = build_rm_attr("_reason", std::slice::from_ref(&r0), "ISM_TRANSITION", "p")
            .unwrap()
            .unwrap();
        assert_eq!(attr, "reason");
        assert_eq!(v[0]["value"], json!("reason 1"));
        assert_eq!(v[0]["_type"], json!("DV_TEXT"));
    }

    // master05 §FEEDER_AUDIT: originating_system_audit round-trip.
    #[test]
    fn feeder_audit_roundtrip() {
        let rm = json!({
            "_type": "OBSERVATION",
            "feeder_audit": {"_type": "FEEDER_AUDIT",
                "originating_system_audit": {"_type": "FEEDER_AUDIT_DETAILS",
                    "system_id": "orig", "version_id": "final"}}
        });
        let mut out = SimNode::default();
        emit_rm_attrs(&rm, "OBSERVATION", &mut out);
        let fa = out.child("_feeder_audit").unwrap();
        let osa = fa.child("originating_system_audit").unwrap();
        assert_eq!(osa.attrs.get("system_id"), Some(&json!("orig")));
        assert_eq!(osa.attrs.get("version_id"), Some(&json!("final")));

        let occ = out.children["_feeder_audit"].occurrences.clone();
        let (attr, v) = build_rm_attr("_feeder_audit", &occ, "OBSERVATION", "p")
            .unwrap()
            .unwrap();
        assert_eq!(attr, "feeder_audit");
        assert_eq!(v["originating_system_audit"]["system_id"], json!("orig"));
        assert_eq!(v["originating_system_audit"]["version_id"], json!("final"));
    }

    // An unrecognised `_`-segment is rejected (master05 per-type tables).
    #[test]
    fn unknown_rm_attr_rejected() {
        let node = SimNode::default();
        assert!(matches!(
            build_rm_attr(
                "_frobnicate",
                std::slice::from_ref(&node),
                "OBSERVATION",
                "p"
            ),
            Err(FlatError::UnknownSuffix { .. })
        ));
    }

    // master05 §INSTRUCTION_DETAILS.
    #[test]
    fn instruction_details_roundtrip() {
        let rm = json!({
            "_type": "ACTION",
            "instruction_details": {"_type": "INSTRUCTION_DETAILS",
                "instruction_id": {"_type": "LOCATABLE_REF", "namespace": "EHR",
                    "type": "VERSIONED_COMPOSITION",
                    "id": {"_type": "OBJECT_VERSION_ID", "value": "4cdc::x::1"},
                    "path": "/content[x]"},
                "activity_id": "activities[at0001]"}
        });
        let mut out = SimNode::default();
        emit_rm_attrs(&rm, "ACTION", &mut out);
        let det = out.child("_instruction_details").unwrap();
        assert_eq!(
            det.attrs.get("activity_id"),
            Some(&json!("activities[at0001]"))
        );
        let iid = det.child("instruction_id").unwrap();
        assert_eq!(iid.attrs.get("path"), Some(&json!("/content[x]")));

        let occ = out.children["_instruction_details"].occurrences.clone();
        let (attr, v) = build_rm_attr("_instruction_details", &occ, "ACTION", "p")
            .unwrap()
            .unwrap();
        assert_eq!(attr, "instruction_details");
        assert_eq!(v["activity_id"], json!("activities[at0001]"));
        assert_eq!(
            v["instruction_id"]["id"]["_type"],
            json!("OBJECT_VERSION_ID")
        );
    }
}
