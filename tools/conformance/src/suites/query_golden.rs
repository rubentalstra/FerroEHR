//! QUERY / AQL golden-diff corpus cases + the golden-result normalizer
//! (area `Qry`; `docs/design/conformance/07-querying.md`).
//!
//! This module owns two things:
//!
//! 1. **The golden normalizer** ([`Rule`], [`compare`]) — the documented
//! suppression layer a served `RESULT_SET` is diffed through. Design §6: *"a
//! diff suppressed by the normalizer must name its rule"* — every suppressed
//! difference records the [`Rule`] that justified it, so the suppression set
//! is auditable. Register 07 G-4 boundary: rules that are **version-specific**
//! (RM 1.2.0 `_type`, default-on `signature`, whole-number formatting,
//! `meta._schema_version`) carry an edition-rung comment — they suppress a
//! *development-edition* wire shape and would be ladder assertions in a full
//! `RESULT_SET` wire adapter (register 90, not yet exposed).
//!
//! 2. **The golden-diff cases** ([`entries`]) — the eight carried per-group ×
//! per-DB-state cases (legacy slugs `qry/corpus-{a-d}-{empty,loaded}-db`),
//! plus the ten **dialect** cases broken out so the vendored-data defects can
//! be adjudicated per-golden.
//!
//! ## Golden-dialect handling — register 07 G-3 (adjudications OUT of code)
//!
//! The two dialect defects are **NOT** dispositioned in case bodies. They are
//! committed in `adjudications/ecc-own.toml` and applied by the runner
//! ([`crate::engine::run`]):
//!
//! - **LIMIT-before-ORDER-BY** — the 2019-era corpus places `LIMIT` before
//! `ORDER BY`, invalid under the AQL 1.1 grammar (`AqlParser.g4`
//! `orderByClause? limitClause?`). Disposition **`corpus-dialect`** → the
//! runner skips the dedicated case with the citation.
//! - **TIMEWINDOW** — AQL 1.1 removed the clause (QUERY `master00-amendment_record`
//! SPECQUERY-20). Disposition **`spec-supersedes-corpus`** → the case runs
//! against the spec-derived expectation (the query must be rejected, `4xx`).
//!
//! The suite carries **no hardcoded skip list**: the eight group cases route a
//! dialect golden to its dedicated case via the spec-cited classifiers
//! [`is_timewindow_query`] / [`is_limit_before_order_by`] (a routing decision,
//! not a silent skip — each golden is asserted by exactly one case), and the
//! dispositions live in the committed register. The dedicated dialect case
//! bodies assert the spec-derived outcome (`4xx`) so they stay green whether or
//! not the runner short-circuits them.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::engine::harness::{CaseError, CaseFuture, DataSetReport, HttpRequest, RunContext};
use crate::engine::registry::CaseEntry;
use crate::model::case::{Binding, Capability, CaseMeta, Compare, Format, ScheduleTrace};
use crate::model::catalog::Area;
use crate::testdata::fixtures;

const JSON: &[Format] = &[Format::Json];

const CORPUS_CITATION: &str = "AQL 1.1 + the vendored golden RESULT_SETs; ITS-REST 1.0.3 QUERY API §execute_ad_hoc_query \
     200_QUERY.yaml; reference: CNF query corpus expected_results";

// ════════════════════════════════════════════════════════════════════════════
// The golden normalizer
// ════════════════════════════════════════════════════════════════════════════

/// A normalization rule that can suppress a difference between a served
/// `RESULT_SET` and a vendored golden. Each variant documents exactly one class
/// of legitimately-ignored difference (design §6). Rules marked
/// **VERSION-SPECIFIC** suppress a development-edition wire shape (register 07
/// G-4) and would be edition-ladder assertions in a full `RESULT_SET` wire
/// adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    /// The `meta` envelope (`_type`, `_created`, `_executed_aql`,
    /// `_schema_version`) is transport metadata, not a query result; ignored.
    /// VERSION-SPECIFIC (`_schema_version`): the schema-version field is a
    /// development-edition/RM-1.2.0 artefact — never asserted, always ignored.
    MetaEnvelopeIgnored,
    /// The top-level `q` echo of the request AQL is ignored (not a result).
    QueryEchoIgnored,
    /// VERSION-SPECIFIC: the RM `_type` discriminator is dropped anywhere in the
    /// tree — our RM 1.2.0 output carries it; RM-1.0.x-era goldens omit it. On
    /// the edition ladder this is the development rung; an RM-1.0.x SUT would
    /// match without it.
    RmTypeIgnored,
    /// VERSION-SPECIFIC: the default-on `signature` (design §6 `SignatureDefaultOn`)
    /// is dropped anywhere in the tree — our SUT signs versions by default; the
    /// goldens predate it. Development-rung behaviour.
    SignatureDefaultOn,
    /// VERSION-SPECIFIC: numeric scalars compare by value, not representation
    /// (`120` vs `120.0`) — whole-number formatting differs by RM version.
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
/// the [`Rule`] that justifies it. VERSION-SPECIFIC (development rung).
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

