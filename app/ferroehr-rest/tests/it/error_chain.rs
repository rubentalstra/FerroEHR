//! Error chaining at the wire seam: a REAL `sqlx` failure keeps its cause
//! walkable for the operator and out of the client's `500` body.
//!
//! The two halves of the [RFC 0201](https://rust-lang.github.io/rfcs/0201-error-chaining.html)
//! contract, checked against a genuine PostgreSQL driver error rather than a
//! stand-in: `Error::source` reaches the concrete `sqlx::Error`, and nothing the
//! driver said — schema object names, column names, SQL text — appears in the
//! rendered response body (OWASP REST Security Cheat Sheet §Error handling,
//! the rule `scripts/checks/error-hygiene.sh` guards).

use std::error::Error;

use axum::response::IntoResponse;
use http::StatusCode;
use http_body_util::BodyExt;

use ferroehr::service::error::ServiceError;
use ferroehr_rest::overview::error::RestError;

use crate::common;

/// The status and raw body bytes of a rendered service failure.
#[expect(
    clippy::expect_used,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) reaches only \
              `#[test]`-annotated functions, not this module's helpers; a fixture that \
              cannot render is a panicking assertion by design (the Rust Book ch11)"
)]
async fn rendered(error: ServiceError) -> (StatusCode, String) {
    let resp = RestError::from(error).into_response();
    let status = resp.status();
    let bytes = resp
        .into_body()
        .collect()
        .await
        .expect("the error body must be collectable")
        .to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// A forced `sqlx` failure carried as the cause of a `500`: the operator can
/// walk the chain down to the `sqlx::Error`, and the client body carries none
/// of what the driver disclosed.
#[tokio::test]
async fn a_forced_sqlx_failure_stays_walkable_and_out_of_the_body() {
    let (_db, pool) = common::migrated_pool().await;

    // A real driver error, against the real migrated schema. Its `Display`
    // names the relation the statement referenced — exactly the internal
    // detail a 5xx body must not disclose.
    let driver_error = sqlx::query("SELECT secret_column FROM ferroehr_no_such_relation")
        .execute(&pool)
        .await
        .expect_err("the query must fail against a migrated database");
    let disclosed = driver_error.to_string();
    assert!(
        disclosed.contains("ferroehr_no_such_relation"),
        "the fixture must produce a real driver diagnostic naming the relation, got {disclosed:?}"
    );

    // The service-layer fault carries it as a source, never in the message.
    let fault = ServiceError::internal("read a stored version row", driver_error);
    let first = Error::source(&fault).expect("the fault must carry its cause");
    let mut hops = Vec::new();
    let mut reached_driver = false;
    let mut next = Some(first);
    while let Some(step) = next {
        hops.push(step.to_string());
        if step.downcast_ref::<sqlx::Error>().is_some() {
            reached_driver = true;
        }
        next = step.source();
    }
    assert!(
        reached_driver,
        "walking the chain must reach the concrete sqlx::Error, got {hops:?}"
    );
    assert!(
        hops.iter()
            .any(|hop| hop.contains("ferroehr_no_such_relation")),
        "the walked chain must still carry the driver diagnostic, got {hops:?}"
    );

    // The wire body says none of it.
    let (status, body) = rendered(fault).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    for leaked in [
        "ferroehr_no_such_relation",
        "secret_column",
        "SELECT",
        "read a stored version row",
    ] {
        assert!(
            !body.contains(leaked),
            "the 500 body leaked {leaked:?}: {body}"
        );
    }
}

/// A driver error routed through the ordinary database seam is equally opaque:
/// `classify_sqlx` substitutes a fixed message per SQLSTATE class, so an
/// undefined-relation fault renders the same curated `500`.
#[tokio::test]
async fn a_database_seam_failure_discloses_no_schema_detail() {
    let (_db, pool) = common::migrated_pool().await;
    let driver_error = sqlx::query("SELECT secret_column FROM ferroehr_no_such_relation")
        .execute(&pool)
        .await
        .expect_err("the query must fail against a migrated database");

    let (status, body) = rendered(ServiceError::Database(driver_error)).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    for leaked in ["ferroehr_no_such_relation", "secret_column", "SELECT"] {
        assert!(
            !body.contains(leaked),
            "the 500 body leaked {leaked:?}: {body}"
        );
    }
}

/// Carrying a cause on a `4xx` changes nothing a client sees: the
/// cause-carrying refusal renders byte-identically to the flat one.
#[tokio::test]
async fn a_carried_cause_leaves_a_4xx_body_byte_identical() {
    let detail = "invalid canonical JSON body: unknown field `_kind`";
    let parse_failure = serde_json::from_str::<u32>("\"not a number\"")
        .expect_err("the fixture must fail to deserialize");

    let flat = rendered(ServiceError::BadRequest(detail.to_owned())).await;
    let carried = rendered(ServiceError::bad_request(detail, parse_failure)).await;
    assert_eq!(carried, flat);
}
