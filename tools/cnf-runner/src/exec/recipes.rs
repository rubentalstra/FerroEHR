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
pub fn ehr_status(row: &BoundRow<'_>, row_index: usize) -> Result<Option<Value>, RecipeError> {
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
                "id": { "_type": "GENERIC_ID", "value": format!("subject-{row_index}"), "scheme": "cnf" }
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
pub fn deterministic_ehr_id(row_index: usize) -> String {
    // A fixed, obviously-synthetic UUID family: index in the first group.
    format!("{row_index:08x}-0000-4000-8000-00000000cnf0").replace("cnf0", "0cf0")
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
        let a = ehr_status(&row(&columns, &cells), 3).unwrap().unwrap();
        let b = ehr_status(&row(&columns, &cells), 3).unwrap().unwrap();
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
        assert!(ehr_status(&row(&columns, &absent), 0).unwrap().is_none());
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

    #[test]
    fn deterministic_ehr_ids_are_stable_and_distinct() {
        assert_eq!(deterministic_ehr_id(1), deterministic_ehr_id(1));
        assert_ne!(deterministic_ehr_id(1), deterministic_ehr_id(2));
    }
}
