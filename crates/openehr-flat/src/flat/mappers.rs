//! Per-`DATA_VALUE` leaf mappers, both directions.
//!
//! The suffix names follow Better's `web-template` `converter/…ToFlatMapper`
//! (`|unit` singular, `|scale` for `DV_SCALE`, `|ordinal` for `DV_ORDINAL`;
//! Better emits **no** `/_type`). RM→flat ([`leaf_to_flat`]) dispatches on the
//! value's canonical `_type`; flat→RM ([`leaf_from_flat`]) is driven by the
//! web-template node's `rmType` (the constrained concrete type), disambiguating
//! a coded-text-with-`other` node by whether a `|code` / `|other` entry exists.

use indexmap::IndexMap;
use serde_json::{Value, json};

use super::sub::FlatView;

/// The accumulating flat map (insertion order preserved).
pub(crate) type FlatMap = IndexMap<String, Value>;

/// Insert `base|suffix` (or the bare `base` when `suffix` is empty) if `v` is a
/// present, non-null JSON value.
fn put(out: &mut FlatMap, base: &str, suffix: &str, v: Option<&Value>) {
    let Some(v) = v else { return };
    if v.is_null() {
        return;
    }
    let key = if suffix.is_empty() {
        base.to_owned()
    } else {
        format!("{base}|{suffix}")
    };
    out.insert(key, v.clone());
}

fn put_str(out: &mut FlatMap, base: &str, suffix: &str, v: Option<&str>) {
    if let Some(v) = v {
        out.insert(format!("{base}|{suffix}"), json!(v));
    }
}

/// A `CODE_PHRASE`'s `code_string` / `terminology_id.value` / `preferred_term`.
fn code_phrase_parts(cp: &Value) -> (Option<&Value>, Option<&str>, Option<&Value>) {
    let code = cp.get("code_string");
    let term = cp
        .get("terminology_id")
        .and_then(|t| t.get("value"))
        .and_then(Value::as_str);
    let preferred = cp.get("preferred_term").filter(|v| !v.is_null());
    (code, term, preferred)
}

