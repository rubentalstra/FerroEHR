//! The QUERY component of the platform crate — the openEHR **Query** service
//! seam (`I_QUERY_SERVICE`, `i_query_service.adoc`; `master08-query_service.adoc`).
//!
//! - [`execute`] — the `QueryService` impl + the execution orchestration
//!   (parse → plan → execute → assemble), paging composition, `ehr_ids`
//!   resolution, and the per-query execution budget. This is the
//!   execution-orchestration heart; the AQL *engine* it drives is
//!   [`crate::aql`] (register 08).
//! - [`result_set`] — `RESULT_SET` / `RESULT_SET_COLUMN` / `RESULT_SET_ROW`
//!   assembly (`result_set.adoc`) + parameter substitution, isolated so the
//!   SM-vs-ITS-REST shape divergences (the `RESULT_SET.id` MUST, G-05-03q) live
//!   in one spec-cited place.
//! - [`plan_cache`] — the bounded cache of lowered AQL plans keyed on query
//!   text (P20; no openEHR spec governs it — our own performance design).

mod config;
mod execute;
mod plan_cache;
mod result_set;

pub use config::QueryConfig;
pub use plan_cache::{PlanCache, PlanCacheStats};
