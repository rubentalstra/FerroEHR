//! master11 — QUERY (AQL) cases + the `QUERY-FIXTURE-*` corpus (design §4.1,
//! §3.4, §6).
//!
//! master11 is a TBD stub upstream (only four non-placeholder headings —
//! `I_QUERY_SERVICE.{smoke_test, execute_stored_query-empty_db,
//! execute_ad_hoc_query-empty_db, execute_ad_hoc_query-loaded_db}`); its Flow
//! cells are `xx`. The real query evidence is the vendored AQL corpus
//! (`query/aql_queries_{valid,invalid}` + the `expected_results/{empty_db,
//! loaded_db}` golden `RESULT_SET`s), driven as `QUERY-FIXTURE-<group>-<db>`
//! `FixtureDerived` cases that sit **outside** the 322-case schedule inventory
//! (design §3.4, §4.2) — the coverage guard's Schedule-provenance-subset
//! invariant is unaffected.
//!
//! ## What is diffed, and why not verbatim
//!
//! Golden diffing runs through the documented [`golden`] normalizer (design §6;
//! see that module for the full rule catalogue). The corpus has two properties
//! that dictate the assertion:
//!
//! 1. **Shared, non-empty SUT.** The runner drives one SUT across all cases with
//!    no DB cleans (design §4.3), so global queries (`FROM EHR e`) see the EHRs
//!    other chapters created — their row *count* is not the golden's. The
//!    deterministic, data-independent part of a golden is its `columns`
//!    projection (a pure function of the SELECT clause), so that is the primary
//!    oracle ([`golden::Mode::ColumnsOnly`]).
//! 2. **`_empty_db` queries filter by a fixed non-existent id** (e.g.
//!    `[ehr_id/value='4cd7ed03-…']`), so their result is DB-state-independent
//!    (always zero rows) and the full `RESULT_SET` — columns **and** empty rows
//!    — is diffed ([`golden::Mode::Full`]).
//!
//! Queries whose text carries an upstream `__MODIFY_…__` substitution token or a
//! `$`-bind parameter (the `loaded_db` variants and a few `$uid` queries) cannot
//! be executed verbatim — the harness substitutes runtime ids we do not have —
//! so they are skipped and reported as such, never silently passed.
//!
//! Invalid queries (`aql_queries_invalid/**`) are load-bearing negatives: the
//! server must reject them (`400`/`422` per ITS-REST `400_QUERY.yaml`).

use uuid::Uuid;

use crate::assert;
use crate::case::{Capability, CaseMeta, Chapter, Compare, Format, Profile, Provenance};
use crate::fixtures::{self, Fixture};
use crate::harness::{CaseError, CaseFuture, DataSetReport, HttpRequest, HttpResponse, RunContext};
use crate::registry::CaseEntry;

#[path = "query_golden.rs"]
pub mod golden;

/// The A–D query corpus groups.
const GROUPS: [&str; 4] = ["A", "B", "C", "D"];

/// The implemented master11 + `QUERY-FIXTURE-*` case entries.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    let mut v = vec![
        // ── master11 real cases (Schedule provenance) ────────────────────────
        schedule("I_QUERY_SERVICE.smoke_test", run_smoke_test),
        schedule(
            "I_QUERY_SERVICE.execute_ad_hoc_query-empty_db",
            run_adhoc_empty_db,
        ),
        schedule(
            "I_QUERY_SERVICE.execute_stored_query-empty_db",
            run_stored_empty_db,
        ),
        schedule(
            "I_QUERY_SERVICE.execute_ad_hoc_query-loaded_db",
            run_adhoc_loaded_db,
        ),
        // ── fixture-derived: invalid queries must be rejected ────────────────
        fixture_derived(
            "QUERY-FIXTURE-invalid",
            "query/aql_queries_invalid/** §rejected",
            run_invalid_queries,
        ),
    ];
    // ── fixture-derived: golden diffs, per group × DB state ──────────────────
    v.push(fixture_derived(
        "QUERY-FIXTURE-A-empty_db",
        "query/expected_results/empty_db/A §columns/full",
        run_a_empty,
    ));
    v.push(fixture_derived(
        "QUERY-FIXTURE-B-empty_db",
        "query/expected_results/empty_db/B §columns/full",
        run_b_empty,
    ));
    v.push(fixture_derived(
        "QUERY-FIXTURE-C-empty_db",
        "query/expected_results/empty_db/C §columns/full",
        run_c_empty,
    ));
    v.push(fixture_derived(
        "QUERY-FIXTURE-D-empty_db",
        "query/expected_results/empty_db/D §columns/full",
        run_d_empty,
    ));
    v.push(fixture_derived(
        "QUERY-FIXTURE-A-loaded_db",
        "query/expected_results/loaded_db/A §columns",
        run_a_loaded,
    ));
    v.push(fixture_derived(
        "QUERY-FIXTURE-B-loaded_db",
        "query/expected_results/loaded_db/B §columns",
        run_b_loaded,
    ));
    v.push(fixture_derived(
        "QUERY-FIXTURE-C-loaded_db",
        "query/expected_results/loaded_db/C §columns",
        run_c_loaded,
    ));
    v.push(fixture_derived(
        "QUERY-FIXTURE-D-loaded_db",
        "query/expected_results/loaded_db/D §columns",
        run_d_loaded,
    ));
    v
}

