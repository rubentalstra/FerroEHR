// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The normative AQL `RESULT_SET` equivalence comparator.
//!
//! The schedule's
//! `RESULT_SET` equivalence rules — each rule is either **\[spec\]**, cited to
//! the vendored QUERY/ITS-REST text, or **\[legislated\]**, a fixed proposed
//! default awaiting upstream ratification; both are implemented exactly as
//! specified.
//!
//! Rules implemented here:
//! 1. Comparison scope is `rows` only; `meta` is always excluded (\[spec\]:
//!    every `ResultSetMetadata` field is optional and implementation
//!    dependent — ITS-REST `schemas/query/ResultSetMetadata.yaml`).
//!    `columns` compare only when the case asserts them (\[spec\]:
//!    `ResultSet.yaml` requires only `rows`); column identity is the `AS`
//!    alias, else `#<0-based index>` (\[spec\]: `ResultSetColumn.yaml`).
//! 2. `ordered` = sequence equality (legal only under a totally-ordering
//!    ORDER BY — enforced at authoring); `set` = BAG (multiset) equality
//!    (\[spec\]: AQL is bag-semantics absent DISTINCT — QUERY
//!    `master03-syntax.adoc` §DISTINCT); `count` = row count; `contains` =
//!    every expected row appears bag-wise, extra rows permitted.
//! 3. Cell equality: an RM-object cell (carries `_type`) compares by
//!    canonical-JSON structural equality (\[spec\]: QUERY
//!    `master04-result_structure.adoc`); a scalar numeric cell compares by
//!    NUMERIC VALUE, not lexeme — `140` = `140.0` (\[legislated\]); a void
//!    cell is JSON `null` and equals only `null` (\[legislated\]).

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

use serde_json::Value;

/// One comparison failure, human-readable and stable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultSetMismatch(pub String);

/// Cell equality under rule 3.
#[must_use]
pub fn cells_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        // numeric-value equality, not lexeme (\[legislated\])
        (Value::Number(x), Value::Number(y)) => match (x.as_f64(), y.as_f64()) {
            (Some(x), Some(y)) => (x - y).abs() == 0.0,
            _ => x == y,
        },
        // void equals only void (\[legislated\])
        (Value::Null, Value::Null) => true,
        // RM objects and everything else: canonical structural equality,
        // with numeric leaves compared by value recursively
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| cells_equal(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, va)| y.get(k).is_some_and(|vb| cells_equal(va, vb)))
        }
        _ => a == b,
    }
}

fn rows_of(result_set: &Value) -> Result<&Vec<Value>, ResultSetMismatch> {
    result_set
        .get("rows")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ResultSetMismatch(
                "response carries no rows array (ResultSet.yaml requires rows)".to_owned(),
            )
        })
}

fn row_equal(a: &Value, b: &Value) -> bool {
    match (a.as_array(), b.as_array()) {
        (Some(a), Some(b)) => a.len() == b.len() && a.iter().zip(b).all(|(x, y)| cells_equal(x, y)),
        _ => cells_equal(a, b),
    }
}

/// Bag containment: every expected row appears in `actual` (multiplicity
/// respected). Returns the leftover-unmatched expected row on failure.
fn bag_contains(actual: &[Value], expected: &[Value]) -> Result<(), ResultSetMismatch> {
    let mut used = vec![false; actual.len()];
    for (i, want) in expected.iter().enumerate() {
        let found = actual
            .iter()
            .enumerate()
            .find(|(j, got)| used.get(*j) == Some(&false) && row_equal(got, want))
            .map(|(j, _)| j);
        match found {
            Some(j) => {
                if let Some(slot) = used.get_mut(j) {
                    *slot = true;
                }
            }
            None => {
                return Err(ResultSetMismatch(format!(
                    "expected row {i} has no bag-wise match: {want}"
                )));
            }
        }
    }
    Ok(())
}

/// `match: ordered` — sequence equality over rows.
///
/// # Errors
/// The first mismatch, positional.
pub fn compare_ordered(result_set: &Value, expected: &[Value]) -> Result<(), ResultSetMismatch> {
    let rows = rows_of(result_set)?;
    if rows.len() != expected.len() {
        return Err(ResultSetMismatch(format!(
            "row count {} != expected {}",
            rows.len(),
            expected.len()
        )));
    }
    for (i, (got, want)) in rows.iter().zip(expected).enumerate() {
        if !row_equal(got, want) {
            return Err(ResultSetMismatch(format!(
                "row {i}: {got} != expected {want}"
            )));
        }
    }
    Ok(())
}

