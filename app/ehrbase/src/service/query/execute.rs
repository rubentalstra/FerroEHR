//! `I_QUERY_SERVICE` (`i_query_service.adoc`) — ad-hoc + stored AQL execution,
//! and the orchestration behind it: parse → plan → execute → assemble the
//! ITS-REST 1.0.3 `RESULT_SET`, the paging composition, the `ehr_ids`
//! resolution, and the per-query execution budget.
//!
//! Paging (ITS-REST QUERY `Request.md`): `fetch` is the row limit and "cannot be
//! combined with AQL-`TOP`"; `offset` is the 0-based start row. A REST
//! `fetch`/`offset` that collides with an AQL `LIMIT`/`OFFSET`/`TOP` is a `400`;
//! otherwise the AQL clause wins when present, else the REST parameter.

use std::sync::LazyLock;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use ehrbase_rest::overview::error::QUERY_TIMEOUT_TAG;
use ehrbase_sm::{AqlQueryRequest, QueryOutcome, QueryService, SmError};
use openehr_query::parser::parse_str;

use super::result_set::{build_params, result_set_json, substitute_params};
use crate::aql::{self, AqlError, ExecError, SqlCtx};
use crate::service::EhrbaseService;
use crate::telemetry::prometheus::{AQL_QUERIES, AQL_QUERY_DURATION};

#[async_trait]
impl QueryService for EhrbaseService {
    /// `execute_ad_hoc_query` — execute an ad hoc query, supplying the query
    /// text. Error `ehr_id_does_not_exist` (a listed EHR does not exist).
    async fn execute_ad_hoc_query(
        &self,
        aql: String,
        request: AqlQueryRequest,
    ) -> Result<QueryOutcome, SmError> {
        self.execute_aql(&aql, None, &request).await
    }

    /// `execute_stored_query` — execute a query stored in the definition service
    /// by its qualified name (`version` a semver.org string, latest when
    /// absent). Error `ehr_id_does_not_exist`.
    async fn execute_stored_query(
        &self,
        qualified_query_name: String,
        version: Option<String>,
        request: AqlQueryRequest,
    ) -> Result<QueryOutcome, SmError> {
        // Resolve the stored AQL text (exact/partial SEMVER, or the latest) via
        // the DEFINITION store, then execute it exactly like an ad-hoc query.
        let stored = self
            .get_stored_query(&qualified_query_name, version.as_deref())
            .await?;
        let aql = stored
            .get("q")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                SmError::exception(format!(
                    "stored query `{qualified_query_name}` has no query text"
                ))
            })?
            .to_owned();
        self.execute_aql(&aql, Some(&qualified_query_name), &request)
            .await
    }
}