/// The outcome of a golden comparison.
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

/// Whether `s` is an upstream substitution token (`__MODIFY_…__`, any
/// `__UPPER_SNAKE__`). Such a golden scalar matches any served value.
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

/// Compare a served `RESULT_SET` against a golden in `mode`. The `meta` envelope
/// and `q` echo are always ignored.
#[must_use]
pub fn compare(golden: &Value, actual: &Value, mode: Mode) -> Comparison {
    let mut applied = BTreeSet::new();
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
/// multiset.
fn rows_match(golden: &[Value], actual: &[Value], applied: &mut BTreeSet<Rule>) -> bool {
    let mut claimed = vec![false; actual.len()];
    for (gi, g) in golden.iter().enumerate() {
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
        return s;
    }
    let mut end = 240;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

// ════════════════════════════════════════════════════════════════════════════
// Dialect classifiers (spec-cited routing — dispositions live in ecc-own.toml)
// ════════════════════════════════════════════════════════════════════════════

/// Whether a query uses the `TIMEWINDOW` clause. AQL 1.1 removed it (QUERY
/// `master00-amendment_record` SPECQUERY-20), so such a query is invalid AQL and
/// the SUT must reject it. Used only to **route** the corpus golden to its
/// dedicated `qry/dialect-timewindow-*` case (the disposition is committed as
/// `spec-supersedes-corpus` in `adjudications/ecc-own.toml`).
#[must_use]
pub fn is_timewindow_query(aql: &str) -> bool {
    aql.to_ascii_uppercase().contains(" TIMEWINDOW ")
}

/// Whether a query places `LIMIT` **before** `ORDER BY` — the 2019-era corpus
/// dialect, invalid under the AQL 1.1 grammar `orderByClause? limitClause?`
/// (`AqlParser.g4`). Used only to **route** the corpus golden to its dedicated
/// `qry/dialect-limit-*` case (the disposition is committed as `corpus-dialect`
/// in `adjudications/ecc-own.toml`).
#[must_use]
pub fn is_limit_before_order_by(aql: &str) -> bool {
    let up = aql.to_ascii_uppercase();
    match (up.find(" LIMIT "), up.find(" ORDER BY ")) {
        (Some(limit), Some(order)) => limit < order,
        _ => false,
    }
}

/// Whether a query is routed to a dedicated dialect case (excluded from the
/// group diff so no golden is double-covered).
fn is_dialect_routed(aql: &str) -> bool {
    is_timewindow_query(aql) || is_limit_before_order_by(aql)
}

/// A query fixture's text is unrunnable verbatim if it carries an upstream
/// `__MODIFY_…__` substitution token or a `$`-bind parameter (the harness would
/// substitute a runtime id / bind a value we do not have). This is a **harness
/// limitation**, not a corpus adjudication — the golden is not defective.
fn unrunnable(aql: &str) -> bool {
    aql.contains("__MODIFY_") || aql.contains('$')
}

// ════════════════════════════════════════════════════════════════════════════
// Cases
// ════════════════════════════════════════════════════════════════════════════

/// The dialect cases: `(slug, group, query-fixture-name, kind)`. Enumerated from
/// the verified corpus (the same queries the group classifiers route out), so
/// each dialect golden is asserted by exactly one case. Dispositions are
/// committed in `adjudications/ecc-own.toml` keyed by the allocated ECC id.
const DIALECT_CASES: &[(&str, &str, &str, DialectKind)] = &[
    // TIMEWINDOW — spec-supersedes-corpus (SPECQUERY-20).
    (
        "qry/dialect-timewindow-a109",
        "A",
        "109_get_ehrs_within_timewindow.json",
        DialectKind::Timewindow,
    ),
    (
        "qry/dialect-timewindow-b103",
        "B",
        "103_get_compositions_within_timewindow.json",
        DialectKind::Timewindow,
    ),
    (
        "qry/dialect-timewindow-c103",
        "C",
        "103_get_entries_within_timewindow.json",
        DialectKind::Timewindow,
    ),
    // LIMIT-before-ORDER-BY — corpus-dialect (AqlParser.g4).
    (
        "qry/dialect-limit-a107",
        "A",
        "107_get_ehrs_top_5.json",
        DialectKind::LimitOrder,
    ),
    (
        "qry/dialect-limit-a110",
        "A",
        "110_get_ehrs_top_5_orderby.json",
        DialectKind::LimitOrder,
    ),
    (
        "qry/dialect-limit-b104",
        "B",
        "104_get_compositions_top_5_ordered_by_starttime_asc.json",
        DialectKind::LimitOrder,
    ),
    (
        "qry/dialect-limit-b105",
        "B",
        "105_get_compositions_top_5_ordered_by_starttime_desc.json",
        DialectKind::LimitOrder,
    ),
    (
        "qry/dialect-limit-b106",
        "B",
        "106_get_compositions_top_5_ordered_by_starttimevalue_asc.json",
        DialectKind::LimitOrder,
    ),
    (
        "qry/dialect-limit-d312",
        "D",
        "312_select_data_values_from_all_ehrs_contains_composition_with_archetype_top_5.json",
        DialectKind::LimitOrder,
    ),
    (
        "qry/dialect-limit-d313",
        "D",
        "313_select_data_values_from_all_ehrs_contains_composition_with_archetype_top_5_orderby.json",
        DialectKind::LimitOrder,
    ),
];

/// The two dialect classes.
#[derive(Debug, Clone, Copy)]
enum DialectKind {
    /// `TIMEWINDOW` removed by AQL 1.1 → spec-supersedes-corpus (assert reject).
    Timewindow,
    /// `LIMIT` before `ORDER BY` → corpus-dialect (runner skips; the body
    /// asserts the spec-derived reject as a self-consistent fallback).
    LimitOrder,
}

/// Every registered golden-diff + dialect case (8 group + 10 dialect).
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    let mut out = vec![
        group_case(
            "qry/corpus-a-empty-db",
            "AQL corpus — A empty db",
            run_a_empty,
        ),
        group_case(
            "qry/corpus-b-empty-db",
            "AQL corpus — B empty db",
            run_b_empty,
        ),
        group_case(
            "qry/corpus-c-empty-db",
            "AQL corpus — C empty db",
            run_c_empty,
        ),
        group_case(
            "qry/corpus-d-empty-db",
            "AQL corpus — D empty db",
            run_d_empty,
        ),
        group_case(
            "qry/corpus-a-loaded-db",
            "AQL corpus — A loaded db",
            run_a_loaded,
        ),
        group_case(
            "qry/corpus-b-loaded-db",
            "AQL corpus — B loaded db",
            run_b_loaded,
        ),
        group_case(
            "qry/corpus-c-loaded-db",
            "AQL corpus — C loaded db",
            run_c_loaded,
        ),
        group_case(
            "qry/corpus-d-loaded-db",
            "AQL corpus — D loaded db",
            run_d_loaded,
        ),
    ];
    out.extend(
        DIALECT_CASES
            .iter()
            .map(|&(slug, group, name, kind)| dialect_case(slug, group, name, kind)),
    );
    out
}

// ── entry builders ────────────────────────────────────────────────────────────

fn group_case(
    id: &'static str,
    title: &'static str,
    run: crate::engine::harness::CaseRun,
) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Qry,
            capability: Capability::AqlBasic,
            formats: JSON,
            citation: CORPUS_CITATION,
            schedule: ScheduleTrace::EccOriginal(
                "schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus",
            ),
            binding: Binding::Rest("POST /query/aql"),
            compare: Compare::IgnoreSet,
        },
        run,
    }
}