/// Emit the flat entries for one populated `DATA_VALUE` leaf at `base`.
///
/// `slot_rm_type` is the web-template node's declared rm type; a plain `DV_TEXT`
/// value in a `DV_CODED_TEXT` slot is written as `|other` (Better's rule).
pub(crate) fn leaf_to_flat(dv: &Value, slot_rm_type: &str, base: &str, out: &mut FlatMap) {
    let ty = dv
        .get("_type")
        .and_then(Value::as_str)
        .unwrap_or(slot_rm_type);
    match ty {
        "DV_TEXT" | "DV_PARAGRAPH" => {
            let coded_slot = slot_rm_type.split('<').next() == Some("DV_CODED_TEXT");
            let has_meta = dv.get("formatting").is_some_and(|v| !v.is_null())
                || dv.get("mappings").is_some_and(|v| !v.is_null());
            if coded_slot {
                put(out, base, "other", dv.get("value"));
            } else if has_meta {
                put(out, base, "value", dv.get("value"));
            } else {
                put(out, base, "", dv.get("value"));
            }
            // PORT NOTE: the SM transformation table
            // (`SM/.../simplified_im_b/master07-transformation_rules.adoc`, RM Data
            // types) marks `DV_TEXT._formatting_` as **skip** (along with
            // `_language_`/`_encoding_`, which we do drop). We deliberately keep
            // `|formatting` for RM→FLAT→RM round-trip fidelity, matching Better
            // (F-10-06). `formatting` is optional, so its presence never breaks
            // canonical validity.
            put(out, base, "formatting", dv.get("formatting"));
        }
        "DV_CODED_TEXT" | "DV_STATE" => {
            put(out, base, "value", dv.get("value"));
            if let Some(dc) = dv.get("defining_code") {
                let (code, term, pref) = code_phrase_parts(dc);
                put(out, base, "code", code);
                put_str(out, base, "terminology", term);
                put(out, base, "preferred_term", pref);
            }
            put(out, base, "formatting", dv.get("formatting"));
        }
        "CODE_PHRASE" => {
            let (code, term, pref) = code_phrase_parts(dv);
            put(out, base, "code", code);
            put_str(out, base, "terminology", term);
            put(out, base, "preferred_term", pref);
        }
        "DV_QUANTITY" => {
            put(out, base, "magnitude", dv.get("magnitude"));
            put(out, base, "unit", dv.get("units"));
            put(out, base, "precision", dv.get("precision"));
            // PORT NOTE: `DV_QUANTITY.units_system` / `units_display_name` are
            // genuine RM 1.2.0 fields (openehr-rm dv_quantity.rs:59,65) with a
            // canonical-JSON/XML home, but the FLAT `|unit_system` /
            // `|unit_display_name` *suffix* representation is a Better vendor
            // extra beyond the common EhrScape suffix set — no normative SDT
            // concrete format exists (SM serial_data_formats is unfinished,
            // F-10-01/05). Per serialization.md these Better-only extras live
            // behind `ehrbase-quirks` and must never be hard-coded onto the
            // default (spec/ITS-REST-common) FLAT path (F-13-25).
            #[cfg(feature = "ehrbase-quirks")]
            {
                put(out, base, "unit_system", dv.get("units_system"));
                put(out, base, "unit_display_name", dv.get("units_display_name"));
            }
            put(out, base, "magnitude_status", dv.get("magnitude_status"));
        }
        "DV_COUNT" => {
            put(out, base, "", dv.get("magnitude"));
            put(out, base, "magnitude_status", dv.get("magnitude_status"));
        }
        "DV_PROPORTION" => {
            put(out, base, "numerator", dv.get("numerator"));
            put(out, base, "denominator", dv.get("denominator"));
            put(out, base, "type", dv.get("type"));
        }
        "DV_ORDINAL" => {
            put(out, base, "value", symbol_value(dv));
            emit_symbol_code(dv, base, out);
            put(out, base, "ordinal", dv.get("value"));
        }
        "DV_SCALE" => {
            put(out, base, "value", symbol_value(dv));
            emit_symbol_code(dv, base, out);
            put(out, base, "scale", dv.get("value"));
        }
        "DV_BOOLEAN" => put(out, base, "", dv.get("value")),
        "DV_DATE_TIME" | "DV_DATE" | "DV_TIME" | "DV_DURATION" | "DV_URI" | "DV_EHR_URI" => {
            put(out, base, "", dv.get("value"));
        }
        "DV_PARSABLE" => {
            // Better `DvParsableToFlatMapper`: bare value + `|formalism`.
            put(out, base, "", dv.get("value"));
            put(out, base, "formalism", dv.get("formalism"));
        }
        "DV_IDENTIFIER" => {
            put(out, base, "id", dv.get("id"));
            put(out, base, "type", dv.get("type"));
            put(out, base, "issuer", dv.get("issuer"));
            put(out, base, "assigner", dv.get("assigner"));
        }
        "DV_MULTIMEDIA" => {
            // Better `DvMultimediaToFlatMapper`: the bare value is the `uri`
            // (inline `data` is not surfaced in FLAT), plus `|mediatype`,
            // `|alternatetext`, and `|size` (only when > 0).
            if let Some(uri) = dv.get("uri").filter(|u| !u.is_null()) {
                put(out, base, "", uri.get("value"));
            }
            if let Some(mt) = dv.get("media_type") {
                put(out, base, "mediatype", mt.get("code_string"));
            }
            put(out, base, "alternatetext", dv.get("alternate_text"));
            if let Some(size) = dv.get("size").filter(|s| s.as_i64().is_some_and(|n| n > 0)) {
                put(out, base, "size", Some(size));
            }
        }
        _ => {
            // Any remaining `DV_*` leaf falls back to its scalar `value`. Optional
            // `DV_ORDERED` reference ranges (`normal_range` / `other_reference_ranges`)
            // are not surfaced — they have no simplified-format (`|suffix`) shape and
            // Better emits them only in the structural `_normal_range` form, outside
            // the simplified scope; they are optional, so RM validity is unaffected.
            if let Some(v) = dv.get("value") {
                put(out, base, "", Some(v));
            }
        }
    }
}

