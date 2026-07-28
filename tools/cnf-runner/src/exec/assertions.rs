//! The pure assertion evaluators — verdict logic over canonical-JSON
//! values, shared by the live driver and the transcript player so any two
//! conformant runners compute identical verdicts.
//!
//! Wire-dependent assertion families (`version`, `signature`,
//! `instance_of`) need reads the driver performs; their FACT comparison
//! still happens here so the judgement stays pure.

use serde_json::Value;
use std::collections::BTreeMap;

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

/// A FLAT body: a one-level object whose keys are path-formed strings.
fn is_flat_map(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            !map.is_empty()
                && map.keys().any(|k| k.contains('/'))
                && map.values().all(|v| !matches!(v, Value::Object(_)))
        }
        _ => false,
    }
}

/// Fold a committed FLAT document's `ctx/*` input-convenience keys onto the
/// RM-tree flat paths a read-back expresses them at, per ITS-REST
/// `simplified_formats` master06 §Context Information: `ctx/participation_*`
/// → `{root}/context/_participation:{i}|*`, `ctx/health_care_facility|*` →
/// `{root}/context/_health_care_facility|*`, and the declared
/// `ctx/id_namespace`/`ctx/id_scheme` defaults expand onto every folded
/// party that carries an `|id` ("default namespace/scheme for external
/// references" — master06 §ID Namespace and Scheme). Pure default-setters
/// whose targets the ignore-set covers (`ctx/time`, `ctx/setting`,
/// composer) pass through untouched for the ignore pass.
fn fold_flat_ctx(committed: &BTreeMap<String, Value>, root: &str) -> BTreeMap<String, Value> {
    let id_namespace = committed.get("ctx/id_namespace").cloned();
    let id_scheme = committed.get("ctx/id_scheme").cloned();
    let mut out: BTreeMap<String, Value> = BTreeMap::new();
    let mut folded_id_carriers: Vec<String> = Vec::new();
    for (key, value) in committed {
        if let Some(rest) = key.strip_prefix("ctx/participation_") {
            // participation_<field>[:i] — default index 0.
            let (field, index) = match rest.split_once(':') {
                Some((f, i)) => (f, i),
                None => (rest, "0"),
            };
            let target = format!("{root}/context/_participation:{index}|{field}");
            if field == "id" {
                folded_id_carriers.push(format!("{root}/context/_participation:{index}"));
            }
            out.insert(target, value.clone());
        } else if let Some(rest) = key.strip_prefix("ctx/health_care_facility|") {
            if rest == "id" {
                folded_id_carriers.push(format!("{root}/context/_health_care_facility"));
            }
            out.insert(
                format!("{root}/context/_health_care_facility|{rest}"),
                value.clone(),
            );
        } else if key == "ctx/id_namespace" || key == "ctx/id_scheme" {
            // consumed as qualifiers below
        } else {
            out.insert(key.clone(), value.clone());
        }
    }
    for carrier in folded_id_carriers {
        if let Some(ns) = &id_namespace {
            out.insert(format!("{carrier}|id_namespace"), ns.clone());
        }
        if let Some(scheme) = &id_scheme {
            out.insert(format!("{carrier}|id_scheme"), scheme.clone());
        }
    }
    out
}

/// Whether a FLAT key falls under an ignore path: the `ctx/*` default-setter
/// spellings map to their master06 targets (`ctx/time` →
/// `context/start_time`, `ctx/setting` → `context/setting`, `ctx/composer_*`
/// → `composer`); any other key matches when its post-root path starts with
/// the ignore path (`uid` also matches the flat `_uid` spelling).
fn flat_key_ignored(key: &str, ignored_paths: &[String]) -> bool {
    let effective: &str = match key {
        "ctx/time" => "context/start_time",
        "ctx/end_time" => "context/end_time",
        "ctx/setting" => "context/setting",
        k if k.starts_with("ctx/composer") => "composer",
        k => k.split_once('/').map_or(k, |(_, rest)| rest),
    };
    let effective = effective.replace("/_uid", "/uid");
    let effective = effective.strip_prefix('_').unwrap_or(&effective);
    ignored_paths.iter().any(|p| {
        effective == p.as_str()
            || effective.starts_with(&format!("{p}/"))
            || effective.starts_with(&format!("{p}|"))
    })
}

/// Canonicalize a FLAT key by eliding every `:0` first-element index
/// (`a:0/b`, `a:0|x`, trailing `a:0`) — the index is optional on the wire
/// (`simplified_formats` master04 §Field Identifiers), so both spellings
/// name the same datum.
fn dezero(key: &str) -> String {
    let inner = key.replace(":0/", "/").replace(":0|", "|");
    inner
        .strip_suffix(":0")
        .map_or(inner.clone(), ToOwned::to_owned)
}

/// Flatten a STRUCTURED body into its FLAT key form per the
/// `simplified_formats` master04 STRUCTURED->FLAT algorithm: object keys
/// join with `/`, array elements append `:{i}` to their segment,
/// `|`-prefixed attribute keys append without a separator, and the
/// empty-string key is the element's main value.
fn flatten_structured(value: &Value, prefix: &str, out: &mut BTreeMap<String, Value>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let next = if key.is_empty() {
                    prefix.to_owned()
                } else if let Some(attr) = key.strip_prefix('|') {
                    format!("{prefix}|{attr}")
                } else if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}/{key}")
                };
                flatten_structured(child, &next, out);
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                flatten_structured(item, &format!("{prefix}:{i}"), out);
            }
        }
        leaf => {
            out.insert(prefix.to_owned(), leaf.clone());
        }
    }
}