fn dialect_case(
    id: &'static str,
    group: &'static str,
    name: &'static str,
    kind: DialectKind,
) -> CaseEntry {
    // The run function needs the (group, name, kind) triple; CaseRun is a bare
    // fn pointer that cannot capture, so a distinct fn is generated per entry
    // via the DIALECT_RUN table (index-matched to DIALECT_CASES).
    let run = dialect_run_for(id);
    let (citation, schedule) = match kind {
        DialectKind::Timewindow => (
            "AQL 1.1 removed TIMEWINDOW (QUERY master00-amendment_record SPECQUERY-20); ITS-REST 1.0.3 \
             QUERY API §execute_ad_hoc_query 400_QUERY.yaml — invalid AQL must be rejected",
            "schedule stub (master11 is TBD); TIMEWINDOW golden, spec-supersedes-corpus (adjudications/ecc-own.toml)",
        ),
        DialectKind::LimitOrder => (
            "AQL 1.1 grammar orders orderByClause? limitClause? (AqlParser.g4); a LIMIT-before-ORDER-BY \
             query is invalid AQL — corpus-dialect defect (adjudications/ecc-own.toml)",
            "schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)",
        ),
    };
    let _ = (group, name); // consumed by the generated run fn.
    CaseEntry {
        meta: CaseMeta {
            id,
            title: dialect_title(id),
            area: Area::Qry,
            capability: Capability::AqlBasic,
            formats: JSON,
            citation,
            schedule: ScheduleTrace::EccOriginal(schedule),
            binding: Binding::Rest("POST /query/aql"),
            compare: Compare::None,
        },
        run,
    }
}

