// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The registered recipe set.
//!
//! This is the ONLY hand-written generation glue in the
//! executor, each entry a committed, seeded, deterministic `row → payload`
//! function whose contract is the digest-pinned document in
//! `corpus/recipes/*.md`. Every recipe here is a registered exception to
//! the data-driven rule and is listed as such in the run report.

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

use serde_json::{Value, json};

use crate::model::case::MatrixCell;

/// A matrix row bound to its column names.
#[derive(Debug, Clone, Copy)]
pub struct BoundRow<'a> {
    /// The matrix column names, in order.
    pub columns: &'a [String],
    /// This row's cells, positional against `columns`.
    pub cells: &'a [MatrixCell],
}

impl BoundRow<'_> {
    /// The cell under a named column.
    #[must_use]
    pub fn cell(&self, column: &str) -> Option<&MatrixCell> {
        self.columns
            .iter()
            .position(|c| c == column)
            .and_then(|i| self.cells.get(i))
    }
}

/// Recipe evaluation error (an interpreter defect, not a conformance
/// outcome — a recipe must be total over its declared rows).
#[derive(Debug, thiserror::Error)]
#[error("recipe {recipe}: {message}")]
pub struct RecipeError {
    recipe: &'static str,
    message: String,
}

fn err(recipe: &'static str, message: impl Into<String>) -> RecipeError {
    RecipeError {
        recipe,
        message: message.into(),
    }
}

/// `ehr_status` — `EHR_STATUS` synthesis from a `create_ehr-main` matrix row
/// (contract: `corpus/recipes/ehr_status.md`).
///
/// Returns `None` when the row declares `ehr_status: absent` (the class-1.a
/// rows: no payload at all).
///
/// # Errors
/// [`RecipeError`] on a row outside the declared matrix shape.
pub fn ehr_status(
    case: &str,
    row: &BoundRow<'_>,
    row_index: usize,
) -> Result<Option<Value>, RecipeError> {
    const NAME: &str = "ehr_status";
    match row.cell("ehr_status") {
        Some(MatrixCell::Absent) => return Ok(None),
        Some(MatrixCell::Provided) => {}
        other => {
            return Err(err(
                NAME,
                format!("ehr_status column must be absent|provided, got {other:?}"),
            ));
        }
    }
    let flag = |column: &str| -> Result<bool, RecipeError> {
        match row.cell(column) {
            Some(MatrixCell::Literal(Value::Bool(b))) => Ok(*b),
            other => Err(err(
                NAME,
                format!("{column} must be a boolean literal, got {other:?}"),
            )),
        }
    };
    let subject = match row.cell("subject") {
        Some(MatrixCell::Provided) => json!({
            "_type": "PARTY_SELF",
            "external_ref": {
                "_type": "PARTY_REF",
                "namespace": "cnf",
                "type": "PERSON",
                "id": { "_type": "GENERIC_ID", "value": format!("subject-{case}-{row_index}"), "scheme": "cnf" }
            }
        }),
        _ => json!({ "_type": "PARTY_SELF" }),
    };
    let mut status = json!({
        "_type": "EHR_STATUS",
        "name": { "_type": "DV_TEXT", "value": "ehr status" },
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
        // An EHR_STATUS is unconditionally an archetype root (RM ehr
        // ehr_status.adoc Is_archetype_root); a root without ARCHETYPED
        // violates RM common locatable.adoc Archetyped_valid, so every
        // valid emitted payload carries the block.
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": {
                "_type": "ARCHETYPE_ID",
                "value": "openEHR-EHR-EHR_STATUS.generic.v1"
            },
            "rm_version": "1.2.0"
        },
        "subject": subject,
        "is_queryable": flag("is_queryable")?,
        "is_modifiable": flag("is_modifiable")?
    });
    if matches!(row.cell("other_details"), Some(MatrixCell::Provided))
        && let Some(map) = status.as_object_mut()
    {
        map.insert(
            "other_details".to_owned(),
            json!({
                "_type": "ITEM_TREE",
                "name": { "_type": "DV_TEXT", "value": "tree" },
                "archetype_node_id": "at0001",
                "items": [{
                    "_type": "ELEMENT",
                    "name": { "_type": "DV_TEXT", "value": "detail" },
                    "archetype_node_id": "at0002",
                    "value": { "_type": "DV_TEXT", "value": "cnf other_details" }
                }]
            }),
        );
    }
    Ok(Some(status))
}

/// The deterministic client `ehr_id` for `ehr_id: provided` rows
/// (contract: pure function of the row index — UUID v4-shaped, seeded from
/// the index so two runners mint identical ids).
#[must_use]
pub fn deterministic_ehr_id(case: &str, row_index: usize) -> String {
    // UUIDv5 over the recipe namespace and "<case>/<row>" — pure and
    // case-scoped, so two runners mint identical ids while distinct cases
    // sharing one SUT never collide (contract: corpus/recipes/ehr_status.md).
    let ns = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, b"cnf.create_ehr");
    uuid::Uuid::new_v5(&ns, format!("{case}/{row_index}").as_bytes()).to_string()
}

