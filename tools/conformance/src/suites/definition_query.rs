//! DEFINITION: stored query provisioning (area SQR).
//!
//! Our own ECC cases (reference: `master05-func_tc_definition_query.adoc`,
//! design-time reading), driving the ITS-REST `/definition/query/{name}/{version}`
//! surface with AQL text from the vendored `query/aql_queries_valid` corpus.
//! Assertions concretize the stored query contract (`200` store with `Location`;
//! `200` list).
//!
//! The negative `valid_query-invalid`/`-bad_formalism` cases assert store-time AQL
//! validation (`400`/`422`).
//!
//! **`list_queries` — D2 split.** ITS-REST's list resource is
//! `GET /definition/query/{qualified_query_name}` (verbs `[get, put]`); a **bare**
//! `GET /definition/query` does not exist in Release-1.0.3 or the tested
//! development@e8a093e OAS. So `sqr/list-queries-non-empty` (a real
//! `GET /definition/query/{name}`) stays a live wire case, while the two bare-list
//! cases (`sqr/list-queries-empty`, `sqr/list-queries-select-items`) report
//! `SKIPPED` — the SM `I_DEFINITION_QUERY.list_queries()` (CNF master05:93) has no
//! ITS-REST binding, so a 404 there is a schedule-vs-ITS-REST gap, not a server
//! defect (`docs/blueprint/07-cnf.md` D2).

use uuid::Uuid;

use crate::assert;
use crate::case::{Capability, CaseMeta, Compare, Format, Profile};
use crate::catalog::Area;
use crate::fixtures;
use crate::harness::{CaseError, CaseFuture, CaseRun, DataSetReport, HttpRequest, RunContext};
use crate::registry::CaseEntry;

/// The implemented master05 case entries.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        entry(
            "sqr/valid-query-valid",
            "Store stored query — valid",
            "ITS-REST 1.0.3 DEFINITION QUERY API §store/list stored query; AQL 1.1",
            run_store_valid,
        ),
        entry(
            "sqr/list-queries-non-empty",
            "List stored queries — non empty",
            "ITS-REST 1.0.3 DEFINITION QUERY API §store/list stored query; AQL 1.1",
            run_list_non_empty,
        ),
        entry(
            "sqr/has-query-xxx",
            "Stored query existence check — xxx",
            "ITS-REST 1.0.3 DEFINITION QUERY API §store/list stored query; AQL 1.1",
            run_has_query,
        ),
        // list_queries (bare) — D2: no ITS-REST binding → skip-with-reason.
        entry(
            "sqr/list-queries-empty",
            "List stored queries — empty",
            "SM I_DEFINITION_QUERY.list_queries (CNF master05:93) — no ITS-REST binding \
             (list resource is GET /definition/query/{qualified_query_name}); skipped, see module docs",
            run_list_all,
        )
        .with_schedule_ref("I_DEFINITION_QUERY.list_queries (CNF master05:93)"),
        entry(
            "sqr/list-queries-select-items",
            "List stored queries — select items",
            "SM I_DEFINITION_QUERY.list_queries (CNF master05:93) — no ITS-REST binding \
             (list resource is GET /definition/query/{qualified_query_name}); skipped, see module docs",
            run_list_after_store,
        )
        .with_schedule_ref("I_DEFINITION_QUERY.list_queries (CNF master05:93)"),
        // valid_query negatives — store-time AQL validation → 400/422.
        entry(
            "sqr/valid-query-bad-formalism",
            "Store stored query — bad formalism",
            "ITS-REST 1.0.3 DEFINITION QUERY API §store/list stored query; AQL 1.1",
            run_store_bad_formalism,
        ),
        entry(
            "sqr/valid-query-invalid",
            "Store stored query — invalid",
            "ITS-REST 1.0.3 DEFINITION QUERY API §store/list stored query; AQL 1.1",
            run_store_invalid,
        ),
    ]
}

fn entry(id: &'static str, title: &'static str, citation: &'static str, run: CaseRun) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Sqr,
            capability: Capability::QueryProvisioning,
            profiles: &[Profile::Standard],
            formats: &[Format::Json],
            citation,
            compare: Compare::Superset,
            schedule_ref: None,
        },
        run,
    }
}

/// A valid AQL query string from the vendored corpus.
fn valid_aql() -> Result<String, CaseError> {
    let fixture = fixtures::read_json("query/aql_queries_valid/A/101_get_ehrs.json")
        .map_err(|e| CaseError::Codec(e.to_string()))?;
    fixture["q"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| CaseError::Assertion("AQL fixture has no `q`".to_owned()))
}

/// Store a query under a fresh qualified name and return `(name, version)`.
async fn store_query(ctx: &RunContext<'_>) -> Result<(String, String), CaseError> {
    let name = format!("org.conformance::q{}", Uuid::new_v4().simple());
    let version = "1.0.0".to_owned();
    let aql = valid_aql()?;
    let resp = ctx
        .send(
            HttpRequest::put(format!("/definition/query/{name}/{version}"))
                .text_body(aql, "text/plain")
                .header("accept", "application/json"),
        )
        .await?;
    assert::status_in(&resp, &[200, 201])?;
    Ok((name, version))
}

macro_rules! case {
    ($body:block) => {
        Box::pin(async move { $body })
    };
}

fn run_store_valid<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        store_query(ctx).await?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_list_non_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (name, _) = store_query(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/definition/query/{name}"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_has_query<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (name, version) = store_query(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/definition/query/{name}/{version}"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// A store attempt with a bad query body; returns the status.
async fn store_bad(ctx: &RunContext<'_>, aql: &str) -> Result<u16, CaseError> {
    let name = format!("org.conformance::bad{}", Uuid::new_v4().simple());
    let resp = ctx
        .send(
            HttpRequest::put(format!("/definition/query/{name}/1.0.0"))
                .text_body(aql.to_owned(), "text/plain")
                .header("accept", "application/json"),
        )
        .await?;
    Ok(resp.status)
}

// list_queries (bare) — D2 skip-with-reason (module docs): a bare
// `GET /definition/query` has no ITS-REST binding (the list resource is
// `GET /definition/query/{qualified_query_name}`), so the SM
// `I_DEFINITION_QUERY.list_queries()` (CNF master05:93) is not wire-exercisable;
// a 404 there is a schedule gap, not a server defect.
const LIST_QUERIES_SKIP: &str = "SM I_DEFINITION_QUERY.list_queries() (CNF master05:93) has no ITS-REST binding — ITS-REST \
     development@e8a093e (and Release-1.0.3) expose GET /definition/query/{qualified_query_name}, \
     not a bare GET /definition/query collection";

fn run_list_all<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(
        async move { Err::<DataSetReport, _>(CaseError::Skipped(LIST_QUERIES_SKIP.to_owned())) },
    )
}

fn run_list_after_store<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(
        async move { Err::<DataSetReport, _>(CaseError::Skipped(LIST_QUERIES_SKIP.to_owned())) },
    )
}

/// valid_query-bad_formalism — a non-AQL body is rejected at store time (400/422).
fn run_store_bad_formalism<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let status = store_bad(ctx, "SELECT * FROM patients; -- SQL, not AQL").await?;
        if matches!(status, 400 | 422) {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(format!(
                "expected 400/422 for a non-AQL query, got {status}"
            )))
        }
    })
}

/// valid_query-invalid — malformed AQL is rejected at store time (400/422).
fn run_store_invalid<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let status = store_bad(ctx, "SELECT FROM WHERE {{{ not valid aql").await?;
        if matches!(status, 400 | 422) {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(format!(
                "expected 400/422 for malformed AQL, got {status}"
            )))
        }
    })
}
