//! The ITS-REST **demographic API** (development edition) —
//! `docs/specs/openehr/ITS-REST/specifications/docs/demographic/` + the
//! `demographic-*.openapi.yaml` OAS group (generated into `openehr_its::rest`).
//!
//! Register (gap rows + target design): `docs/design/its-rest/demographic.md`.
//! [`dispatch`] implements the generated operation contract over the
//! `ehrbase-sm` native API.

pub mod dispatch;
