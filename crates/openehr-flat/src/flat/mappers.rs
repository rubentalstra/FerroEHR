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

/// Emit the shared `DV_QUANTIFIED`/`DV_ORDERED` scalar extras — `|magnitude_status`,
/// `|normal_status` (`normal_status.code_string`), `|accuracy`,
/// `|accuracy_is_percent` — present on the `DV_AMOUNT` family (master05
/// §§DV_QUANTITY, DV_COUNT, DV_PROPORTION, DV_DURATION). `with_accuracy` is false
/// for the temporal family, whose `accuracy` is a `DV_DURATION` carried as the
/// `/_accuracy` sub-path instead (master05 §§DV_DATE, DV_DATE_TIME, DV_TIME).
fn emit_quantified_extras(dv: &Value, base: &str, out: &mut FlatMap, with_accuracy: bool) {
    put(out, base, "magnitude_status", dv.get("magnitude_status"));
    put_str(
        out,
        base,
        "normal_status",
        dv.pointer("/normal_status/code_string")
            .and_then(Value::as_str),
    );
    if with_accuracy {
        put(out, base, "accuracy", dv.get("accuracy"));
        put(
            out,
            base,
            "accuracy_is_percent",
            dv.get("accuracy_is_percent"),
        );
    }
}

/// Emit the flat entries for one populated `DATA_VALUE` leaf at `base`.
///
/// `slot_rm_type` is the web-template node's declared rm type; a plain `DV_TEXT`
/// value in an **open** `DV_CODED_TEXT` slot is written as `|other` (master02
/// §"Open Value-Sets and the `|other` Suffix"). `list_open` is the slot's
/// `listOpen`: `|other` is only emitted when the slot is open (`Some(true)` or,
/// when the template omits the flag, `None`); a `DV_TEXT` in a closed slot
/// (`Some(false)`) is a data defect and falls back to a plain value.
#[allow(clippy::too_many_lines)] // one dispatch over the DATA_VALUE leaf set
pub(crate) fn leaf_to_flat(
    dv: &Value,
    slot_rm_type: &str,
    base: &str,
    list_open: Option<bool>,
    out: &mut FlatMap,
) {
    let ty = dv
        .get("_type")
        .and_then(Value::as_str)
        .unwrap_or(slot_rm_type);
    match ty {
        "DV_TEXT" | "DV_PARAGRAPH" => {
            let coded_slot = slot_rm_type.split('<').next() == Some("DV_CODED_TEXT");
            let open_slot = coded_slot && list_open != Some(false);
            let has_meta = dv.get("formatting").is_some_and(|v| !v.is_null())
                || dv.get("mappings").is_some_and(|v| !v.is_null());
            if open_slot {
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
            //. `formatting` is optional, so its presence never breaks
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
            // default (spec/ITS-REST-common) FLAT path.
            #[cfg(feature = "ehrbase-quirks")]
            {
                put(out, base, "unit_system", dv.get("units_system"));
                put(out, base, "unit_display_name", dv.get("units_display_name"));
            }
            emit_quantified_extras(dv, base, out, true);
        }
        "DV_COUNT" => {
            put(out, base, "", dv.get("magnitude"));
            emit_quantified_extras(dv, base, out, true);
        }
        "DV_PROPORTION" => {
            put(out, base, "numerator", dv.get("numerator"));
            put(out, base, "denominator", dv.get("denominator"));
            put(out, base, "type", dv.get("type"));
            // `precision` + the computed bare `magnitude` (numerator/denominator,
            // "calculated on output" per master05 §DV_PROPORTION).
            put(out, base, "precision", dv.get("precision"));
            if let (Some(n), Some(d)) = (
                dv.get("numerator").and_then(Value::as_f64),
                dv.get("denominator").and_then(Value::as_f64),
            ) && d != 0.0
            {
                put(out, base, "", Some(&json!(n / d)));
            }
            emit_quantified_extras(dv, base, out, true);
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
        "DV_DURATION" => {
            put(out, base, "", dv.get("value"));
            emit_quantified_extras(dv, base, out, true);
        }
        "DV_DATE_TIME" | "DV_DATE" | "DV_TIME" => {
            // Temporal family: bare value + magnitude_status/normal_status; the
            // `accuracy` (a DV_DURATION) is the `/_accuracy` sub-path, emitted by
            // the `_`-attribute layer (master05 §§DV_DATE, DV_DATE_TIME, DV_TIME).
            put(out, base, "", dv.get("value"));
            emit_quantified_extras(dv, base, out, false);
        }
        "DV_URI" | "DV_EHR_URI" => {
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
            // master05 §DV_MULTIMEDIA: bare value = `uri.value`, plus the full
            // attribute set. `/_thumbnail`, `/_charset`, `/_language` sub-paths
            // are emitted by the `_`-attribute layer.
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
            if let Some(ca) = dv.get("compression_algorithm") {
                put(out, base, "compression_algorithm", ca.get("code_string"));
            }
            put(out, base, "integrity_check", dv.get("integrity_check"));
            if let Some(ica) = dv.get("integrity_check_algorithm") {
                put(
                    out,
                    base,
                    "integrity_check_algorithm",
                    ica.get("code_string"),
                );
            }
            // Inline `data` (Array<Octet>) is base64 text in canonical JSON.
            put(out, base, "data", dv.get("data"));
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
    // `|raw` bypass (master02/master04 §"Raw canonical JSON"): the value is
    // pre-serialized canonical JSON embedded verbatim as the node's RM value. It
    // MUST carry `_type`; a raw value without one is ignored (falls through to
    // normal leaf decomposition). Write-only — RM→FLAT always decomposes.
    if let Some(raw) = view.suffix("raw")
        && raw.get("_type").and_then(Value::as_str).is_some()
    {
        return Some(raw.clone());
    }
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

#[allow(clippy::match_same_arms)] // arms kept explicit per RM type for clarity
fn build_for(base: &str, view: &FlatView) -> Option<Value> {
    match base {
        "DV_TEXT" | "DV_PARAGRAPH" => text_from_flat(view, base),
        "DV_CODED_TEXT" | "DV_STATE" => {
            // Coded-text-with-`other`: `|other` ⇒ the value was plain DV_TEXT.
            // A bare value first tries the SDT path+terse coded form
            // (`terminology::code|text|` — SIM-B master04 §S_DV_CODED_TEXT);
            // a bare string that is not terse-coded is the free-text
            // alternative of a coded-text-with-other slot.
            if let Some(other) = view.suffix("other") {
                text_value(Some(other.clone()), None)
            } else {
                coded_text_from_flat(view, base).or_else(|| {
                    text_value(view.bare().cloned(), view.suffix("formatting").cloned())
                })
            }
        }
        "CODE_PHRASE" => code_phrase_from_flat(view),
        "DV_QUANTITY" => quantity_from_flat(view),
        "DV_COUNT" => count_from_flat(view),
        "DV_PROPORTION" => proportion_from_flat(view),
        "DV_ORDINAL" => ordinal_from_flat(view, "DV_ORDINAL", "ordinal"),
        "DV_SCALE" => ordinal_from_flat(view, "DV_SCALE", "scale"),
        "DV_BOOLEAN" => bare_typed(view, "DV_BOOLEAN"),
        "DV_DURATION" => temporal_from_flat(view, base, true),
        "DV_DATE_TIME" | "DV_DATE" | "DV_TIME" => temporal_from_flat(view, base, false),
        "DV_URI" | "DV_EHR_URI" => bare_typed(view, base),
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
    let (code, terminology, terse_value) = coded_parts(view)?;
    let mut o = serde_json::Map::new();
    o.insert("_type".into(), json!(ty));
    if let Some(v) = view
        .suffix("value")
        .cloned()
        .or(terse_value.map(Value::String))
    {
        o.insert("value".into(), v);
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
    let (code, terminology, _) = coded_parts(view)?;
    Some(code_phrase_obj(
        code,
        &terminology,
        view.suffix("preferred_term").cloned(),
    ))
}

/// The coded parts of a coded leaf: the regular suffixed form
/// (`|code`/`|terminology`), or the SDT path+terse string
/// `"terminology::code|value|"` / `"terminology::code"` (SIM-B master04
/// `S_DV_CODED_TEXT` section — the same shape the Better FLAT dialect accepts).
fn coded_parts(view: &FlatView) -> Option<(Value, String, Option<String>)> {
    if let Some(code) = view.suffix("code") {
        let terminology = view
            .suffix("terminology")
            .and_then(Value::as_str)
            .unwrap_or("local")
            .to_owned();
        return Some((code.clone(), terminology, None));
    }
    let (terminology, code, value) = parse_terse_coded(view.bare()?.as_str()?)?;
    Some((Value::String(code), terminology, value))
}

/// Parse the SDT terse coded form: `terminology::code|value|` (trailing pipe
/// optional) or the value-less `terminology::code`.
fn parse_terse_coded(s: &str) -> Option<(String, String, Option<String>)> {
    let (terminology, rest) = s.split_once("::")?;
    if terminology.is_empty() || rest.is_empty() {
        return None;
    }
    match rest.split_once('|') {
        Some((code, tail)) => {
            if code.is_empty() {
                return None;
            }
            let value = tail.strip_suffix('|').unwrap_or(tail);
            Some((
                terminology.to_owned(),
                code.to_owned(),
                Some(value.to_owned()),
            ))
        }
        None => Some((terminology.to_owned(), rest.to_owned(), None)),
    }
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

/// Apply the shared `DV_AMOUNT`/`DV_ORDERED` reverse extras onto `o`:
/// `|magnitude_status`, `|accuracy`, `|accuracy_is_percent`, and `|normal_status`
/// (rebuilt as a `CODE_PHRASE` in the implicit `openehr` terminology, master05
/// note). `with_accuracy` is false for the temporal family (its `accuracy` is a
/// `DV_DURATION` carried via the `/_accuracy` sub-path).
fn apply_quantified_extras(
    o: &mut serde_json::Map<String, Value>,
    view: &FlatView,
    with_accuracy: bool,
) {
    if let Some(v) = view.suffix("magnitude_status") {
        o.insert("magnitude_status".into(), v.clone());
    }
    if with_accuracy {
        if let Some(v) = view.suffix("accuracy") {
            o.insert("accuracy".into(), v.clone());
        }
        if let Some(v) = view.suffix("accuracy_is_percent") {
            o.insert("accuracy_is_percent".into(), v.clone());
        }
    }
    if let Some(code) = view.suffix("normal_status").and_then(Value::as_str) {
        o.insert(
            "normal_status".into(),
            code_phrase_obj(json!(code), "openehr", None),
        );
    }
}

fn quantity_from_flat(view: &FlatView) -> Option<Value> {
    let magnitude = view.suffix("magnitude")?.clone();
    let mut o = serde_json::Map::new();
    o.insert("_type".into(), json!("DV_QUANTITY"));
    o.insert("magnitude".into(), magnitude);
    for (suffix, field) in [("unit", "units"), ("precision", "precision")] {
        if let Some(v) = view.suffix(suffix) {
            o.insert(field.into(), v.clone());
        }
    }
    apply_quantified_extras(&mut o, view, true);
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
    apply_quantified_extras(&mut o, view, true);
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
    if let Some(p) = view.suffix("precision") {
        o.insert("precision".into(), p.clone());
    }
    // The bare value is the computed magnitude ("calculated on output"); it is
    // derived from numerator/denominator on read, so it is not stored back.
    apply_quantified_extras(&mut o, view, true);
    Some(Value::Object(o))
}

/// A temporal leaf (`DV_DATE`/`DV_DATE_TIME`/`DV_TIME`) or `DV_DURATION` — bare
/// value plus the `DV_ORDERED`/`DV_AMOUNT` extras. `with_accuracy` distinguishes
/// the numeric-accuracy `DV_DURATION` from the temporal family whose accuracy is
/// a `/_accuracy` DV_DURATION sub-path (master05 §§DV_DATE/DATE_TIME/TIME/DURATION).
fn temporal_from_flat(view: &FlatView, ty: &str, with_accuracy: bool) -> Option<Value> {
    let value = view.bare()?.clone();
    let mut o = serde_json::Map::new();
    o.insert("_type".into(), json!(ty));
    o.insert("value".into(), value);
    apply_quantified_extras(&mut o, view, with_accuracy);
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
    if let Some(ca) = view.suffix("compression_algorithm").and_then(Value::as_str) {
        // ValueSet `openehr_compression_algorithms` (master05 §DV_MULTIMEDIA).
        o.insert(
            "compression_algorithm".into(),
            code_phrase_obj(json!(ca), "openehr_compression_algorithms", None),
        );
    }
    if let Some(ic) = view.suffix("integrity_check") {
        o.insert("integrity_check".into(), ic.clone());
    }
    if let Some(ica) = view
        .suffix("integrity_check_algorithm")
        .and_then(Value::as_str)
    {
        o.insert(
            "integrity_check_algorithm".into(),
            code_phrase_obj(json!(ica), "openehr_integrity_check_algorithms", None),
        );
    }
    // Inline `data` is base64 text (Array<Octet> in canonical JSON).
    if let Some(d) = view.suffix("data") {
        o.insert("data".into(), d.clone());
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
    // Requires at least a media_type, uri or inline data to be a meaningful leaf.
    if o.contains_key("media_type") || o.contains_key("uri") || o.contains_key("data") {
        Some(Value::Object(o))
    } else {
        None
    }
}

fn bare_typed(view: &FlatView, ty: &str) -> Option<Value> {
    let value = view.bare()?.clone();
    Some(json!({"_type": ty, "value": value}))
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;
    use crate::flat::sub::{Entry, parse_key};

    fn view_of(keys: &[(&str, Value)]) -> Vec<Entry> {
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

    // DV_QUANTITY secondary attributes (master05 §DV_QUANTITY).
    #[test]
    fn quantity_extras_roundtrip() {
        let dv = json!({
            "_type": "DV_QUANTITY", "magnitude": 65.9, "units": "unit",
            "precision": 1, "magnitude_status": "~", "accuracy": 50.5,
            "accuracy_is_percent": true,
            "normal_status": {"_type": "CODE_PHRASE",
                "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "openehr"},
                "code_string": "N"}
        });
        let mut out = FlatMap::new();
        leaf_to_flat(&dv, "DV_QUANTITY", "q", None, &mut out);
        assert_eq!(out.get("q|accuracy"), Some(&json!(50.5)));
        assert_eq!(out.get("q|accuracy_is_percent"), Some(&json!(true)));
        assert_eq!(out.get("q|normal_status"), Some(&json!("N")));
        assert_eq!(out.get("q|magnitude_status"), Some(&json!("~")));

        let es = view_of(&[
            ("magnitude", json!(65.9)),
            ("unit", json!("unit")),
            ("accuracy", json!(50.5)),
            ("accuracy_is_percent", json!(true)),
            ("normal_status", json!("N")),
            ("magnitude_status", json!("~")),
        ]);
        let dv2 = leaf_from_flat("DV_QUANTITY", &FlatView::new(&es)).unwrap();
        assert_eq!(dv2["accuracy"], json!(50.5));
        assert_eq!(dv2["normal_status"]["code_string"], json!("N"));
        assert_eq!(
            dv2["normal_status"]["terminology_id"]["value"],
            json!("openehr")
        );
    }

    // DV_PROPORTION — precision + computed magnitude on output.
    #[test]
    fn proportion_computed_magnitude() {
        let dv = json!({"_type": "DV_PROPORTION", "numerator": 20.5,
            "denominator": 12.4, "type": 0, "precision": 1});
        let mut out = FlatMap::new();
        leaf_to_flat(&dv, "DV_PROPORTION", "p", None, &mut out);
        assert_eq!(out.get("p|numerator"), Some(&json!(20.5)));
        assert_eq!(out.get("p|precision"), Some(&json!(1)));
        // Bare computed magnitude = numerator / denominator.
        let mag = out.get("p").and_then(Value::as_f64).unwrap();
        assert!((mag - 20.5 / 12.4).abs() < 1e-9);
    }

    // DV_MULTIMEDIA full attribute set (master05 §DV_MULTIMEDIA).
    #[test]
    fn multimedia_full_roundtrip() {
        let dv = json!({
            "_type": "DV_MULTIMEDIA",
            "uri": {"_type": "DV_URI", "value": "http://x/s"},
            "media_type": {"_type": "CODE_PHRASE",
                "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "IANA_media-types"},
                "code_string": "video/H261"},
            "size": 504,
            "compression_algorithm": {"_type": "CODE_PHRASE",
                "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "openehr_compression_algorithms"},
                "code_string": "zlib"},
            "integrity_check": "abcd",
            "integrity_check_algorithm": {"_type": "CODE_PHRASE",
                "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "openehr_integrity_check_algorithms"},
                "code_string": "SHA-256"},
            "data": "Z2hn"
        });
        let mut out = FlatMap::new();
        leaf_to_flat(&dv, "DV_MULTIMEDIA", "m", None, &mut out);
        assert_eq!(out.get("m|compression_algorithm"), Some(&json!("zlib")));
        assert_eq!(out.get("m|integrity_check"), Some(&json!("abcd")));
        assert_eq!(
            out.get("m|integrity_check_algorithm"),
            Some(&json!("SHA-256"))
        );
        assert_eq!(out.get("m|data"), Some(&json!("Z2hn")));

        let es = view_of(&[
            ("", json!("http://x/s")),
            ("mediatype", json!("video/H261")),
            ("size", json!(504)),
            ("compression_algorithm", json!("zlib")),
            ("integrity_check", json!("abcd")),
            ("integrity_check_algorithm", json!("SHA-256")),
            ("data", json!("Z2hn")),
        ]);
        let dv2 = leaf_from_flat("DV_MULTIMEDIA", &FlatView::new(&es)).unwrap();
        assert_eq!(dv2["compression_algorithm"]["code_string"], json!("zlib"));
        assert_eq!(dv2["integrity_check"], json!("abcd"));
        assert_eq!(dv2["data"], json!("Z2hn"));
    }

    // `|raw` canonical-JSON bypass (master02/master04 §"Raw canonical JSON").
    #[test]
    fn raw_bypass_returns_verbatim() {
        // The FLAT key is `<path>|raw`; here the leaf-relative key is just `|raw`.
        let es = view_of(&[(
            "|raw",
            json!({"_type": "DV_QUANTITY", "magnitude": 120, "unit": "mm[Hg]"}),
        )]);
        let dv = leaf_from_flat("DV_TEXT", &FlatView::new(&es)).unwrap();
        assert_eq!(dv["_type"], json!("DV_QUANTITY"));
        assert_eq!(dv["magnitude"], json!(120));
    }

    // a `|raw` value without `_type` is ignored (falls through to a leaf).
    #[test]
    fn raw_without_type_ignored() {
        let es = view_of(&[("|raw", json!({"magnitude": 1})), ("", json!("plain"))]);
        let dv = leaf_from_flat("DV_TEXT", &FlatView::new(&es)).unwrap();
        assert_eq!(dv["_type"], json!("DV_TEXT"));
        assert_eq!(dv["value"], json!("plain"));
    }

    // temporal family magnitude_status/normal_status.
    #[test]
    fn datetime_extras() {
        let dv = json!({"_type": "DV_DATE_TIME", "value": "2022-01-12T13:22:34Z",
            "magnitude_status": "~",
            "normal_status": {"_type": "CODE_PHRASE",
                "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "openehr"},
                "code_string": "N"}});
        let mut out = FlatMap::new();
        leaf_to_flat(&dv, "DV_DATE_TIME", "d", None, &mut out);
        assert_eq!(out.get("d"), Some(&json!("2022-01-12T13:22:34Z")));
        assert_eq!(out.get("d|normal_status"), Some(&json!("N")));
        let es = view_of(&[
            ("", json!("2022-01-12T13:22:34Z")),
            ("normal_status", json!("N")),
        ]);
        let dv2 = leaf_from_flat("DV_DATE_TIME", &FlatView::new(&es)).unwrap();
        assert_eq!(dv2["normal_status"]["code_string"], json!("N"));
    }

    // G-7 emit gating: a DV_TEXT in a CLOSED coded slot is not `|other`.
    #[test]
    fn other_gated_on_open_slot() {
        let dv = json!({"_type": "DV_TEXT", "value": "free text"});
        let mut open = FlatMap::new();
        leaf_to_flat(&dv, "DV_CODED_TEXT", "c", Some(true), &mut open);
        assert_eq!(open.get("c|other"), Some(&json!("free text")));

        let mut closed = FlatMap::new();
        leaf_to_flat(&dv, "DV_CODED_TEXT", "c", Some(false), &mut closed);
        assert_eq!(closed.get("c|other"), None);
        assert_eq!(closed.get("c"), Some(&json!("free text")));
    }
}
