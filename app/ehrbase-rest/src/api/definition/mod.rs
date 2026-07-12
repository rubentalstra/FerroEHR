//! The ITS-REST **definition API** (development edition) —
//! `docs/specs/openehr/ITS-REST/specifications/docs/definition/` + the
//! `definition-*.openapi.yaml` OAS group (generated into `openehr_its::rest`).
//!
//! Register (gap rows + target design): `docs/design/its-rest/definition.md`.
//! [`dispatch`] implements the generated operation contract over the
//! `ehrbase-sm` native API.

pub mod dispatch;
