// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Query execution requests and outcomes.
//!
//! The realization of `ADHOC_QUERY_EXECUTE_SPEC` /
//! `STORED_QUERY_EXECUTE_SPEC` (`adhoc_query_execute_spec.adoc`,
//! `stored_query_execute_spec.adoc`) plus the execute-call parameters
//! (`i_query_service.adoc`).

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 5): AQL result rows are arbitrary \
              projections by specification (QUERY 1.1)"
)]

use std::collections::BTreeMap;

use serde_json::Value;

/// A normalized AQL query request: the paging window, the EHR scope, and the
/// `$parameter` bindings.
///
/// Realizes the SM execute-spec classes plus the execute-call parameters:
/// `query_parameters` (the spec types them `Hash<String, String>`; carried as
/// JSON [`Value`]s — the ITS-REST wire binds typed parameters, a documented
/// widening), `row_offset` / `rows_to_fetch`, and `ehr_ids`.
///
/// `formalism` (`ADHOC_QUERY_EXECUTE_SPEC.formalism`, default `"aql"`) is
/// fixed to AQL — another formalism is rejected typed, which the SM sanctions
/// ("matching one of: aql; any other string value").
#[derive(Debug, Clone, Default)]
pub struct AqlQueryRequest {
    /// `ehr_ids: List<UUID> [0..1]` — "Specific set of EHRs on which to
    /// execute the query. If none supplied, a full population query will be
    /// performed on all EHRs whose status has the `is_queryable` flag set to
    /// `True`" (`i_query_service.adoc`). Empty = unscoped. The ITS-REST
    /// single `ehr_id` parameter is the one-element case. Error
    /// `ehr_id_does_not_exist` when a listed EHR does not exist.
    pub ehr_ids: Vec<String>,
    /// `row_offset [0..1]` — "offset in query response rows to return …
    /// A zero or negative value means offset of zero."
    pub offset: Option<i64>,
    /// `rows_to_fetch [0..1]` — "number of query response rows to fetch …
    /// A zero or negative value means 'all'."
    pub fetch: Option<i64>,
    /// `query_parameters` — "Parameters to substitute in query … each tag
    /// must match a parameter name in the query" (`$name` binds, no `$`
    /// prefix).
    pub parameters: BTreeMap<String, Value>,
    /// The ABAC patient-scope subject id (no openEHR spec governs this — our
    /// own access-control extension): when set, the engine pre-filters every
    /// VO root to EHRs whose subject equals it.
    pub subject_scope: Option<String>,
    /// Whether the executor should collect the touched EHR-id / template-id
    /// sets for the ABAC query post-check (our own extension).
    pub collect_attributes: bool,
}

impl AqlQueryRequest {
    /// The single-EHR scope of the ITS-REST wire (`ehr_id` query parameter /
    /// `openehr-ehr-id` header), if exactly one EHR is scoped.
    #[must_use]
    pub fn single_ehr_id(&self) -> Option<&str> {
        match self.ehr_ids.as_slice() {
            [one] => Some(one),
            _ => None,
        }
    }
}

/// The outcome of an AQL execution.
///
/// Carries the assembled `RESULT_SET` (`result_set.adoc`; rendered as ITS-REST
/// canonical JSON) plus — when the caller asked for them
/// ([`AqlQueryRequest::collect_attributes`]) — the distinct EHR ids and
/// template ids the query touched, for the ABAC post-check.
#[derive(Debug, Clone, Default)]
pub struct QueryOutcome {
    /// The `RESULT_SET` (canonical JSON) the adapter renders.
    pub result_set: Value,
    /// The distinct EHR ids the query touched (empty unless collected).
    pub ehr_ids: Vec<String>,
    /// The distinct template ids the query touched (empty unless collected).
    pub template_ids: Vec<String>,
}

impl QueryOutcome {
    /// An outcome with no collected attributes.
    #[must_use]
    pub fn plain(result_set: Value) -> Self {
        Self {
            result_set,
            ehr_ids: Vec::new(),
            template_ids: Vec::new(),
        }
    }
}
