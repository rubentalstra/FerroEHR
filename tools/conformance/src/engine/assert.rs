//! Response assertions (design §4.3, §2.2a): status/header checks and the
//! upstream `jsonlib` payload-comparison semantics — Exact, Superset, and
//! `IgnoreSet` — over `serde_json::Value`.
//!
//! A failed assertion is a genuine conformance finding, so every assertion
//! yields a [`CaseError::Assertion`] carrying a message that names what was
//! expected (the same dual-citation discipline the spec-audit uses — the caller
//! cites the ITS-REST/RM clause in the message).

use serde_json::Value;

use crate::case::Compare;
use crate::harness::{CaseError, HttpResponse};

/// The default ignore-set for [`Compare::IgnoreSet`]: the RM `_type` discriminator
/// (present on our RM 1.2.0 output, often absent from RM-1.0.x-era fixtures) plus
/// the `signature` our SUT adds by default (the `SignatureDefaultOn` rule, §6) —
/// these keys are ignored anywhere in the tree when diffing.
pub const DEFAULT_IGNORE_KEYS: &[&str] = &["_type", "signature"];

/// Assert the response status equals `expected`.
///
/// # Errors
/// [`CaseError::Assertion`] if the status differs.
pub fn status(response: &HttpResponse, expected: u16) -> Result<(), CaseError> {
    if response.status == expected {
        Ok(())
    } else {
        Err(CaseError::Assertion(format!(
            "expected status {expected}, got {} (body: {})",
            response.status,
            truncate(&response.text(), 300)
        )))
    }
}

/// Assert the response status is one of `allowed`.
///
/// # Errors
/// [`CaseError::Assertion`] if the status is not in `allowed`.
pub fn status_in(response: &HttpResponse, allowed: &[u16]) -> Result<(), CaseError> {
    if allowed.contains(&response.status) {
        Ok(())
    } else {
        Err(CaseError::Assertion(format!(
            "expected status in {allowed:?}, got {}",
            response.status
        )))
    }
}

/// Assert a header is present and non-empty.
///
/// # Errors
/// [`CaseError::Assertion`] if the header is absent or empty.
pub fn header_present(response: &HttpResponse, name: &str) -> Result<(), CaseError> {
    match response.header(name) {
        Some(v) if !v.is_empty() => Ok(()),
        _ => Err(CaseError::Assertion(format!("expected header {name:?}"))),
    }
}

/// Assert a header equals `expected`.
///
/// # Errors
/// [`CaseError::Assertion`] if absent or unequal.
pub fn header_eq(response: &HttpResponse, name: &str, expected: &str) -> Result<(), CaseError> {
    match response.header(name) {
        Some(v) if v == expected => Ok(()),
        Some(v) => Err(CaseError::Assertion(format!(
            "header {name:?}: expected {expected:?}, got {v:?}"
        ))),
        None => Err(CaseError::Assertion(format!("expected header {name:?}"))),
    }
}

/// Compare `actual` against `expected` in `mode` (§2.2a).
///
/// # Errors
/// [`CaseError::Assertion`] on a mismatch.
pub fn compare(mode: Compare, expected: &Value, actual: &Value) -> Result<(), CaseError> {
    let ok = match mode {
        Compare::Exact => exact(expected, actual),
        Compare::Superset => superset(expected, actual),
        Compare::IgnoreSet => ignore_set(expected, actual, DEFAULT_IGNORE_KEYS),
    };
    if ok {
        Ok(())
    } else {
        Err(CaseError::Assertion(format!(
            "payload comparison ({mode:?}) failed: expected {}, got {}",
            truncate(&expected.to_string(), 300),
            truncate(&actual.to_string(), 300)
        )))
    }
}

/// Exact deep equality.
fn exact(expected: &Value, actual: &Value) -> bool {
    expected == actual
}

/// Superset: every field/element of `expected` is present and matches in
/// `actual`, which may carry more (the upstream `jsonlib` superset mode).
fn superset(expected: &Value, actual: &Value) -> bool {
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => e
            .iter()
            .all(|(k, ev)| a.get(k).is_some_and(|av| superset(ev, av))),
        (Value::Array(e), Value::Array(a)) => {
            e.len() <= a.len() && e.iter().zip(a).all(|(ev, av)| superset(ev, av))
        }
        _ => expected == actual,
    }
}

/// Exact equality, but keys in `ignore` are dropped anywhere in the tree before
/// comparing.
fn ignore_set(expected: &Value, actual: &Value, ignore: &[&str]) -> bool {
    strip(expected, ignore) == strip(actual, ignore)
}

/// Recursively drop `ignore` keys from a value.
fn strip(value: &Value, ignore: &[&str]) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(k, _)| !ignore.contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), strip(v, ignore)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(|v| strip(v, ignore)).collect()),
        other => other.clone(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_owned();
    }
    // Truncate at the last char boundary at or before `max`.
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn resp(status: u16, headers: &[(&str, &str)]) -> HttpResponse {
        HttpResponse {
            status,
            headers: headers
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            body: Vec::new(),
        }
    }

    #[test]
    fn status_and_headers() {
        let r = resp(201, &[("etag", "\"abc\""), ("location", "/ehr/1")]);
        assert!(status(&r, 201).is_ok());
        assert!(status(&r, 200).is_err());
        assert!(status_in(&r, &[200, 201]).is_ok());
        assert!(header_present(&r, "ETag").is_ok());
        assert!(header_present(&r, "x-missing").is_err());
        assert!(header_eq(&r, "location", "/ehr/1").is_ok());
        assert!(header_eq(&r, "location", "/ehr/2").is_err());
    }

    #[test]
    fn exact_mode() {
        let a = json!({"x": 1, "y": [1, 2]});
        assert!(compare(Compare::Exact, &a, &a.clone()).is_ok());
        assert!(compare(Compare::Exact, &a, &json!({"x": 1})).is_err());
    }

    #[test]
    fn superset_mode() {
        let expected = json!({"x": 1});
        let actual = json!({"x": 1, "extra": true});
        assert!(compare(Compare::Superset, &expected, &actual).is_ok());
        // Exact would reject the extra field.
        assert!(compare(Compare::Exact, &expected, &actual).is_err());
        // A missing expected field still fails superset.
        assert!(compare(Compare::Superset, &json!({"z": 9}), &actual).is_err());
        // Arrays: expected must be a prefix-length subset that matches element-wise.
        assert!(
            compare(
                Compare::Superset,
                &json!({"a": [1]}),
                &json!({"a": [1, 2, 3]})
            )
            .is_ok()
        );
        assert!(compare(Compare::Superset, &json!({"a": [9]}), &json!({"a": [1, 2]})).is_err());
    }

    #[test]
    fn ignore_set_mode() {
        // `_type` and `signature` differ but are ignored; the rest matches.
        let expected = json!({"value": "x", "_type": "DV_TEXT"});
        let actual = json!({"value": "x", "_type": "DV_CODED_TEXT", "signature": "sha256:…"});
        assert!(compare(Compare::IgnoreSet, &expected, &actual).is_ok());
        // A real value difference still fails.
        let actual2 = json!({"value": "y", "_type": "DV_TEXT"});
        assert!(compare(Compare::IgnoreSet, &expected, &actual2).is_err());
    }
}
