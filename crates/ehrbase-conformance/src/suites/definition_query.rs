//! master05 — DEFINITION: stored query provisioning (design §4.1).
//!
//! Transcribed from `master05-func_tc_definition_query.adoc`, driving the
//! ITS-REST `/definition/query/{name}/{version}` surface with AQL text from the
//! vendored `query/aql_queries_valid` corpus. Assertions concretize the stored
//! query contract (`200` store with `Location`; `200` list).
//!
//! The negative `valid_query-invalid`/`-bad_formalism` cases assert store-time AQL
//! validation (`400`/`422`); `list_queries` is realized via `GET /definition/query`.

use uuid::Uuid;

use crate::assert;
use crate::case::{Capability, CaseMeta, Chapter, Compare, Format, Profile, Provenance};
use crate::fixtures;
use crate::harness::{CaseError, CaseFuture, CaseRun, DataSetReport, HttpRequest, RunContext};
use crate::registry::CaseEntry;

/// The implemented master05 case entries.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        entry("I_DEFINITION_QUERY.valid_query-valid", run_store_valid),
        entry(
            "I_DEFINITION_QUERY.list_queries-non_empty",
            run_list_non_empty,
        ),
        entry("I_DEFINITION_QUERY.has_query-xxx", run_has_query),
        // list_queries — GET /definition/query (the stored-query list).
        entry("I_DEFINITION_QUERY.list_queries-empty", run_list_all),
        entry(
            "I_DEFINITION_QUERY.list_queries-select_items",
            run_list_after_store,
        ),
        // valid_query negatives — store-time AQL validation → 400/422.
        entry(
            "I_DEFINITION_QUERY.valid_query-bad_formalism",
            run_store_bad_formalism,
        ),
        entry("I_DEFINITION_QUERY.valid_query-invalid", run_store_invalid),
    ]
}

fn entry(id: &'static str, run: CaseRun) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            chapter: Chapter::Master05,
            capability: Capability::QueryProvisioning,
            profiles: &[Profile::Standard],
            formats: &[Format::Json],
            provenance: Provenance::Schedule,
            schedule_ref: id,
            upstream_tags: &[],
            compare: Compare::Superset,
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

/// `list_queries` — GET /definition/query returns the stored-query list (200).
fn run_list_all<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(HttpRequest::get("/definition/query").header("accept", "application/json"))
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// `list_queries` after storing one → 200 (the list is non-empty / selectable).
fn run_list_after_store<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        store_query(ctx).await?;
        let resp = ctx
            .send(HttpRequest::get("/definition/query").header("accept", "application/json"))
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
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