/// The deterministic per-row constraint-template id for a content case.
///
/// For a case declaring `constraint_context.constraint_columns` (issue #228)
/// the runner
/// synthesizes one OPT per decision-table row and uploads it under this id, and
/// the row's committed carrier stamps the same id into
/// `archetype_details.template_id`. Pure function of (case id, row, row
/// cells) — two runners mint identical ids, and a re-adjudicated row (or a
/// synthesis change) mints a NEW id, so a 409-tolerant re-upload against a
/// dirty server can never silently reuse a stale OPT with different
/// constraints. (No openEHR spec governs the corpus template packaging —
/// our own corpus-authoring design.)
#[must_use]
pub fn synth_template_id(case_id: &str, row: usize, cells: &[MatrixCell]) -> String {
    const NS: uuid::Uuid = uuid::Uuid::from_bytes([
        0x6f, 0x96, 0x19, 0xff, 0x8b, 0x86, 0xd0, 0x11, 0xb4, 0x2d, 0x00, 0xcf, 0x4f, 0xc9, 0x64,
        0xfe,
    ]);
    let slug: String = case_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    // Content digest of the row cells: UUIDv5 (deterministic) over the
    // encoded cells, truncated for readability.
    let encoded = cells
        .iter()
        .map(|cell| match cell {
            MatrixCell::Absent => "\u{1}absent".to_owned(),
            MatrixCell::Provided => "\u{1}provided".to_owned(),
            MatrixCell::Null => "\u{1}null".to_owned(),
            MatrixCell::Literal(v) => v.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\u{2}");
    let digest = uuid::Uuid::new_v5(&NS, encoded.as_bytes())
        .simple()
        .to_string();
    let short = digest.get(..8).unwrap_or("00000000");
    format!("cnf.tpl.{slug}.r{row}.{short}")
}

/// `bp_series` — the generated blood-pressure corpus (contract:
/// `corpus/recipes/bp_series.md`): composition k has systolic 100+10k,
/// diastolic 60+5k, event time 2026-01-01T00:00:00Z + k hours.
///
/// # Errors
/// [`RecipeError`] when the index is outside the declared set (0..10).
pub fn bp_series(k: usize) -> Result<Value, RecipeError> {
    const NAME: &str = "bp_series";
    if k >= 10 {
        return Err(err(
            NAME,
            format!("index {k} outside the declared set 0..10"),
        ));
    }
    let systolic = 100 + 10 * k;
    let diastolic = 60 + 5 * k;
    let hour = k;
    let time = format!("2026-01-01T{hour:02}:00:00Z");
    Ok(json!({
        "_type": "COMPOSITION",
        "name": { "_type": "DV_TEXT", "value": format!("blood pressure {k}") },
        "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-COMPOSITION.encounter.v1" },
            "template_id": { "_type": "TEMPLATE_ID", "value": "cnf.blood_pressure" },
            "rm_version": "1.0.2"
        },
        "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
        "territory": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_3166-1" }, "code_string": "NL" },
        "category": { "_type": "DV_CODED_TEXT", "value": "event",
            "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "433" } },
        "composer": { "_type": "PARTY_SELF" },
        "context": { "_type": "EVENT_CONTEXT",
            "start_time": { "_type": "DV_DATE_TIME", "value": time },
            "setting": { "_type": "DV_CODED_TEXT", "value": "other care",
                "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "238" } } },
        "content": [{
            "_type": "OBSERVATION",
            "name": { "_type": "DV_TEXT", "value": "Blood pressure" },
            "archetype_node_id": "openEHR-EHR-OBSERVATION.blood_pressure.v2",
            "archetype_details": {
                "_type": "ARCHETYPED",
                "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-OBSERVATION.blood_pressure.v2" },
                "rm_version": "1.0.2"
            },
            "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
            "encoding": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "IANA_character-sets" }, "code_string": "UTF-8" },
            "subject": { "_type": "PARTY_SELF" },
            "data": { "_type": "HISTORY", "name": { "_type": "DV_TEXT", "value": "history" }, "archetype_node_id": "at0001",
                "origin": { "_type": "DV_DATE_TIME", "value": time },
                "events": [{ "_type": "POINT_EVENT", "name": { "_type": "DV_TEXT", "value": "any event" }, "archetype_node_id": "at0006",
                    "time": { "_type": "DV_DATE_TIME", "value": time },
                    "data": { "_type": "ITEM_TREE", "name": { "_type": "DV_TEXT", "value": "blood pressure" }, "archetype_node_id": "at0003",
                        "items": [
                            { "_type": "ELEMENT", "name": { "_type": "DV_TEXT", "value": "Systolic" }, "archetype_node_id": "at0004",
                              "value": { "_type": "DV_QUANTITY", "magnitude": systolic, "units": "mm[Hg]" } },
                            { "_type": "ELEMENT", "name": { "_type": "DV_TEXT", "value": "Diastolic" }, "archetype_node_id": "at0005",
                              "value": { "_type": "DV_QUANTITY", "magnitude": diastolic, "units": "mm[Hg]" } }
                        ] } }] } }]
    }))
}

/// `query_bp` — the AQL-chapter query corpus (contract:
/// `corpus/recipes/query_bp.md`): identical series semantics to
/// [`bp_series`]; separated so the two corpus keys stay independently
/// digest-pinned.
///
/// # Errors
/// As [`bp_series`].
pub fn query_bp(k: usize) -> Result<Value, RecipeError> {
    bp_series(k)
}

/// A decision-table row bound to its column names, with typed accessors that
/// read only the genuine RM *instance* columns (`null`/`absent` cells return
/// `None`, so the omitted attribute makes the RM mandatory check fire).
struct RowView<'a> {
    columns: &'a [String],
    cells: &'a [MatrixCell],
}

impl RowView<'_> {
    fn cell(&self, name: &str) -> Option<&MatrixCell> {
        self.columns
            .iter()
            .position(|c| c == name)
            .and_then(|i| self.cells.get(i))
    }

    /// The literal JSON under a column, or `None` for null/absent/provided.
    fn literal(&self, name: &str) -> Option<&Value> {
        match self.cell(name) {
            Some(MatrixCell::Literal(v)) => Some(v),
            _ => None,
        }
    }

    fn text(&self, name: &str) -> Option<&str> {
        self.literal(name).and_then(Value::as_str)
    }
}

/// `content_instance` — the content-chapter generation recipe.
///
/// Builds the
/// spec-correct RM data-value instance for one decision-table row, injects it
/// at the case's constrained ELEMENT.value in the minimal-event carrier
/// composition, and hands the result to the ordinary commit flow — one
/// executor serves functional and content cases alike.
///
/// A decision table carries two column axes: the *instance* axis (the genuine
/// RM attributes of the value under test) and the *constraint* axis (columns
/// that describe the template's baked constraint, e.g. `C_STRING.pattern`,
/// `range.lower`, `cardinality`). Only the instance axis is projected into the
/// committed value; the constraint axis is the template's job. Each `DV_*`
/// class is built at its correct RM shape (RM `data_types` — `DV_CODED_TEXT`,
/// `DV_ORDINAL`/`DV_SCALE`, `DV_INTERVAL<T>`, `DV_MULTIMEDIA`, `DV_IDENTIFIER`,
/// and the simple leaf types), never as a flat 1:1 column→attribute map.
#[must_use]
pub fn content_instance(
    rm_class: &str,
    template_id: &str,
    columns: &[String],
    cells: &[MatrixCell],
) -> Value {
    let row = RowView { columns, cells };
    if is_structural(rm_class) {
        return structural_instance(rm_class, template_id, &row);
    }
    let value = build_value(rm_class, template_id, &row);
    let mut composition = base_carrier_composition();
    // The committed carrier resolves its template from
    // archetype_details.template_id — stamp the case's constraint template.
    if let Some(tid) = composition.pointer_mut("/archetype_details/template_id/value") {
        *tid = Value::String(template_id.to_owned());
    }
    if let Some(value) = value
        && let Some(items) = composition
            .pointer_mut("/content/0/data/events/0/data/items")
            .and_then(Value::as_array_mut)
        && let Some(element) = items.first_mut()
        && let Some(map) = element.as_object_mut()
    {
        map.insert("value".to_owned(), value);
    }
    composition
}

