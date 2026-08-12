// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Operational-template ingestion, storage, and derived-runtime resolution.
//!
//! This module owns the platform crate's template surface: OPT 1.4 XML
//! ingestion, the `template_store` repository access, and the cached derived
//! [`WebTemplate`](openehr_its::flat::webtemplate::model::WebTemplate) runtime form. It is decomposed
//! the way the spec itself layers the material — **resource identity →
//! operational form → derived runtime artefact**:
//!
//! - `identity` — the `ARCHETYPE_ID` / `TEMPLATE_ID` identity law
//!   (`BASE/docs/base_types/master05-identification_package.adoc`,
//!   §Composite Identifiers and Case: case-preserving *and* case-insensitive),
//!   applied at every lookup and cache boundary.
//! - `ingest` — OPT 1.4 canonical XML → `OperationalTemplate` parse;
//!   the artefact is an `AUTHORED_RESOURCE`
//!   (`BASE/docs/resource/master02-resource_package.adoc` §Meta-data).
//! - `store` — the `template_store` repository:
//!   insert-only upload (`409` on a duplicate id) and the
//!   `template_id`-keyed retrieval/listing behind the ITS-REST
//!   `/definition/template/adl1.4` surface
//!   (`ITS-REST/specifications/definition.openapi.yaml`).
//! - `runtime` — the derived near-runtime form
//!   (`BASE/docs/architecture_overview/master10-archetypes.adoc`
//!   §Archetypes and Templates at Runtime, §Deploying Archetypes and
//!   Templates), `moka`-cached; the `WebTemplate` *format* is
//!   spec-silent and stays in `openehr_its::flat`.
//!
//! # The inherent-method surface on `FerroEhrService`
//!
//! The service methods live on [`crate::service::FerroEhrService`] (defined
//! across `store` and `runtime`): `store_template`, `get_template_xml`,
//! `template_summaries` (store); `web_template_for`, `template_example`
//! (runtime). The UUID-keyed SM `I_DEFINITION_ADL14` OPT
//! operations (`opt_get` / `opt_get_by_template_id` / `opt_list` / …,
//! `SM/docs/openehr_platform/master04-definition_package.adoc`) are owned by
//! the definition register (`service/definition/adl14.rs`), which reads the
//! same `template_store` table directly; this module deliberately does **not**
//! duplicate them.
//!
//! # NOTE residue
//!
//! - The `WebTemplate` JSON *format* is the Better `web-template`
//!   SDT format, **not** openEHR-normative; kept in `openehr_its::flat`, never
//!   presented as canonical (see `runtime`).
//! - An unknown template on a *commit* path is a `422`
//!   (ITS-REST `responses/422.yaml`: "the underlying template is not known")
//!   whereas the `adl1.4/{template_id}` GET surface is a `404`
//!   (`responses/404_unknown_template_id.yaml`); both re-verified against the
//!   ITS-REST responses + CNF (see `runtime`).
//! - Re-uploading an existing `template_id` is a `409`
//!   (`responses/409_template_already_exists.yaml`), never a silent overwrite
//!   (see `store`).
//! - The SM `list_matching_opts` is typed `List<ARCHETYPE_ID>`
//!   though OPTs are UUID-keyed; the definition register returns `template_id`
//!   strings (the meaningful pattern target). Spec defect, re-verified; the
//!   operation itself lives in `service/definition/adl14.rs`.
//! - OPT 1.4 has **no normative prose master**; its structure is
//!   governed by the ITS-XML v1 Template XSD + AOM 1.4 (see `ingest`).
//! - The `AUTHORED_RESOURCE` meta-data
//!   (`language`/`description`/`translations`/`revision_history`) is parsed
//!   but not surfaced/queried; we index `template_id`/`concept`/root only
//!   (the spec permits an optional `_description_`) (see `ingest`).

pub(crate) mod identity;
mod ingest;
mod runtime;
mod store;