impl EhrbaseService {
    /// Parse, plan, execute, and assemble an AQL query into an ITS-REST
    /// `RESULT_SET`. `name` is the stored-query name for the result metadata
    /// (`None` for an ad-hoc query).
    ///
    /// Emits the AQL metrics: `aql_query_duration_seconds{phase}` for `plan` and
    /// `execute`, and `aql_queries_total{outcome}` exactly once per call.
    pub(super) async fn execute_aql(
        &self,
        aql: &str,
        name: Option<&str>,
        request: &AqlQueryRequest,
    ) -> Result<QueryOutcome, SmError> {
        let plan_start = Instant::now();
        let mut ast = match parse_str(aql) {
            Ok(ast) => ast,
            Err(e) => {
                count_query("analysis_error");
                return Err(SmError::precondition(format!("invalid AQL: {e}")));
            }
        };
        // Semantic-analysis pre-pass: resolve every `TERMINOLOGY('expand', …)`
        // used in a `matches` operand through the terminology-service seam and
        // merge the codes into the value list, before planning
        // (QUERY master03 lines 756–759).
        // TODO(w3f-integrate): aql seam (register 08) — `aql::expand_matches`.
        if let Err(e) = aql::expand_matches(&mut ast, self).await {
            count_query(plan_outcome(&e));
            return Err(map_plan_error(e));
        }
        let params = build_params(request);
        let ir = match aql::plan(&ast, &params) {
            Ok(ir) => ir,
            Err(e) => {
                count_query(plan_outcome(&e));
                return Err(map_plan_error(e));
            }
        };
        record_phase("plan", plan_start);

        let (limit, offset) = match compose_paging(ir.limit, ir.offset, request) {
            Ok(paging) => paging,
            Err(e) => {
                count_query("analysis_error");
                return Err(e);
            }
        };
        // Multi-EHR scoping (`ehr_ids: List<UUID>`): a malformed id is a client
        // precondition (`400`); a well-formed but absent id raises
        // `ehr_id_does_not_exist` (`i_query_service.adoc`).
        let ehr_ids = match self.resolve_ehr_ids(&request.ehr_ids).await {
            Ok(ids) => ids,
            Err(e) => {
                count_query("analysis_error");
                return Err(e);
            }
        };
        let ctx = SqlCtx {
            system_id: self.effective_system_id(),
            ehr_ids,
            subject_scope: request.subject_scope.clone(),
            limit,
            offset,
        };

        let exec_start = Instant::now();
        // Query-level execution budget (`EHRBASE_QUERY__TIMEOUT_MS`, our own
        // operational extension — no openEHR spec governs a query timeout): when
        // set, the DB execution is bounded so an over-long query is reported as
        // `408 Request Timeout` rather than hanging until the global request
        // timeout. Default off → zero drift.
        // TODO(w3f-integrate): aql seam (register 08) — `aql::execute`.
        let exec = aql::execute(&self.pool, &ir, &params, &ctx);
        let exec_result = match *QUERY_TIMEOUT {
            Some(budget) => match tokio::time::timeout(budget, exec).await {
                Ok(inner) => inner,
                Err(_elapsed) => {
                    count_query("exec_error");
                    return Err(query_timeout_error(budget));
                }
            },
            None => exec.await,
        };
        let result = match exec_result {
            Ok(result) => result,
            Err(e) => {
                count_query(exec_outcome(&e));
                return Err(map_exec_error(e));
            }
        };
        record_phase("execute", exec_start);

        // `_executed_aql` carries the parameter-SUBSTITUTED text; `q` keeps the
        // original query as submitted.
        let executed = substitute_params(aql, &params);
        let mut outcome = QueryOutcome::plain(result_set_json(aql, &executed, name, &result));
        // ABAC query post-check attributes (no openEHR spec governs this — our
        // own access-control extension): collect the touched EHR/template sets
        // independently of the projection, when the PEP asked for them.
        if request.collect_attributes {
            match aql::exec::collect_scope(&self.pool, &ir, &params, &ctx).await {
                Ok(scope) => {
                    outcome.ehr_ids = scope.ehr_ids;
                    outcome.template_ids = scope.template_ids;
                }
                Err(e) => {
                    count_query(exec_outcome(&e));
                    return Err(map_exec_error(e));
                }
            }
        }
        count_query("ok");
        Ok(outcome)
    }

    /// Resolve the request's `ehr_ids` (string form) into the scoped `Uuid` set
    /// (`i_query_service.adoc` `ehr_ids: List<UUID> [0..1]`): a malformed id is a
    /// client precondition (`400`); a well-formed but non-existent id raises
    /// `ehr_id_does_not_exist`. An empty list scopes nothing (the population gate
    /// over `is_queryable` EHRs applies).
    async fn resolve_ehr_ids(&self, ehr_ids: &[String]) -> Result<Vec<Uuid>, SmError> {
        if ehr_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut ids = Vec::with_capacity(ehr_ids.len());
        for id in ehr_ids {
            let uuid = Uuid::parse_str(id)
                .map_err(|_| SmError::precondition(format!("invalid ehr_id `{id}`")))?;
            ids.push(uuid);
        }
        // Verify existence in one round-trip; report the first absent id.
        let present: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM ehr WHERE id = ANY($1)")
            .bind(&ids)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| SmError::exception(e.to_string()))?;
        if let Some(missing) = ids.iter().find(|id| !present.contains(id)) {
            return Err(SmError::ehr_not_found(format!("no EHR with id {missing}")));
        }
        Ok(ids)
    }
}

