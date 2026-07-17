//! `I_QUERY_SERVICE` (`i_query_service.adoc`) — ad-hoc + stored AQL execution,
//! and the orchestration behind it: parse → plan → execute → assemble the
//! ITS-REST 1.0.3 `RESULT_SET`, the paging composition, the `ehr_ids`
//! resolution, and the per-query execution budget.
//!
//! Paging (ITS-REST QUERY `Request.md`): `fetch` is the row limit and "cannot
//! be combined with AQL-`TOP`"; `offset` is the 0-based start row. A REST
//! `fetch`/`offset` that collides with an AQL `LIMIT`/`OFFSET`/`TOP` is a
//! `400`; otherwise the AQL clause wins when present, else the REST parameter.

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::ids::EhrId;
use serde_json::Value;
use uuid::Uuid;

use openehr_query::parser::parse_str;

use crate::aql;
use crate::aql::error::{AqlError, ExecError};
use crate::aql::ir::{Params, QueryIr};
use crate::aql::sql::SqlCtx;
use crate::service::EhrbaseService;
use crate::service::query::request::{AqlQueryRequest, QueryOutcome};
use crate::service::status::{QUERY_TIMEOUT_TAG, SmError};
use crate::telemetry::prometheus::{AQL_QUERIES, AQL_QUERY_DURATION};

use super::result_set::{build_params, result_set_json, substitute_params};

impl EhrbaseService {
    /// `execute_ad_hoc_query` — execute an ad hoc query, supplying the query
    /// text (`i_query_service.adoc`).
    ///
    /// # Errors
    ///
    /// - a query that does not parse, an unsupported AQL construct, a path/
    ///   typing error, an unbound `$parameter`, a REST-vs-AQL paging
    ///   collision, or a malformed `ehr_id` → precondition (`400`);
    /// - a listed EHR that does not exist → `ehr_id_does_not_exist`
    ///   (`i_query_service.adoc` Errors);
    /// - a database/assembly failure, or an execution-budget overrun (tagged
    ///   [`QUERY_TIMEOUT_TAG`], rendered `408` at the wire) → exception.
    pub async fn execute_ad_hoc_query(
        &self,
        aql: String,
        request: AqlQueryRequest,
    ) -> Result<QueryOutcome, SmError> {
        self.execute_aql(&aql, None, &request).await
    }

