//! `ctx/` composition-context shortcuts (`EHRbase` Simplified-data-template
//! Context; Better `CtxConstants` / `CtxSetter`).
//!
//! On RM→flat the composition-level context the web-template does not model as
//! tree nodes is emitted as `ctx/…` keys; on flat→RM those keys (with the
//! documented defaults — `time` = epoch, `setting` = openEHR `238` "other
//! care") rebuild the mandatory RM context so the composition is schema-valid.
//!
//! Two families of shortcut (per the `EHRbase` Context doc):
//!
//! * **Bidirectional** (composition-`context` level): `language`, `territory`,
//!   `composer_*`, `id_namespace`/`id_scheme`, `time`, `end_time`, `setting`,
//!   **`participation_*`**, **`health_care_facility`**, **`location`** — emitted
//!   by [`emit_ctx`] and rebuilt by [`apply_ctx`], round-trip stable.
//! * **Input defaults** (per-`ENTRY`/structural, applied by [`apply_ctx`] only —
//!   on output these live in their structural positions, so emitting them as
//!   `ctx/` would be ambiguous): `provider_name`/`provider_id`, `work_flow_id`,
//!   `instruction_narrative`, `action_ism_transition_current_state`,
//!   `activity_timing`, `history_origin`. A FLAT body may set them; they fill
//!   the matching RM field on every entry that lacks it.

use serde_json::{Map, Value, json};

use super::defaults::{
    DEFAULT_SETTING_CODE, DEFAULT_SETTING_TERM, DEFAULT_SETTING_VALUE, DEFAULT_TIME,
};
// One shared `CODE_PHRASE` builder (F-13-22): `graph` owns the canonical RM-node
// JSON builders; this module reuses it rather than re-inlining the shape.
use super::graph::code_phrase;
use super::mappers::FlatMap;

fn non_empty_str(v: Option<&Value>) -> Option<&str> {
    v.and_then(Value::as_str).filter(|s| !s.is_empty())
}

// ── RM → flat ───────────────────────────────────────────────────────────────

