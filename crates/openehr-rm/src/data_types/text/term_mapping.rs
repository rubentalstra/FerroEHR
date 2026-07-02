//! `TERM_MAPPING` — a coded term mapped to a `DV_TEXT`.
//!
//! openEHR class: `TERM_MAPPING`, package `rm.data_types.text`.
//!
//! Represents a coded term mapped to a `DV_TEXT`, and the relative match of
//! the target term with respect to the mapped item. Plain or coded text
//! items may appear in the EHR for which one or more mappings in
//! alternative terminologies are required. Mappings are only used to
//! enable computer processing, so they can only be instances of
//! `DV_CODED_TEXT`.
//!
//! Used for adding classification terms (e.g. adding ICD classifiers to
//! SNOMED descriptive terms), or mapping into equivalents in other
//! terminologies (e.g. across nursing vocabularies).
//!
//! PORT NOTE: `TERM_MAPPING` declares no `Inherit` row in its published
//! class table — transcribed as a standalone leaf struct, not a
//! `DATA_VALUE` subtype (it is never used as an `ELEMENT` value in its own
//! right, only as the element type of `DV_TEXT.mappings`).
use super::code_phrase::CodePhrase;
use super::dv_coded_text::DvCodedText;

/// Canonical `_type` discriminator string for this class in serialized
/// form (ADR-001 Refinements: serde derives wait until P4).
pub const TYPE_NAME: &str = "TERM_MAPPING";

/// The relative match of a `TERM_MAPPING`'s target term with respect to the
/// mapped text item.
///
/// PORT NOTE: the spec types `match` as a bare `char` with four legal
/// values enumerated in prose (`'>'`, `'='`, `'<'`, `'?'`) plus an
/// `is_valid_match_code` validity function. Modelled here as a closed Rust
/// `enum` rather than a raw `char` field, since the value space is closed
/// and enumerated (matching the "closed subtype set → enum" spirit of
/// ADR-001 §4, applied to a closed value domain rather than a class
/// hierarchy) — this makes `is_valid_match_code`'s check structural
/// (exhaustive `match` / `FromStr`) instead of a runtime string comparison,
/// and callers cannot construct an invalid match code at all. `as_char()`
/// recovers the spec's literal `char` representation for serialization
/// (P4) and for the `is_valid_match_code` free function kept alongside for
/// fidelity to the spec's own signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MatchKind {
    /// `'>'`: the mapping is to a broader term, e.g. original text =
    /// "arbovirus infection", target = "viral infection".
    Broader,
    /// `'='`: the mapping is to a (supposedly) equivalent to the original
    /// item.
    Equivalent,
    /// `'<'`: the mapping is to a narrower term, e.g. original text =
    /// "diabetes", mapping = "diabetes mellitus".
    Narrower,
    /// `'?'`: the kind of mapping is unknown.
    Unknown,
}

impl MatchKind {
    /// Recover the spec's literal `char` encoding of this match kind.
    pub fn as_char(self) -> char {
        match self {
            MatchKind::Broader => '>',
            MatchKind::Equivalent => '=',
            MatchKind::Narrower => '<',
            MatchKind::Unknown => '?',
        }
    }

    /// `is_valid_match_code(c: char) -> Boolean`.
    ///
    /// __Post__: `Result := c = '>' or c = '=' or c = '<' or c = '?'`.
    ///
    /// True if `c` is one of the four legal match characters. Kept as a
    /// free function (rather than only relying on `TryFrom` failing) to
    /// preserve the spec's own function signature for fidelity.
    pub fn is_valid_match_code(c: char) -> bool {
        matches!(c, '>' | '=' | '<' | '?')
    }
}

impl TryFrom<char> for MatchKind {
    type Error = char;

    fn try_from(c: char) -> Result<Self, Self::Error> {
        match c {
            '>' => Ok(MatchKind::Broader),
            '=' => Ok(MatchKind::Equivalent),
            '<' => Ok(MatchKind::Narrower),
            '?' => Ok(MatchKind::Unknown),
            other => Err(other),
        }
    }
}