    /// `execute_stored_query` — execute a query stored in the definition
    /// service by its qualified name (`version` a semver.org string, latest
    /// when absent) (`i_query_service.adoc`).
    ///
    /// # Errors
    ///
    /// - no stored query matches the name/version, or the stored record
    ///   carries no query text → the definition-store error / exception;
    /// - otherwise exactly the [`EhrbaseService::execute_ad_hoc_query`]
    ///   failure conditions (the stored text executes identically).
    pub async fn execute_stored_query(
        &self,
        qualified_query_name: String,
        version: Option<String>,
        request: AqlQueryRequest,
    ) -> Result<QueryOutcome, SmError> {
        // Resolve the stored AQL text (exact/partial SEMVER, or the latest)
        // via the DEFINITION store, then execute it exactly like an ad-hoc
        // query.
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

    /// Parse, plan, execute, and assemble an AQL query into an ITS-REST
    /// `RESULT_SET`. `name` is the stored-query name for the result metadata
    /// (`None` for an ad-hoc query).
    ///
    /// Emits the AQL metrics: `aql_query_duration_seconds{phase}` for `plan`
    /// and `execute` (each recorded only when its phase completes), and
    /// `aql_queries_total{outcome}` exactly once per call — the failure arms
    /// all route through [`Failure`], so the count-once property holds by
    /// construction.
    ///
    /// # Errors
    ///
    /// See [`EhrbaseService::execute_ad_hoc_query`] — this is the shared
    /// orchestration both public calls delegate to.
    pub(crate) async fn execute_aql(
        &self,
        aql: &str,
        name: Option<&str>,
        request: &AqlQueryRequest,
    ) -> Result<QueryOutcome, SmError> {
        match self.execute_aql_inner(aql, name, request).await {
            Ok(outcome) => {
                count_query("ok");
                Ok(outcome)
            }
            Err(failure) => {
                count_query(failure.outcome);
                Err(failure.error)
            }
        }
    }

    /// The fallible body of [`EhrbaseService::execute_aql`]: every failure
    /// carries its `aql_queries_total{outcome}` label so the caller counts
    /// uniformly.
    async fn execute_aql_inner(
        &self,
        aql: &str,
        name: Option<&str>,
        request: &AqlQueryRequest,
    ) -> Result<QueryOutcome, Failure> {
        let plan_start = Instant::now();
        let params = build_params(request);
        // Parse + terminology-expand + lower to the typed IR, reusing a
        // cached plan when the query text has been seen. The IR is a pure
        // function of the query text; the parameter *values*, paging, and
        // scope all bind downstream, so a cached plan serves every caller of
        // the same text correctly.
        let ir = self.plan_query(aql, &params).await?;
        record_phase("plan", plan_start);

        let (limit, offset) = compose_paging(ir.limit, ir.offset, request)?;
        // Multi-EHR scoping (`ehr_ids: List<UUID>`): a malformed id is a
        // client precondition (`400`); a well-formed but absent id raises
        // `ehr_id_does_not_exist` (`i_query_service.adoc`).
        let ehr_ids = self.resolve_ehr_ids(&request.ehr_ids).await?;
        let ctx = SqlCtx {
            system_id: self.effective_system_id(),
            ehr_ids,
            subject_scope: request.subject_scope.clone(),
            limit,
            offset,
        };

        let exec_start = Instant::now();
        // Query-level execution budget (`[query].timeout_ms`, our own
        // operational extension — no openEHR spec governs a query timeout):
        // when set, the DB execution is bounded so an over-long query is
        // reported as `408 Request Timeout` rather than hanging until the
        // global request timeout. Default off → zero drift.
        let exec = crate::aql::exec::execute(&self.pool, &ir, &params, &ctx);
        let result = match self.query_timeout {
            Some(budget) => tokio::time::timeout(budget, exec)
                .await
                .map_err(|_elapsed| Failure::timeout(budget))?,
            None => exec.await,
        }
        .map_err(Failure::exec)?;
        record_phase("execute", exec_start);

        // `_executed_aql` carries the parameter-SUBSTITUTED text; `q` keeps
        // the original query as submitted.
        let executed = substitute_params(aql, &params);
        let mut outcome = QueryOutcome::plain(result_set_json(aql, &executed, name, &result));
        // ABAC query post-check attributes (no openEHR spec governs this —
        // our own access-control extension): collect the touched EHR/template
        // sets independently of the projection, when the PEP asked for them.
        if request.collect_attributes {
            let scope = aql::exec::collect_scope(&self.pool, &ir, &params, &ctx)
                .await
                .map_err(Failure::exec)?;
            outcome.ehr_ids = scope.ehr_ids;
            outcome.template_ids = scope.template_ids;
        }
        Ok(outcome)
    }

    /// Parse, terminology-expand, and lower `aql` into a typed [`QueryIr`],
    /// reusing a cached plan on a repeat of the same query text. `params` is
    /// validated present against the (cached or fresh) plan on every call —
    /// the values themselves bind later at SQL-build time, so the cached plan
    /// is independent of them.
    ///
    /// A plan whose `matches` operands were resolved through the terminology
    /// service (`crate::aql::terminology::expand_matches` reported an expansion) is **not**
    /// cached: the resolution may differ on a later execution (QUERY master03
    /// §TERMINOLOGY), so such a query always re-parses and re-expands.
    ///
    /// # Errors
    ///
    /// [`Failure`] on: unparseable AQL (precondition), a terminology
    /// expansion / lowering / parameter-binding [`AqlError`] (mapped by
    /// [`map_plan_error`], labelled by [`outcome_of`]).
    async fn plan_query(&self, aql: &str, params: &Params) -> Result<Arc<QueryIr>, Failure> {
        // Cache hit: the plan is request-independent, but the caller's
        // bindings still must satisfy it.
        if let Some(ir) = self.plan_cache.get(aql).await {
            aql::check_params(&ir, params).map_err(Failure::plan)?;
            return Ok(ir);
        }
        // Miss: full parse → terminology expansion → lowering.
        let mut ast = parse_str(aql)
            .map_err(|e| Failure::analysis(SmError::precondition(format!("invalid AQL: {e}"))))?;
        // Semantic-analysis pre-pass: resolve every `TERMINOLOGY('expand', …)`
        // used in a `matches` operand through the terminology-service seam
        // and merge the codes into the value list, before planning
        // (QUERY master03 lines 756–759).
        let expanded = crate::aql::terminology::expand_matches(&mut ast, self)
            .await
            .map_err(Failure::plan)?;
        let ir = Arc::new(aql::lower_query(&ast).map_err(Failure::plan)?);
        // Cache only terminology-free plans (a resolved expansion may change).
        if !expanded {
            self.plan_cache
                .insert(aql.to_owned(), Arc::clone(&ir))
                .await;
        }
        aql::check_params(&ir, params).map_err(Failure::plan)?;
        Ok(ir)
    }

    /// Resolve the request's `ehr_ids` (string form) into the scoped `Uuid`
    /// set (`i_query_service.adoc` `ehr_ids: List<UUID> [0..1]`): a malformed
    /// id is a client precondition (`400`); a well-formed but non-existent id
    /// raises `ehr_id_does_not_exist`. An empty list scopes nothing (the
    /// population gate over `is_queryable` EHRs applies).
    ///
    /// # Errors
    ///
    /// [`Failure`] (all labelled `analysis_error`) on: an unparseable UUID
    /// (precondition), a database fault during the existence check
    /// (exception), or a listed EHR that does not exist
    /// (`ehr_id_does_not_exist`).
    async fn resolve_ehr_ids(&self, ehr_ids: &[String]) -> Result<Vec<EhrId>, Failure> {
        if ehr_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut ids = Vec::with_capacity(ehr_ids.len());
        for id in ehr_ids {
            let uuid = Uuid::parse_str(id).map_err(|_| {
                Failure::analysis(SmError::precondition(format!("invalid ehr_id `{id}`")))
            })?;
            ids.push(EhrId(uuid));
        }
        // Verify existence in one round-trip; report the first absent id.
        let present: Vec<EhrId> = sqlx::query_scalar("SELECT id FROM ehr WHERE id = ANY($1)")
            .bind(&ids)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Failure::analysis(SmError::exception(e.to_string())))?;
        if let Some(missing) = ids.iter().find(|id| !present.contains(id)) {
            return Err(Failure::analysis(SmError::ehr_not_found(format!(
                "no EHR with id {missing}"
            ))));
        }
        Ok(ids)
    }
}