/// Emit the `ctx/…` keys for a composition's context.
///
/// The bidirectional shortcuts (`SM/.../app_context.adoc`) are symmetric with
/// [`apply_ctx`]: `language`/`territory`/`composer_*`/`id_namespace`/`id_scheme`,
/// `time`/`end_time`/`setting`, `location`/`health_care_facility`, and the
/// indexed `participation_*` family (`_name`/`_function`/`_mode`/`_id`, the
/// `_identifiers` and a `PARTY_RELATED` `_relationship`). COMPOSITION-level
/// `links` are not a `ctx/` key: `COMPOSITION` is a `LOCATABLE`
/// (`RM/.../common/locatable.adoc`), so its links round-trip through the
/// `_link:i` RM-attribute family (`super::rmattr`), not `ctx/`. The per-`ENTRY`
/// input-only defaults (`provider_*`, `work_flow_id`, `history_origin`,
/// `activity_timing`, `instruction_narrative`,
/// `action_ism_transition_current_state`) stay input-only by design — the spec
/// frames them as input shortcuts and their output home is the structural
/// position, so emitting them as `ctx/` would be ambiguous.
#[allow(clippy::too_many_lines)] // one linear emitter over the context field set
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
            // `id_scheme` (`app_context.adoc`): the composer's `GENERIC_ID.scheme`.
            if let Some(sch) = non_empty_str(c.pointer("/external_ref/id/scheme")) {
                out.insert("ctx/id_scheme".to_owned(), json!(sch));
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

    // context.location
    if let Some(loc) = ctx.and_then(|c| c.get("location")).filter(|v| !v.is_null()) {
        out.insert("ctx/location".to_owned(), loc.clone());
    }
    // context.health_care_facility (PARTY_IDENTIFIED)
    if let Some(hcf) = ctx.and_then(|c| c.get("health_care_facility")) {
        if let Some(name) = hcf.get("name").filter(|v| !v.is_null()) {
            out.insert("ctx/health_care_facility|name".to_owned(), name.clone());
        }
        if let Some(id) = non_empty_str(hcf.pointer("/external_ref/id/value")) {
            out.insert("ctx/health_care_facility|id".to_owned(), json!(id));
        }
    }
    // context.participations[]
    if let Some(parts) = ctx
        .and_then(|c| c.get("participations"))
        .and_then(Value::as_array)
    {
        for (i, p) in parts.iter().enumerate() {
            if let Some(name) = p.pointer("/performer/name").filter(|v| !v.is_null()) {
                out.insert(format!("ctx/participation_name:{i}"), name.clone());
            }
            if let Some(f) = p.pointer("/function/value").filter(|v| !v.is_null()) {
                out.insert(format!("ctx/participation_function:{i}"), f.clone());
            }
            if let Some(m) = p.pointer("/mode/value").filter(|v| !v.is_null()) {
                out.insert(format!("ctx/participation_mode:{i}"), m.clone());
            }
            if let Some(id) = non_empty_str(p.pointer("/performer/external_ref/id/value")) {
                out.insert(format!("ctx/participation_id:{i}"), json!(id));
            }
            // `participation_identifiers` (`app_context.adoc`): the performer's
            // `PARTY_IDENTIFIED.identifiers[*].id` (List<String>). Rare — no
            // corpus fixture carries them — but surfaced for symmetry.
            if let Some(ids) = p
                .pointer("/performer/identifiers")
                .and_then(Value::as_array)
            {
                for (j, ident) in ids.iter().enumerate() {
                    if let Some(v) = ident.get("id").filter(|v| !v.is_null()) {
                        out.insert(format!("ctx/participation_identifiers:{i}.{j}"), v.clone());
                    }
                }
            }
            // A `PARTY_RELATED` performer's `relationship` (DV_CODED_TEXT),
            // emitted coded as three indexed-scalar keys (the `name:index` shape
            // the STRUCTURED nester round-trips), so the reverse rebuilds the
            // same relationship.
            if let Some(rel) = p
                .pointer("/performer/relationship")
                .filter(|v| !v.is_null())
            {
                if let Some(code) = rel.pointer("/defining_code/code_string") {
                    out.insert(
                        format!("ctx/participation_relationship_code:{i}"),
                        code.clone(),
                    );
                }
                if let Some(v) = rel.get("value").filter(|v| !v.is_null()) {
                    out.insert(
                        format!("ctx/participation_relationship_value:{i}"),
                        v.clone(),
                    );
                }
                if let Some(t) = rel.pointer("/defining_code/terminology_id/value") {
                    out.insert(
                        format!("ctx/participation_relationship_terminology:{i}"),
                        t.clone(),
                    );
                }
            }
        }
    }
}

// ── flat → RM ─────────────────────────────────────────────────────────────────

/// Read a `ctx/<name>` value from the FLAT input.
fn ctx_get<'a>(flat: &'a Map<String, Value>, name: &str) -> Option<&'a Value> {
    flat.get(&format!("ctx/{name}"))
}

/// Whether the FLAT input carries explicit Event-context *content* — a
/// participation, location, health-care facility, or an end-time — beyond the
/// always-emitted `ctx/time` + `ctx/setting` defaults. Used to decide whether a
/// persistent Composition (which need carry no context; RM ehr
/// `master05-composition_package.adoc` §"Persistent Compositions") should still
/// get a context built because the caller genuinely supplied one.
fn has_explicit_context_content(flat: &Map<String, Value>) -> bool {
    flat.keys().any(|k| {
        k == "ctx/end_time"
            || k == "ctx/location"
            || k.starts_with("ctx/health_care_facility")
            || k.starts_with("ctx/participation_")
    })
}