/// Whether the content case's `rm_class` describes carrier *shape* (a
/// structural family) rather than an ELEMENT.value domain constraint. These are
/// committed as a per-row-shaped carrier by [`structural_instance`] against a
/// per-row synthesized OPT ([`crate::exec::content_synth`]).
fn is_structural(rm_class: &str) -> bool {
    matches!(
        rm_class,
        "COMPOSITION" | "EVENT" | "HISTORY" | "ITEM_STRUCTURE" | "OBSERVATION"
    )
}

/// Build the ELEMENT.value data value for one row, dispatched on the RM class.
///
/// Returns `None` for the structural `rm_class`es (COMPOSITION / EVENT / HISTORY
/// / `ITEM_STRUCTURE` / OBSERVATION), which are handled by
/// [`structural_instance`] instead (carrier-shape projection, not a value).
fn build_value(rm_class: &str, template_id: &str, row: &RowView<'_>) -> Option<Value> {
    let dv = match rm_class {
        "DV_INTERVAL" => build_interval(template_id, row),
        "DV_CODED_TEXT" => build_coded_text(row),
        // RM data_types §DV_ORDINAL / §DV_SCALE — symbol (DV_CODED_TEXT) + value.
        "DV_ORDINAL" | "DV_SCALE" => build_ordinal(rm_class, row),
        "DV_MULTIMEDIA" => build_multimedia(row),
        "DV_IDENTIFIER" => build_identifier(row),
        other if other.starts_with("DV_") => build_simple(other, row),
        _ => return None,
    };
    Some(dv)
}

/// The genuine RM instance attributes of each simple leaf value class
/// (RM `data_types`); every other decision-table column is constraint axis.
fn simple_attrs(rm_class: &str) -> &'static [&'static str] {
    match rm_class {
        "DV_COUNT" => &["magnitude"],
        "DV_QUANTITY" => &["magnitude", "units"],
        "DV_PROPORTION" => &["type", "numerator", "denominator", "precision"],
        "DV_PARSABLE" => &["value", "formalism"],
        // DV_TEXT, DV_BOOLEAN, DV_URI, DV_EHR_URI, DV_DATE, DV_TIME,
        // DV_DATE_TIME, DV_DURATION — single `value` attribute.
        _ => &["value"],
    }
}

fn build_simple(rm_class: &str, row: &RowView<'_>) -> Value {
    let mut map = serde_json::Map::new();
    map.insert("_type".to_owned(), Value::String(rm_class.to_owned()));
    for attr in simple_attrs(rm_class) {
        if let Some(v) = row.literal(attr) {
            map.insert((*attr).to_owned(), v.clone());
        }
    }
    Value::Object(map)
}

/// RM `data_types` §`DV_CODED_TEXT` — `value` (the display text, mandatory) plus a
/// `defining_code` `CODE_PHRASE` (`terminology_id` + `code_string`, both mandatory).
///
/// The `value` axis is a genuine instance column: when the decision table
/// carries a `value` column it drives `DV_CODED_TEXT.value` verbatim — including
/// an empty string, which must fire the sole value invariant (RM
/// `dv_text.adoc` §Invariants — `Valid_value: not value.is_empty`). No RM
/// invariant requires `value` to equal the coded rubric (the "value must be the
/// rubric" text is `dv_coded_text.adoc` Description prose, not an invariant —
/// see AMB-55), so an arbitrary non-empty value is accepted. Absent the column,
/// value defaults to a non-empty placeholder so the value dimension is not
/// exercised on cases that do not test it.
fn build_coded_text(row: &RowView<'_>) -> Value {
    let code = row.text("code_string");
    let term = row.text("terminology_id");
    let mut map = serde_json::Map::new();
    map.insert(
        "_type".to_owned(),
        Value::String("DV_CODED_TEXT".to_owned()),
    );
    if code.is_some() || term.is_some() {
        let mut cp = serde_json::Map::new();
        cp.insert("_type".to_owned(), Value::String("CODE_PHRASE".to_owned()));
        if let Some(t) = term {
            cp.insert("terminology_id".to_owned(), terminology_id(t));
        }
        if let Some(c) = code {
            cp.insert("code_string".to_owned(), Value::String(c.to_owned()));
        }
        let rubric = row.text("value").unwrap_or("coded");
        map.insert("value".to_owned(), Value::String(rubric.to_owned()));
        map.insert("defining_code".to_owned(), Value::Object(cp));
    }
    Value::Object(map)
}

/// RM `data_types` §`DV_ORDINAL` / §`DV_SCALE` — `symbol` (a coded term, given in the
/// table as `terminology::code`) and `value`.
fn build_ordinal(rm_class: &str, row: &RowView<'_>) -> Value {
    let mut map = serde_json::Map::new();
    map.insert("_type".to_owned(), Value::String(rm_class.to_owned()));
    if let Some(sym) = row.text("symbol") {
        map.insert("symbol".to_owned(), coded_symbol(sym));
    }
    if let Some(v) = row.literal("value") {
        map.insert("value".to_owned(), v.clone());
    }
    Value::Object(map)
}

/// RM `data_types` §`DV_MULTIMEDIA` — `media_type` (a `CODE_PHRASE` against the IANA
/// media-type set) and `size`; a `uri` is always attached to satisfy the
/// `Not_empty` invariant (data or uri present).
fn build_multimedia(row: &RowView<'_>) -> Value {
    let mut map = serde_json::Map::new();
    map.insert(
        "_type".to_owned(),
        Value::String("DV_MULTIMEDIA".to_owned()),
    );
    if let Some(mt) = row.text("media_type") {
        let mut cp = serde_json::Map::new();
        cp.insert("_type".to_owned(), Value::String("CODE_PHRASE".to_owned()));
        cp.insert(
            "terminology_id".to_owned(),
            terminology_id("IANA_media-types"),
        );
        cp.insert("code_string".to_owned(), Value::String(mt.to_owned()));
        map.insert("media_type".to_owned(), Value::Object(cp));
    }
    if let Some(sz) = row.literal("size") {
        map.insert("size".to_owned(), sz.clone());
    }
    map.insert(
        "uri".to_owned(),
        json!({ "_type": "DV_URI", "value": "http://cnf.example/media" }),
    );
    Value::Object(map)
}