fn symbol_value(dv: &Value) -> Option<&Value> {
    dv.get("symbol").and_then(|s| s.get("value"))
}

fn emit_symbol_code(dv: &Value, base: &str, out: &mut FlatMap) {
    if let Some(dc) = dv.get("symbol").and_then(|s| s.get("defining_code")) {
        let (code, term, _) = code_phrase_parts(dc);
        put(out, base, "code", code);
        put_str(out, base, "terminology", term);
    }
}

// ── flat → RM ─────────────────────────────────────────────────────────────────

/// Build a `DATA_VALUE` JSON object for the web-template node of `rm_type` from
/// the flat entries under `base` (a [`FlatView`] keyed by suffix / bare).
///
/// Falls back to suffix-driven type inference when the declared type produces
/// nothing (Better emits no `/_type`, so a `|code`-less choice alternative or a
/// value whose concrete type differs from the constraint is recovered here).
pub(crate) fn leaf_from_flat(rm_type: &str, view: &FlatView) -> Option<Value> {
    let base = rm_type.split('<').next().unwrap_or(rm_type);
    if let Some(dv) = build_for(base, view) {
        return Some(dv);
    }
    let inferred = infer_leaf_type(view)?;
    if inferred != base {
        return build_for(inferred, view);
    }
    None
}

/// Infer a leaf's concrete type from the distinctive suffixes present.
fn infer_leaf_type(view: &FlatView) -> Option<&'static str> {
    if view.suffix("magnitude").is_some() {
        Some("DV_QUANTITY")
    } else if view.suffix("numerator").is_some() {
        Some("DV_PROPORTION")
    } else if view.suffix("ordinal").is_some() {
        Some("DV_ORDINAL")
    } else if view.suffix("scale").is_some() {
        Some("DV_SCALE")
    } else if view.suffix("mediatype").is_some() {
        Some("DV_MULTIMEDIA")
    } else if view.suffix("id").is_some() {
        Some("DV_IDENTIFIER")
    } else if view.suffix("formalism").is_some() {
        Some("DV_PARSABLE")
    } else if view.suffix("code").is_some() {
        Some("DV_CODED_TEXT")
    } else {
        None
    }
}

fn build_for(base: &str, view: &FlatView) -> Option<Value> {
    match base {
        "DV_TEXT" | "DV_PARAGRAPH" => text_from_flat(view, base),
        "DV_CODED_TEXT" | "DV_STATE" => {
            // Coded-text-with-`other`: `|other` (or a bare value with no `|code`)
            // ⇒ the value was plain DV_TEXT.
            if let Some(other) = view.suffix("other") {
                text_value(Some(other.clone()), None)
            } else if view.suffix("code").is_none() && view.bare().is_some() {
                text_value(view.bare().cloned(), view.suffix("formatting").cloned())
            } else {
                coded_text_from_flat(view, base)
            }
        }
        "CODE_PHRASE" => code_phrase_from_flat(view),
        "DV_QUANTITY" => quantity_from_flat(view),
        "DV_COUNT" => count_from_flat(view),
        "DV_PROPORTION" => proportion_from_flat(view),
        "DV_ORDINAL" => ordinal_from_flat(view, "DV_ORDINAL", "ordinal"),
        "DV_SCALE" => ordinal_from_flat(view, "DV_SCALE", "scale"),
        "DV_BOOLEAN" => bare_typed(view, "DV_BOOLEAN"),
        "DV_DATE_TIME" | "DV_DATE" | "DV_TIME" | "DV_DURATION" | "DV_URI" | "DV_EHR_URI" => {
            bare_typed(view, base)
        }
        "DV_IDENTIFIER" => identifier_from_flat(view),
        "DV_MULTIMEDIA" => multimedia_from_flat(view),
        "DV_PARSABLE" => parsable_from_flat(view),
        _ => bare_typed(view, base),
    }
}

