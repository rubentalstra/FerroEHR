//! The ITS-REST **admin API** (Release-1.1.0, DEVELOPMENT status) —
//! `docs/specs/openehr/ITS-REST/specifications/docs/admin/` + the
//! `admin-*.openapi.yaml` OAS group (generated into `openehr_its::rest`).
//!
//! [`dispatch`] implements the generated operation contract over the
//! `ehrbase-sm` native API.
//!
//! Two further groups sit beside it under the same `/admin/` prefix and the
//! same gates, realizing SM admin interfaces the release never surfaced —
//! **our own extensions, governed by no ITS-REST operation**: [`archive`]
//! (`I_ADMIN_ARCHIVE`) and [`report`] (the four `I_ADMIN_SERVICE` statistics
//! calls). Each carries its own spec-silence flag and register citation.

pub mod archive;
pub mod dispatch;
pub(crate) mod openapi_routes;
pub mod report;
