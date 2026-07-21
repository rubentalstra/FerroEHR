//! `RESULT_SET` assembly (`result_set.adoc`; ITS-REST 1.1.0
//! `schemas/query/ResultSet`) and AQL parameter substitution
//! (QUERY `master03-syntax.adoc` §Parameters).

use std::sync::LazyLock;

use jiff::Timestamp;
use regex::Regex;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::aql::exec::QueryResult;
use crate::aql::ir::{ParamValue, Params};
use crate::service::query::request::AqlQueryRequest;

/// The `RESULT_SET` `_schema_version` this server emits — "the version of the
/// specification defining the serialized object" (ITS-REST
/// `schemas/query/ResultSet`), i.e. the implemented ITS-REST release, read
/// from the `openehr-its` crate version so it can never lag a pin bump.
const RESULT_SET_SCHEMA_VERSION: &str = openehr_its::SPEC_VERSION;

/// Build the typed [`Params`] from the request's `query_parameters` map
/// (values arrive as JSON scalars; complex values degrade to their JSON
/// text — the documented widening on [`AqlQueryRequest::parameters`]).
pub(super) fn build_params(request: &AqlQueryRequest) -> Params {
    let mut params = Params::new();
    for (name, value) in &request.parameters {
        params.insert(name.clone(), param_value(value));
    }
    params
}

/// One JSON parameter value as a typed [`ParamValue`]: scalars map directly;
/// a number outside `i64`/`f64` and any array/object degrade to their JSON
/// text.
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

/// The `$name` parameter-reference token in an AQL query (QUERY `master03`
/// §Parameters/Syntax l.106: `$` followed by an identifier).
static PARAM_REF: LazyLock<Regex> = LazyLock::new(|| {
    // The pattern is a fixed literal, valid by construction — a build-time
    // invariant, not a runtime condition.
    #[allow(clippy::expect_used)]
    Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)").expect("valid param-ref regex")
});

/// Render the executed AQL: substitute each bound `$parameter` with its value.
///
/// `RESULT_SET.meta._executed_aql` is "the executed AQL" — the query after
/// parameter binding (QUERY `master03` §Parameters NOTE l.113; ITS-REST
/// `schemas/query/ResultSetMeta`). A `Str` becomes a single-quoted AQL string
/// literal (embedded `'` doubled), `Int`/`Real`/`Bool` render as their
/// literal form, `Null` as `NULL`; a `$name` with no binding is left verbatim
/// (the engine already rejects an unbound parameter at planning time).
pub(super) fn substitute_params(aql: &str, params: &Params) -> String {
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

/// Assemble the ITS-REST 1.1.0 `RESULT_SET` document
/// (`schemas/query/ResultSet`: `meta` + `q` + `columns[]` + `rows[][]`). `q`
/// is the query as submitted; `executed` is the parameter-substituted text
/// for `_executed_aql`; `name` (a stored query's qualified name) is emitted
/// only when present.
///
/// `RESULT_SET.id [1..1]` (`result_set.adoc`: "unique identifier of this
/// result set") is emitted additively as a `uuidv7()`-derived id.
///
/// NOTE: the SM `RESULT_SET` makes `id` mandatory, but the
/// ITS-REST 1.1.0 `ResultSet` schema omits it; we emit it additively so the
/// SM requirement is met without breaking the ITS-REST shape (an extra field
/// a 1.0.3 client ignores).
pub(super) fn result_set_json(
    aql: &str,
    executed: &str,
    name: Option<&str>,
    result: &QueryResult,
) -> Value {
    let columns: Vec<Value> = result
        .columns
        .iter()
        .map(|c| match &c.path {
            // NOTE (SM `result_set_column.adoc`):
            // `RESULT_SET_COLUMN.archetype_id [0..1]` is optional and omitted —
            // the engine's `ColumnMeta` carries name + path only, and the spec
            // itself flags it "check on whether needed". If a future path pass
            // derives the referenced archetype id, it can be populated here
            // additively; the optional cardinality means omitting it is
            // conformant.
            Some(path) => json!({ "name": c.name, "path": path }),
            None => json!({ "name": c.name }),
        })
        .collect();
    let mut out = json!({
        "id": Uuid::now_v7().to_string(),
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