/// The composer `external_ref.id` OBJECT_ID. A `ctx/id_scheme` makes it a
/// `GENERIC_ID` (the scheme-bearing id, `app_context.adoc` `id_scheme`);
/// without one it is a `HIER_OBJECT_ID` (no scheme) — so a HIER_OBJECT_ID
/// composer round-trips as itself rather than gaining a fabricated scheme.
fn composer_object_id(flat: &Map<String, Value>, id: Option<&Value>) -> Value {
    let value = id.and_then(Value::as_str).unwrap_or("");
    match ctx_get(flat, "id_scheme").and_then(Value::as_str) {
        Some(scheme) => json!({"_type": "GENERIC_ID", "value": value, "scheme": scheme}),
        None => json!({"_type": "HIER_OBJECT_ID", "value": value}),
    }
}

/// Apply the `ctx/…` keys (and defaults) onto a composition object, filling the
/// mandatory `language` / `territory` / `composer` / `context` and the
/// per-`ENTRY` input defaults.
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
                        "id": composer_object_id(flat, id),
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

    // context — `COMPOSITION.context` is optional (0..1;
    // `RM/.../ehr/composition.adoc` §Attributes). A `431|persistent|`
    // Composition idiomatically carries NO Event context — "Persistent
    // Compositions may optionally have an Event context. In openEHR releases up
    // to 1.0.3, Persistent Compositions had no Event context. This was relaxed in
    // subsequent releases…" (RM ehr `master05-composition_package.adoc`
    // §"Persistent Compositions"; the pre-1.0.4 invariant forbidding it was
    // removed by SPECRM-52). So a persistent Composition WITH a context is valid,
    // but we must not *fabricate* a default one where the source carried none.
    // We therefore synthesise the context for an event/episodic/other-category
    // Composition (context is expected there), and for a persistent Composition
    // only when it already has an archetyped `other_context` or the FLAT carries
    // explicit context content (participations / location / facility / end_time)
    // — never merely the always-emitted `ctx/time` + `ctx/setting` defaults.
    let persistent = comp
        .get("category")
        .and_then(|c| c.pointer("/defining_code/code_string"))
        .and_then(Value::as_str)
        == Some("431");
    let synthesize_context =
        !persistent || comp.contains_key("context") || has_explicit_context_content(flat);
    if synthesize_context {
        let ctx = comp
            .entry("context".to_owned())
            .or_insert_with(|| json!({"_type": "EVENT_CONTEXT"}));
        if let Value::Object(ctx) = ctx {
            ctx.entry("_type".to_owned())
                .or_insert_with(|| json!("EVENT_CONTEXT"));
            // `EVENT_CONTEXT.start_time`: the `ctx/time` value, or — when unset —
            // the current time (`SM/.../app_context.adoc` `time`: "If not
            // specified current time will be used"), never the epoch. A
            // round-trip always carries `ctx/time` (emit_ctx emits it), so
            // `now()` only materialises for a client FLAT that omits it, and
            // never destabilises `flat ⇄ flat`.
            let time = ctx_get(flat, "time")
                .cloned()
                .unwrap_or_else(|| json!(jiff::Timestamp::now().to_string()));
            ctx.entry("start_time".to_owned())
                .or_insert_with(|| json!({"_type": "DV_DATE_TIME", "value": time}));
            if let Some(end) = ctx_get(flat, "end_time") {
                ctx.entry("end_time".to_owned())
                    .or_insert_with(|| json!({"_type": "DV_DATE_TIME", "value": end}));
            }
            ctx.entry("setting".to_owned())
                .or_insert_with(|| setting_from_ctx(flat));
            if let Some(loc) = ctx_get(flat, "location") {
                ctx.entry("location".to_owned())
                    .or_insert_with(|| loc.clone());
            }
            if let Some(hcf) = health_care_facility_from_ctx(flat) {
                ctx.entry("health_care_facility".to_owned())
                    .or_insert_with(|| hcf);
            }
            let parts = participations_from_ctx(flat);
            if !parts.is_empty() {
                ctx.entry("participations".to_owned())
                    .or_insert_with(|| Value::Array(parts));
            }
        }
    }

    // Per-ENTRY input defaults (only fired when the corresponding ctx key is
    // present; a round-trip FLAT never carries them, so this is a no-op there).
    apply_entry_defaults(flat, comp);
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

