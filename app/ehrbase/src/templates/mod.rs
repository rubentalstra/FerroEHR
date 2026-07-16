//! Operational-template ingestion, storage, and derived-runtime resolution.
//!
//! This module owns the platform crate's template surface: OPT 1.4 XML
//! ingestion, the `template_store` repository access, and the cached derived
//! [`WebTemplate`](openehr_flat::WebTemplate) runtime form. It is structured by
//! the spec's own decomposition — **resource identity → operational form →
//! derived runtime artefact**:
//!
//! - [`identity`] — the `ARCHETYPE_ID` / `TEMPLATE_ID` identity law
//!   (`BASE/docs/base_types/master05-identification_package.adoc`), in
//!   particular §Composite Identifiers and Case (case-preserving,
//!   case-insensitive — G-T04) applied at every lookup/cache boundary.
//! - [`ingest`] — OPT 1.4 canonical XML → `OperationalTemplate` parse plus the
//!   top-level structural well-formedness gate (S-05); the artefact is an
//!   `AUTHORED_RESOURCE`
//!   (`BASE/docs/resource/master02-resource_package.adoc`, S-01/S-02/S-03).
//! - [`store`] — the `template_store` repository (S-05/S-07/S-11): insert-only
//!   upload (`409` on a duplicate id, G-T09), and the `template_id`-keyed
//!   retrieval/listing the ITS-REST `adl1.4` surface uses.
//! - [`runtime`] — `web_template_for` / `template_example`, the S-08/S-09
//!   derived near-runtime form, `moka`-cached (G-T05); the `WebTemplate` *format*
//!   is spec-silent and stays in `openehr-flat` (G-T06).
//!
//! # The inherent-method surface on `EhrbaseService`
//!
//! The service methods live on [`crate::service::EhrbaseService`] (defined
//! across [`store`] and [`runtime`]): `store_template`, `get_template_meta`,
//! `get_template_xml`, `list_templates` (store); `web_template_for`,
//! `template_example` (runtime). The UUID-keyed SM `I_DEFINITION_ADL14` OPT
//! operations (`opt_get` / `opt_get_by_template_id` / `opt_list` / …) are owned
//! by the definition register (`service/definition/adl14.rs`), which reads the
//! same `template_store` table directly; this module deliberately does **not**
//! duplicate them.
//!
//! # PORT NOTE residue (re-cited)
//!
//! - **G-T06** — the `WebTemplate` JSON *format* is the Better `web-template` SDT
//!   format, **not** openEHR-normative; kept in `openehr-flat`, never presented
//!   as canonical (see [`runtime`]).
//! - **G-T08** — an unknown template on a *commit* path is a `422`
//!   (ITS-REST `422_COMPOSITION` "underlying template is not known") whereas the
//!   `adl1.4/{id}` GET surface is a `404`; both re-verified against the ITS-REST
//!   responses + CNF (see [`runtime`]).
//! - **G-T09** — re-uploading an existing `template_id` is a `409`
//!   (`409_template_already_exists.yaml`), never a silent overwrite (see
//!   [`store`]).
//! - **G-T10** — the SM `list_matching_opts` is typed `List<ARCHETYPE_ID>`
//!   though OPTs are UUID-keyed; the definition register returns `template_id`
//!   strings (the meaningful pattern target). Spec defect, re-verified; the
//!   operation itself lives in `service/definition/adl14.rs`.
//! - **G-T11** — OPT 1.4 has **no normative prose master**; its structure is
//!   governed by the ITS-XML v1 Template XSD + AOM 1.4 (see [`ingest`]).
//! - **G-T12** — the `AUTHORED_RESOURCE` meta-data
//!   (`language`/`description`/`translations`/`revision_history`) is parsed but
//!   not surfaced/queried; we index `template_id`/`concept`/root only (the spec
//!   permits an optional `_description_`) (see [`ingest`]).

pub(crate) mod identity;
pub(crate) mod ingest;
mod runtime;
mod store;

// Re-export for the SM Definitions provisioning surface
// (`service/definition/adl14.rs` `valid_opt` / `upload_opt`), which validates
// OPT well-formedness before delegating to `store_template`.
