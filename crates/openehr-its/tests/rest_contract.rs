#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
//! Smoke test for the generated ITS-REST contract: the DTOs serde
//! round-trip, the route table is populated, and the server trait is nameable.
use openehr_its::rest::generated::query;

/// The generated server trait is a real, nameable bound.
fn _assert_is_trait<T: query::QueryApi>() {}

#[test]
fn query_contract_is_usable() {
    // Route table reflects the OAS operations.
    assert_eq!(query::ROUTES.len(), 6);
    assert!(
        query::ROUTES
            .iter()
            .any(|(m, p, _)| *m == "POST" && *p == "/query/aql")
    );

    // A DTO deserializes from canonical JSON and back.
    let j = serde_json::json!({"q": "SELECT c FROM COMPOSITION c", "offset": 0, "fetch": 10});
    let dto: query::AdhocQueryExecute = serde_json::from_value(j).expect("deserialize DTO");
    assert_eq!(dto.q, "SELECT c FROM COMPOSITION c");
    assert_eq!(dto.fetch, Some(10));

    let rs = query::ResultSet {
        meta: None,
        name: None,
        q: None,
        columns: None,
        rows: vec![],
    };
    let s = serde_json::to_string(&rs).expect("serialize ResultSet");
    assert!(s.contains("\"rows\""));
}