/// `match: set` — bag (multiset) equality: duplicates are significant.
///
/// # Errors
/// The unmatched row (either direction).
pub fn compare_bag(result_set: &Value, expected: &[Value]) -> Result<(), ResultSetMismatch> {
    let rows = rows_of(result_set)?;
    if rows.len() != expected.len() {
        return Err(ResultSetMismatch(format!(
            "row count {} != expected {} (bag equality)",
            rows.len(),
            expected.len()
        )));
    }
    bag_contains(rows, expected)
}

/// `match: contains` — every expected row appears bag-wise; extras allowed.
///
/// # Errors
/// The unmatched expected row.
pub fn compare_contains(result_set: &Value, expected: &[Value]) -> Result<(), ResultSetMismatch> {
    bag_contains(rows_of(result_set)?, expected)
}

/// `match: count` — row count only.
///
/// # Errors
/// The count mismatch.
pub fn compare_count(result_set: &Value, expected: u64) -> Result<(), ResultSetMismatch> {
    #[expect(
        clippy::as_conversions,
        reason = "row count widens exactly: usize is at most 64 bits on every supported target"
    )]
    let n = rows_of(result_set)?.len() as u64;
    if n == expected {
        Ok(())
    } else {
        Err(ResultSetMismatch(format!(
            "row count {n} != expected {expected}"
        )))
    }
}

/// Column identity check (rule 1): asserted names against the `AS` alias,
/// else `#<index>`.
///
/// # Errors
/// The first name mismatch, or a missing columns array when asserted.
pub fn compare_columns(result_set: &Value, expected: &[String]) -> Result<(), ResultSetMismatch> {
    let columns = result_set
        .get("columns")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ResultSetMismatch("columns asserted but the response carries none".to_owned())
        })?;
    if columns.len() != expected.len() {
        return Err(ResultSetMismatch(format!(
            "column count {} != expected {}",
            columns.len(),
            expected.len()
        )));
    }
    for (i, (col, want)) in columns.iter().zip(expected).enumerate() {
        let name = col
            .get("name")
            .and_then(Value::as_str)
            .map_or_else(|| format!("#{i}"), ToOwned::to_owned);
        if &name != want {
            return Err(ResultSetMismatch(format!(
                "column {i}: {name:?} != expected {want:?}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn numeric_value_not_lexeme() {
        assert!(cells_equal(&json!(140), &json!(140.0)));
        assert!(!cells_equal(&json!(140), &json!(140.5)));
        assert!(cells_equal(&json!(null), &json!(null)));
        assert!(!cells_equal(&json!(null), &json!(0)));
    }

    #[test]
    fn rm_object_cells_compare_structurally_with_numeric_leaves() {
        let a = json!({"_type": "DV_QUANTITY", "magnitude": 140, "units": "mm[Hg]"});
        let b = json!({"_type": "DV_QUANTITY", "magnitude": 140.0, "units": "mm[Hg]"});
        assert!(cells_equal(&a, &b));
    }

    #[test]
    fn bag_semantics_respect_duplicates() {
        let rs = json!({"rows": [["a"], ["a"], ["b"]], "meta": {"_href": "ignored"}});
        assert!(compare_bag(&rs, &[json!(["a"]), json!(["b"]), json!(["a"])]).is_ok());
        // duplicate significance: two a's expected, one present
        let rs2 = json!({"rows": [["a"], ["b"], ["b"]]});
        assert!(compare_bag(&rs2, &[json!(["a"]), json!(["a"]), json!(["b"])]).is_err());
    }

    #[test]
    fn ordered_contains_count_and_columns() {
        let rs = json!({
            "columns": [{"name": "uid"}],
            "rows": [["u1"], ["u2"], ["u3"]]
        });
        assert!(compare_ordered(&rs, &[json!(["u1"]), json!(["u2"]), json!(["u3"])]).is_ok());
        assert!(compare_ordered(&rs, &[json!(["u2"]), json!(["u1"]), json!(["u3"])]).is_err());
        assert!(compare_contains(&rs, &[json!(["u3"]), json!(["u1"])]).is_ok());
        assert!(compare_count(&rs, 3).is_ok());
        assert!(compare_columns(&rs, &["uid".to_owned()]).is_ok());
        assert!(compare_columns(&rs, &["id".to_owned()]).is_err());
    }
}