/// Per-query execution budget, read once from `EHRBASE_QUERY__TIMEOUT_MS`
/// (milliseconds). Unset, unparseable, or `0` disables the budget (the default),
/// so the only guard on an over-long query stays the global request timeout. A
/// positive value bounds every AQL query's DB execution; on overrun the query is
/// reported as `408 Request Timeout` (`Requests_and_responses.md` §HTTP status
/// codes). No openEHR spec governs a query timeout — our own extension.
static QUERY_TIMEOUT: LazyLock<Option<Duration>> = LazyLock::new(|| {
    std::env::var("EHRBASE_QUERY__TIMEOUT_MS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|ms| *ms > 0)
        .map(Duration::from_millis)
});

/// The `408` query-execution-timeout error: an `exception` [`SmError`] whose
/// message is prefixed with [`QUERY_TIMEOUT_TAG`] so the REST adapter renders it
/// as `408 Request Timeout` (`Requests_and_responses.md` §HTTP status codes).
/// The tag is stripped at the wire, leaving the clean detail below.
fn query_timeout_error(budget: Duration) -> SmError {
    SmError::exception(format!(
        "{QUERY_TIMEOUT_TAG}query execution exceeded the maximum time of {}ms; \
         the server aborted the query",
        budget.as_millis()
    ))
}

/// Increment `aql_queries_total{outcome}` (spec-silent observability — our own
/// design). Closed outcome set.
fn count_query(outcome: &'static str) {
    metrics::counter!(AQL_QUERIES, "outcome" => outcome).increment(1);
}

/// Record `aql_query_duration_seconds{phase}` for a completed phase.
fn record_phase(phase: &'static str, start: Instant) {
    metrics::histogram!(AQL_QUERY_DURATION, "phase" => phase).record(start.elapsed().as_secs_f64());
}

/// The outcome label for a planning error.
fn plan_outcome(e: &AqlError) -> &'static str {
    match e {
        AqlError::Feature(_) => "feature_rejected",
        AqlError::Analysis(_) | AqlError::Sql(_) => "analysis_error",
        AqlError::Exec(_) => "exec_error",
    }
}

/// The outcome label for an execution error.
fn exec_outcome(e: &AqlError) -> &'static str {
    match e {
        AqlError::Feature(_) => "feature_rejected",
        AqlError::Analysis(_) | AqlError::Sql(_) => "analysis_error",
        AqlError::Exec(_) => "exec_error",
    }
}

/// Compose the effective `(limit, offset)` from the AQL clause and the REST
/// paging parameters, rejecting collisions (ITS-REST query `Request.md`; QUERY
/// §Query structure/LIMIT).
fn compose_paging(
    aql_limit: Option<i64>,
    aql_offset: Option<i64>,
    request: &AqlQueryRequest,
) -> Result<(Option<i64>, Option<i64>), SmError> {
    if request.fetch.is_some() && aql_limit.is_some() {
        return Err(SmError::precondition(
            "the `fetch` query parameter cannot be combined with an AQL LIMIT/TOP clause \
             (ITS-REST query Request; QUERY §Query structure/LIMIT)"
                .to_owned(),
        ));
    }
    if request.offset.is_some() && aql_offset.is_some() {
        return Err(SmError::precondition(
            "the `offset` query parameter cannot be combined with an AQL OFFSET clause \
             (ITS-REST query Request; QUERY §Query structure)"
                .to_owned(),
        ));
    }
    Ok((aql_limit.or(request.fetch), aql_offset.or(request.offset)))
}

/// Map a planning error to an ITS-REST status: an unsupported feature, a path/
/// typing error, or an unrenderable construct all mean the query as written
/// cannot be served → `400 Bad Request` (ITS-REST `400_QUERY.yaml`).
fn map_plan_error(e: AqlError) -> SmError {
    match e {
        AqlError::Feature(_) | AqlError::Analysis(_) | AqlError::Sql(_) => {
            SmError::precondition(e.to_string())
        }
        AqlError::Exec(inner) => map_exec_error(inner.into()),
    }
}

/// Map an execution error: a database/assembly failure is the server's fault
/// (`500`); a SQL-lowering failure that surfaces here is still a bad query.
fn map_exec_error(e: AqlError) -> SmError {
    match e {
        AqlError::Exec(ExecError::Database(db)) => SmError::exception(db.to_string()),
        AqlError::Exec(ExecError::Assembly(a)) => SmError::exception(a.to_string()),
        AqlError::Exec(ExecError::Terminology(msg)) => SmError::exception(msg),
        AqlError::Feature(_) | AqlError::Analysis(_) | AqlError::Sql(_) => {
            SmError::precondition(e.to_string())
        }
    }
}
