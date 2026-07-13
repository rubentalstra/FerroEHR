//! The QUERY golden-result normalizer (design §2.2a, §6).
//!
//! The vendored AQL corpus ships golden `RESULT_SET`s under
//! `query/expected_results/{empty_db,loaded_db}/{A-D}/`. Diffing a served
//! `RESULT_SET` against a golden must first pass through a **documented
//! normalizer** that suppresses the differences that are legitimately
//! SUT-specific / RM-version-sensitive rather than conformance failures — and,
//! per design §6, *"a diff suppressed by the normalizer must name its rule"*.
//! Every rule below is a [`Rule`] variant, and [`compare`] reports exactly which
//! rules it invoked to reach a match.
//!
//! ## Why the goldens cannot be diffed verbatim
//!
//! - The `meta` envelope carries a generation `_created` timestamp and an
//!   `_executed_aql`/`q` echo of the request — none of which are query
//!   *results* ([`Rule::MetaEnvelopeIgnored`], [`Rule::QueryEchoIgnored`]).
//! - `loaded_db` goldens do not hold literal values: every result cell is an
//!   upstream **substitution token** of the form `__MODIFY_…__` (the harness
//!   replaced it with a runtime-generated EHR id / composition uid / timestamp
//!   after loading data). A golden token therefore matches *any* served value
//!   ([`Rule::ModifyPlaceholderWildcard`]).
//! - Our RM 1.2.0 output carries a `_type` discriminator (often absent from the
//!   RM-1.0.x-era goldens) and a default-on `signature` the goldens predate
//!   ([`Rule::RmTypeIgnored`], [`Rule::SignatureDefaultOn`] — the
//!   `SignatureDefaultOn` rule of design §6).
//! - Whole-number formatting differs by RM version (`120` vs `120.0`), so
//!   numeric cells compare by value, not representation
//!   ([`Rule::NumberFormatInsensitive`]).
//! - AQL without `ORDER BY` leaves row order unspecified, so rows are matched as
//!   an order-insensitive multiset ([`Rule::RowOrderInsensitive`]).
//!
//! The `columns` array (`{name, path}`) is the golden's **deterministic**,
//! data-independent part — a pure function of the SELECT clause — so it is the
//! primary oracle ([`Mode::ColumnsOnly`]); [`Mode::Full`] additionally diffs the
//! `rows` and is used for the empty-DB literal-filter queries whose result is
//! DB-state-independent (they filter by a fixed non-existent id).

use std::collections::BTreeSet;

use serde_json::Value;

/// A normalization rule that can suppress a difference between a served
/// `RESULT_SET` and a vendored golden. Each variant documents exactly one class
/// of legitimately-ignored difference (design §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    /// The `meta` envelope (`_type`, `_created`, `_executed_aql`,
    /// `_schema_version`) is transport metadata, not a query result; ignored.
    MetaEnvelopeIgnored,
    /// The top-level `q` echo of the request AQL is ignored (not a result).
    QueryEchoIgnored,
    /// The RM `_type` discriminator is dropped anywhere in the tree (our RM
    /// 1.2.0 output carries it; RM-1.0.x-era goldens frequently omit it).
    RmTypeIgnored,
    /// The default-on `signature` (design §6 `SignatureDefaultOn`) is dropped
    /// anywhere in the tree (our SUT signs versions; the goldens predate it).
    SignatureDefaultOn,
    /// Numeric scalars compare by value, not representation (`120` vs `120.0`).
    NumberFormatInsensitive,
    /// A golden `__MODIFY_…__` substitution token matches any served value (the
    /// upstream loaded-DB placeholders, replaced with runtime ids at load time).
    ModifyPlaceholderWildcard,
    /// Rows are matched as an order-insensitive multiset (AQL without `ORDER BY`
    /// leaves row order unspecified).
    RowOrderInsensitive,
}

impl Rule {
    /// A stable snake-case label for report messages.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Rule::MetaEnvelopeIgnored => "meta_envelope_ignored",
            Rule::QueryEchoIgnored => "query_echo_ignored",
            Rule::RmTypeIgnored => "rm_type_ignored",
            Rule::SignatureDefaultOn => "signature_default_on",
            Rule::NumberFormatInsensitive => "number_format_insensitive",
            Rule::ModifyPlaceholderWildcard => "modify_placeholder_wildcard",
            Rule::RowOrderInsensitive => "row_order_insensitive",
        }
    }
}

