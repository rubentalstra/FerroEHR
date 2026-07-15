//! The ITS-REST **admin API** (development edition) —
//! `docs/specs/openehr/ITS-REST/specifications/docs/admin/` + the
//! `admin-*.openapi.yaml` OAS group (generated into `openehr_its::rest`).
//!
//! Register (gap rows + target design): `docs/design/its-rest/admin.md`.
//! [`dispatch`] implements the generated operation contract over the
//! `ehrbase-sm` native API.

pub mod dispatch;
mod openapi_routes;

pub(crate) use openapi_routes::routes;