/// A master11 Schedule-provenance case (`AqlBasic`, STANDARD, JSON).
fn schedule(id: &'static str, run: for<'a> fn(&'a RunContext<'a>) -> CaseFuture<'a>) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            chapter: Chapter::Master11,
            capability: Capability::AqlBasic,
            profiles: &[Profile::Standard],
            formats: &[Format::Json],
            provenance: Provenance::Schedule,
            schedule_ref: id,
            upstream_tags: &[],
            compare: Compare::IgnoreSet,
        },
        run,
    }
}

/// A `QUERY-FIXTURE-*` `FixtureDerived` case (outside the 322 inventory, §3.4).
fn fixture_derived(
    id: &'static str,
    schedule_ref: &'static str,
    run: for<'a> fn(&'a RunContext<'a>) -> CaseFuture<'a>,
) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            chapter: Chapter::Master11,
            capability: Capability::AqlBasic,
            profiles: &[Profile::Standard],
            formats: &[Format::Json],
            provenance: Provenance::FixtureDerived,
            schedule_ref,
            upstream_tags: &[],
            compare: Compare::IgnoreSet,
        },
        run,
    }
}

macro_rules! case {
    ($body:block) => {
        Box::pin(async move { $body })
    };
}

// ── shared helpers ────────────────────────────────────────────────────────────

fn fx(e: fixtures::FixtureError) -> CaseError {
    CaseError::Assertion(format!("fixture: {e}"))
}

/// Execute an ad-hoc AQL query (`POST /query/aql`, body `{"q": …}`), returning
/// the raw response for the case to assert.
async fn adhoc(ctx: &RunContext<'_>, aql: &str) -> Result<HttpResponse, CaseError> {
    let body = serde_json::json!({ "q": aql });
    ctx.send(
        HttpRequest::post("/query/aql")
            .json_body(&body)?
            .header("accept", "application/json"),
    )
    .await
}

/// A query fixture's text is unrunnable verbatim if it carries an upstream
/// `__MODIFY_…__` substitution token or a `$`-bind parameter (the harness would
/// substitute a runtime id / bind a value we do not have).
fn unrunnable(aql: &str) -> bool {
    aql.contains("__MODIFY_") || aql.contains('$')
}

/// The query text for a golden of `name` in `group` (paired by identical base
/// name under `aql_queries_valid/<group>/<name>`), or `None` when the paired
/// query fixture is absent.
fn paired_query(group: &str, name: &str) -> Option<String> {
    let rel = format!("query/aql_queries_valid/{group}/{name}");
    fixtures::read_json(&rel)
        .ok()
        .and_then(|v| v["q"].as_str().map(str::to_owned))
}

