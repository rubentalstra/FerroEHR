//! The one admin-console e2e integration binary (`testing.md` §Where tests
//! live): every journey battery is a module here; the shared
//! browser/router/DB fixture lives in `common`.

mod common;

// A real in-process boot test, not a browser journey: it serves the router
// itself, so it needs the server build (`--features ssr`).
#[cfg(feature = "ssr")]
mod boot_oidc_outage;

mod e2e_admin_ops;
mod e2e_audit;
mod e2e_browse;
mod e2e_chart;
mod e2e_composition;
mod e2e_directory;
mod e2e_docs_shots;
mod e2e_ehr_ops;
mod e2e_ehr_status;
mod e2e_login;
mod e2e_operations;
mod e2e_paging;
mod e2e_scopes;
mod e2e_stored_query_runner;
mod e2e_system;
