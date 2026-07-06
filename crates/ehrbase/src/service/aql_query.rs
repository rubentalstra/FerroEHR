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

use jiff::Timestamp;
use serde_json::{Value, json};
use uuid::Uuid;

use ehrbase_rest::AqlQueryRequest;
use openehr_its::rest::runtime::ApiError;
use openehr_query::parser::parse_str;

use crate::aql::{self, AqlError, ExecError, ParamValue, Params, QueryResult, SqlCtx};

use super::EhrbaseService;

/// The `RESULT_SET` schema version this server emits (ITS-REST 1.0.3).
const RESULT_SET_SCHEMA_VERSION: &str = "1.0.3";

impl EhrbaseService {
    /// Parse, plan, execute, and assemble an AQL query into an ITS-REST
    /// `RESULT_SET` JSON value. `name` is the stored-query name for the result
    /// metadata (`None` for an ad-hoc query).
    pub(super) async fn execute_aql(
        &self,
        aql: &str,
        name: Option<&str>,
        request: &AqlQueryRequest,
    ) -> Result<Value, ApiError> {
        let ast = parse_str(aql).map_err(|e| ApiError::BadRequest(format!("invalid AQL: {e}")))?;
        let params = build_params(request);
        let ir = aql::plan(&ast, &params).map_err(map_plan_error)?;

        let (limit, offset) = compose_paging(ir.limit, ir.offset, request)?;
        let ehr_id = parse_ehr_id(request.ehr_id.as_deref())?;
        let ctx = SqlCtx {
            system_id: self.system_id.clone(),
            ehr_id,
            limit,
            offset,
        };

        let result = aql::execute(&self.pool, &ir, &params, &ctx)
            .await
            .map_err(map_exec_error)?;
        Ok(result_set_json(aql, name, &result))
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
) -> Result<(Option<i64>, Option<i64>), ApiError> {
    if request.fetch.is_some() && aql_limit.is_some() {
        return Err(ApiError::BadRequest(
            "the `fetch` query parameter cannot be combined with an AQL LIMIT/TOP clause \
             (ITS-REST query Request; QUERY §Query structure/LIMIT)"
                .to_owned(),
        ));
    }
    if request.offset.is_some() && aql_offset.is_some() {
        return Err(ApiError::BadRequest(
            "the `offset` query parameter cannot be combined with an AQL OFFSET clause \
             (ITS-REST query Request; QUERY §Query structure)"
                .to_owned(),
        ));
    }
    Ok((aql_limit.or(request.fetch), aql_offset.or(request.offset)))
}

/// Parse the `ehr_id` scope into a UUID (`400` on a malformed id).
fn parse_ehr_id(ehr_id: Option<&str>) -> Result<Option<Uuid>, ApiError> {
    match ehr_id {
        None => Ok(None),
        Some(id) => Uuid::parse_str(id)
            .map(Some)
            .map_err(|_| ApiError::BadRequest(format!("invalid ehr_id `{id}`"))),
    }
}

/// Assemble the ITS-REST 1.0.3 `RESULT_SET` document (`schemas/query/ResultSet`:
/// `meta` + `q` + `columns[] {name, path}` + `rows[][]`).
fn result_set_json(aql: &str, name: Option<&str>, result: &QueryResult) -> Value {
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
            "_executed_aql": aql,
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
fn map_plan_error(e: AqlError) -> ApiError {
    match e {
        AqlError::Feature(_) | AqlError::Analysis(_) | AqlError::Sql(_) => {
            ApiError::BadRequest(e.to_string())
        }
        AqlError::Exec(inner) => map_exec_error(inner.into()),
    }
}

/// Map an execution error: a database/assembly failure is the server's fault
/// (`500`); a SQL-lowering failure that surfaces here is still a bad query.
fn map_exec_error(e: AqlError) -> ApiError {
    match e {
        AqlError::Exec(ExecError::Database(db)) => ApiError::Internal(db.to_string()),
        AqlError::Exec(ExecError::Assembly(a)) => ApiError::Internal(a.to_string()),
        AqlError::Feature(_) | AqlError::Analysis(_) | AqlError::Sql(_) => {
            ApiError::BadRequest(e.to_string())
        }
    }
}