fn parsable_from_flat(view: &FlatView) -> Option<Value> {
    let value = view.bare().or_else(|| view.suffix("value"))?.clone();
    let formalism = view
        .suffix("formalism")
        .cloned()
        .unwrap_or_else(|| json!(""));
    Some(json!({"_type": "DV_PARSABLE", "value": value, "formalism": formalism}))
}

fn text_value(value: Option<Value>, formatting: Option<Value>) -> Option<Value> {
    let value = value?;
    let mut o = serde_json::Map::new();
    o.insert("_type".into(), json!("DV_TEXT"));
    o.insert("value".into(), value);
    if let Some(f) = formatting {
        o.insert("formatting".into(), f);
    }
    Some(Value::Object(o))
}

fn text_from_flat(view: &FlatView, ty: &str) -> Option<Value> {
    let value = view.bare().or_else(|| view.suffix("value"))?.clone();
    let mut o = serde_json::Map::new();
    o.insert("_type".into(), json!(ty));
    o.insert("value".into(), value);
    if let Some(f) = view.suffix("formatting") {
        o.insert("formatting".into(), f.clone());
    }
    Some(Value::Object(o))
}

fn coded_text_from_flat(view: &FlatView, ty: &str) -> Option<Value> {
    let code = view.suffix("code")?.clone();
    let terminology = view
        .suffix("terminology")
        .and_then(Value::as_str)
        .unwrap_or("local")
        .to_owned();
    let mut o = serde_json::Map::new();
    o.insert("_type".into(), json!(ty));
    if let Some(v) = view.suffix("value") {
        o.insert("value".into(), v.clone());
    }
    o.insert(
        "defining_code".into(),
        code_phrase_obj(code, &terminology, view.suffix("preferred_term").cloned()),
    );
    if let Some(f) = view.suffix("formatting") {
        o.insert("formatting".into(), f.clone());
    }
    Some(Value::Object(o))
}

fn code_phrase_from_flat(view: &FlatView) -> Option<Value> {
    let code = view.suffix("code")?.clone();
    let terminology = view
        .suffix("terminology")
        .and_then(Value::as_str)
        .unwrap_or("local")
        .to_owned();
    Some(code_phrase_obj(
        code,
        &terminology,
        view.suffix("preferred_term").cloned(),
    ))
}

fn code_phrase_obj(code: Value, terminology: &str, preferred_term: Option<Value>) -> Value {
    let mut o = serde_json::Map::new();
    o.insert("_type".into(), json!("CODE_PHRASE"));
    o.insert(
        "terminology_id".into(),
        json!({"_type": "TERMINOLOGY_ID", "value": terminology}),
    );
    o.insert("code_string".into(), code);
    if let Some(pt) = preferred_term {
        o.insert("preferred_term".into(), pt);
    }
    Value::Object(o)
}

fn quantity_from_flat(view: &FlatView) -> Option<Value> {
    let magnitude = view.suffix("magnitude")?.clone();
    let mut o = serde_json::Map::new();
    o.insert("_type".into(), json!("DV_QUANTITY"));
    o.insert("magnitude".into(), magnitude);
    for (suffix, field) in [
        ("unit", "units"),
        ("precision", "precision"),
        ("magnitude_status", "magnitude_status"),
    ] {
        if let Some(v) = view.suffix(suffix) {
            o.insert(field.into(), v.clone());
        }
    }
    // PORT NOTE: Better-only `|unit_system` / `|unit_display_name` extras — gated
    // per serialization.md so the default FLAT path stays spec-common (F-13-25;
    // see `leaf_to_flat`). The underlying RM 1.2.0 fields remain first-class in
    // canonical JSON/XML regardless.
    #[cfg(feature = "ehrbase-quirks")]
    for (suffix, field) in [
        ("unit_system", "units_system"),
        ("unit_display_name", "units_display_name"),
    ] {
        if let Some(v) = view.suffix(suffix) {
            o.insert(field.into(), v.clone());
        }
    }
    Some(Value::Object(o))
}