/// Run every golden in `expected_results/<db>/<group>` against the SUT, diffing
/// the served `RESULT_SET` through the [`golden`] normalizer. `_empty_db`
/// queries (fixed non-existent id) get the full columns+rows diff; every other
/// query gets the columns-only diff (shared-SUT-safe). Substitution/param
/// queries are skipped.
async fn run_golden_group(
    ctx: &RunContext<'_>,
    group: &str,
    db: &str,
) -> Result<DataSetReport, CaseError> {
    let goldens = fixtures::aql_expected(db, group).map_err(fx)?;
    let mut passed = 0u32;
    let mut total = 0u32;
    let mut skipped = 0u32;
    let mut first_fail: Option<String> = None;

    for gold in goldens {
        let Some(aql) = paired_query(group, &gold.name) else {
            continue; // a golden with no paired query fixture
        };
        if unrunnable(&aql) {
            skipped += 1;
            continue;
        }
        total += 1;
        let golden_value = gold.json().map_err(fx)?;
        let resp = adhoc(ctx, &aql).await?;
        if resp.status != 200 {
            // The corpus declares this a valid query; a rejection is a finding.
            first_fail.get_or_insert(format!(
                "{}/{}: valid query rejected with status {} (body: {})",
                group,
                gold.name,
                resp.status,
                truncate(&resp.text(), 160)
            ));
            continue;
        }
        let actual = resp.json()?;
        let mode = if gold.name.ends_with("_empty_db.json") {
            golden::Mode::Full
        } else {
            golden::Mode::ColumnsOnly
        };
        let cmp = golden::compare(&golden_value, &actual, mode);
        if cmp.matched {
            passed += 1;
        } else {
            first_fail.get_or_insert(format!(
                "{}/{} ({:?}, suppressed via [{}]): {}",
                group,
                gold.name,
                mode,
                cmp.applied_labels(),
                cmp.detail.unwrap_or_default()
            ));
        }
    }

    if total == 0 {
        return Err(CaseError::Skipped(format!(
            "all {skipped} {group}/{db} goldens require id-substitution/binds (unrunnable verbatim)"
        )));
    }
    if passed == total {
        Ok(DataSetReport { passed, total })
    } else {
        Err(CaseError::Assertion(format!(
            "{passed}/{total} {group}/{db} goldens matched ({skipped} skipped); first divergence: {}",
            first_fail.unwrap_or_default()
        )))
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_owned();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

// ── master11 real cases ───────────────────────────────────────────────────────

/// `I_QUERY_SERVICE.smoke_test`: a minimal ad-hoc query executes and returns a
/// well-formed `RESULT_SET` (`200`; `meta._type = RESULTSET`; a `columns`
/// array). The schedule Flow is `xx`; concretized against ITS-REST
/// `200_QUERY.yaml` (a `RESULT_SET` body).
fn run_smoke_test<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = adhoc(ctx, "SELECT e/ehr_id/value FROM EHR e").await?;
        assert::status(&resp, 200)?;
        let body = resp.json()?;
        if body["meta"]["_type"] != "RESULTSET" {
            return Err(CaseError::Assertion(format!(
                "smoke_test: expected meta._type RESULTSET, got {}",
                body["meta"]["_type"]
            )));
        }
        if !body["columns"].is_array() {
            return Err(CaseError::Assertion(
                "smoke_test: RESULT_SET has no columns array".to_owned(),
            ));
        }
        Ok(DataSetReport::SINGLE)
    })
}

/// `I_QUERY_SERVICE.execute_ad_hoc_query-empty_db`: an ad-hoc query filtering by
/// a fixed non-existent id returns the empty golden `RESULT_SET` (full diff).
fn run_adhoc_empty_db<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let name = "200_get_ehr_by_id_empty_db.json";
        let aql = paired_query("A", name).ok_or_else(|| {
            CaseError::Assertion("missing A/200 empty_db query fixture".to_owned())
        })?;
        let golden_value = fixtures::aql_golden("empty_db", "A", name)
            .map_err(fx)?
            .ok_or_else(|| CaseError::Assertion("missing A/200 empty_db golden".to_owned()))?;
        let resp = adhoc(ctx, &aql).await?;
        assert::status(&resp, 200)?;
        let actual = resp.json()?;
        let cmp = golden::compare(&golden_value, &actual, golden::Mode::Full);
        if cmp.matched {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(format!(
                "adhoc empty_db golden mismatch (suppressed via [{}]): {}",
                cmp.applied_labels(),
                cmp.detail.unwrap_or_default()
            )))
        }
    })
}

/// `I_QUERY_SERVICE.execute_stored_query-empty_db`: store a query, then execute
/// it by name (`GET /query/{name}`) — the empty golden `RESULT_SET`.
fn run_stored_empty_db<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let name = "200_get_ehr_by_id_empty_db.json";
        let aql = paired_query("A", name).ok_or_else(|| {
            CaseError::Assertion("missing A/200 empty_db query fixture".to_owned())
        })?;
        let golden_value = fixtures::aql_golden("empty_db", "A", name)
            .map_err(fx)?
            .ok_or_else(|| CaseError::Assertion("missing A/200 empty_db golden".to_owned()))?;

        // Store under a fresh qualified name (PUT /definition/query/{name}/{ver}).
        let qname = format!("org.conformance::stored_{}", Uuid::new_v4().simple());
        let store = ctx
            .send(
                HttpRequest::put(format!("/definition/query/{qname}/1.0.0"))
                    .text_body(aql, "text/plain")
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status_in(&store, &[200, 201])?;

        // Execute the stored query by name.
        let resp = ctx
            .send(HttpRequest::get(format!("/query/{qname}")).header("accept", "application/json"))
            .await?;
        assert::status(&resp, 200)?;
        let actual = resp.json()?;
        let cmp = golden::compare(&golden_value, &actual, golden::Mode::Full);
        if cmp.matched {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(format!(
                "stored empty_db golden mismatch (suppressed via [{}]): {}",
                cmp.applied_labels(),
                cmp.detail.unwrap_or_default()
            )))
        }
    })
}

