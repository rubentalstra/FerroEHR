// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The ITS-REST **query API** (Release-1.1.0, STABLE) —
//! `docs/specs/openehr/ITS-REST/specifications/docs/query/` + the
//! `query-*.openapi.yaml` OAS group (generated into `openehr_its::rest`).
//!
//! The `dispatch` module is the operation match implementing the generated
//! operation contract over the `ferroehr-sm` native API; it splits along the
//! spec's query-type axis — `adhoc` (`/query/aql`) and `stored`
//! (`/query/{qualified_query_name}[/{version}]`) — over shared request-decoding
//! and `RESULT_SET` rendering in `response`.

mod adhoc;
pub(crate) mod dispatch;
pub(crate) mod openapi_routes;
mod response;
mod stored;