fn count_from_flat(view: &FlatView) -> Option<Value> {
    let magnitude = view.bare()?.clone();
    let mut o = serde_json::Map::new();
    o.insert("_type".into(), json!("DV_COUNT"));
    o.insert("magnitude".into(), magnitude);
    if let Some(s) = view.suffix("magnitude_status") {
        o.insert("magnitude_status".into(), s.clone());
    }
    Some(Value::Object(o))
}

fn proportion_from_flat(view: &FlatView) -> Option<Value> {
    let numerator = view.suffix("numerator")?.clone();
    let mut o = serde_json::Map::new();
    o.insert("_type".into(), json!("DV_PROPORTION"));
    o.insert("numerator".into(), numerator);
    if let Some(d) = view.suffix("denominator") {
        o.insert("denominator".into(), d.clone());
    }
    if let Some(t) = view.suffix("type") {
        o.insert("type".into(), t.clone());
    }
    Some(Value::Object(o))
}

fn ordinal_from_flat(view: &FlatView, ty: &str, numeric_suffix: &str) -> Option<Value> {
    let numeric = view.suffix(numeric_suffix)?.clone();
    let code = view.suffix("code")?.clone();
    let terminology = view
        .suffix("terminology")
        .and_then(Value::as_str)
        .unwrap_or("local")
        .to_owned();
    let value = view.suffix("value").cloned().unwrap_or(json!(""));
    let symbol = json!({
        "_type": "DV_CODED_TEXT",
        "value": value,
        "defining_code": code_phrase_obj(code, &terminology, None),
    });
    Some(json!({"_type": ty, "value": numeric, "symbol": symbol}))
}

fn identifier_from_flat(view: &FlatView) -> Option<Value> {
    let id = view.suffix("id").or_else(|| view.bare())?.clone();
    let mut o = serde_json::Map::new();
    o.insert("_type".into(), json!("DV_IDENTIFIER"));
    o.insert("id".into(), id);
    // `issuer`/`assigner`/`type` are optional on the wire; only set when present
    // (fabricating empty strings would break the FLAT round-trip).
    for s in ["type", "issuer", "assigner"] {
        if let Some(v) = view.suffix(s) {
            o.insert(s.into(), v.clone());
        }
    }
    Some(Value::Object(o))
}

fn multimedia_from_flat(view: &FlatView) -> Option<Value> {
    // Better `DvMultimediaFactory`: the bare value is the `uri`; `media_type` +
    // `size` are RM-mandatory (size defaults to 0 when the FLAT lacks `|size`).
    let mut o = serde_json::Map::new();
    o.insert("_type".into(), json!("DV_MULTIMEDIA"));
    if let Some(uri) = view.bare() {
        o.insert(
            "uri".into(),
            json!({"_type": "DV_URI", "value": uri.clone()}),
        );
    }
    if let Some(mt) = view.suffix("mediatype") {
        o.insert(
            "media_type".into(),
            code_phrase_obj(mt.clone(), "IANA_media-types", None),
        );
    }
    if let Some(a) = view.suffix("alternatetext") {
        o.insert("alternate_text".into(), a.clone());
    }
    let size = view
        .suffix("size")
        .and_then(Value::as_i64)
        .unwrap_or_else(|| {
            view.suffix("size")
                .and_then(|v| v.as_str()?.parse().ok())
                .unwrap_or(0)
        });
    o.insert("size".into(), json!(size));
    // Requires at least a media_type or uri to be a meaningful multimedia leaf.
    if o.contains_key("media_type") || o.contains_key("uri") {
        Some(Value::Object(o))
    } else {
        None
    }
}

fn bare_typed(view: &FlatView, ty: &str) -> Option<Value> {
    let value = view.bare()?.clone();
    Some(json!({"_type": ty, "value": value}))
}
