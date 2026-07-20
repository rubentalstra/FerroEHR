//! Archetype / template artefact validation (the ingestion surface).
//!
//! The platform crate carries **three distinct validation surfaces**; this
//! module owns the two artefact validators (surface A) and re-expresses the
//! seams to the other two:
//!
//! - **Surface A — artefact validity (owned here).** The AOM 1.4/2.4
//!   constraint catalogue applied to an *uploaded* artefact, before it becomes
//!   an operational template:
//! - `opt` — OPT 1.4 artefact validity, the AOM2/08 catalogue on a
//   !     flattened `openehr_its::opt14::OperationalTemplate`;
//!   - `structure` (A3) — OPT XML well-formedness (a CNF ingestion guard,
//!     not an AOM constraint kind).
//!
//! ADL2 *source* validation is no longer a subset probe here: it is the full
//! `openehr-adl` engine (parse → AOM2 phases 1–3), invoked directly from
//! `service::definition::{adl2,wire}`.
//! - **Surface B — instance `valid_value` cascade (a seam, NOT here).** The
//!   recursive top-down data-conformance function ("the key function of an
//!   archetype-enabled kernel", `AM/docs/AOM1.4/master04-constraint_model_
//!   package.adoc` §`Valid_value`) runs at commit time over the compacted
//!   `WebTemplate`. It lives in `openehr-flat` (a spec crate) and is invoked from
//!   the EHR commit choke point (`service::ehr::composition_validate`). Its
//!   closed-world semantics are recorded as a spec-cited NOTE below.
//! - **Surface C — per-kind RM structural validators (a seam, NOT here).** The
//!   RM-invariant checks on template-less commit bodies (`EHR_STATUS`,
//!   `EHR_ACCESS`, FOLDER, party roots) live with their kinds under
//!   `service/`; the commit dispatch owns the routing (F-07-01 single seam).
//!
//! The boundaries between the files under `opt` follow the **AM constraint
//! taxonomy** (kind-of-check: structural invariant / RM conformance /
//! primitive / terminology / interval), which is the axis the AOM2/08
//! catalogue itself uses — so a reviewer can trace file → catalogue section.
//!
//! # Entry surface
//!
//! The functions below are the crate-facing contract
//! (`crate::validation::<fn>`); the machinery behind them is module-private.
//! They are definitions, not re-exports — every internal import names its
//! defining module.
//!
//! # Cross-surface wiring and recorded deferrals
//!
//! - **Surface-A callers.** The A entry points are reached here from `service/`:
//!   `templates::store` calls [`validate_opt_artefact`] +
//!   [`validate_opt_structure`]; `service::definition::adl14` calls
//!   [`validate_opt_structure`]. (ADL2 source validation is the `openehr-adl`
//!   engine, called from `service::definition::{adl2,wire}` — not this module.)
//! - **Surface-C dispatch (wired).** `service::ehr`'s `validate_for_commit`
//!   routes each template-less kind (`EHR_STATUS`/`EHR_ACCESS`/FOLDER/party/
//!   party-relationship) to its per-kind RM validator, so no commit path
//!   bypasses validation (F-07-01 single seam).
//! - **NOTE — surface-B closed-world semantics.** The
//!   closed-world check lives in `openehr-flat::validation` (a spec crate,
//!   outside this crate's ownership). AOM 1.4 defines only the positive
//!   `valid_value` cascade (`AM/docs/AOM1.4/master04-constraint_model_package.adoc`
//!   §`Valid_value`, silent on present-but-unmatched nodes); closure — reject
//!   unmatched *archetyped* siblings, tolerate RM-permitted metadata, tolerate
//!   unlisted archetype-rooted fillers under slotless attributes — follows the
//!   AOM2 `c_conforms_to` / VSONCT/VSONCO formalization
//!   (`AM/docs/AOM2/master08-validation.adoc` §Phase 2, lines 96–101).
//! - **NOTE — terminology binding resolution.** VTTBK/VTCBK
//!   here check binding *keys* only; resolving ac-code value sets against the
//!   live terminology service (`TerminologyService`) at ingestion is unwired,
//!   to land with the `CONSTRAINT_REF` policy
//!   (`AM/docs/AOM2/master08-validation.adoc` §Terminology).

mod opt;
mod structure;

use openehr_its::opt14::OperationalTemplate;

use crate::service::error::ServiceError;

/// Reject an OPT the tolerant `openehr_its::opt14` codec would silently
/// accept: a foreign top-level element (CNF `invalid_templates/alien_tags`)
/// or a duplicated single-valued top-level element (CNF
/// `invalid_templates/multiple_elements`).
///
/// # Errors
///
/// [`ServiceError::Unprocessable`] (→ ITS-REST `422`) naming the offending
/// element, or describing the XML parse failure when the document is
/// malformed.
pub(crate) fn validate_opt_structure(xml: &str) -> Result<(), ServiceError> {
    structure::validate_opt_structure(xml)
}

/// Validate an uploaded OPT 1.4 artefact against the AOM2/08
/// standalone-artefact validity rules
/// (`AM/docs/AOM2/master08-validation.adoc`; rule map in the `opt` module
/// doc). A fully valid artefact returns `Ok`.
///
/// # Errors
///
/// The first violation found, as [`ServiceError::BadRequest`] (→ ITS-REST
/// `400`, what the CNF `I_DEFINITION_ADL14` upload/validate suites assert for
/// an invalid OPT) carrying the AOM2 rule code in the message
/// (`"<CODE>: <detail>"`).
pub fn validate_opt_artefact(opt: &OperationalTemplate) -> Result<(), ServiceError> {
    opt::validate_opt_artefact(opt)
}
