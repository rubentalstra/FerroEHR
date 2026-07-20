//! The ITS-REST **admin API** (development edition) —
//! `docs/specs/openehr/ITS-REST/specifications/docs/admin/` + the
//! `admin-*.openapi.yaml` OAS group (generated into `openehr_its::rest`).
//!
//! [`dispatch`] implements the generated operation contract over the
//! `ehrbase-sm` native API.

pub mod dispatch;
pub(crate) mod openapi_routes;