/// `TERM_MAPPING` has three attributes and no ancestor state to embed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TermMapping {
    /// `match`: `char` (`1..1`), modelled as [`MatchKind`] — see the
    /// enum's own doc comment.
    ///
    /// PORT NOTE: field named `match_` (spec attribute name `match` is a
    /// Rust reserved keyword). A future serde derive (P4) should add
    /// `#[serde(rename = "match")]` here.
    pub match_: MatchKind,

    /// `purpose`: `DV_CODED_TEXT` (`0..1`).
    ///
    /// Purpose of the mapping e.g. `"automated data mining"`, `"billing"`,
    /// `"interoperability"`.
    ///
    /// Invariant `Purpose_valid`: `purpose /= Void implies
    /// terminology(Terminology_id_openehr).has_code_for_group_id(
    /// Group_id_term_mapping_purpose, purpose.defining_code)`.
    ///
    /// TODO(port): invariant requires a live terminology-service lookup
    /// (`openehr_terminology::TerminologyService`/`TerminologyAccess`); not
    /// yet enforced by a constructor/`Validate` impl.
    pub purpose: Option<DvCodedText>,

    /// `target`: `CODE_PHRASE` (`1..1`).
    ///
    /// The target term of the mapping.
    pub target: CodePhrase,
}

impl TermMapping {
    /// `narrower(): Boolean`.
    ///
    /// __Post__: `match = '<' implies Result`.
    ///
    /// The mapping is to a narrower term.
    pub fn narrower(&self) -> bool {
        matches!(self.match_, MatchKind::Narrower)
    }

    /// `broader(): Boolean`.
    ///
    /// __Post__: `match = '>' implies Result`.
    ///
    /// The mapping is to a broader term.
    pub fn broader(&self) -> bool {
        matches!(self.match_, MatchKind::Broader)
    }

    /// `equivalent(): Boolean`.
    ///
    /// The mapping is to an equivalent term.
    pub fn equivalent(&self) -> bool {
        matches!(self.match_, MatchKind::Equivalent)
    }

    /// `unknown(): Boolean`.
    ///
    /// __Post__: `match = '?' implies Result`.
    ///
    /// The kind of mapping is unknown.
    pub fn unknown(&self) -> bool {
        matches!(self.match_, MatchKind::Unknown)
    }

    /// `Match_valid`: `is_valid_match_code(match)`.
    ///
    /// PORT NOTE: structurally guaranteed by the `MatchKind` enum's closed
    /// value space — a `TermMapping` cannot hold an invalid match code, so
    /// this predicate always returns `true`. Kept for fidelity to the
    /// spec's invariant list and as a stable hook if a future phase widens
    /// `match_` back to a raw `char`.
    pub fn invariant_match_valid(&self) -> bool {
        true
    }

    /// `Purpose_valid`: `purpose /= Void implies terminology(
    /// Terminology_id_openehr).has_code_for_group_id(
    /// Group_id_term_mapping_purpose, purpose.defining_code)`.
    ///
    /// TODO(port): requires a live `TerminologyService`/`TerminologyAccess`
    /// lookup (`openehr_terminology`), not available at this layer without
    /// threading a service reference through; left unimplemented.
    pub fn invariant_purpose_valid(&self) -> bool {
        // TODO(port): call into openehr_terminology::TerminologyAccess::
        // has_code_for_group_id(Group_id_term_mapping_purpose,
        // purpose.defining_code) once a service handle is available here.
        todo!("Purpose_valid requires a live terminology service lookup")
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.text — docs/research/spec-cache/RM-1.1.0/uml_classes/term_mapping.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-text_package.adoc §Class Descriptions / term_mapping.adoc §TERM_MAPPING Class
//   confidence: medium
//   todos: 3
//   note: `match` char narrowed to a closed MatchKind enum (four legal values, is_valid_match_code recast as TryFrom/matches!); match_ field renamed for the Rust keyword collision; Purpose_valid invariant left as todo!() pending a terminology-service handle (mentioned on both the field doc and the invariant method, plus one inline comment, hence 3).
// ─────────────────────────────────────────────