/// RM `data_types` §`DV_IDENTIFIER` — the `id` attribute is mandatory; the table's
/// `attribute` column names which of issuer/assigner/id/type carries the value
/// under test.
fn build_identifier(row: &RowView<'_>) -> Value {
    let mut map = serde_json::Map::new();
    map.insert(
        "_type".to_owned(),
        Value::String("DV_IDENTIFIER".to_owned()),
    );
    let attribute = row.text("attribute").unwrap_or("id");
    if let Some(v) = row.text("value") {
        map.insert(attribute.to_owned(), Value::String(v.to_owned()));
    }
    // `id` is mandatory; when the value under test targets another field, keep
    // a valid `id` present so the mandatory check does not mask the field test.
    if attribute != "id" {
        map.insert("id".to_owned(), Value::String("cnf-id".to_owned()));
    }
    Value::Object(map)
}

/// RM `data_types` §`DV_INTERVAL` — `lower`/`upper` are full data-value limit
/// objects (never bare scalars) plus the four boundary flags. The inner
/// limit type is taken from the template id (`interval_<inner>_*`).
fn build_interval(template_id: &str, row: &RowView<'_>) -> Value {
    let inner = interval_inner(template_id);
    let mut map = serde_json::Map::new();
    map.insert("_type".to_owned(), Value::String("DV_INTERVAL".to_owned()));
    for flag in [
        "lower_unbounded",
        "upper_unbounded",
        "lower_included",
        "upper_included",
    ] {
        if let Some(v) = row.literal(flag) {
            map.insert(flag.to_owned(), v.clone());
        }
    }
    if let Some(lower) = build_limit(inner, row, "lower") {
        map.insert("lower".to_owned(), lower);
    }
    if let Some(upper) = build_limit(inner, row, "upper") {
        map.insert("upper".to_owned(), upper);
    }
    Value::Object(map)
}

/// The inner limit type of a `DV_INTERVAL<T>` template, from its id
/// (`cnf.tpl.interval_<inner>_*`). `date_time` is matched before `date`/`time`.
fn interval_inner(template_id: &str) -> &'static str {
    if template_id.contains("date_time") {
        "DV_DATE_TIME"
    } else if template_id.contains("_date") {
        "DV_DATE"
    } else if template_id.contains("time") {
        "DV_TIME"
    } else if template_id.contains("duration") {
        "DV_DURATION"
    } else if template_id.contains("quantity") {
        "DV_QUANTITY"
    } else if template_id.contains("ordinal") {
        "DV_ORDINAL"
    } else if template_id.contains("scale") {
        "DV_SCALE"
    } else if template_id.contains("proportion") {
        "DV_PROPORTION"
    } else {
        "DV_COUNT"
    }
}

/// Build one interval limit object for `side` (`lower`/`upper`); `None` when
/// the side is absent (unbounded).
fn build_limit(inner: &str, row: &RowView<'_>, side: &str) -> Option<Value> {
    match inner {
        "DV_ORDINAL" | "DV_SCALE" => {
            let sym = row.text(&format!("{side}_symbol"));
            let val = row.literal(&format!("{side}_value"));
            if sym.is_none() && val.is_none() {
                return None;
            }
            let mut m = serde_json::Map::new();
            m.insert("_type".to_owned(), Value::String(inner.to_owned()));
            if let Some(s) = sym {
                m.insert("symbol".to_owned(), coded_symbol(s));
            }
            if let Some(v) = val {
                m.insert("value".to_owned(), v.clone());
            }
            Some(Value::Object(m))
        }
        "DV_PROPORTION" => {
            let mut m = serde_json::Map::new();
            m.insert(
                "_type".to_owned(),
                Value::String("DV_PROPORTION".to_owned()),
            );
            let mut any = false;
            for (col, attr) in [
                ("type", "type"),
                ("numerator", "numerator"),
                ("denominator", "denominator"),
                ("precision", "precision"),
            ] {
                if let Some(v) = row.literal(&format!("{side}_{col}")) {
                    m.insert(attr.to_owned(), v.clone());
                    any = true;
                }
            }
            any.then_some(Value::Object(m))
        }
        "DV_QUANTITY" => {
            // The table gives the quantity limit as "<magnitude> <units>".
            let text = row.text(side)?;
            let (mag, units) = text.split_once(' ').unwrap_or((text, ""));
            let magnitude = mag
                .parse::<f64>()
                .ok()
                .and_then(serde_json::Number::from_f64)
                .map_or(Value::Null, Value::Number);
            Some(json!({ "_type": "DV_QUANTITY", "magnitude": magnitude, "units": units }))
        }
        "DV_COUNT" => {
            let v = row.literal(side)?;
            Some(json!({ "_type": "DV_COUNT", "magnitude": v.clone() }))
        }
        // Scalar-valued limits (DV_DATE / DV_TIME / DV_DATE_TIME / DV_DURATION).
        _ => {
            let v = row.literal(side)?;
            let mut m = serde_json::Map::new();
            m.insert("_type".to_owned(), Value::String(inner.to_owned()));
            m.insert("value".to_owned(), v.clone());
            Some(Value::Object(m))
        }
    }
}

/// A `terminology::code` cell as a `DV_CODED_TEXT` symbol (RM `data_types`
/// §`DV_ORDINAL` uses a coded-text symbol whose `defining_code` is that pair).
fn coded_symbol(symbol: &str) -> Value {
    let (term, code) = symbol.split_once("::").unwrap_or(("local", symbol));
    json!({
        "_type": "DV_CODED_TEXT",
        "value": code,
        "defining_code": {
            "_type": "CODE_PHRASE",
            "terminology_id": terminology_id(term),
            "code_string": code
        }
    })
}

fn terminology_id(value: &str) -> Value {
    json!({ "_type": "TERMINOLOGY_ID", "value": value })
}