/// A `PARTY_IDENTIFIED` from `ctx/health_care_facility|name`/`|id`.
fn health_care_facility_from_ctx(flat: &Map<String, Value>) -> Option<Value> {
    let name = ctx_get(flat, "health_care_facility|name");
    let id = ctx_get(flat, "health_care_facility|id").and_then(Value::as_str);
    if name.is_none() && id.is_none() {
        return None;
    }
    Some(party_identified(flat, name, id, "ORGANISATION"))
}

/// Build a `PARTY_IDENTIFIED` (name + optional `external_ref`) or a `PARTY_SELF`
/// (when neither name nor id is given), honouring `ctx/id_scheme`/`id_namespace`.
fn party_identified(
    flat: &Map<String, Value>,
    name: Option<&Value>,
    id: Option<&str>,
    party_type: &str,
) -> Value {
    let mut p = Map::new();
    if id.is_some() {
        p.insert("_type".into(), json!("PARTY_IDENTIFIED"));
        if let Some(n) = name {
            p.insert("name".into(), n.clone());
        }
        p.insert(
            "external_ref".into(),
            json!({
                "_type": "PARTY_REF",
                "id": {
                    "_type": "GENERIC_ID",
                    "value": id.unwrap_or(""),
                    "scheme": ctx_get(flat, "id_scheme").and_then(Value::as_str).unwrap_or("id_scheme"),
                },
                "namespace": ctx_get(flat, "id_namespace").and_then(Value::as_str).unwrap_or("EHR"),
                "type": party_type,
            }),
        );
    } else if let Some(n) = name {
        p.insert("_type".into(), json!("PARTY_IDENTIFIED"));
        p.insert("name".into(), n.clone());
    } else {
        p.insert("_type".into(), json!("PARTY_SELF"));
    }
    Value::Object(p)
}

