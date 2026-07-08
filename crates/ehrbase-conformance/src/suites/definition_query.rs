//! master05 — DEFINITION: stored query provisioning (design §4.1).
//!
//! Transcribed from `master05-func_tc_definition_query.adoc`, driving the
//! ITS-REST `/definition/query/{name}/{version}` surface with AQL text from the
//! vendored `query/aql_queries_valid` corpus. Assertions concretize the stored
//! query contract (`200` store with `Location`; `200` list).
//!
//! The negative `valid_query-invalid`/`-bad_formalism` cases depend on store-time
//! AQL validation semantics and stay `NotYetTranscribed` until specified.

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
