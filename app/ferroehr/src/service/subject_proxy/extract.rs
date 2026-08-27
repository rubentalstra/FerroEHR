// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! `frame_path` extraction: turn a `DATA_FRAME_SAMPLE` into a typed
//! `VARIABLE_VALUE` (`subject_variable.adoc` `value()`: "Extract the value
//! from the source retrieve frame, reprocessing if necessary to obtain
//! intended type (single, list, `time_series`)").
//!
//! NOTE (selector grammar). The SM leaves `frame_path` undefined ("Path
//! within `last_frame` result"); the documented grammar here is our own
//! realization:
//!
//! - `OPENEHR_SAMPLE` (a `RESULT_SET`): `frame_path` is a **column selector**
//!   matched against a column `name`; `"col @ timecol"` pairs the column with
//!   a time column to yield `VARIABLE_VALUE_TIME_SERIES`. Without a time
//!   pairing: 0 rows ⇒ `SINGLE{None}`, 1 row ⇒ `SINGLE`, many ⇒ `LIST`.
//! - `HL7_FHIR_SAMPLE`: `frame_path` is a JSON pointer into the resource
//!   (RFC 6901; a bare name is shorthand for `/name`). A JSON-array target
//!   becomes `LIST`.
//! - A non-existent selector fails closed to `SINGLE{None}`.
//!
//! The variable's declared `type_name` is enforced: every extracted scalar is
//! validated against it; a mismatch is an extraction error (surfaced as an
//! unavailable `VARIABLE_SAMPLE` with the reason), never a silently wrong
//! type.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 6): FHIR resources are an external standard \
              with no RM type (typed-FHIR evaluation tracked separately)"
)]

use std::collections::BTreeMap;

use serde_json::Value;

use crate::service::subject_proxy::sample::{DataFrameSample, FramePayload};
use crate::service::subject_proxy::value::VariableValue;

/// Extract the typed [`VariableValue`] for (`frame_path`, `type_name`) from a
/// frame sample. `Err` is an extraction/typing failure (reason text).
pub(super) fn extract_value(
    sample: &DataFrameSample,
    frame_path: &str,
    type_name: &str,
) -> Result<VariableValue, String> {
    if sample.is_unavailable {
        return Ok(VariableValue::none());
    }
    let raw = match &sample.result {
        Some(FramePayload::Openehr { result_set }) => extract_result_set(result_set, frame_path)?,
        Some(FramePayload::Hl7Fhir { resource }) => extract_json_pointer(resource, frame_path),
        Some(FramePayload::Hl7v2 { .. }) => {
            return Err("HL7v2 frame payloads have no extraction support".to_owned());
        }
        None => return Ok(VariableValue::none()),
    };
    coerce(raw, type_name)
}

/// Raw (untyped) extraction outcome.
enum Raw {
    None,
    Single(Value),
    List(Vec<Value>),
    TimeSeries(BTreeMap<String, Value>),
}

/// `RESULT_SET` column selection (`"col"` or `"col @ timecol"`).
fn extract_result_set(result_set: &Value, frame_path: &str) -> Result<Raw, String> {
    let (value_col, time_col) = match frame_path.split_once('@') {
        Some((v, t)) => (v.trim(), Some(t.trim())),
        None => (frame_path.trim(), None),
    };
    let Some(columns) = result_set.get("columns").and_then(Value::as_array) else {
        return Ok(Raw::None);
    };
    let col_idx = |name: &str| {
        columns
            .iter()
            .position(|c| c.get("name").and_then(Value::as_str) == Some(name))
    };
    let Some(vi) = col_idx(value_col) else {
        return Ok(Raw::None);
    };
    let Some(rows) = result_set.get("rows").and_then(Value::as_array) else {
        return Ok(Raw::None);
    };

    if let Some(time_col) = time_col {
        // Time-series pairing: `col @ timecol` (VARIABLE_VALUE_TIME_SERIES).
        let Some(ti) = col_idx(time_col) else {
            return Err(format!(
                "frame_path time column {time_col:?} not in the RESULT_SET"
            ));
        };
        let mut series = BTreeMap::new();
        for row in rows {
            let (Some(time), Some(value)) = (row.get(ti), row.get(vi)) else {
                continue;
            };
            let Some(key) = time.as_str() else {
                continue; // a null/non-text time cannot key the series
            };
            series.insert(key.to_owned(), value.clone());
        }
        return Ok(Raw::TimeSeries(series));
    }

    let values: Vec<Value> = rows.iter().filter_map(|r| r.get(vi)).cloned().collect();
    Ok(match values.len() {
        0 => Raw::None,
        1 => Raw::Single(values.into_iter().next().unwrap_or(Value::Null)),
        _ => Raw::List(values),
    })
}