/// Box a plain async result as a [`CaseFuture`].
macro_rules! boxed {
    ($body:block) => {
        Box::pin(async move $body)
    };
}

fn codec(e: &fixtures::FixtureError) -> CaseError {
    CaseError::Codec(e.to_string())
}

// ── the group-diff runner ─────────────────────────────────────────────────────

/// Run every golden in `expected_results/<db>/<group>` against the SUT, diffing
/// the served `RESULT_SET` through the normalizer. `_empty_db` queries (fixed
/// non-existent id) get the full columns+rows diff; every other query gets the
/// columns-only diff (shared-SUT-safe). Structurally-unrunnable goldens
/// (substitution/bind) and dialect-routed goldens are skipped (the latter are
/// owned by their dedicated `qry/dialect-*` case — routing, not a silent skip).
async fn run_golden_group(
    ctx: &RunContext<'_>,
    group: &str,
    db: &str,
) -> Result<DataSetReport, CaseError> {
    let goldens = fixtures::aql_expected(db, group).map_err(|e| codec(&e))?;
    let mut passed = 0u32;
    let mut total = 0u32;
    let mut skipped = 0u32;
    let mut first_fail: Option<String> = None;

    for gold in goldens {
        let Some(aql) = paired_query(group, &gold.name)? else {
            continue; // a golden with no paired query fixture
        };
        // Dialect goldens are asserted by their dedicated cases + committed
        // adjudications (register 07 G-3); the group routes them out.
        if is_dialect_routed(&aql) || unrunnable(&aql) {
            skipped += 1;
            continue;
        }
        total += 1;
        let golden_value = gold.json().map_err(|e| codec(&e))?;
        let resp = adhoc(ctx, &aql).await?;
        if resp.status != 200 {
            first_fail.get_or_insert(format!(
                "{group}/{}: valid query rejected with status {}",
                gold.name, resp.status
            ));
            continue;
        }
        let mode = if gold.name.ends_with("_empty_db.json") {
            Mode::Full
        } else {
            Mode::ColumnsOnly
        };
        let cmp = compare(&golden_value, &resp.json()?, mode);
        if cmp.matched {
            passed += 1;
        } else {
            first_fail.get_or_insert(format!(
                "{group}/{} ({mode:?}, suppressed via [{}]): {}",
                gold.name,
                cmp.applied_labels(),
                cmp.detail.unwrap_or_default()
            ));
        }
    }

    if total == 0 {
        return Err(CaseError::Skipped(format!(
            "all {skipped} {group}/{db} goldens are dialect-routed or require id-substitution/binds"
        )));
    }
    if passed == total {
        Ok(DataSetReport::all(passed))
    } else {
        Err(CaseError::Assertion(format!(
            "{passed}/{total} {group}/{db} goldens matched ({skipped} skipped); first divergence: {}",
            first_fail.unwrap_or_default()
        )))
    }
}

/// The paired query text for a golden of `name` in `group` (paired by identical
/// base name), or `None` when the golden has no paired query fixture.
fn paired_query(group: &str, golden_name: &str) -> Result<Option<String>, CaseError> {
    let fixtures = fixtures::aql_valid(group).map_err(|e| codec(&e))?;
    match fixtures.iter().find(|f| f.name == golden_name) {
        Some(fixture) => Ok(Some(fixtures::aql_text(fixture).map_err(|e| codec(&e))?)),
        None => Ok(None),
    }
}