/// A failed execution step carrying both the ITS-REST-mapped error and the
/// `aql_queries_total{outcome}` label to record for it — so the orchestration
/// can fail from any point (parse, terminology expansion, lowering, parameter
/// binding, paging, scoping, execution, scope collection) and the caller
/// counts + returns uniformly, exactly once.
struct Failure {
    outcome: &'static str,
    error: SmError,
}

impl Failure {
    /// A failure with the fixed `analysis_error` label (client preconditions
    /// and pre-execution service faults — the historical bookkeeping).
    fn analysis(error: SmError) -> Self {
        Self {
            outcome: "analysis_error",
            error,
        }
    }

    /// A planning-stage [`AqlError`] (terminology expansion, lowering,
    /// parameter binding) — labelled by [`outcome_of`], mapped by
    /// [`map_plan_error`].
    fn plan(e: AqlError) -> Self {
        Self {
            outcome: outcome_of(&e),
            error: map_plan_error(e),
        }
    }

    /// An execution-stage [`AqlError`] — labelled by [`outcome_of`], mapped
    /// by [`map_exec_error`].
    fn exec(e: AqlError) -> Self {
        Self {
            outcome: outcome_of(&e),
            error: map_exec_error(e),
        }
    }

    /// The execution-budget overrun: an `exception` [`SmError`] whose message
    /// is prefixed with [`QUERY_TIMEOUT_TAG`] so the REST adapter renders it
    /// as `408 Request Timeout` (`Requests_and_responses.md` §HTTP status
    /// codes). The tag is stripped at the wire, leaving the clean detail.
    fn timeout(budget: Duration) -> Self {
        Self {
            outcome: "exec_error",
            error: SmError::exception(format!(
                "{QUERY_TIMEOUT_TAG}query execution exceeded the maximum time of {}ms; \
                 the server aborted the query",
                budget.as_millis()
            )),
        }
    }
}

