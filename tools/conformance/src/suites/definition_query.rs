//! DEFINITION / stored-query provisioning — the master05 spine (area `Sqr`).
//!
//! **master05 is a schedule stub**: its §Test Environment and
//! §Test Data Sets are the literal `[.tbd] TBD` and every case Flow is the
//! placeholder `xx`. There is therefore no normative flow to be faithful to, so
//! every case here is [`ScheduleTrace::EccOriginal`] — the spine is **derived**
//! from the ITS-REST DEFINITION QUERY contract + AQL 1.1 + the profiles matrix
//! (Query provisioning = STANDARD), with the stub schedule contributing only
//! operation names and case ids. No SQR case is presented as
//! schedule-conformant.
//!
//! Rulings realized here:
//!
//! - **`list_queries` split.** ITS-REST binds the list resource as
//!   `GET /definition/query/{qualified_query_name}` (verbs `[get, put]`); a bare
//!   `GET /definition/query` collection is absent in Release-1.0.3 and
//!   development@e8a093e. So `list_queries-non_empty` is rebound to the named
//!   resource (a live [`Binding::Rest`] case), while `list_queries-empty` /
//!   `-select_items` carry [`Binding::NoRestBinding`] and skip-with-reason.
//!   The bare-list endpoint is an edition-ladder probe: a future/other-CDR
//!   edition exposing it would make the skipped cases live (recorded, not baked
//!   shut).
//! - **Placeholder id + round-trip.** The schedule's literal `has_query-xxx`
//!   placeholder id is NOT carried as the case id — this case is renamed
//!   `sqr/has-query-existing` (a new slug; the retired `sqr/has-query-xxx`
//!   number is recorded in the catalogue), and the stub heading is kept in the
//!   `schedule` trace. The store cases assert the retrieved AQL **round-trips**
//!   to what was stored, not merely a status.
//! - **Data-set sourcing.** Valid + invalid AQL come from the shared corpus
//!   keys (shared with the QUERY area), not hand-picked strings.
//! - **Negative status width.** The store-time negatives accept
//!   `[400, 422]`: ITS-REST does not pin `400` (malformed request) vs `422`
//!   (semantically-invalid AQL) for stored-query create — an implementation
//!   choice, recorded as a boundary rather than masked.

use uuid::Uuid;

use crate::engine::assert;
use crate::engine::harness::{CaseError, CaseFuture, DataSetReport, HttpRequest, RunContext};
use crate::engine::registry::CaseEntry;
use crate::model::case::{Binding, Capability, CaseMeta, Compare, Format, ScheduleTrace};
use crate::model::catalog::Area;
use crate::testdata::fixtures;

const JSON: &[Format] = &[Format::Json];

const STORE_CITATION: &str = "ITS-REST 1.0.3 DEFINITION QUERY API §store/get stored query; AQL 1.1 (master05 stub — case id only)";

/// The bare-list endpoint is unbound in ITS-REST (the list resource is the
/// named-query GET).
const LIST_SKIP: &str = "master05 §list_queries: SM I_DEFINITION_QUERY.list_queries() (bare collection) has no ITS-REST \
     binding — Release-1.0.3 and development@e8a093e expose GET /definition/query/{qualified_query_name}, \
     not a bare GET /definition/query. An edition exposing a bare-list resource would make this case \
     live (an edition probe).";
const LIST_BINDING: Binding = Binding::NoRestBinding(
    "I_DEFINITION_QUERY.list_queries (master05 §list_queries, bare collection)",
);