/// Rebuild `context.participations` from the indexed `ctx/participation_*` keys.
fn participations_from_ctx(flat: &Map<String, Value>) -> Vec<Value> {
    // Determine how many participations are addressed (max index + 1).
    let mut max_idx: Option<usize> = None;
    for key in flat.keys() {
        let Some(rest) = key.strip_prefix("ctx/participation_") else {
            continue;
        };
        if let Some((_, idx)) = rest.split_once(':') {
            // The index may be followed by `|suffix` (relationship) or `.j`
            // (identifiers) — take the leading digits.
            let num: String = idx.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(i) = num.parse::<usize>() {
                max_idx = Some(max_idx.map_or(i, |m| m.max(i)));
            }
        }
    }
    let Some(max) = max_idx else {
        return Vec::new();
    };

    let get = |field: &str, i: usize| flat.get(&format!("ctx/participation_{field}:{i}"));
    let mut out = Vec::new();
    for i in 0..=max {
        let name = get("name", i);
        let function = get("function", i);
        let mode = get("mode", i);
        let id = get("id", i).and_then(Value::as_str);
        // `participation_identifiers:{i}.{j}` (DV_IDENTIFIER `id`s on the
        // performer) and a `PARTY_RELATED` `participation_relationship_*:{i}` —
        // the symmetric inverse of `emit_ctx`.
        let identifiers = participation_identifiers(flat, i);
        let rel_code = get("relationship_code", i).and_then(Value::as_str);
        if name.is_none()
            && function.is_none()
            && mode.is_none()
            && id.is_none()
            && identifiers.is_empty()
            && rel_code.is_none()
        {
            continue;
        }
        let mut p = Map::new();
        p.insert("_type".into(), json!("PARTICIPATION"));
        p.insert(
            "function".into(),
            json!({"_type": "DV_TEXT", "value": function.and_then(Value::as_str).unwrap_or("")}),
        );
        let mut performer = party_identified(flat, name, id, "PERSON");
        if let Value::Object(pm) = &mut performer {
            if !identifiers.is_empty() {
                // identifiers make it a PARTY_IDENTIFIED even without a name.
                pm.insert("_type".into(), json!("PARTY_IDENTIFIED"));
                pm.insert("identifiers".into(), Value::Array(identifiers));
            }
            if let Some(code) = rel_code {
                // A PARTY_RELATED performer: relationship coded from the openEHR
                // `subject_relationship` group (`participation.adoc`).
                let value = get("relationship_value", i)
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let term = get("relationship_terminology", i)
                    .and_then(Value::as_str)
                    .unwrap_or("openehr");
                pm.insert("_type".into(), json!("PARTY_RELATED"));
                pm.insert(
                    "relationship".into(),
                    json!({
                        "_type": "DV_CODED_TEXT",
                        "value": value,
                        "defining_code": code_phrase(term, code),
                    }),
                );
            }
        }
        p.insert("performer".into(), performer);
        if let Some(m) = mode {
            // PORT NOTE (F-10-07): `ctx/participation_mode` carries a free-text
            // mode value with no code, but `PARTICIPATION.mode` is a
            // `DV_CODED_TEXT` coded from the openEHR `participation_mode` group.
            // We default the code to the group's `193` "not specified" (a valid
            // member) rather than the fabricated, invalid `openehr::0` — so the
            // rebuilt composition passes terminology validation.
            p.insert(
                "mode".into(),
                json!({
                    "_type": "DV_CODED_TEXT",
                    "value": m,
                    "defining_code": code_phrase("openehr", "193"),
                }),
            );
        }
        out.push(Value::Object(p));
    }
    out
}

/// Rebuild a performer's `identifiers` (DV_IDENTIFIER) from the
/// `ctx/participation_identifiers:{i}.{j}` keys (their `id` strings), ordered by
/// `j` (`app_context.adoc` `participation_identifiers`).
fn participation_identifiers(flat: &Map<String, Value>, i: usize) -> Vec<Value> {
    let prefix = format!("ctx/participation_identifiers:{i}.");
    let mut indexed: Vec<(usize, Value)> = Vec::new();
    for (key, value) in flat {
        if let Some(j) = key
            .strip_prefix(&prefix)
            .and_then(|s| s.parse::<usize>().ok())
        {
            indexed.push((j, json!({"_type": "DV_IDENTIFIER", "id": value.clone()})));
        }
    }
    indexed.sort_by_key(|(j, _)| *j);
    indexed.into_iter().map(|(_, v)| v).collect()
}

/// Apply the per-`ENTRY` input-default shortcuts by walking `content`.
fn apply_entry_defaults(flat: &Map<String, Value>, comp: &mut Map<String, Value>) {
    let provider_name = ctx_get(flat, "provider_name");
    let provider_id = ctx_get(flat, "provider_id").and_then(Value::as_str);
    let workflow_id = ctx_get(flat, "work_flow_id").and_then(Value::as_str);
    let narrative = ctx_get(flat, "instruction_narrative").and_then(Value::as_str);
    let ism_state = ctx_get(flat, "action_ism_transition_current_state").and_then(Value::as_str);
    let timing = ctx_get(flat, "activity_timing").and_then(Value::as_str);
    let history_origin = ctx_get(flat, "history_origin");

    let anything = provider_name.is_some()
        || provider_id.is_some()
        || workflow_id.is_some()
        || narrative.is_some()
        || ism_state.is_some()
        || timing.is_some()
        || history_origin.is_some();
    if !anything {
        return;
    }

    let provider = if provider_name.is_some() || provider_id.is_some() {
        Some(party_identified(flat, provider_name, provider_id, "PERSON"))
    } else {
        None
    };
    let workflow_ref = workflow_id.map(|id| {
        json!({
            "_type": "OBJECT_REF",
            "namespace": ctx_get(flat, "id_namespace").and_then(Value::as_str).unwrap_or("EHR"),
            "type": "WORKFLOW",
            "id": {"_type": "GENERIC_ID", "value": id, "scheme": ctx_get(flat, "id_scheme").and_then(Value::as_str).unwrap_or("id_scheme")},
        })
    });

    let ctx = EntryDefaults {
        provider: provider.as_ref(),
        workflow_ref: workflow_ref.as_ref(),
        narrative,
        ism_state,
        timing,
        history_origin,
    };
    if let Some(content) = comp.get_mut("content").and_then(Value::as_array_mut) {
        for item in content.iter_mut() {
            walk_entry_defaults(item, &ctx);
        }
    }
}