/// Compose the effective `(limit, offset)` from the AQL clause and the REST
/// paging parameters, rejecting collisions (ITS-REST query `Request.md`;
/// QUERY §Query structure/LIMIT).
fn compose_paging(
    aql_limit: Option<i64>,
    aql_offset: Option<i64>,
    request: &AqlQueryRequest,
) -> Result<(Option<i64>, Option<i64>), Failure> {
    if request.fetch.is_some() && aql_limit.is_some() {
        return Err(Failure::analysis(SmError::precondition(
            "the `fetch` query parameter cannot be combined with an AQL LIMIT/TOP clause \
             (ITS-REST query Request; QUERY §Query structure/LIMIT)"
                .to_owned(),
        )));
    }
    if request.offset.is_some() && aql_offset.is_some() {
        return Err(Failure::analysis(SmError::precondition(
            "the `offset` query parameter cannot be combined with an AQL OFFSET clause \
             (ITS-REST query Request; QUERY §Query structure)"
                .to_owned(),
        )));
    }
    Ok((aql_limit.or(request.fetch), aql_offset.or(request.offset)))
}

/// The `aql_queries_total{outcome}` label for an [`AqlError`] (spec-silent
/// observability — our own design). Closed label set: `feature_rejected`,
/// `analysis_error`, `exec_error` (+ `ok`, recorded by the caller).
fn outcome_of(e: &AqlError) -> &'static str {
    match e {
        AqlError::Feature(_) => "feature_rejected",
        AqlError::Analysis(_) | AqlError::Sql(_) => "analysis_error",
        AqlError::Exec(_) => "exec_error",
    }
}

/// Increment `aql_queries_total{outcome}` — called exactly once per
/// [`EhrbaseService::execute_aql`] call.
fn count_query(outcome: &'static str) {
    metrics::counter!(AQL_QUERIES, "outcome" => outcome).increment(1);
}

/// Record `aql_query_duration_seconds{phase}` for a completed phase.
fn record_phase(phase: &'static str, start: Instant) {
    metrics::histogram!(AQL_QUERY_DURATION, "phase" => phase).record(start.elapsed().as_secs_f64());
}

/// Map a planning error to an ITS-REST status: an unsupported feature, a
/// path/typing error, or an unrenderable construct all mean the query as
/// written cannot be served → `400 Bad Request` (ITS-REST `400_QUERY.yaml`).
fn map_plan_error(e: AqlError) -> SmError {
    match e {
        AqlError::Feature(_) | AqlError::Analysis(_) | AqlError::Sql(_) => {
            SmError::precondition(e.to_string())
        }
        AqlError::Exec(inner) => map_exec_error(inner.into()),
    }
}

/// Map an execution error: a database/assembly/terminology failure is the
/// server's fault (`500`); a SQL-lowering failure that surfaces here is still
/// a bad query (`400`).
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