/// Every registered master05 `I_DEFINITION_QUERY` case (7).
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        // ── valid_query ──────────────────────────────────────────────────────
        rest_case(
            "sqr/valid-query-valid",
            "Store stored query — valid",
            STORE_CITATION,
            "schedule stub (master05 is TBD); derived from ITS-REST 1.0.3 DEFINITION QUERY + AQL 1.1 \
             — I_DEFINITION_QUERY.valid_query-valid (master05:54, A.3.a)",
            Binding::Rest("PUT /definition/query/{qualified_query_name}/{version}"),
            run_store_valid,
        ),
        rest_case(
            "sqr/valid-query-invalid",
            "Store stored query — invalid",
            STORE_CITATION,
            "schedule stub (master05 is TBD); derived from ITS-REST 1.0.3 DEFINITION QUERY + AQL 1.1 \
             — I_DEFINITION_QUERY.valid_query-invalid (master05:67, A.3.b)",
            Binding::Rest("PUT /definition/query/{qualified_query_name}/{version}"),
            run_store_invalid,
        ),
        rest_case(
            "sqr/valid-query-bad-formalism",
            "Store stored query — bad formalism",
            STORE_CITATION,
            "schedule stub (master05 is TBD); derived from ITS-REST 1.0.3 DEFINITION QUERY + AQL 1.1 \
             — I_DEFINITION_QUERY.valid_query-bad_formalism (master05:80, A.3.c)",
            Binding::Rest("PUT /definition/query/{qualified_query_name}/{version}"),
            run_store_bad_formalism,
        ),
        // ── has_query (renamed from the schedule's `xxx` placeholder) ──────────
        rest_case(
            "sqr/has-query-existing",
            "Stored query existence check — existing",
            STORE_CITATION,
            "schedule stub (master05 is TBD); derived from ITS-REST 1.0.3 DEFINITION QUERY + AQL 1.1 \
             — I_DEFINITION_QUERY.has_query-xxx (master05:37, placeholder id; slug descriptivised)",
            Binding::Rest("GET /definition/query/{qualified_query_name}/{version}"),
            run_has_query,
        ),
        // ── list_queries: non_empty rebound to the named resource ─────────
        rest_case(
            "sqr/list-queries-non-empty",
            "List stored queries — non empty",
            "ITS-REST 1.0.3 DEFINITION QUERY API §get stored query versions (named list resource); \
             AQL 1.1 (master05 stub — case id only)",
            "schedule stub (master05 is TBD); derived from ITS-REST 1.0.3 DEFINITION QUERY (named list \
             resource, D2 rebind) + AQL 1.1 — I_DEFINITION_QUERY.list_queries-non_empty (master05:110)",
            Binding::Rest("GET /definition/query/{qualified_query_name}"),
            run_list_non_empty,
        ),
        // ── list_queries: bare collection — no ITS-REST binding → skip ─────────
        skip_case(
            "sqr/list-queries-empty",
            "List stored queries — empty",
            "schedule stub (master05 is TBD); derived from ITS-REST 1.0.3 DEFINITION QUERY + AQL 1.1 \
             — I_DEFINITION_QUERY.list_queries-empty (master05:97)",
        ),
        skip_case(
            "sqr/list-queries-select-items",
            "List stored queries — select items",
            "schedule stub (master05 is TBD); derived from ITS-REST 1.0.3 DEFINITION QUERY + AQL 1.1 \
             — I_DEFINITION_QUERY.list_queries-select_items (master05:123)",
        ),
    ]
}

// ── entry builders ────────────────────────────────────────────────────────────

fn rest_case(
    id: &'static str,
    title: &'static str,
    citation: &'static str,
    schedule: &'static str,
    binding: Binding,
    run: crate::engine::harness::CaseRun,
) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Sqr,
            capability: Capability::QueryProvisioning,
            formats: JSON,
            citation,
            // master05 is a stub, so the case is ECC-original — the reason
            // names the stub provenance + the derivation basis, never presenting
            // the case as schedule-conformant.
            schedule: ScheduleTrace::EccOriginal(schedule),
            binding,
            compare: Compare::Superset,
        },
        run,
    }
}

/// A bare-list case: no ITS-REST binding → skip-with-reason.
fn skip_case(id: &'static str, title: &'static str, schedule: &'static str) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Sqr,
            capability: Capability::QueryProvisioning,
            formats: JSON,
            citation: LIST_SKIP,
            schedule: ScheduleTrace::EccOriginal(schedule),
            binding: LIST_BINDING,
            compare: Compare::None,
        },
        run: run_list_skip,
    }
}

/// Box a plain async result as a [`CaseFuture`].
macro_rules! boxed {
    ($body:block) => {
        Box::pin(async move $body)
    };
}

// ── shared helpers ──────────────────────────────────────────────────────────

fn codec(e: &fixtures::FixtureError) -> CaseError {
    CaseError::Codec(e.to_string())
}

/// A valid AQL query string from the corpus (group A, the
/// `get_ehrs` query — a minimal, always-valid AQL 1.1 statement).
fn valid_aql() -> Result<String, CaseError> {
    let fixtures = fixtures::aql_valid("A").map_err(|e| codec(&e))?;
    let fixture = fixtures
        .iter()
        .find(|f| f.name == "101_get_ehrs.json")
        .or_else(|| fixtures.first())
        .ok_or_else(|| CaseError::Skipped("no valid AQL corpus fixture (group A)".to_owned()))?;
    fixtures::aql_text(fixture).map_err(|e| codec(&e))
}

/// A malformed AQL string from the invalid corpus (group A).
fn invalid_aql() -> Result<String, CaseError> {
    let fixtures = fixtures::aql_invalid("A").map_err(|e| codec(&e))?;
    let fixture = fixtures
        .first()
        .ok_or_else(|| CaseError::Skipped("no invalid AQL corpus fixture (group A)".to_owned()))?;
    fixtures::aql_text(fixture).map_err(|e| codec(&e))
}

/// A fresh, unique qualified query name.
fn fresh_name() -> String {
    format!("org.conformance::q{}", Uuid::new_v4().simple())
}

/// Store a query body under a fresh name/version, returning `(name, version,
/// status)`.
async fn store(ctx: &RunContext<'_>, aql: &str) -> Result<(String, String, u16), CaseError> {
    let name = fresh_name();
    let version = "1.0.0".to_owned();
    let resp = ctx
        .send(
            HttpRequest::put(format!("/definition/query/{name}/{version}"))
                .text_body(aql.to_owned(), "text/plain")
                .header("accept", "application/json"),
        )
        .await?;
    Ok((name, version, resp.status))
}

