// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The ITS-REST **admin API** (Release-1.1.0, DEVELOPMENT status) —
//! `docs/specs/openehr/ITS-REST/specifications/docs/admin/` + the
//! `admin-*.openapi.yaml` OAS group (generated into `openehr_its::rest`).
//!
//! [`dispatch`] implements the generated operation contract over the
//! `ferroehr-sm` native API.
//!
//! Three further groups sit beside it under the same `/admin/` prefix and the
//! same gates, realizing SM admin interfaces the release never surfaced —
//! **our own extensions, governed by no ITS-REST operation**: [`archive`]
//! (`I_ADMIN_ARCHIVE`), [`report`] (the four `I_ADMIN_SERVICE` statistics
//! calls) and [`dump_load`] (`I_ADMIN_DUMP_LOAD`). Each carries its own
//! spec-silence flag and register citation.

pub mod archive;
pub mod dispatch;
pub mod dump_load;
pub(crate) mod openapi_routes;
pub mod report;