/// Object keys dropped anywhere in the tree before comparing, each recorded as
/// the [`Rule`] that justifies it.
const IGNORE_KEYS: [(&str, Rule); 2] = [
    ("_type", Rule::RmTypeIgnored),
    ("signature", Rule::SignatureDefaultOn),
];

/// How much of the `RESULT_SET` to diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Diff only `columns` (the deterministic, data-independent projection) —
    /// sound against a shared, non-empty SUT.
    ColumnsOnly,
    /// Diff `columns` **and** `rows` — used for DB-state-independent queries
    /// (empty-DB literal-id filters) whose full result is predictable.
    Full,
}

/// The outcome of a golden comparison: whether it matched, which normalization
/// rules were invoked to suppress a difference, and (on mismatch) the detail.
#[derive(Debug, Clone)]
pub struct Comparison {
    /// Whether the served `RESULT_SET` matched the golden after normalization.
    pub matched: bool,
    /// The normalization rules that were invoked (a suppressed diff names its
    /// rule, design §6).
    pub applied: BTreeSet<Rule>,
    /// On mismatch, a human-readable description of what still differed.
    pub detail: Option<String>,
}

impl Comparison {
    fn ok(applied: BTreeSet<Rule>) -> Self {
        Self {
            matched: true,
            applied,
            detail: None,
        }
    }

    fn fail(applied: BTreeSet<Rule>, detail: String) -> Self {
        Self {
            matched: false,
            applied,
            detail: Some(detail),
        }
    }