/// `I_QUERY_SERVICE.execute_ad_hoc_query-loaded_db`: commit a real composition
/// to a fresh EHR, then run an EHR-scoped `CONTAINS COMPOSITION` query and
/// assert the loaded content is returned (a well-formed `RESULT_SET` with at
/// least the committed composition). Uses the proven master07 event fixture +
/// OPT so the data load is robust on the shared SUT (design §4.3: self-contained
/// through the API, fresh EHR per case).
fn run_adhoc_loaded_db<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = crate::suites::support::create_ehr(ctx).await?;
        crate::suites::support::ensure_opt(ctx, "nested/nested.opt").await?;
        let body = fixtures::read_json("compositions/CANONICAL_JSON/nested.en.v1__full.json")
            .map_err(fx)?;
        let commit = ctx
            .send(
                HttpRequest::post(format!("/ehr/{ehr_id}/composition"))
                    .json_body(&body)?
                    .header("accept", "application/json")
                    .header("prefer", "return=representation"),
            )
            .await?;
        assert::status(&commit, 201)?;

        let aql = format!(
            "SELECT c/uid/value FROM EHR e[ehr_id/value='{ehr_id}'] CONTAINS COMPOSITION c"
        );
        let resp = adhoc(ctx, &aql).await?;
        assert::status(&resp, 200)?;
        let actual = resp.json()?;
        let rows = actual["rows"].as_array().map_or(0, Vec::len);
        if rows == 0 {
            return Err(CaseError::Assertion(
                "loaded_db: EHR-scoped CONTAINS COMPOSITION returned no rows after a commit"
                    .to_owned(),
            ));
        }
        // The single projected column is the composition uid path.
        if actual["columns"][0]["path"] != "/uid/value" {
            return Err(CaseError::Assertion(format!(
                "loaded_db: expected column path /uid/value, got {}",
                actual["columns"][0]["path"]
            )));
        }
        Ok(DataSetReport::SINGLE)
    })
}

// ── fixture-derived: invalid queries ──────────────────────────────────────────

/// Every vendored invalid query must be rejected (`400`/`422`, ITS-REST
/// `400_QUERY.yaml`). A `2xx` for a malformed query is the finding.
fn run_invalid_queries<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let mut invalid: Vec<Fixture> = Vec::new();
        for group in GROUPS {
            if let Ok(mut g) = fixtures::aql_invalid(group) {
                invalid.append(&mut g);
            }
        }
        if invalid.is_empty() {
            return Err(CaseError::Skipped(
                "no invalid query fixtures vendored".to_owned(),
            ));
        }
        let mut passed = 0u32;
        let mut total = 0u32;
        let mut first_fail = None;
        for fixture in invalid {
            let aql = fixtures::aql_text(&fixture).map_err(fx)?;
            total += 1;
            let resp = adhoc(ctx, &aql).await?;
            if (400..500).contains(&resp.status) {
                passed += 1;
            } else {
                first_fail.get_or_insert(format!(
                    "{}: invalid query accepted with status {}",
                    fixture.name, resp.status
                ));
            }
        }
        if passed == total {
            Ok(DataSetReport { passed, total })
        } else {
            Err(CaseError::Assertion(format!(
                "{passed}/{total} invalid queries rejected; first: {}",
                first_fail.unwrap_or_default()
            )))
        }
    })
}

// ── fixture-derived: per-group golden diffs (bare-fn wrappers) ────────────────

fn run_a_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ run_golden_group(ctx, "A", "empty_db").await })
}
fn run_b_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ run_golden_group(ctx, "B", "empty_db").await })
}
fn run_c_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ run_golden_group(ctx, "C", "empty_db").await })
}
fn run_d_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ run_golden_group(ctx, "D", "empty_db").await })
}
fn run_a_loaded<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ run_golden_group(ctx, "A", "loaded_db").await })
}
fn run_b_loaded<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ run_golden_group(ctx, "B", "loaded_db").await })
}
fn run_c_loaded<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ run_golden_group(ctx, "C", "loaded_db").await })
}
fn run_d_loaded<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ run_golden_group(ctx, "D", "loaded_db").await })
}
