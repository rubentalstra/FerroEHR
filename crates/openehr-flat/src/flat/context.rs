//! `ctx/` composition-context shortcuts (Better `CtxConstants` / `EHRbase` Context).
//!
//! On RM→flat the composition-level context that the web-template does not model
//! as tree nodes (language, territory, composer, `EVENT_CONTEXT` start-time /
//! setting) is emitted as `ctx/…` keys; on flat→RM those keys (with Better's
//! defaults — `time` = now, `setting` = openEHR `238` "other care") rebuild the
//! mandatory RM context so the composition is schema-valid.
//!
//! Covered here: `language`, `territory`, `composer_name` / `composer_id` /
//! `composer_self` / `id_namespace`, `time`, `end_time`, `setting`. Not yet:
//! participations, health-care facility, location, provider, workflow id,
//! action/instruction context (recorded as `TODO(port)`).

use serde_json::{Map, Value, json};

use super::mappers::FlatMap;

/// Better's context defaults (`ConversionContext.Builder`).
const DEFAULT_TIME: &str = "1970-01-01T00:00:00Z";
const DEFAULT_SETTING_CODE: &str = "238";
const DEFAULT_SETTING_VALUE: &str = "other care";
const DEFAULT_SETTING_TERM: &str = "openehr";

fn non_empty_str(v: Option<&Value>) -> Option<&str> {
    v.and_then(Value::as_str).filter(|s| !s.is_empty())
}

fn code_phrase(terminology: &str, code: &str) -> Value {
    json!({
        "_type": "CODE_PHRASE",
        "terminology_id": {"_type": "TERMINOLOGY_ID", "value": terminology},
        "code_string": code,
    })
}

/// Emit the `ctx/…` keys for a composition's context.
pub(crate) fn emit_ctx(comp: &Value, out: &mut FlatMap) {
    if let Some(code) = comp.pointer("/language/code_string") {
        out.insert("ctx/language".to_owned(), code.clone());
    }
    if let Some(code) = comp.pointer("/territory/code_string") {
        out.insert("ctx/territory".to_owned(), code.clone());
    }
    match comp.get("composer") {
        Some(c) if c.get("_type").and_then(Value::as_str) == Some("PARTY_SELF") => {
            out.insert("ctx/composer_self".to_owned(), json!(true));
        }
        Some(c) => {
            if let Some(name) = c.get("name").filter(|v| !v.is_null()) {
                out.insert("ctx/composer_name".to_owned(), name.clone());
            }
            if let Some(id) = non_empty_str(c.pointer("/external_ref/id/value")) {
                out.insert("ctx/composer_id".to_owned(), json!(id));
            }
            if let Some(ns) = non_empty_str(c.pointer("/external_ref/namespace")) {
                out.insert("ctx/id_namespace".to_owned(), json!(ns));
            }
        }
        None => {}
    }
    // `EVENT_CONTEXT` start-time + setting are RM-mandatory and never tree nodes,
    // so they are always surfaced via ctx/ (default-filled when the source lacks
    // them, matching `apply_ctx`, so the round-trip is stable).
    let ctx = comp.get("context");
    let time = ctx
        .and_then(|c| c.pointer("/start_time/value"))
        .cloned()
        .unwrap_or_else(|| json!(DEFAULT_TIME));
    out.insert("ctx/time".to_owned(), time);
    if let Some(t) = ctx.and_then(|c| c.pointer("/end_time/value")) {
        out.insert("ctx/end_time".to_owned(), t.clone());
    }
    let setting = ctx.and_then(|c| c.get("setting")).filter(|v| !v.is_null());
    let code = setting
        .and_then(|s| s.pointer("/defining_code/code_string"))
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_SETTING_CODE);
    let value = setting
        .and_then(|s| s.get("value"))
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_SETTING_VALUE);
    let term = setting
        .and_then(|s| s.pointer("/defining_code/terminology_id/value"))
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_SETTING_TERM);
    out.insert("ctx/setting|code".to_owned(), json!(code));
    out.insert("ctx/setting|value".to_owned(), json!(value));
    out.insert("ctx/setting|terminology".to_owned(), json!(term));
}

/// Read a `ctx/<name>` value from the FLAT input.
fn ctx_get<'a>(flat: &'a Map<String, Value>, name: &str) -> Option<&'a Value> {
    flat.get(&format!("ctx/{name}"))
}

/// Apply the `ctx/…` keys (and defaults) onto a composition object, filling the
/// mandatory `language` / `territory` / `composer` / `context`.
pub(crate) fn apply_ctx(flat: &Map<String, Value>, comp: &mut Map<String, Value>) {
    // language / territory
    let lang = ctx_get(flat, "language")
        .and_then(Value::as_str)
        .unwrap_or("en");
    comp.entry("language".to_owned())
        .or_insert_with(|| code_phrase("ISO_639-1", lang));
    if let Some(terr) = ctx_get(flat, "territory").and_then(Value::as_str) {
        comp.entry("territory".to_owned())
            .or_insert_with(|| code_phrase("ISO_3166-1", terr));
    }

    // composer
    if !comp.contains_key("composer") {
        let composer = if ctx_get(flat, "composer_self")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            json!({"_type": "PARTY_SELF"})
        } else {
            let mut c = Map::new();
            c.insert("_type".into(), json!("PARTY_IDENTIFIED"));
            if let Some(name) = ctx_get(flat, "composer_name") {
                c.insert("name".into(), name.clone());
            }
            let id = ctx_get(flat, "composer_id");
            let namespace = ctx_get(flat, "id_namespace");
            if id.is_some() || namespace.is_some() {
                c.insert(
                    "external_ref".into(),
                    json!({
                        "_type": "PARTY_REF",
                        "id": {
                            "_type": "GENERIC_ID",
                            "value": id.and_then(Value::as_str).unwrap_or(""),
                            "scheme": ctx_get(flat, "id_scheme").and_then(Value::as_str).unwrap_or("id_scheme"),
                        },
                        "namespace": namespace.and_then(Value::as_str).unwrap_or("EHR"),
                        "type": "PERSON",
                    }),
                );
            }
            // A PARTY_IDENTIFIED needs at least a name or an external ref.
            if !c.contains_key("name") && !c.contains_key("external_ref") {
                c.insert("name".into(), json!("openEHR"));
            }
            Value::Object(c)
        };
        comp.insert("composer".to_owned(), composer);
    }

    // context
    if !comp.contains_key("context") {
        let time = ctx_get(flat, "time")
            .cloned()
            .unwrap_or_else(|| json!(DEFAULT_TIME));
        let mut ctx = Map::new();
        ctx.insert("_type".into(), json!("EVENT_CONTEXT"));
        ctx.insert(
            "start_time".into(),
            json!({"_type": "DV_DATE_TIME", "value": time}),
        );
        if let Some(end) = ctx_get(flat, "end_time") {
            ctx.insert(
                "end_time".into(),
                json!({"_type": "DV_DATE_TIME", "value": end}),
            );
        }
        ctx.insert("setting".into(), setting_from_ctx(flat));
        comp.insert("context".to_owned(), Value::Object(ctx));
    }
}

fn setting_from_ctx(flat: &Map<String, Value>) -> Value {
    let code = ctx_get(flat, "setting|code")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_SETTING_CODE);
    let value = ctx_get(flat, "setting|value")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_SETTING_VALUE);
    let term = ctx_get(flat, "setting|terminology")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_SETTING_TERM);
    json!({
        "_type": "DV_CODED_TEXT",
        "value": value,
        "defining_code": code_phrase(term, code),
    })
}