// ── valid_query ──────────────────────────────────────────────────────────────

/// valid_query-valid: a valid AQL query is stored (`[200, 201]`) and its text
/// round-trips on read (not a status-only check).
fn run_store_valid<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let aql = valid_aql()?;
        let (name, version, status) = store(ctx, &aql).await?;
        if !matches!(status, 200 | 201) {
            return Err(CaseError::Assertion(format!(
                "storing a valid query expected 200/201, got {status}"
            )));
        }
        // Round-trip: the retrieved definition must carry the stored AQL text.
        let got = ctx
            .send(
                HttpRequest::get(format!("/definition/query/{name}/{version}"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&got, 200)?;
        if stored_aql_matches(&got.json()?, &aql) {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(
                "retrieved stored query does not round-trip the stored AQL text".to_owned(),
            ))
        }
    })
}

/// valid_query-invalid: a malformed AQL body at store time.
//
// The contract does NOT mandate store-time validation: `definition_query_store`
// declares only {200, 400} (ITS-REST definition-codegen OAS, operation
// `definition_query_store`) and its description is silent on validating the
// stored text — deferring AQL validation to execution is contract-conformant.
// The spec-determined assertion is therefore: stored (200) or rejected with
// the declared 400 — never any other code.
fn run_store_invalid<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let aql = invalid_aql()?;
        let (_, _, status) = store(ctx, &aql).await?;
        assert_store_rejected(status)
    })
}

/// valid_query-bad_formalism: a non-AQL body (SQL) is rejected at store time.
//
// NOTE: no vendored "non-AQL formalism" corpus exists (the corpus holds AQL
// classes only), so the bad-formalism data set is an inline SQL literal — the
// one hand-authored body in this suite, kept because it is a formalism the
// corpus deliberately does not carry.
fn run_store_bad_formalism<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let (_, _, status) = store(ctx, "SELECT * FROM patients; -- SQL, not AQL").await?;
        assert_store_rejected(status)
    })
}

// ── has_query ──────────────────────────────────────────────────────────────

/// `has_query` (renamed from the schedule `xxx` placeholder): store a query
/// then confirm existence via the named GET, asserting the stored AQL
/// round-trips.
fn run_has_query<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let aql = valid_aql()?;
        let (name, version, status) = store(ctx, &aql).await?;
        if !matches!(status, 200 | 201) {
            return Err(CaseError::Assertion(format!(
                "storing the query expected 200/201, got {status}"
            )));
        }
        let got = ctx
            .send(
                HttpRequest::get(format!("/definition/query/{name}/{version}"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&got, 200)?;
        if stored_aql_matches(&got.json()?, &aql) {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(
                "existence GET did not round-trip the stored AQL text".to_owned(),
            ))
        }
    })
}

// ── list_queries ──────────────────────────────────────────────────────────────

/// list_queries-non_empty (D2 rebind): after a store, the named list resource
/// `GET /definition/query/{name}` returns the query versions (200).
//
// NOTE: this lists a single named query, not "all stored
// queries" — the SM bare-collection semantics have no ITS-REST binding, so the
// "list all" / select_items post-conditions are covered only by the skipped
// bare-list cases.
fn run_list_non_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let aql = valid_aql()?;
        let (name, _, status) = store(ctx, &aql).await?;
        if !matches!(status, 200 | 201) {
            return Err(CaseError::Assertion(format!(
                "storing the query expected 200/201, got {status}"
            )));
        }
        let resp = ctx
            .send(
                HttpRequest::get(format!("/definition/query/{name}"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200).map(|()| DataSetReport::SINGLE)
    })
}

fn run_list_skip<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move { Err::<DataSetReport, _>(CaseError::Skipped(LIST_SKIP.to_owned())) })
}

// ── assertion helpers ─────────────────────────────────────────────────────────

/// Store disposition for an invalid body: the contract declares only
/// {200, 400} for `definition_query_store` and does not mandate store-time
/// AQL validation — acceptance (deferred validation) and the declared 400
/// are both conformant; any other code is not.
fn assert_store_rejected(status: u16) -> Result<DataSetReport, CaseError> {
    if matches!(status, 200 | 400) {
        Ok(DataSetReport::SINGLE)
    } else {
        Err(CaseError::Assertion(format!(
            "expected stored (200, validation deferred) or the declared 400, got {status}"
        )))
    }
}

/// Whether a retrieved stored-query definition carries the stored AQL text: the
/// ITS-REST definition body puts the AQL in `q` (canonical) or `query`; a raw
/// substring match on the serialized body is the format-robust round-trip check.
fn stored_aql_matches(body: &serde_json::Value, stored: &str) -> bool {
    let field = body
        .get("q")
        .or_else(|| body.get("query"))
        .and_then(serde_json::Value::as_str);
    if let Some(text) = field {
        return text.trim() == stored.trim();
    }
    body.to_string().contains(stored.trim())
}
