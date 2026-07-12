//! The ITS-REST **ehr API** (development edition) —
//! `docs/specs/openehr/ITS-REST/specifications/docs/ehr/` + the
//! `ehr-*.openapi.yaml` OAS group (generated into `openehr_its::rest`).
//!
//! Register (gap rows + target design): `docs/design/its-rest/ehr.md`.
//! [`dispatch`] implements the generated operation contract over the
//! `ehrbase-sm` native API.

pub mod dispatch;