/// FHIR resource extraction: JSON pointer (bare names are `/name`).
fn extract_json_pointer(resource: &Value, frame_path: &str) -> Raw {
    let pointer = if frame_path.starts_with('/') {
        frame_path.to_owned()
    } else {
        format!("/{frame_path}")
    };
    match resource.pointer(&pointer) {
        None | Some(Value::Null) => Raw::None,
        Some(Value::Array(items)) => Raw::List(items.clone()),
        Some(v) => Raw::Single(v.clone()),
    }
}

/// Validate/coerce raw extraction against the declared
/// `SUBJECT_VARIABLE.type_name` and shape the final `VARIABLE_VALUE`.
fn coerce(raw: Raw, type_name: &str) -> Result<VariableValue, String> {
    let check = |v: &Value| check_type(v, type_name);
    match raw {
        Raw::None => Ok(VariableValue::none()),
        Raw::Single(v) => {
            check(&v)?;
            Ok(VariableValue::Single { value: Some(v) })
        }
        Raw::List(items) => {
            for v in &items {
                check(v)?;
            }
            Ok(VariableValue::List { value: items })
        }
        Raw::TimeSeries(series) => {
            for v in series.values() {
                check(v)?;
            }
            Ok(VariableValue::TimeSeries { value: series })
        }
    }
}

/// Type check one extracted value against `type_name`.
///
/// NOTE: the SM does not standardise the `type_name` vocabulary ("Formal
/// type name from defining model"); the families below cover the master10
/// examples (`Date`, `Quantity`, `Boolean`) plus the common scalar names.
/// Unknown names (and `Any`) pass through unchecked.
fn check_type(value: &Value, type_name: &str) -> Result<(), String> {
    // A null cell is "no data", acceptable for every declared type.
    if value.is_null() {
        return Ok(());
    }
    let matches = match type_name.to_lowercase().as_str() {
        "boolean" => value.is_boolean(),
        "integer" | "count" => value.is_i64() || value.is_u64() || is_rm_family(value, "DV_COUNT"),
        "real" | "double" | "decimal" => value.is_number(),
        "quantity" => value.is_number() || is_rm_family(value, "DV_QUANTITY"),
        "string" | "text" => value.is_string() || is_rm_family(value, "DV_TEXT"),
        "date" => {
            return check_time_str(value, type_name, |s| s.parse::<jiff::civil::Date>().is_ok());
        }
        "datetime" | "date_time" => {
            return check_time_str(value, type_name, |s| {
                s.parse::<jiff::Timestamp>().is_ok() || s.parse::<jiff::civil::DateTime>().is_ok()
            });
        }
        "time" => {
            return check_time_str(value, type_name, |s| s.parse::<jiff::civil::Time>().is_ok());
        }
        "duration" => {
            return check_time_str(value, type_name, |s| s.parse::<jiff::Span>().is_ok());
        }
        // Any / unknown model type names: pass through
        _ => true,
    };
    if matches {
        return Ok(());
    }
    Err(format!(
        "value {value} does not match declared type_name {type_name:?}"
    ))
}

/// Temporal types arrive as ISO-8601 strings (or the matching RM `DV_*`
/// canonical object, whose `value` is the string).
fn check_time_str(value: &Value, type_name: &str, ok: impl Fn(&str) -> bool) -> Result<(), String> {
    let s = value
        .as_str()
        .or_else(|| value.get("value").and_then(Value::as_str));
    match s {
        Some(s) if ok(s) => Ok(()),
        _ => Err(format!(
            "value {value} does not match declared type_name {type_name:?}"
        )),
    }
}