    /// The applied rules rendered as a comma-separated label list (for reports).
    #[must_use]
    pub fn applied_labels(&self) -> String {
        self.applied
            .iter()
            .map(|r| r.label())
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// Whether `s` is an upstream substitution token (`__MODIFY_…__`,
/// `__IGNORE_…__`, or any `__UPPER_SNAKE__`). Such a golden scalar matches any
/// served value ([`Rule::ModifyPlaceholderWildcard`]).
#[must_use]
pub fn is_placeholder(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() > 4
        && s.starts_with("__")
        && s.ends_with("__")
        && s[2..s.len() - 2]
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

/// Compare a served `RESULT_SET` against a golden in `mode`, returning the
/// [`Comparison`]. The `meta` envelope and `q` echo are always ignored.
#[must_use]
pub fn compare(golden: &Value, actual: &Value, mode: Mode) -> Comparison {
    let mut applied = BTreeSet::new();
    // The envelope + echo are structurally present on both and always ignored.
    applied.insert(Rule::MetaEnvelopeIgnored);
    applied.insert(Rule::QueryEchoIgnored);

    // (1) columns — the deterministic oracle.
    let g_cols = golden.get("columns").unwrap_or(&Value::Null);
    let a_cols = actual.get("columns").unwrap_or(&Value::Null);
    if !matches(g_cols, a_cols, &mut applied) {
        return Comparison::fail(
            applied,
            format!(
                "columns differ: golden={}, served={}",
                compact(g_cols),
                compact(a_cols)
            ),
        );
    }

    if mode == Mode::ColumnsOnly {
        return Comparison::ok(applied);
    }

    // (2) rows — order-insensitive multiset with wildcard/number/ignore-key
    // normalization.
    let empty = Vec::new();
    let g_rows = golden
        .get("rows")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let a_rows = actual
        .get("rows")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    if g_rows.len() != a_rows.len() {
        return Comparison::fail(
            applied,
            format!(
                "row count differs: golden={}, served={}",
                g_rows.len(),
                a_rows.len()
            ),
        );
    }
    if !rows_match(g_rows, a_rows, &mut applied) {
        return Comparison::fail(
            applied,
            format!("{} row(s) did not match after normalization", g_rows.len()),
        );
    }
    Comparison::ok(applied)
}

/// Match two golden/served row lists of equal length as an order-insensitive
/// multiset: each golden row must find an as-yet-unclaimed served row it
/// matches. Records [`Rule::RowOrderInsensitive`] when a non-identity pairing
/// was needed.
fn rows_match(golden: &[Value], actual: &[Value], applied: &mut BTreeSet<Rule>) -> bool {
    let mut claimed = vec![false; actual.len()];
    for (gi, g) in golden.iter().enumerate() {
        // Prefer the positional partner to keep ordered results honest.
        let mut found = None;
        if gi < actual.len() && !claimed[gi] {
            let mut probe = BTreeSet::new();
            if matches(g, &actual[gi], &mut probe) {
                found = Some((gi, probe, false));
            }
        }
        if found.is_none() {
            for (ai, a) in actual.iter().enumerate() {
                if claimed[ai] {
                    continue;
                }
                let mut probe = BTreeSet::new();
                if matches(g, a, &mut probe) {
                    found = Some((ai, probe, true));
                    break;
                }
            }
        }
        match found {
            Some((ai, probe, reordered)) => {
                claimed[ai] = true;
                applied.extend(probe);
                if reordered {
                    applied.insert(Rule::RowOrderInsensitive);
                }
            }
            None => return false,
        }
    }
    true
}

/// Structural match with normalization, recording each invoked [`Rule`].
fn matches(golden: &Value, actual: &Value, applied: &mut BTreeSet<Rule>) -> bool {
    // Wildcard: a golden substitution token matches anything.
    if let Value::String(s) = golden
        && is_placeholder(s)
    {
        applied.insert(Rule::ModifyPlaceholderWildcard);
        return true;
    }
    match (golden, actual) {
        (Value::Object(_), Value::Object(_)) => {
            let g = stripped(golden, applied);
            let a = stripped(actual, applied);
            let (Value::Object(gm), Value::Object(am)) = (&g, &a) else {
                return false;
            };
            if gm.len() != am.len() {
                return false;
            }
            gm.iter()
                .all(|(k, gv)| am.get(k).is_some_and(|av| matches(gv, av, applied)))
        }
        (Value::Array(g), Value::Array(a)) => {
            g.len() == a.len() && g.iter().zip(a).all(|(gv, av)| matches(gv, av, applied))
        }
        (Value::Number(g), Value::Number(a)) => {
            if g == a {
                true
            } else if let (Some(gf), Some(af)) = (g.as_f64(), a.as_f64()) {
                if (gf - af).abs() < f64::EPSILON {
                    applied.insert(Rule::NumberFormatInsensitive);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        }
        _ => golden == actual,
    }
}

/// Drop the [`IGNORE_KEYS`] from an object's top level (recursion is handled by
/// [`matches`]), recording the rule for each key actually removed.
fn stripped(value: &Value, applied: &mut BTreeSet<Rule>) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                if let Some((_, rule)) = IGNORE_KEYS.iter().find(|(key, _)| key == k) {
                    applied.insert(*rule);
                    continue;
                }
                out.insert(k.clone(), v.clone());
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

/// A compact, length-bounded rendering of a value for a failure message.
fn compact(v: &Value) -> String {
    let s = v.to_string();
    if s.len() <= 240 {
        s
    } else {
        let mut end = 240;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn golden(columns: Value, rows: Value) -> Value {
        json!({
            "meta": {
                "_type": "RESULTSET",
                "_created": "2019-10-19T19:23:02.672Z",
                "_executed_aql": "SELECT e/ehr_id/value FROM EHR e",
            },
            "q": "SELECT e/ehr_id/value FROM EHR e",
            "columns": columns,
            "rows": rows,
        })
    }

    #[test]
    fn placeholder_detection() {
        assert!(is_placeholder("__MODIFY_EHR_ID_1__"));
        assert!(is_placeholder("__MODIFY_COMPOSITION_UID_1_VALUE__"));
        assert!(is_placeholder("__IGNORE__"));
        assert!(!is_placeholder("__mixed_Case__"));
        assert!(!is_placeholder("plain value"));
        assert!(!is_placeholder("____"));
        assert!(!is_placeholder("__ __"));
    }

    #[test]
    fn meta_and_echo_always_ignored() {
        // Identical columns + rows but a wildly different meta/_created and q.
        let g = golden(json!([{"name": "#0", "path": "/ehr_id/value"}]), json!([]));
        let mut a = g.clone();
        a["meta"]["_created"] = json!("2099-01-01T00:00:00Z");
        a["meta"]["_schema_version"] = json!("1.0.3");
        a["q"] = json!("SELECT something ELSE");
        let c = compare(&g, &a, Mode::Full);
        assert!(c.matched, "{:?}", c.detail);
        assert!(c.applied.contains(&Rule::MetaEnvelopeIgnored));
        assert!(c.applied.contains(&Rule::QueryEchoIgnored));
    }

    #[test]
    fn columns_only_ignores_rows() {
        let g = golden(json!([{"name": "#0", "path": "/ehr_id/value"}]), json!([]));
        let mut a = g.clone();
        a["rows"] = json!([["some-ehr-id"]]); // populated SUT
        assert!(compare(&g, &a, Mode::ColumnsOnly).matched);
        // Full would reject the row-count difference.
        assert!(!compare(&g, &a, Mode::Full).matched);
    }

    #[test]
    fn column_path_divergence_is_reported() {
        let g = golden(json!([{"name": "#0", "path": "/ehr_id/value"}]), json!([]));
        // Our engine omits `path` for EHR pseudo-attribute columns (F-open-20).
        let mut a = g.clone();
        a["columns"] = json!([{"name": "#0"}]);
        let c = compare(&g, &a, Mode::ColumnsOnly);
        assert!(!c.matched);
        assert!(c.detail.unwrap().contains("columns differ"));
    }

    #[test]
    fn modify_placeholder_matches_any_value() {
        let cols = json!([{"name": "#0", "path": "/ehr_id/value"}]);
        let g = golden(
            cols.clone(),
            json!([["__MODIFY_EHR_ID_1__"], ["__MODIFY_EHR_ID_2__"]]),
        );
        let mut a = golden(cols, json!([]));
        a["rows"] = json!([["real-ehr-a"], ["real-ehr-b"]]);
        let c = compare(&g, &a, Mode::Full);
        assert!(c.matched, "{:?}", c.detail);
        assert!(c.applied.contains(&Rule::ModifyPlaceholderWildcard));
    }

    #[test]
    fn number_format_insensitive() {
        let cols = json!([{"name": "#0", "path": "/magnitude"}]);
        let g = golden(cols.clone(), json!([[120]]));
        let a = golden(cols, json!([[120.0]]));
        let c = compare(&g, &a, Mode::Full);
        assert!(c.matched, "{:?}", c.detail);
        assert!(c.applied.contains(&Rule::NumberFormatInsensitive));
    }

    #[test]
    fn rm_type_and_signature_ignored_in_cells() {
        let cols = json!([{"name": "#0", "path": "/uid"}]);
        // Golden cell lacks _type; served cell adds RM 1.2.0 _type + signature.
        let g = golden(cols.clone(), json!([[{"value": "x"}]]));
        let a = golden(
            cols,
            json!([[{"value": "x", "_type": "HIER_OBJECT_ID", "signature": "sha256:..."}]]),
        );
        let c = compare(&g, &a, Mode::Full);
        assert!(c.matched, "{:?}", c.detail);
        assert!(c.applied.contains(&Rule::RmTypeIgnored));
        assert!(c.applied.contains(&Rule::SignatureDefaultOn));
    }

    #[test]
    fn row_order_insensitive_multiset() {
        let cols = json!([{"name": "#0", "path": "/ehr_id/value"}]);
        let g = golden(cols.clone(), json!([["a"], ["b"], ["c"]]));
        let a = golden(cols, json!([["c"], ["a"], ["b"]]));
        let c = compare(&g, &a, Mode::Full);
        assert!(c.matched, "{:?}", c.detail);
        assert!(c.applied.contains(&Rule::RowOrderInsensitive));
    }

    #[test]
    fn genuine_row_value_difference_still_fails() {
        let cols = json!([{"name": "#0", "path": "/ehr_id/value"}]);
        let g = golden(cols.clone(), json!([["expected"]]));
        let a = golden(cols, json!([["different"]]));
        let c = compare(&g, &a, Mode::Full);
        assert!(!c.matched);
    }

    #[test]
    fn empty_rows_match_exactly() {
        let cols = json!([{"name": "#0", "path": "/ehr_id/value"}]);
        let g = golden(cols.clone(), json!([]));
        let a = golden(cols, json!([]));
        assert!(compare(&g, &a, Mode::Full).matched);
    }
}
