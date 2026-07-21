//! The pure assertion evaluators — verdict logic over canonical-JSON
//! values, shared by the live driver and the transcript player so any two
//! conformant runners compute identical verdicts.
//!
//! Wire-dependent assertion families (`version`, `signature`,
//! `instance_of`) need reads the driver performs; their FACT comparison
//! still happens here so the judgement stays pure.

use serde_json::Value;

use crate::exec::resultset;
use crate::exec::state::{Captured, VarStore};
use crate::model::assertion::{Assertion, IgnoreSpec};
use crate::refgrammar::{Segment, Template, ValueRef};
use crate::vocab::IgnoreSetName;

/// One assertion failure (stable, human-readable — lands in the outcome
/// record and the report).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertionFailure(pub String);

/// Resolve an RM path segment sequence (`context/setting`,
/// `content[0]/data/events[0]/...`) over a canonical-JSON value.
/// Supported addressing: object attributes and `[<index>]` list positions —
/// the subset the catalogue's field assertions use.
#[must_use]
pub fn resolve_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = root;
    for raw in path.split('/').filter(|s| !s.is_empty()) {
        let (attr, index) = match raw.split_once('[') {
            Some((attr, rest)) => {
                let index: usize = rest.strip_suffix(']')?.parse().ok()?;
                (attr, Some(index))
            }
            None => (raw, None),
        };
        if !attr.is_empty() {
            current = current.get(attr)?;
        }
        if let Some(i) = index {
            current = current.get(i)?;
        }
    }
    Some(current)
}

/// Render a template against the store (captures become their scalar
/// values; other reference kinds must have been pre-resolved by the driver).
///
/// # Errors
/// A message when a capture is unbound or non-scalar.
pub fn render_template(template: &Template, vars: &VarStore) -> Result<String, String> {
    let mut out = String::new();
    for segment in template.segments() {
        match segment {
            Segment::Lit(s) => out.push_str(s),
            Segment::Ref(ValueRef::Capture { name, .. }) => match vars.get(name) {
                Some(Captured::Scalar(s)) => out.push_str(s),
                Some(_) => return Err(format!("capture {name} is not scalar")),
                None => return Err(format!("capture {name} is not bound")),
            },
            Segment::Ref(other) => {
                return Err(format!("reference {other} must be resolved by the driver"));
            }
        }
    }
    Ok(out)
}

/// Strip the named ignore-sets and explicit paths from a value (top-level
/// path removal; nested server-assigned paths use `/`-separated forms).
#[must_use]
pub fn strip_ignored(value: &Value, ignored_paths: &[String]) -> Value {
    fn remove(value: &mut Value, segments: &[&str]) {
        let Some((head, rest)) = segments.split_first() else {
            return;
        };
        match value {
            Value::Object(map) => {
                if rest.is_empty() {
                    map.remove(*head);
                } else if let Some(next) = map.get_mut(*head) {
                    remove(next, rest);
                }
            }
            Value::Array(items) => {
                for item in items {
                    remove(item, segments);
                }
            }
            _ => {}
        }
    }
    let mut out = value.clone();
    for path in ignored_paths {
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        remove(&mut out, &segments);
    }
    out
}

/// The `equivalent` comparison: structural equality after stripping the
/// resolved ignore paths from BOTH sides (numeric leaves by value, via the
/// result-set cell rule — canonical JSON carries RM numbers).
#[must_use]
pub fn equivalent(actual: &Value, expected: &Value, ignored_paths: &[String]) -> bool {
    let a = strip_ignored(actual, ignored_paths);
    let b = strip_ignored(expected, ignored_paths);
    resultset::cells_equal(&a, &b)
}

/// Resolve the `ignoring:` list into concrete paths: named sets come from
/// the binding (`server_assigned`) and the selectors vocabulary
/// (`ctx_defaults`); explicit paths pass through.
#[must_use]
pub fn resolve_ignore_sets(
    specs: &[IgnoreSpec],
    server_assigned: &[String],
    ctx_defaults: &[String],
) -> Vec<String> {
    let mut paths = Vec::new();
    for spec in specs {
        match spec {
            IgnoreSpec::Named(IgnoreSetName::ServerAssigned) => {
                paths.extend(server_assigned.iter().cloned());
            }
            IgnoreSpec::Named(IgnoreSetName::CtxDefaults) => {
                paths.extend(ctx_defaults.iter().cloned());
            }
            IgnoreSpec::Path(p) => paths.push(p.clone()),
        }
    }
    paths
}