/// STRUCTURED is recognized by its attribute-key shape: `|`-prefixed or
/// empty-string keys somewhere in the tree (master04 §Structured format) —
/// canonical JSON never carries either, so a canonical body is never
/// misread as simplified.
fn has_simplified_leaf_keys(value: &Value) -> bool {
    match value {
        Value::Object(map) => map
            .iter()
            .any(|(k, v)| k.is_empty() || k.starts_with('|') || has_simplified_leaf_keys(v)),
        Value::Array(items) => items.iter().any(has_simplified_leaf_keys),
        _ => false,
    }
}

/// A simplified body in either wire form, as a FLAT key map: a FLAT body
/// verbatim, a STRUCTURED body via the master04 flattening. `None` for
/// canonical/other bodies (a canonical COMPOSITION carries `_type`).
fn simplified_as_flat(value: &Value) -> Option<BTreeMap<String, Value>> {
    let Value::Object(map) = value else {
        return None;
    };
    if is_flat_map(value) {
        return Some(map.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
    }
    if !map.is_empty()
        && map.keys().all(|k| !k.contains('/'))
        && !map.contains_key("_type")
        && (map.contains_key("ctx") || has_simplified_leaf_keys(value))
    {
        let mut out = BTreeMap::new();
        flatten_structured(value, "", &mut out);
        return Some(out);
    }
    None
}

/// FLAT round-trip equivalence: fold the committed side's ctx keys onto the
/// read-back's RM-path forms (master06), drop ignore-set keys from both
/// sides, then require every committed entry to appear in the read-back
/// with an equal value (keys compared in their `:0`-elided canonical form,
/// master04 §Field Identifiers). Read-back surplus (template-derived
/// defaults such as `category|*`, terminology qualifiers, server-assigned
/// ids) is tolerated: the export is the full RM projection of the committed
/// data (`simplified_formats` master04), so the round-trip guarantee is
/// that no committed datum is lost or altered. (No openEHR spec defines a
/// round-trip comparator — our own design over the master04/master06
/// semantics.)
fn flat_equivalent(
    actual: &BTreeMap<String, Value>,
    committed: &BTreeMap<String, Value>,
    ignored_paths: &[String],
) -> bool {
    let root = actual
        .keys()
        .find(|k| !k.starts_with("ctx/"))
        .and_then(|k| k.split(['/', ':']).next())
        .unwrap_or_default()
        .to_owned();
    let normalized: BTreeMap<String, &Value> = actual.iter().map(|(k, v)| (dezero(k), v)).collect();
    let folded = fold_flat_ctx(committed, &root);
    folded
        .iter()
        .filter(|(k, _)| !flat_key_ignored(k, ignored_paths))
        .all(|(k, want)| {
            normalized
                .get(&dezero(k))
                .is_some_and(|got| resultset::cells_equal(got, want))
        })
}

/// The `equivalent` comparison: structural equality after stripping the
/// resolved ignore paths from BOTH sides (numeric leaves by value, via the
/// result-set cell rule — canonical JSON carries RM numbers), with canonical
/// `_type` self-tag PRESENCE normalized (see [`rm_cells_equal`]). FLAT bodies
/// take the master06-aware round-trip rule ([`flat_equivalent`]).
#[must_use]
pub fn equivalent(actual: &Value, expected: &Value, ignored_paths: &[String]) -> bool {
    if let (Some(a), Some(e)) = (simplified_as_flat(actual), simplified_as_flat(expected)) {
        return flat_equivalent(&a, &e, ignored_paths);
    }
    let a = strip_ignored(actual, ignored_paths);
    let b = strip_ignored(expected, ignored_paths);
    rm_cells_equal(&a, &b)
}

/// Canonical-RM structural equality for `equivalent`: the result-set cell
/// rule everywhere, EXCEPT that a `_type` self-tag present on only one side
/// of an object is not a content difference. ITS-REST overview Resources.md
/// §JSON Format makes `_type` presence conditional ("should be used to
/// specify the RM type whenever polymorphism is involved, or when the
/// underlying definition in RM type is abstract") while the MUST governs its
/// VALUE — so a decode→re-encode path that self-tags every object (the
/// canonical codec) and a sparsely-tagged committed twin describe the same
/// RM content. Present on BOTH sides, the tags must be equal (a genuine
/// polymorphic-type substitution stays detectable).
fn rm_cells_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let keys: std::collections::BTreeSet<&str> = x
                .keys()
                .chain(y.keys())
                .map(String::as_str)
                .filter(|k| *k != "_type")
                .collect();
            let type_tags_agree = match (x.get("_type"), y.get("_type")) {
                (Some(ta), Some(tb)) => ta == tb,
                _ => true,
            };
            type_tags_agree
                && keys.iter().all(|k| match (x.get(*k), y.get(*k)) {
                    (Some(va), Some(vb)) => rm_cells_equal(va, vb),
                    _ => false,
                })
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(va, vb)| rm_cells_equal(va, vb))
        }
        _ => resultset::cells_equal(a, b),
    }
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