// ---------------------------------------------------------------------------
// Structural content instances — per-row carrier SHAPE (content counts, event
// slot/subtype, existence-committed attributes), committed against the matching
// per-row synthesized OPT (`crate::exec::content_synth`). RM shapes per RM
// `ehr`/`data_structures`/`data_types`; node ids mirror the synthesized OPT.
// ---------------------------------------------------------------------------

/// A committed/absent flag column: `present` ⇒ true, anything else ⇒ false.
fn committed(row: &RowView<'_>, column: &str) -> bool {
    matches!(row.text(column), Some("present"))
}

/// The COMPOSITION carrier honouring the three-state `context_committed`
/// data axis (`absent` | `present` | `present_with_other`); a missing column
/// keeps the context present — the official tables' fixed condition for the
/// context-less families.
fn composition_carrier(template_id: &str, content: Vec<Value>, row: &RowView<'_>) -> Value {
    let token = row.text("context_committed").unwrap_or("present");
    let mut comp = comp_shell(template_id, content, token != "absent");
    if token == "present_with_other"
        && let Some(ctx) = comp.pointer_mut("/context").and_then(Value::as_object_mut)
    {
        ctx.insert(
            "other_context".to_owned(),
            json!({ "_type": "ITEM_TREE", "name": { "_type": "DV_TEXT", "value": "Other context" }, "archetype_node_id": "at0011", "items": [] }),
        );
    }
    comp
}

