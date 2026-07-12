//! The ITS-REST **query API** (development edition) —
//! `docs/specs/openehr/ITS-REST/specifications/docs/query/` + the
//! `query-*.openapi.yaml` OAS group (generated into `openehr_its::rest`).
//!
//! Register (gap rows + target design): `docs/design/its-rest/query.md`.
//! The `dispatch` module is the operation match implementing the generated
//! operation contract over the `ehrbase-sm` native API; it splits along the
//! spec's query-type axis — [`adhoc`] (`/query/aql`) and [`stored`]
//! (`/query/{qualified_query_name}[/{version}]`) — over shared request-decoding
//! and `RESULT_SET` rendering in [`response`].

mod adhoc;
mod dispatch;
mod response;
mod stored;

pub(crate) use dispatch::dispatch;
