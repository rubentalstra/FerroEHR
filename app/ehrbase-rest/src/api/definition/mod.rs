//! The ITS-REST **definition API** (development edition) —
//! `docs/specs/openehr/ITS-REST/specifications/docs/definition/` + the
//! `definition-*.openapi.yaml` OAS group (generated into `openehr_its::rest`).
//!
//! Register (gap rows + target design): `docs/design/its-rest/definition.md`.
//! The group is split along the three spec resources the OAS `tags` name —
//! `ADL1.4`, `ADL2`, and `Query` — one module each, with [`dispatch`] as the
//! operation-id `match` that fans out to them:
//!
//! - [`template_adl14`] — `definition_template_adl1.4_{list,upload,get,example_get}`.
//! - [`template_adl2`] — `definition_template_adl2_{list,upload,get,example_get,version_get}`.
//! - [`stored_query`] — `definition_query_{list,store,version_get,version_store}`.
//!
//! Each module implements the generated operation contract over the
//! `ehrbase-sm` native API (`DefinitionAdl14Service` / `DefinitionAdl2Service` /
//! `DefinitionQueryService` + the wire-shaped `DefinitionAdapter` extension).

mod dispatch;
mod stored_query;
mod template_adl14;
mod template_adl2;

pub(crate) use dispatch::dispatch;

// The `adl1.4/{id}/example` handler negotiates the Simplified-Formats
// (FLAT/STRUCTURED) representations through the shared converter seam, exactly
// as the `ehr` group does; the group-level alias lets `template_adl14`'s
// `super::flat::…` resolve to that converter module. The shared converters
// (`crate::formats::dispatch::{composition_flat_response,composition_structured_response}`)
// are `pub(crate)`, so this cross-group call resolves directly.
use crate::formats::dispatch as flat;
