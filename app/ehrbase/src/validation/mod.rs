//! Archetype / template artefact validation (the ingestion surface).
//!
//! The platform crate carries **three distinct validation surfaces**; this
//! module owns the two artefact validators (surface A) and re-expresses the
//! seams to the other two:
//!
//! - **Surface A — artefact validity (owned here).** The AOM 1.4/2.4
//!   constraint catalogue applied to an *uploaded* artefact, before it becomes
//!   an operational template:
//!   - [`opt`] (A1) — OPT 1.4 artefact validity, the AOM2/08 catalogue on a
//!     flattened `openehr_its::opt14::OperationalTemplate`;
//!   - [`adl2`] (A2) — ADL2 *source* registration validity, the honestly
//!     enforceable header / ODIN / terminology subset (no cADL compiler);
//!   - [`structure`] (A3) — OPT XML well-formedness (a CNF ingestion guard,
//!     not an AOM constraint kind).
//! - **Surface B — instance `valid_value` cascade (a seam, NOT here).** The
//!   recursive top-down data-conformance function ("the key function of an
//!   archetype-enabled kernel", `AM/docs/AOM1.4/master04-constraint_model_
//!   package.adoc` §`Valid_value`) runs at commit time over the compacted
//!   `WebTemplate`. It lives in `openehr-flat` (a spec crate) and is invoked from
//!   the EHR commit choke point (`service::ehr::composition_validate`). Its
//!   closed-world semantics are recorded as a spec-cited PORT NOTE below.
//! - **Surface C — per-kind RM structural validators (a seam, NOT here).** The
//!   RM-invariant checks on template-less commit bodies (`EHR_STATUS`,
//!   `EHR_ACCESS`, FOLDER, party roots) live with their kinds under
//!   `service/`; the commit dispatch owns the routing (F-07-01 single seam).
//!
//! The boundaries between the files under [`opt`] follow the **AM constraint
//! taxonomy** (kind-of-check: structural invariant / RM conformance /
//! primitive / terminology / interval), which is the axis the AOM2/08
//! catalogue itself uses — so a reviewer can trace file → catalogue section.
//!
//! # Cross-surface wiring and recorded deferrals
//!
//! - **Surface-A callers.** The A entry points are reached here from `service/`:
//!   `templates::store` calls [`opt::validate_opt_artefact`] +
//!   [`structure::validate_opt_structure`]; `service::definition::{adl2,wire}`
//!   call [`adl2::validate_adl2_source`] + [`adl2::check_specialisation_depth`];
//!   `service::definition::adl14` calls [`structure::validate_opt_structure`].
//!   The legacy `service::{opt_validation,adl2_validation}` modules are gone.
//! - **Surface-C dispatch (wired).** `service::ehr`'s `validate_for_commit`
//!   routes each template-less kind (`EHR_STATUS`/`EHR_ACCESS`/FOLDER/party/
//!   party-relationship) to its per-kind RM validator, so no commit path
//!   bypasses validation (F-07-01 single seam).
//! - **PORT NOTE — surface-B closed-world semantics (G-09-05).** The
//!   closed-world check lives in `openehr-flat::validation` (a spec crate,
//!   outside this crate's ownership). AOM 1.4 defines only the positive
//!   `valid_value` cascade (`AM/docs/AOM1.4/master04-constraint_model_package.adoc`
//!   §`Valid_value`, silent on present-but-unmatched nodes); closure — reject
//!   unmatched *archetyped* siblings, tolerate RM-permitted metadata, tolerate
//!   unlisted archetype-rooted fillers under slotless attributes — follows the
//!   AOM2 `c_conforms_to` / VSONCT/VSONCO formalization
//!   (`AM/docs/AOM2/master08-validation.adoc` §Phase 2, lines 96–101).
//!   Re-expressing the `openehr-flat` PORT NOTE against those spec sections
//!   (it presently cites an ADR) is the future work this note records.
//! - **PORT NOTE — terminology binding resolution (T17/T15).** VTTBK/VTCBK
//!   here check binding *keys* only; resolving ac-code value sets against the
//!   live terminology service (`TerminologyService`) at ingestion is unwired,
//!   to land with the `CONSTRAINT_REF` policy
//!   (`AM/docs/AOM2/master08-validation.adoc` §Terminology; blueprint 03-am §3).

pub mod adl2;
pub mod opt;
pub mod structure;

// The flat entry surface callers wire to (`crate::validation::<fn>`). The
// artefact-metadata / violation types stay reachable via `adl2::` — they are
// the return / error types of `validate_adl2_source`, not named by callers.
pub(crate) use adl2::{check_specialisation_depth, validate_adl2_source};
pub(crate) use opt::validate_opt_artefact;
pub(crate) use structure::validate_opt_structure;
