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
//!   package.adoc` §Valid_value) runs at commit time over the compacted
//!   WebTemplate. It lives in `openehr-flat` (a spec crate) and is invoked from
//!   the EHR commit choke point (`service::ehr::composition_validate`). W-3f
//!   only re-expresses its closed-world PORT NOTE as a spec citation — see the
//!   `TODO(w3f-integrate)` below.
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
//! # Integration seams — `TODO(w3f-integrate)` (reconciled at the fix pass)
//!
//! - **Callers to re-point.** The A entry points move here from `service/`:
//!   `service::template` (POST template) calls [`opt::validate_opt_artefact`]
//!   + [`structure::validate_opt_structure`];
//!   `service::definition::adl2` calls [`adl2::validate_adl2_source`] +
//!   [`adl2::check_specialisation_depth`];
//!   `service::definition::adl14` / `service::definition` call
//!   [`structure::validate_opt_structure`]. The old
//!   `service::{opt_validation,adl2_validation}` modules and
//!   `service::template::validate_opt_structure` are removed once callers point
//!   here (declaring `mod validation;` before that risks a `dead_code` deny —
//!   wire and delete atomically).
//! - **TODO(w3f-integrate): surface B closed-world citation (G-09-05).** The
//!   `openehr-flat::validation` closed-world PORT NOTE currently cites
//!   `ADR-012`, which violates the owner rule "cite spec not ADR". It must be
//!   re-expressed (that file is outside this crate's ownership) as: AOM 1.4
//!   defines only the positive `valid_value` cascade (`AOM1.4/master04-
//!   constraint_model_package.adoc` §Valid_value, which is silent on
//!   present-but-unmatched nodes); closure — reject unmatched *archetyped*
//!   siblings, tolerate RM-permitted metadata, tolerate unlisted
//!   archetype-rooted fillers under slotless attributes — follows the AOM2
//!   `c_conforms_to` / VSONCT/VSONCO formalization (`AOM2/master08-validation.
//!   adoc` §Phase 2, lines 96–101). ADR-012 stays decision-history only.
//! - **TODO(w3f-integrate): surface C dispatch.** `service::ehr`'s
//!   `validate_for_commit` should route the per-kind (`EHR_STATUS`/FOLDER/
//!   party) validators so no commit path bypasses validation (F-07-01).
//! - **TODO(w3f-integrate): terminology binding resolution (T17/T15).**
//!   VTTBK/VTCBK here check binding *keys*; resolving ac-code value sets
//!   against the live terminology service (`TerminologyService`) at ingestion
//!   is unwired — land it with the CONSTRAINT_REF policy (blueprint 03-am §3).

pub mod adl2;
pub mod opt;
pub mod structure;

// The flat entry surface callers wire to (`crate::validation::<fn>`). The
// artefact-metadata / violation types stay reachable via `adl2::` — they are
// the return / error types of `validate_adl2_source`, not named by callers.
pub(crate) use adl2::{check_specialisation_depth, validate_adl2_source};
pub(crate) use opt::validate_opt_artefact;
pub(crate) use structure::validate_opt_structure;