/// Execute an ad-hoc AQL query (`POST /query/aql`, body `{"q": …}`).
async fn adhoc(
    ctx: &RunContext<'_>,
    aql: &str,
) -> Result<crate::engine::harness::HttpResponse, CaseError> {
    ctx.send(
        HttpRequest::post("/query/aql")
            .json_body(&serde_json::json!({ "q": aql }))?
            .header("accept", "application/json"),
    )
    .await
}

// ── per-group×db group-case fns ────────────────────────────────────────────────

fn run_a_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ run_golden_group(ctx, "A", "empty_db").await })
}
fn run_b_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ run_golden_group(ctx, "B", "empty_db").await })
}
fn run_c_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ run_golden_group(ctx, "C", "empty_db").await })
}
fn run_d_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ run_golden_group(ctx, "D", "empty_db").await })
}
fn run_a_loaded<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ run_golden_group(ctx, "A", "loaded_db").await })
}
fn run_b_loaded<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ run_golden_group(ctx, "B", "loaded_db").await })
}
fn run_c_loaded<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ run_golden_group(ctx, "C", "loaded_db").await })
}
fn run_d_loaded<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ run_golden_group(ctx, "D", "loaded_db").await })
}

// ── dialect cases: assert the spec-derived reject (4xx) ────────────────────────

/// A dialect query must be rejected: TIMEWINDOW (removed from AQL 1.1) and
/// LIMIT-before-ORDER-BY (invalid clause order) are both invalid AQL, so the SUT
/// must return a `4xx`. For the `corpus-dialect` (LIMIT) cases the runner
/// short-circuits to `Skipped` via `ecc-own.toml`; this body is the
/// self-consistent fallback so the case is green either way.
async fn run_dialect_reject(
    ctx: &RunContext<'_>,
    group: &str,
    name: &str,
) -> Result<DataSetReport, CaseError> {
    let fixtures = fixtures::aql_valid(group).map_err(|e| codec(&e))?;
    let fixture = fixtures.iter().find(|f| f.name == name).ok_or_else(|| {
        CaseError::Assertion(format!("missing dialect query fixture {group}/{name}"))
    })?;
    let aql = fixtures::aql_text(fixture).map_err(|e| codec(&e))?;
    let resp = adhoc(ctx, &aql).await?;
    if (400..500).contains(&resp.status) {
        Ok(DataSetReport::SINGLE)
    } else {
        Err(CaseError::Assertion(format!(
            "dialect query {group}/{name} is invalid AQL 1.1 and must be rejected (4xx), got {}",
            resp.status
        )))
    }
}

/// The title for a dialect case slug (looked up in [`DIALECT_CASES`]).
fn dialect_title(id: &str) -> &'static str {
    DIALECT_CASES
        .iter()
        .find(|(slug, _, _, _)| *slug == id)
        .map_or(
            "AQL corpus — dialect query",
            |_| "AQL corpus — dialect-adjudicated query rejected",
        )
}

/// Resolve the generated run fn for a dialect slug. Each slug has a distinct,
/// zero-capture fn (fn pointers cannot capture the group/name), generated by
/// [`dialect_runs!`] and matched here by slug.
fn dialect_run_for(id: &str) -> crate::engine::harness::CaseRun {
    dialect_run_lookup(id).unwrap_or(run_dialect_missing)
}

/// Fallback run fn for an unknown dialect slug (defensive; never hit for a
/// registered case).
fn run_dialect_missing<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Err::<DataSetReport, _>(CaseError::Assertion(
            "no dialect run fn registered for this slug".to_owned(),
        ))
    })
}

/// Generate one zero-capture run fn per dialect case + the slug→fn lookup.
macro_rules! dialect_runs {
    ($( ($fn:ident, $slug:literal, $group:literal, $name:literal) ),* $(,)?) => {
        $(
            fn $fn<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
                boxed!({ run_dialect_reject(ctx, $group, $name).await })
            }
        )*
        fn dialect_run_lookup(id: &str) -> Option<crate::engine::harness::CaseRun> {
            match id {
                $( $slug => Some($fn), )*
                _ => None,
            }
        }
    };
}

