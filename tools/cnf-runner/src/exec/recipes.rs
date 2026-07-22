//! The registered recipe set — the ONLY hand-written generation glue in the
//! executor, each entry a committed, seeded, deterministic `row → payload`
//! function whose contract is the digest-pinned document in
//! `corpus/recipes/*.md`. Every recipe here is a registered exception to
//! the data-driven rule and is listed as such in the run report.

use serde_json::{Value, json};

use crate::model::case::MatrixCell;

/// A matrix row bound to its column names.
#[derive(Debug, Clone, Copy)]
pub struct BoundRow<'a> {
    pub columns: &'a [String],
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
/// (contract: `corpus/recipes/ehr_status.md`). Returns `None` when the row
/// declares `ehr_status: absent` (the class-1.a rows: no payload at all).
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

/// `content_instance` — the content-chapter generation recipe: build the
/// spec-correct RM data-value instance for one decision-table row, inject it
/// at the case's constrained ELEMENT.value in the minimal-event carrier
/// composition, and hand the result to the ordinary commit flow — one
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

/// Build the ELEMENT.value data value for one row, dispatched on the RM class.
///
/// Returns `None` for the structural `rm_class`es (COMPOSITION / EVENT / HISTORY
/// / `ITEM_STRUCTURE` / OBSERVATION), whose decision tables describe carrier
/// *shape* (content counts, event slot types, existence-tightened
/// attributes), not an ELEMENT value.
// TODO: the structural rm_classes need per-row carrier-shape projection driven
// by their constraint-axis columns (an event slot type, an omitted summary, N
// content items) plus a per-row constraint template — the single-template
// execution model cannot represent their varying-constraint decision tables.
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
        map.insert("value".to_owned(), Value::String("coded".to_owned()));
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
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)] // test fixtures
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
    fn structural_rm_class_injects_no_value() {
        let c = cols(&["cardinality", "content_count", "expected"]);
        let cells = vec![
            MatrixCell::Literal(json!("3to5")),
            MatrixCell::Literal(json!(3)),
            MatrixCell::Literal(json!("created")),
        ];
        let comp = content_instance(
            "COMPOSITION",
            "cnf.tpl.ecc_composition_content_cardinality",
            &c,
            &cells,
        );
        assert!(
            comp["content"][0]["data"]["events"][0]["data"]["items"][0]
                .get("value")
                .is_none()
        );
    }

    #[test]
    fn deterministic_ehr_ids_are_stable_and_distinct() {
        assert_eq!(deterministic_ehr_id("a", 1), deterministic_ehr_id("a", 1));
        assert_ne!(deterministic_ehr_id("a", 1), deterministic_ehr_id("a", 2));
        assert_ne!(deterministic_ehr_id("a", 1), deterministic_ehr_id("b", 1));
    }
}
