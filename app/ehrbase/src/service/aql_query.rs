//! AQL query execution wiring (P16): parse → plan → SQL → execute → assemble the
//! ITS-REST 1.0.3 `RESULT_SET`. This is the service-side glue between the REST
//! `QueryService` seam and the [`crate::aql`] engine.
//!
//! Paging composition follows the ITS-REST QUERY spec
//! (`docs/specs/openehr/ITS-REST/specifications/docs/query/Request.md`): `fetch`
//! is the row limit and "cannot be combined with AQL-`TOP`"; `offset` is the
//! 0-based start row. We reject a REST `fetch`/`offset` that collides with an AQL
//! `LIMIT`/`OFFSET`/`TOP` (`400`), and otherwise take the AQL clause when present
//! else the REST parameter.

use std::sync::LazyLock;
use std::time::Instant;

use jiff::Timestamp;
use regex::Regex;
use serde_json::{Value, json};
use uuid::Uuid;

use ehrbase_rest::{AqlQueryRequest, QueryOutcome};
use ehrbase_sm::SmError;
use openehr_query::parser::parse_str;

use crate::aql::{self, AqlError, ExecError, ParamValue, Params, QueryResult, SqlCtx};
use crate::telemetry::prometheus::{AQL_QUERIES, AQL_QUERY_DURATION};

use super::EhrbaseService;

/// The `RESULT_SET` schema version this server emits (ITS-REST 1.0.3).
const RESULT_SET_SCHEMA_VERSION: &str = "1.0.3";

impl EhrbaseService {
    /// Parse, plan, execute, and assemble an AQL query into an ITS-REST
    /// `RESULT_SET` JSON value. `name` is the stored-query name for the result
    /// metadata (`None` for an ad-hoc query).
    ///
    /// Emits the §1.2 AQL metrics: `aql_query_duration_seconds{phase}` for the
    /// `plan` and `execute` phases (the engine folds sql-lowering + assembly
    /// into `execute`), and `aql_queries_total{outcome}` exactly once per call.
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
        // Semantic-analysis pre-pass (B4 stage (a)): resolve every
        // `TERMINOLOGY('expand', …)` used in a `matches` operand through the
        // terminology-service seam and merge the codes into the value list,
        // before planning/SQL generation (master03 lines 756–759).
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
        let result = match aql::execute(&self.pool, &ir, &params, &ctx).await {
            Ok(result) => result,
            Err(e) => {
                count_query(exec_outcome(&e));
                return Err(map_exec_error(e));
            }
        };
        record_phase("execute", exec_start);

        // `_executed_aql` carries the parameter-SUBSTITUTED text (see
        // `substitute_params`); `q` keeps the original query as submitted.
        let executed = substitute_params(aql, &params);
        let mut outcome = QueryOutcome::plain(result_set_json(aql, &executed, name, &result));
        // ABAC query post-check attributes (§6.4): collect the touched EHR/template
        // sets independently of the projection, when the PEP asked for them.
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

    /// Resolve the request's `ehr_ids` (string form) into the scoped `Uuid` set.
    ///
    /// Realizes `I_QUERY_SERVICE.execute_*`'s `ehr_ids: List<UUID> [0..1]`
    /// (`docs/specs/openehr/SM/docs/UML/classes/i_query_service.adoc`): a
    /// malformed id is a client precondition (`400`); a well-formed but
    /// non-existent id raises `ehr_id_does_not_exist`. An empty list scopes
    /// nothing (the population gate over `is_queryable` EHRs applies).
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

/// Increment `aql_queries_total{outcome}` (§1.2). Closed outcome set.
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

/// Build the typed [`Params`] from the request's `query_parameters` map (values
/// arrive as JSON scalars; complex values degrade to their JSON text).
fn build_params(request: &AqlQueryRequest) -> Params {
    let mut params = Params::new();
    for (name, value) in &request.parameters {
        params.insert(name.clone(), param_value(value));
    }
    params
}

fn param_value(value: &Value) -> ParamValue {
    match value {
        Value::Null => ParamValue::Null,
        Value::Bool(b) => ParamValue::Bool(*b),
        Value::Number(n) => n
            .as_i64()
            .map(ParamValue::Int)
            .or_else(|| n.as_f64().map(ParamValue::Real))
            .unwrap_or_else(|| ParamValue::Str(n.to_string())),
        Value::String(s) => ParamValue::Str(s.clone()),
        other => ParamValue::Str(other.to_string()),
    }
}

/// Compose the effective `(limit, offset)` from the AQL clause and the REST
/// paging parameters, rejecting collisions (ITS-REST query Request; QUERY
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

/// The `$name` parameter-reference token in an AQL query (QUERY §Parameters:
/// `$` followed by an identifier).
static PARAM_REF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)").expect("valid param-ref regex"));

/// Render the executed AQL: substitute each bound `$parameter` with its value.
///
/// The `RESULT_SET.meta._executed_aql` field is "the executed AQL" — the query
/// after parameter binding (ITS-REST `schemas/query/ResultSetMeta`; QUERY
/// §Parameters). We string-render each bound value into the query text:
/// a `Str` becomes a single-quoted AQL string literal (embedded `'` doubled),
/// `Int`/`Real`/`Bool` render as their literal form, `Null` as `NULL`. A
/// `$name` with no binding is left verbatim (the engine already rejects an
/// unbound parameter at planning time).
fn substitute_params(aql: &str, params: &Params) -> String {
    PARAM_REF
        .replace_all(aql, |caps: &regex::Captures<'_>| {
            match params.get(&caps[1]) {
                Some(value) => render_param(value),
                None => caps[0].to_owned(),
            }
        })
        .into_owned()
}

/// Render one bound parameter as an AQL literal (see [`substitute_params`]).
fn render_param(value: &ParamValue) -> String {
    match value {
        ParamValue::Int(n) => n.to_string(),
        ParamValue::Real(r) => r.to_string(),
        ParamValue::Bool(b) => b.to_string(),
        ParamValue::Null => "NULL".to_owned(),
        ParamValue::Str(s) => format!("'{}'", s.replace('\'', "''")),
    }
}

/// Assemble the ITS-REST 1.0.3 `RESULT_SET` document (`schemas/query/ResultSet`:
/// `meta` + `q` + `columns[] {name, path}` + `rows[][]`). `q` is the query as
/// submitted; `executed` is the parameter-substituted text for `_executed_aql`.
fn result_set_json(aql: &str, executed: &str, name: Option<&str>, result: &QueryResult) -> Value {
    let columns: Vec<Value> = result
        .columns
        .iter()
        .map(|c| match &c.path {
            Some(path) => json!({ "name": c.name, "path": path }),
            None => json!({ "name": c.name }),
        })
        .collect();
    let mut out = json!({
        "meta": {
            "_type": "RESULTSET",
            "_schema_version": RESULT_SET_SCHEMA_VERSION,
            "_created": Timestamp::now().to_string(),
            "_executed_aql": executed,
        },
        "q": aql,
        "columns": columns,
        "rows": result.rows,
    });
    if let (Some(name), Value::Object(map)) = (name, &mut out) {
        map.insert("name".to_owned(), Value::String(name.to_owned()));
    }
    out
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