/// Evaluate a `field` assertion over a response body.
///
/// # Errors
/// [`AssertionFailure`] describing the violated predicate.
pub fn eval_field(
    body: &Value,
    path: &str,
    equals: Option<&Value>,
    not_equals: Option<&Value>,
    exists: Option<bool>,
    absent: Option<bool>,
    matches: Option<&str>,
) -> Result<(), AssertionFailure> {
    let found = resolve_path(body, path);
    if let Some(true) = exists {
        return found
            .map(|_| ())
            .ok_or_else(|| AssertionFailure(format!("{path}: expected present, is absent")));
    }
    if let Some(true) = absent {
        return match found {
            None => Ok(()),
            Some(v) => Err(AssertionFailure(format!(
                "{path}: expected absent, found {v}"
            ))),
        };
    }
    let Some(actual) = found else {
        return Err(AssertionFailure(format!(
            "{path}: path resolves to nothing"
        )));
    };
    if let Some(want) = equals {
        if resultset::cells_equal(actual, want) {
            return Ok(());
        }
        return Err(AssertionFailure(format!(
            "{path}: {actual} != expected {want}"
        )));
    }
    if let Some(reject) = not_equals {
        if resultset::cells_equal(actual, reject) {
            return Err(AssertionFailure(format!(
                "{path}: equals the client-supplied value {reject} (must be server-set)"
            )));
        }
        return Ok(());
    }
    if let Some(pattern) = matches {
        let re = regex::Regex::new(pattern)
            .map_err(|e| AssertionFailure(format!("{path}: pattern does not compile: {e}")))?;
        let text = match actual {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        if re.is_match(&text) {
            return Ok(());
        }
        return Err(AssertionFailure(format!(
            "{path}: {text:?} does not match {pattern:?}"
        )));
    }
    Ok(())
}

/// Evaluate the aggregate `unique` assertion over the per-row stores
/// (law e: collected across all rows, evaluated once).
///
/// # Errors
/// [`AssertionFailure`] naming the duplicate value.
pub fn eval_unique(
    over: &crate::ids::CaptureName,
    all_rows: &[VarStore],
) -> Result<(), AssertionFailure> {
    let mut seen: Vec<&str> = Vec::new();
    for (row, store) in all_rows.iter().enumerate() {
        let Some(value) = store.scalar(over) else {
            continue; // rows that never bound the capture do not participate
        };
        if seen.contains(&value) {
            return Err(AssertionFailure(format!(
                "unique over ${{{over}}}: value {value:?} repeats at row {row}"
            )));
        }
        seen.push(value);
    }
    Ok(())
}

/// Evaluate a `returns` assertion against a (scalar-shaped) body.
///
/// # Errors
/// [`AssertionFailure`] describing the mismatch.
pub fn eval_returns(
    body: &Value,
    equals: Option<&Value>,
    matches: Option<&str>,
) -> Result<(), AssertionFailure> {
    if let Some(want) = equals {
        if resultset::cells_equal(body, want) {
            return Ok(());
        }
        return Err(AssertionFailure(format!(
            "returns: {body} != expected {want}"
        )));
    }
    if let Some(pattern) = matches {
        let re = regex::Regex::new(pattern)
            .map_err(|e| AssertionFailure(format!("returns pattern does not compile: {e}")))?;
        let text = match body {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        if re.is_match(&text) {
            return Ok(());
        }
        return Err(AssertionFailure(format!(
            "returns: {text:?} does not match {pattern:?}"
        )));
    }
    Ok(())
}

/// Whether an assertion is wire-dependent (its facts need a driver read:
/// versioned-object reads for `version`/`signature`, schema validation for
/// `instance_of`). The pure evaluators above handle the rest.
#[must_use]
pub fn is_wire_dependent(assertion: &Assertion) -> bool {
    matches!(
        assertion,
        Assertion::Version { .. } | Assertion::Signature { .. } | Assertion::InstanceOf { .. }
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn path_resolution_addresses_objects_and_lists() {
        let body = json!({
            "context": { "setting": { "value": "other care" } },
            "content": [ { "data": { "events": [ { "time": "t0" } ] } } ]
        });
        assert_eq!(
            resolve_path(&body, "context/setting/value").unwrap(),
            &json!("other care")
        );
        assert_eq!(
            resolve_path(&body, "content[0]/data/events[0]/time").unwrap(),
            &json!("t0")
        );
        assert!(resolve_path(&body, "content[1]").is_none());
    }

    #[test]
    fn field_predicates() {
        let body =
            json!({ "is_queryable": true, "audit": { "time_committed": "2026-07-21T10:00:00Z" } });
        assert!(
            eval_field(
                &body,
                "is_queryable",
                Some(&json!(true)),
                None,
                None,
                None,
                None
            )
            .is_ok()
        );
        assert!(eval_field(&body, "is_queryable", None, None, Some(true), None, None).is_ok());
        assert!(eval_field(&body, "subject", None, None, None, Some(true), None).is_ok());
        // the server-set predicate: stored time must differ from the client value
        assert!(
            eval_field(
                &body,
                "audit/time_committed",
                None,
                Some(&json!("1990-01-01T00:00:00Z")),
                None,
                None,
                None
            )
            .is_ok()
        );
        assert!(
            eval_field(
                &body,
                "audit/time_committed",
                None,
                Some(&json!("2026-07-21T10:00:00Z")),
                None,
                None,
                None
            )
            .is_err()
        );
    }

    #[test]
    fn equivalence_strips_normative_ignore_sets_only() {
        let committed = json!({ "name": { "value": "v1" }, "content": [{"x": 1}] });
        let served = json!({
            "uid": { "value": "generated::sut::1" },
            "name": { "value": "v1" },
            "content": [{"x": 1}]
        });
        assert!(equivalent(&served, &committed, &["uid".to_owned()]));
        assert!(!equivalent(&served, &committed, &[])); // nothing stripped -> uid differs
    }

    #[test]
    fn unique_is_aggregate_across_rows() {
        let name = crate::ids::CaptureName::parse("new_ehr_id").unwrap();
        let mut a = VarStore::default();
        a.set(name.clone(), Captured::Scalar("id-1".into()));
        let mut b = VarStore::default();
        b.set(name.clone(), Captured::Scalar("id-2".into()));
        assert!(eval_unique(&name, &[a.clone(), b]).is_ok());
        let mut c = VarStore::default();
        c.set(name.clone(), Captured::Scalar("id-1".into()));
        assert!(eval_unique(&name, &[a, c]).is_err());
    }
}
