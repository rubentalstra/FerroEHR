//! `QueryApi` implementation (Stage-1 `NotImplemented` stubs; P12 fills them).

use serde_json::Value;

use openehr_its::rest::generated::query::{
    QueryApi, QueryExecuteAdhocQueryBodyParams, QueryExecuteAdhocQueryParams,
    QueryExecuteStoredQueryBodyParams, QueryExecuteStoredQueryParams,
    QueryExecuteStoredQueryVersionBodyParams, QueryExecuteStoredQueryVersionParams,
};

crate::api::stub_api!(QueryApi, {
    query_execute_adhoc_query(QueryExecuteAdhocQueryParams) -> Value;
    query_execute_adhoc_query_body(QueryExecuteAdhocQueryBodyParams, Value) -> Value;
    query_execute_stored_query(QueryExecuteStoredQueryParams) -> Value;
    query_execute_stored_query_body(QueryExecuteStoredQueryBodyParams, Value) -> Value;
    query_execute_stored_query_version(QueryExecuteStoredQueryVersionParams) -> Value;
    query_execute_stored_query_version_body(QueryExecuteStoredQueryVersionBodyParams, Value) -> Value;
});