/// An integer count column (`content_count`/`events_count`); missing ⇒ 0.
fn count(row: &RowView<'_>, column: &str) -> usize {
    row.literal(column)
        .and_then(Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(0)
}

/// ELEMENT at0004 with a valid `DV_TEXT` value.
fn min_element() -> Value {
    json!({
        "_type": "ELEMENT",
        "name": { "_type": "DV_TEXT", "value": "value" },
        "archetype_node_id": "at0004",
        "value": { "_type": "DV_TEXT", "value": "cnf value" }
    })
}

/// `ITEM_TREE` at `node` carrying one value ELEMENT.
fn min_item_tree(node: &str) -> Value {
    json!({
        "_type": "ITEM_TREE",
        "name": { "_type": "DV_TEXT", "value": "tree" },
        "archetype_node_id": node,
        "items": [min_element()]
    })
}

/// An EVENT (or `POINT_EVENT/INTERVAL_EVENT`) at0002; `with_data`/`with_state`
/// gate the optional attributes. `INTERVAL_EVENT` carries its mandatory `width`
/// + `math_function` (RM `data_structures` §`INTERVAL_EVENT`).
fn min_event(slot_type: &str, with_data: bool, with_state: bool) -> Value {
    let mut ev = serde_json::Map::new();
    ev.insert("_type".to_owned(), Value::String(slot_type.to_owned()));
    ev.insert(
        "name".to_owned(),
        json!({ "_type": "DV_TEXT", "value": "any event" }),
    );
    ev.insert(
        "archetype_node_id".to_owned(),
        Value::String("at0002".to_owned()),
    );
    ev.insert(
        "time".to_owned(),
        json!({ "_type": "DV_DATE_TIME", "value": "2026-01-01T00:00:00Z" }),
    );
    if slot_type == "INTERVAL_EVENT" {
        ev.insert(
            "width".to_owned(),
            json!({ "_type": "DV_DURATION", "value": "PT1H" }),
        );
        ev.insert(
            "math_function".to_owned(),
            json!({ "_type": "DV_CODED_TEXT", "value": "mean",
                "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "146" } }),
        );
    }
    if with_data {
        ev.insert("data".to_owned(), min_item_tree("at0003"));
    }
    if with_state {
        ev.insert(
            "state".to_owned(),
            json!({ "_type": "ITEM_TREE", "name": { "_type": "DV_TEXT", "value": "state" }, "archetype_node_id": "at0005", "items": [] }),
        );
    }
    Value::Object(ev)
}

/// OBSERVATION openEHR-EHR-OBSERVATION.minimal.v1 with an explicit events list
/// and optional OBSERVATION-level state (HISTORY at0005) / protocol (at0006).
#[expect(
    clippy::needless_pass_by_value,
    reason = "args are serialized into the JSON tree"
)]
fn min_observation(events: Vec<Value>, state: Option<Value>, protocol: Option<Value>) -> Value {
    let mut obs = serde_json::Map::new();
    obs.insert("_type".to_owned(), Value::String("OBSERVATION".to_owned()));
    obs.insert(
        "name".to_owned(),
        json!({ "_type": "DV_TEXT", "value": "content observation" }),
    );
    obs.insert(
        "archetype_node_id".to_owned(),
        Value::String("openEHR-EHR-OBSERVATION.minimal.v1".to_owned()),
    );
    obs.insert("archetype_details".to_owned(), json!({
        "_type": "ARCHETYPED",
        "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-OBSERVATION.minimal.v1" },
        "rm_version": "1.0.2"
    }));
    obs.insert("language".to_owned(), json!({ "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" }));
    obs.insert("encoding".to_owned(), json!({ "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "IANA_character-sets" }, "code_string": "UTF-8" }));
    obs.insert("subject".to_owned(), json!({ "_type": "PARTY_SELF" }));
    obs.insert("data".to_owned(), json!({
        "_type": "HISTORY", "name": { "_type": "DV_TEXT", "value": "history" }, "archetype_node_id": "at0001",
        "origin": { "_type": "DV_DATE_TIME", "value": "2026-01-01T00:00:00Z" },
        "events": events
    }));
    if let Some(state) = state {
        obs.insert("state".to_owned(), state);
    }
    if let Some(protocol) = protocol {
        obs.insert("protocol".to_owned(), protocol);
    }
    Value::Object(obs)
}

/// The COMPOSITION shell (category/composer/\[context\]) carrying `content`
/// (omitted entirely when empty — COMPOSITION.content existence 0..1, so an
/// absent list is 0 items; a present-but-empty list is an RM error).
fn comp_shell(template_id: &str, content: Vec<Value>, with_context: bool) -> Value {
    let mut comp = serde_json::Map::new();
    comp.insert("_type".to_owned(), Value::String("COMPOSITION".to_owned()));
    comp.insert(
        "name".to_owned(),
        json!({ "_type": "DV_TEXT", "value": "content case carrier" }),
    );
    comp.insert(
        "archetype_node_id".to_owned(),
        Value::String("openEHR-EHR-COMPOSITION.minimal.v1".to_owned()),
    );
    comp.insert("archetype_details".to_owned(), json!({
        "_type": "ARCHETYPED",
        "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-COMPOSITION.minimal.v1" },
        "template_id": { "_type": "TEMPLATE_ID", "value": template_id },
        "rm_version": "1.0.2"
    }));
    comp.insert("language".to_owned(), json!({ "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" }));
    comp.insert("territory".to_owned(), json!({ "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_3166-1" }, "code_string": "NL" }));
    comp.insert("category".to_owned(), json!({ "_type": "DV_CODED_TEXT", "value": "event",
        "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "433" } }));
    comp.insert("composer".to_owned(), json!({ "_type": "PARTY_SELF" }));
    if with_context {
        comp.insert("context".to_owned(), json!({ "_type": "EVENT_CONTEXT",
            "start_time": { "_type": "DV_DATE_TIME", "value": "2026-01-01T00:00:00Z" },
            "setting": { "_type": "DV_CODED_TEXT", "value": "other care",
                "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "238" } } }));
    }
    if !content.is_empty() {
        comp.insert("content".to_owned(), Value::Array(content));
    }
    Value::Object(comp)
}

/// A minimal `ITEM_STRUCTURE` instance of the given subtype (at0003), for the
/// EVALUATION type-narrowing carrier (RM `data_structures` §`ITEM_STRUCTURE`).
fn min_item_structure(rm_type: &str) -> Value {
    let base = json!({
        "_type": rm_type,
        "name": { "_type": "DV_TEXT", "value": "structure" },
        "archetype_node_id": "at0003"
    });
    let mut map = base.as_object().cloned().unwrap_or_default();
    // ITEM_SINGLE.item is mandatory (1..1); ITEM_TABLE.rows / ITEM_LIST.items /
    // ITEM_TREE.items are existence 0..1 — omit them for a minimal valid structure.
    if rm_type == "ITEM_SINGLE" {
        map.insert("item".to_owned(), min_element());
    }
    Value::Object(map)
}

/// An EVALUATION openEHR-EHR-EVALUATION.minimal.v1 carrying the given data.
#[expect(
    clippy::needless_pass_by_value,
    reason = "`data` is serialized into the JSON tree"
)]
fn min_evaluation(data: Value) -> Value {
    json!({
        "_type": "EVALUATION",
        "name": { "_type": "DV_TEXT", "value": "content evaluation" },
        "archetype_node_id": "openEHR-EHR-EVALUATION.minimal.v1",
        "archetype_details": { "_type": "ARCHETYPED", "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-EVALUATION.minimal.v1" }, "rm_version": "1.0.2" },
        "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
        "encoding": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "IANA_character-sets" }, "code_string": "UTF-8" },
        "subject": { "_type": "PARTY_SELF" },
        "data": data
    })
}

/// Build the per-row structural carrier for one content row.
fn structural_instance(rm_class: &str, template_id: &str, row: &RowView<'_>) -> Value {
    match rm_class {
        "COMPOSITION" if row.cell("cardinality").is_some() => {
            let n = count(row, "content_count");
            let content: Vec<Value> = (0..n)
                .map(|_| min_observation(vec![min_event("POINT_EVENT", true, false)], None, None))
                .collect();
            composition_carrier(template_id, content, row)
        }
        "COMPOSITION" => {
            let obs = min_observation(vec![min_event("POINT_EVENT", true, false)], None, None);
            composition_carrier(template_id, vec![obs], row)
        }
        "EVENT" if row.cell("slot_type").is_some() => {
            let committed_type = row.text("committed_type").unwrap_or("POINT_EVENT");
            let obs = min_observation(vec![min_event(committed_type, true, false)], None, None);
            comp_shell(template_id, vec![obs], true)
        }
        "EVENT" => {
            let event = min_event(
                "POINT_EVENT",
                committed(row, "data_committed"),
                committed(row, "state_committed"),
            );
            let obs = min_observation(vec![event], None, None);
            comp_shell(template_id, vec![obs], true)
        }
        "HISTORY" if row.cell("cardinality").is_some() => {
            let n = count(row, "events_count");
            let events: Vec<Value> = (0..n)
                .map(|_| min_event("POINT_EVENT", true, false))
                .collect();
            let mut obs = min_observation(events, None, None);
            // The combined master16 family adds the summary data axis (the
            // single-axis family carries no summary_committed column).
            if committed(row, "summary_committed")
                && let Some(history) = obs.pointer_mut("/data").and_then(Value::as_object_mut)
            {
                history.insert(
                    "summary".to_owned(),
                    json!({ "_type": "ITEM_TREE", "name": { "_type": "DV_TEXT", "value": "summary" }, "archetype_node_id": "at0007", "items": [] }),
                );
            }
            comp_shell(template_id, vec![obs], true)
        }
        "HISTORY" => {
            let n = count(row, "events_count");
            let events: Vec<Value> = (0..n)
                .map(|_| min_event("POINT_EVENT", true, false))
                .collect();
            let mut obs = min_observation(events, None, None);
            if committed(row, "summary_committed")
                && let Some(history) = obs.pointer_mut("/data").and_then(Value::as_object_mut)
            {
                history.insert(
                    "summary".to_owned(),
                    json!({ "_type": "ITEM_TREE", "name": { "_type": "DV_TEXT", "value": "summary" }, "archetype_node_id": "at0007", "items": [] }),
                );
            }
            comp_shell(template_id, vec![obs], true)
        }
        "ITEM_STRUCTURE" => {
            let committed_type = row.text("committed_type").unwrap_or("ITEM_TREE");
            let eval = min_evaluation(min_item_structure(committed_type));
            comp_shell(template_id, vec![eval], true)
        }
        "OBSERVATION" => {
            let event = min_event("POINT_EVENT", committed(row, "data_committed"), false);
            let state = committed(row, "state_committed").then(|| json!({
                "_type": "HISTORY", "name": { "_type": "DV_TEXT", "value": "state" }, "archetype_node_id": "at0005",
                "origin": { "_type": "DV_DATE_TIME", "value": "2026-01-01T00:00:00Z" },
                "events": [min_event("POINT_EVENT", true, false)]
            }));
            let protocol = committed(row, "protocol_committed").then(|| json!({
                "_type": "ITEM_TREE", "name": { "_type": "DV_TEXT", "value": "protocol" }, "archetype_node_id": "at0006", "items": []
            }));
            let obs = min_observation(vec![event], state, protocol);
            comp_shell(template_id, vec![obs], true)
        }
        // Not a structural family — caller guards with `is_structural`.
        _ => comp_shell(template_id, Vec::new(), true),
    }
}

/// The minimal-event carrier the content instances commit inside.
fn base_carrier_composition() -> Value {
    json!({
        "_type": "COMPOSITION",
        "name": { "_type": "DV_TEXT", "value": "content case carrier" },
        "archetype_node_id": "openEHR-EHR-COMPOSITION.minimal.v1",
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-COMPOSITION.minimal.v1" },
            "template_id": { "_type": "TEMPLATE_ID", "value": "cnf.minimal_event" },
            "rm_version": "1.0.2"
        },
        "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
        "territory": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_3166-1" }, "code_string": "NL" },
        "category": { "_type": "DV_CODED_TEXT", "value": "event",
            "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "433" } },
        "composer": { "_type": "PARTY_SELF" },
        "context": { "_type": "EVENT_CONTEXT",
            "start_time": { "_type": "DV_DATE_TIME", "value": "2026-01-01T00:00:00Z" },
            "setting": { "_type": "DV_CODED_TEXT", "value": "other care",
                "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "238" } } },
        "content": [{
            "_type": "OBSERVATION",
            "name": { "_type": "DV_TEXT", "value": "content observation" },
            "archetype_node_id": "openEHR-EHR-OBSERVATION.minimal.v1",
            "archetype_details": {
                "_type": "ARCHETYPED",
                "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-OBSERVATION.minimal.v1" },
                "rm_version": "1.0.2"
            },
            "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
            "encoding": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "IANA_character-sets" }, "code_string": "UTF-8" },
            "subject": { "_type": "PARTY_SELF" },
            "data": { "_type": "HISTORY", "name": { "_type": "DV_TEXT", "value": "history" }, "archetype_node_id": "at0001",
                "origin": { "_type": "DV_DATE_TIME", "value": "2026-01-01T00:00:00Z" },
                "events": [{ "_type": "POINT_EVENT", "name": { "_type": "DV_TEXT", "value": "any event" }, "archetype_node_id": "at0002",
                    "time": { "_type": "DV_DATE_TIME", "value": "2026-01-01T00:00:00Z" },
                    "data": { "_type": "ITEM_TREE", "name": { "_type": "DV_TEXT", "value": "tree" }, "archetype_node_id": "at0003",
                        "items": [{ "_type": "ELEMENT", "name": { "_type": "DV_TEXT", "value": "value" }, "archetype_node_id": "at0004" }] } }] } }]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row<'a>(columns: &'a [String], cells: &'a [MatrixCell]) -> BoundRow<'a> {
        BoundRow { columns, cells }
    }

    #[test]
    fn ehr_status_recipe_is_deterministic_and_total() {
        let columns: Vec<String> = [
            "ehr_status",
            "is_queryable",
            "is_modifiable",
            "subject",
            "other_details",
            "ehr_id",
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        let cells = vec![
            MatrixCell::Provided,
            MatrixCell::Literal(serde_json::json!(true)),
            MatrixCell::Literal(serde_json::json!(false)),
            MatrixCell::Provided,
            MatrixCell::Provided,
            MatrixCell::Absent,
        ];
        let a = ehr_status("case-a", &row(&columns, &cells), 3)
            .unwrap()
            .unwrap();
        let b = ehr_status("case-a", &row(&columns, &cells), 3)
            .unwrap()
            .unwrap();
        assert_eq!(a, b); // deterministic
        assert_eq!(a["is_queryable"], serde_json::json!(true));
        assert_eq!(a["is_modifiable"], serde_json::json!(false));
        assert!(a["other_details"].is_object());

        let absent = vec![
            MatrixCell::Absent,
            MatrixCell::Literal(serde_json::json!("-")),
            MatrixCell::Literal(serde_json::json!("-")),
            MatrixCell::Literal(serde_json::json!("-")),
            MatrixCell::Literal(serde_json::json!("-")),
            MatrixCell::Absent,
        ];
        assert!(
            ehr_status("case-a", &row(&columns, &absent), 0)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn bp_series_matches_its_contract() {
        let c0 = bp_series(0).unwrap();
        let c9 = bp_series(9).unwrap();
        assert_eq!(
            c0["content"][0]["data"]["events"][0]["data"]["items"][0]["value"]["magnitude"],
            serde_json::json!(100)
        );
        assert_eq!(
            c9["content"][0]["data"]["events"][0]["data"]["items"][0]["value"]["magnitude"],
            serde_json::json!(190)
        );
        assert!(bp_series(10).is_err());
        assert_eq!(bp_series(4).unwrap(), query_bp(4).unwrap());
    }

    fn cols(names: &[&str]) -> Vec<String> {
        names.iter().map(ToString::to_string).collect()
    }

    fn value_of(comp: &Value) -> &Value {
        &comp["content"][0]["data"]["events"][0]["data"]["items"][0]["value"]
    }

    #[test]
    fn coded_text_builds_defining_code_and_drops_constraint_axis() {
        let c = cols(&[
            "code_string",
            "terminology_id",
            "C_CODE_PHRASE.code_list",
            "expected",
        ]);
        let cells = vec![
            MatrixCell::Literal(json!("ABC")),
            MatrixCell::Literal(json!("local")),
            MatrixCell::Literal(json!("[ABC, OPQ]")),
            MatrixCell::Literal(json!("created")),
        ];
        let v = content_instance(
            "DV_CODED_TEXT",
            "cnf.tpl.dv_coded_text_c_code_phrase",
            &c,
            &cells,
        );
        let dv = value_of(&v);
        assert_eq!(dv["_type"], json!("DV_CODED_TEXT"));
        assert_eq!(dv["defining_code"]["code_string"], json!("ABC"));
        assert_eq!(
            dv["defining_code"]["terminology_id"]["value"],
            json!("local")
        );
        // constraint-axis column is never emitted as an attribute.
        assert!(dv.get("C_CODE_PHRASE.code_list").is_none());
    }

    #[test]
    fn coded_text_mandatory_row_omits_defining_code() {
        let c = cols(&["code_string", "terminology_id", "expected"]);
        let cells = vec![
            MatrixCell::Null,
            MatrixCell::Null,
            MatrixCell::Literal(json!("rejected")),
        ];
        let dv = value_of(&content_instance("DV_CODED_TEXT", "t", &c, &cells)).clone();
        assert_eq!(dv["_type"], json!("DV_CODED_TEXT"));
        assert!(dv.get("defining_code").is_none());
        assert!(dv.get("value").is_none());
    }

    #[test]
    fn coded_text_value_column_drives_value_verbatim() {
        // The `value` column, when present, drives DV_CODED_TEXT.value verbatim
        // (RM dv_text.adoc §Attributes value) — an arbitrary non-empty label,
        // and an empty string (which must fire Valid_value); absent the column
        // it defaults to a non-empty placeholder.
        let c = cols(&["code_string", "terminology_id", "value", "expected"]);
        let with_value = |v: Value| {
            value_of(&content_instance(
                "DV_CODED_TEXT",
                "t",
                &c,
                &[
                    MatrixCell::Literal(json!("ABC")),
                    MatrixCell::Literal(json!("local")),
                    MatrixCell::Literal(v),
                    MatrixCell::Literal(json!("accepted")),
                ],
            ))
            .clone()
        };
        assert_eq!(
            with_value(json!("Rubric for ABC"))["value"],
            json!("Rubric for ABC")
        );
        assert_eq!(with_value(json!("ABC"))["value"], json!("ABC"));
        // Empty value is committed verbatim so the SUT's Valid_value check fires.
        assert_eq!(with_value(json!(""))["value"], json!(""));
        // A null value cell defaults to the non-empty placeholder.
        let defaulted = value_of(&content_instance(
            "DV_CODED_TEXT",
            "t",
            &c,
            &[
                MatrixCell::Literal(json!("ABC")),
                MatrixCell::Literal(json!("local")),
                MatrixCell::Null,
                MatrixCell::Literal(json!("accepted")),
            ],
        ))
        .clone();
        assert_eq!(defaulted["value"], json!("coded"));
    }

    #[test]
    fn ordinal_builds_coded_symbol_and_value() {
        let c = cols(&["symbol", "value", "expected"]);
        let cells = vec![
            MatrixCell::Literal(json!("local::at0005")),
            MatrixCell::Literal(json!(1)),
            MatrixCell::Literal(json!("created")),
        ];
        let dv = value_of(&content_instance(
            "DV_ORDINAL",
            "cnf.tpl.ordinal_open",
            &c,
            &cells,
        ))
        .clone();
        assert_eq!(dv["value"], json!(1));
        assert_eq!(
            dv["symbol"]["defining_code"]["terminology_id"]["value"],
            json!("local")
        );
        assert_eq!(
            dv["symbol"]["defining_code"]["code_string"],
            json!("at0005")
        );
    }

    #[test]
    fn interval_builds_nested_count_limits_and_flags() {
        let c = cols(&[
            "lower",
            "upper",
            "lower_unbounded",
            "upper_unbounded",
            "lower_included",
            "upper_included",
            "expected",
        ]);
        let cells = vec![
            MatrixCell::Literal(json!(0)),
            MatrixCell::Literal(json!(100)),
            MatrixCell::Literal(json!(false)),
            MatrixCell::Literal(json!(false)),
            MatrixCell::Literal(json!(true)),
            MatrixCell::Literal(json!(true)),
            MatrixCell::Literal(json!("created")),
        ];
        let dv = value_of(&content_instance(
            "DV_INTERVAL",
            "cnf.tpl.interval_count_range",
            &c,
            &cells,
        ))
        .clone();
        assert_eq!(dv["_type"], json!("DV_INTERVAL"));
        assert_eq!(dv["lower"], json!({ "_type": "DV_COUNT", "magnitude": 0 }));
        assert_eq!(
            dv["upper"],
            json!({ "_type": "DV_COUNT", "magnitude": 100 })
        );
        assert_eq!(dv["lower_included"], json!(true));
    }

    #[test]
    fn interval_unbounded_side_is_omitted_and_quantity_is_parsed() {
        let c = cols(&[
            "lower",
            "upper",
            "lower_unbounded",
            "upper_unbounded",
            "expected",
        ]);
        let cells = vec![
            MatrixCell::Null,
            MatrixCell::Literal(json!("100 mg")),
            MatrixCell::Literal(json!(true)),
            MatrixCell::Literal(json!(false)),
            MatrixCell::Literal(json!("created")),
        ];
        let dv = value_of(&content_instance(
            "DV_INTERVAL",
            "cnf.tpl.interval_quantity_open",
            &c,
            &cells,
        ))
        .clone();
        assert!(dv.get("lower").is_none());
        assert_eq!(
            dv["upper"],
            json!({ "_type": "DV_QUANTITY", "magnitude": 100.0, "units": "mg" })
        );
    }

    #[test]
    fn structural_rm_class_builds_per_row_carrier() {
        // Issue #228: a structural rm_class no longer injects an ELEMENT.value —
        // it builds a per-row-shaped carrier (here `content_count` content items)
        // committed against the per-row synthesized cardinality OPT.
        let c = cols(&["cardinality", "content_count", "expected"]);
        let cells = vec![
            MatrixCell::Literal(json!("3to5")),
            MatrixCell::Literal(json!(3)),
            MatrixCell::Literal(json!("created")),
        ];
        let comp = content_instance(
            "COMPOSITION",
            "cnf.tpl.cont_composition_content_cardinality.r22",
            &c,
            &cells,
        );
        assert_eq!(comp["content"].as_array().map(Vec::len), Some(3));
        // content_count 0 => the content attribute is omitted (an empty present
        // list is an RM error), so the "any/0" row is accepted.
        let zero = content_instance(
            "COMPOSITION",
            "cnf.tpl.cont_composition_content_cardinality.r0",
            &cols(&["cardinality", "content_count", "expected"]),
            &[
                MatrixCell::Literal(json!("any")),
                MatrixCell::Literal(json!(0)),
                MatrixCell::Literal(json!("created")),
            ],
        );
        assert!(zero.get("content").is_none());
    }

    #[test]
    fn deterministic_ehr_ids_are_stable_and_distinct() {
        assert_eq!(deterministic_ehr_id("a", 1), deterministic_ehr_id("a", 1));
        assert_ne!(deterministic_ehr_id("a", 1), deterministic_ehr_id("a", 2));
        assert_ne!(deterministic_ehr_id("a", 1), deterministic_ehr_id("b", 1));
    }
}
