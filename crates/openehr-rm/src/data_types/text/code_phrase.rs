//! `CODE_PHRASE` — a fully coordinated term from a terminology service.
//!
//! openEHR class: `CODE_PHRASE`, package `rm.data_types.text`.
//!
//! A fully coordinated (i.e. all coordination has been performed) term from
//! a terminology service (as distinct from a particular terminology).
//!
//! PORT NOTE: `CODE_PHRASE` in this specification declares no `Inherit` row
//! in its published class table (unlike `DV_TEXT`, `DV_CODED_TEXT`, etc.,
//! which all inherit `DATA_VALUE`). Transcribed as a plain leaf struct with
//! no embedded parent state — this is a standalone class, not a
//! `DATA_VALUE` subtype, matching its use elsewhere in the RM (e.g.
//! `DV_CODED_TEXT.defining_code`, `DV_TEXT.language`/`encoding`,
//! `TERM_MAPPING.target`) as an attribute *type*, never as an `ELEMENT`
//! value in its own right.
use openehr_base::identification::terminology_id::TerminologyId;

/// Canonical `_type` discriminator string for this class in serialized
/// form (ADR-001 Refinements: serde derives wait until P4).
pub const TYPE_NAME: &str = "CODE_PHRASE";

/// `CODE_PHRASE` has three attributes and no ancestor state to embed, so it
/// is transcribed as a plain leaf struct.
///
/// # Relationship to `openehr_terminology::TerminologyCode`
///
/// PORT NOTE: `openehr-terminology`'s `TerminologyAccess`/`CodeSetAccess`
/// service trait signatures (P2, `crates/openehr-terminology/src/
/// terminology_code.rs`) return a local `TerminologyCode` struct — a
/// deliberate, documented stand-in for this very class, because
/// `openehr-terminology` sits *below* `openehr-rm` in the dependency graph
/// (Section 9: `openehr-rm` depends on `openehr-terminology`, never the
/// reverse) and therefore cannot reference `CODE_PHRASE` directly no matter
/// how it is transcribed here. See the row "`CODE_PHRASE` in service
/// signatures (pre-P3)" in `docs/ROSETTA.md` and the `TerminologyCode`
/// file's own `PORT NOTE`/trailer for the other half of this relationship.
///
/// There is also `openehr_foundation::terminology_types::terminology_code::
/// TerminologyCode` — the BASE 1.2.0 *foundation-types* `Terminology_code`
/// class, a distinct spec class from this one (four attributes:
/// `terminology_id`, `terminology_version`, `code_string`, `uri`, versus
/// `CODE_PHRASE`'s three: `terminology_id`, `code_string`,
/// `preferred_term`). The two are structurally similar (both pair a
/// terminology identifier with a code string) but are not the same
/// spec class and this transcription does not conflate them.
///
/// **Reconciliation of all three (`CODE_PHRASE`, `openehr_terminology::
/// TerminologyCode`, `openehr_foundation::terminology_types::
/// TerminologyCode`) is deferred to P17** (make-it-compile), per the
/// invoking task's explicit instruction — this doc comment records the
/// relationship now so a P17 reviewer does not have to re-derive it.
/// `TODO(port):` swap `openehr-terminology`'s service-signature stand-in
/// for this real `CodePhrase` type once the dependency direction question
/// (`openehr-terminology` would need to depend on `openehr-rm`, which
/// inverts the crate graph in Section 9 — likely resolved by moving the
/// service *traits* to a layer both can see, or by having
/// `openehr-terminology` stay code/string-only and letting `openehr-rm`
/// wrap its results into `CodePhrase` at the call site) is settled.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CodePhrase {
    /// `terminology_id`: `TERMINOLOGY_ID` (`1..1`).
    ///
    /// Identifier of the distinct terminology from which the code_string
    /// (or its elements) was extracted.
    pub terminology_id: TerminologyId,

    /// `code_string`: `String` (`1..1`).
    ///
    /// The key used by the terminology service to identify a concept or
    /// coordination of concepts. This string is most likely parsable inside
    /// the terminology service, but nothing can be assumed about its syntax
    /// outside that context.
    ///
    /// Invariant `Code_string_valid`: `not code_string.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl; recorded here as a doc note pending the RM invariant framework.
    pub code_string: String,

    /// `preferred_term`: `String` (`0..1`).
    ///
    /// Optional attribute to carry preferred term corresponding to the code
    /// or expression in `code_string`. Typical use in integration
    /// situations which create mappings, and representing data for which
    /// both a (non-preferred) actual term and a preferred term are both
    /// required.
    pub preferred_term: Option<String>,
}

impl CodePhrase {
    /// `Code_string_valid`: `not code_string.is_empty`.
    ///
    /// TODO(port): wire into a `Validate` impl once the RM invariant
    /// framework lands.
    pub fn invariant_code_string_valid(&self) -> bool {
        !self.code_string.is_empty()
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.text — docs/research/spec-cache/RM-1.1.0/uml_classes/code_phrase.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-text_package.adoc §Class Descriptions / code_phrase.adoc §CODE_PHRASE Class
//   confidence: high
//   todos: 3
//   note: no Inherit row published for this class (transcribed as a standalone leaf, not a DATA_VALUE subtype); terminology_id forward-references openehr_base::identification::terminology_id::TerminologyId (already transcribed, same-direction dependency); see the doc comment for the full CODE_PHRASE / openehr_terminology::TerminologyCode / openehr_foundation Terminology_code reconciliation deferred to P17 (the reconciliation note plus the Code_string_valid field/method doc pair account for the 3).
// ─────────────────────────────────────────────