dialect_runs![
    (
        run_tw_a109,
        "qry/dialect-timewindow-a109",
        "A",
        "109_get_ehrs_within_timewindow.json"
    ),
    (
        run_tw_b103,
        "qry/dialect-timewindow-b103",
        "B",
        "103_get_compositions_within_timewindow.json"
    ),
    (
        run_tw_c103,
        "qry/dialect-timewindow-c103",
        "C",
        "103_get_entries_within_timewindow.json"
    ),
    (
        run_lim_a107,
        "qry/dialect-limit-a107",
        "A",
        "107_get_ehrs_top_5.json"
    ),
    (
        run_lim_a110,
        "qry/dialect-limit-a110",
        "A",
        "110_get_ehrs_top_5_orderby.json"
    ),
    (
        run_lim_b104,
        "qry/dialect-limit-b104",
        "B",
        "104_get_compositions_top_5_ordered_by_starttime_asc.json"
    ),
    (
        run_lim_b105,
        "qry/dialect-limit-b105",
        "B",
        "105_get_compositions_top_5_ordered_by_starttime_desc.json"
    ),
    (
        run_lim_b106,
        "qry/dialect-limit-b106",
        "B",
        "106_get_compositions_top_5_ordered_by_starttimevalue_asc.json"
    ),
    (
        run_lim_d312,
        "qry/dialect-limit-d312",
        "D",
        "312_select_data_values_from_all_ehrs_contains_composition_with_archetype_top_5.json"
    ),
    (
        run_lim_d313,
        "qry/dialect-limit-d313",
        "D",
        "313_select_data_values_from_all_ehrs_contains_composition_with_archetype_top_5_orderby.json"
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn golden(columns: &Value, rows: &Value) -> Value {
        json!({
            "meta": { "_type": "RESULTSET", "_created": "2019-10-19T19:23:02.672Z" },
            "q": "SELECT e/ehr_id/value FROM EHR e",
            "columns": columns,
            "rows": rows,
        })
    }

    #[test]
    fn placeholder_detection() {
        assert!(is_placeholder("__MODIFY_EHR_ID_1__"));
        assert!(!is_placeholder("__mixed_Case__"));
        assert!(!is_placeholder("plain value"));
    }

    #[test]
    fn columns_only_ignores_rows() {
        let g = golden(
            &json!([{"name": "#0", "path": "/ehr_id/value"}]),
            &json!([]),
        );
        let mut a = g.clone();
        a["rows"] = json!([["some-ehr-id"]]);
        assert!(compare(&g, &a, Mode::ColumnsOnly).matched);
        assert!(!compare(&g, &a, Mode::Full).matched);
    }

    #[test]
    fn modify_placeholder_matches_any_value() {
        let cols = json!([{"name": "#0", "path": "/ehr_id/value"}]);
        let g = golden(&cols, &json!([["__MODIFY_EHR_ID_1__"]]));
        let mut a = golden(&cols, &json!([]));
        a["rows"] = json!([["real-ehr-a"]]);
        let c = compare(&g, &a, Mode::Full);
        assert!(c.matched, "{:?}", c.detail);
        assert!(c.applied.contains(&Rule::ModifyPlaceholderWildcard));
    }

    #[test]
    fn rm_type_and_signature_and_number_ignored() {
        let cols = json!([{"name": "#0", "path": "/uid"}]);
        let g = golden(&cols, &json!([[{"value": "x"}]]));
        let a = golden(
            &cols,
            &json!([[{"value": "x", "_type": "HIER_OBJECT_ID", "signature": "sha256:..."}]]),
        );
        let c = compare(&g, &a, Mode::Full);
        assert!(c.matched, "{:?}", c.detail);
        assert!(c.applied.contains(&Rule::RmTypeIgnored));
        assert!(c.applied.contains(&Rule::SignatureDefaultOn));
    }

    #[test]
    fn dialect_classifiers() {
        assert!(is_timewindow_query(
            "SELECT e FROM EHR e TIMEWINDOW PT12H/2019-10-24"
        ));
        assert!(is_limit_before_order_by(
            "SELECT e FROM EHR e LIMIT 5 ORDER BY e/x"
        ));
        assert!(!is_limit_before_order_by(
            "SELECT e FROM EHR e ORDER BY e/x LIMIT 5"
        ));
    }

    #[test]
    fn every_dialect_case_has_a_run_fn() {
        for (slug, _, _, _) in DIALECT_CASES {
            assert!(dialect_run_lookup(slug).is_some(), "no run fn for {slug}");
        }
    }
}