/// The per-entry input defaults to apply while walking the content tree.
struct EntryDefaults<'a> {
    provider: Option<&'a Value>,
    workflow_ref: Option<&'a Value>,
    narrative: Option<&'a str>,
    ism_state: Option<&'a str>,
    timing: Option<&'a str>,
    history_origin: Option<&'a Value>,
}

fn walk_entry_defaults(node: &mut Value, ctx: &EntryDefaults<'_>) {
    let Value::Object(m) = node else { return };
    if let Some("OBSERVATION" | "EVALUATION" | "INSTRUCTION" | "ACTION" | "ADMIN_ENTRY") =
        m.get("_type").and_then(Value::as_str)
    {
        if let Some(p) = ctx.provider {
            m.entry("provider".to_owned()).or_insert_with(|| p.clone());
        }
        if let Some(w) = ctx.workflow_ref {
            m.entry("workflow_id".to_owned())
                .or_insert_with(|| w.clone());
        }
    }
    match m.get("_type").and_then(Value::as_str) {
        Some("INSTRUCTION") => {
            if let Some(n) = ctx.narrative {
                m.insert(
                    "narrative".to_owned(),
                    json!({"_type": "DV_TEXT", "value": n}),
                );
            }
        }
        Some("ACTION") => {
            if let Some(s) = ctx.ism_state {
                // PORT NOTE (F-10-07): `ISM_TRANSITION.current_state` is coded
                // from the openEHR `instruction_states` group; default the code
                // to `524` "initial" (matching `graph::fill_structural_mandatory`
                // and `from_flat`, the one source of truth) rather than the
                // invalid, fabricated `openehr::0`.
                m.insert(
                    "ism_transition".to_owned(),
                    json!({"_type": "ISM_TRANSITION", "current_state": {"_type": "DV_CODED_TEXT", "value": s, "defining_code": code_phrase("openehr", "524")}}),
                );
            }
        }
        Some("ACTIVITY") => {
            if let Some(t) = ctx.timing {
                m.entry("timing".to_owned()).or_insert_with(
                    || json!({"_type": "DV_PARSABLE", "value": t, "formalism": "timing"}),
                );
            }
        }
        Some("HISTORY") => {
            if let Some(o) = ctx.history_origin {
                m.insert(
                    "origin".to_owned(),
                    json!({"_type": "DV_DATE_TIME", "value": o}),
                );
            }
        }
        _ => {}
    }
    // Recurse into the structural children that carry entries/activities/events.
    for key in [
        "content",
        "items",
        "activities",
        "events",
        "data",
        "description",
    ] {
        match m.get_mut(key) {
            Some(Value::Array(arr)) => {
                for c in arr.iter_mut() {
                    walk_entry_defaults(c, ctx);
                }
            }
            Some(obj @ Value::Object(_)) => walk_entry_defaults(obj, ctx),
            _ => {}
        }
    }
}