/// Whether a canonical-JSON RM object belongs to the `_type` family (e.g.
/// `DV_QUANTITY` for a declared `Quantity`).
fn is_rm_family(value: &Value, family: &str) -> bool {
    value
        .get("_type")
        .and_then(Value::as_str)
        .is_some_and(|t| t.starts_with(family))
}

#[cfg(test)]
mod tests {
    use crate::service::subject_proxy::sample::Sample;
    use serde_json::json;

    use super::*;

    fn openehr_sample(result_set: Value) -> DataFrameSample {
        Sample::available(FramePayload::Openehr { result_set })
    }

    #[test]
    fn column_selector_single_list_and_none() {
        let rs = json!({
            "columns": [{"name": "bp", "path": "/x"}],
            "rows": [[120.0]]
        });
        assert_eq!(
            extract_value(&openehr_sample(rs), "bp", "Quantity").expect("single"),
            VariableValue::Single {
                value: Some(json!(120.0))
            }
        );

        let rs = json!({
            "columns": [{"name": "bp"}],
            "rows": [[120.0], [130.0]]
        });
        assert_eq!(
            extract_value(&openehr_sample(rs), "bp", "Quantity").expect("list"),
            VariableValue::List {
                value: vec![json!(120.0), json!(130.0)]
            }
        );

        let rs = json!({ "columns": [{"name": "bp"}], "rows": [] });
        assert_eq!(
            extract_value(&openehr_sample(rs), "nope", "Any").expect("none"),
            VariableValue::none(),
            "unknown column fails closed"
        );
    }

    #[test]
    fn time_series_selector_pairs_columns() {
        // `col @ timecol` → VARIABLE_VALUE_TIME_SERIES
        // (`variable_value_time_series.adoc`).
        let rs = json!({
            "columns": [{"name": "bp"}, {"name": "t"}],
            "rows": [[120.0, "2026-07-01T10:00:00Z"], [130.0, "2026-07-02T10:00:00Z"]]
        });
        let got = extract_value(&openehr_sample(rs), "bp @ t", "Quantity").expect("series");
        let VariableValue::TimeSeries { value } = got else {
            panic!("expected time series, got {got:?}");
        };
        assert_eq!(value.len(), 2);
        assert_eq!(value["2026-07-01T10:00:00Z"], json!(120.0));
    }

    #[test]
    fn fhir_json_pointer_extraction() {
        let resource = json!({
            "resourceType": "Patient",
            "birthDate": "1980-02-29",
            "name": [{"family": "Doe"}]
        });
        let sample = Sample::available(FramePayload::Hl7Fhir { resource });
        assert_eq!(
            extract_value(&sample, "birthDate", "Date").expect("date"),
            VariableValue::Single {
                value: Some(json!("1980-02-29"))
            }
        );
        assert_eq!(
            extract_value(&sample, "/name", "Any").expect("array"),
            VariableValue::List {
                value: vec![json!({"family": "Doe"})]
            }
        );
        assert_eq!(
            extract_value(&sample, "/no/such", "Any").expect("none"),
            VariableValue::none()
        );
    }

    #[test]
    fn type_name_is_enforced() {
        let rs = json!({
            "columns": [{"name": "dob"}],
            "rows": [["not-a-date"]]
        });
        let err = extract_value(&openehr_sample(rs), "dob", "Date").expect_err("type mismatch");
        assert!(err.contains("type_name"), "reason names the type: {err}");

        // DV_QUANTITY canonical object satisfies a declared Quantity.
        let rs = json!({
            "columns": [{"name": "bp"}],
            "rows": [[{"_type": "DV_QUANTITY", "magnitude": 120.0, "units": "mm[Hg]"}]]
        });
        assert!(extract_value(&openehr_sample(rs), "bp", "Quantity").is_ok());
    }

    #[test]
    fn unavailable_and_empty_fail_closed() {
        let sample: DataFrameSample = Sample::unavailable("backend down");
        assert_eq!(
            extract_value(&sample, "x", "Any").expect("unavailable"),
            VariableValue::none()
        );
    }
}
