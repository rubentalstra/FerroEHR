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
use openehr_foundation::serde_support::{TypeName, TypeTag};
use openehr_terminology::{
    OpenehrTerminologyGroupIdentifiers, TerminologyAccess, TerminologyCode, TerminologyService,
};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class, single-sourced
/// into its [`TypeName`] impl (ADR-002).
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
// PORT NOTE: schema verification per the invoking task's instruction:
// `openehr_rm_1.1.0_all.json` (ITS-JSON @ 5acae05), `#/definitions/
// TERM_MAPPING/properties/match`, types this field `{"type": "string"}` — a
// JSON string (the schema states no length constraint, but the spec's own
// value domain is the four single characters `>`/`=`/`</?`), not a JSON
// number — so the `#[serde(into = "i32", try_from = "i32")]` pattern used
// for `ProportionKind` below does not apply here. Serialized instead as its
// single-character `String` form via `#[serde(into = "String", try_from =
// "String")]`, delegating to the existing `as_char()`/`TryFrom<char>` pair.
// `TryFrom<String>` below returns `String` (the whole rejected input) as
// `Self::Error` rather than `char`, since a non-single-character string
// cannot be narrowed to a `char` first without an extra fallible step of
// its own.
#[serde(into = "String", try_from = "String")]
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

/// `#[serde(into = "String")]` conversion — canonical-JSON serialization
/// target. Delegates to [`MatchKind::as_char`].
impl From<MatchKind> for String {
    fn from(kind: MatchKind) -> Self {
        kind.as_char().to_string()
    }
}

/// `#[serde(try_from = "String")]` conversion — canonical-JSON
/// deserialization source. Rejects any string that is not exactly one of
/// the four legal single-character match codes; the rejected `String` is
/// returned as-is in `Self::Error` (see the enum-level PORT NOTE for why
/// this is a `String`, not a `char`).
impl TryFrom<String> for MatchKind {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let mut chars = s.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => MatchKind::try_from(c).map_err(|_| s),
            _ => Err(s),
        }
    }
}

/// `TERM_MAPPING` has three attributes and no ancestor state to embed.
///
/// PORT NOTE (ADR-002): self-tags via the `type_tag` first field;
/// [`MatchKind`] itself carries **no** `_type` — it is a closed *value*
/// domain (a spec `char`), not an RM class, and serializes as its bare
/// one-character JSON string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TermMapping {
    /// Canonical `_type` discriminator (`"TERM_MAPPING"`), always
    /// serialized first (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// `match`: `char` (`1..1`), modelled as [`MatchKind`] — see the
    /// enum's own doc comment.
    ///
    /// PORT NOTE: field named `match_` (spec attribute name `match` is a
    /// Rust reserved keyword), now carrying `#[serde(rename = "match")]`.
    #[serde(rename = "match")]
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
    /// Enforced by [`TermMapping::invariant_purpose_valid`], which takes a
    /// `&TerminologyService` (ADR-003 decision 8).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<DvCodedText>,

    /// `target`: `CODE_PHRASE` (`1..1`).
    ///
    /// The target term of the mapping.
    pub target: CodePhrase,
}

impl TypeName for TermMapping {
    const NAME: &'static str = TYPE_NAME;
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
    /// Per ADR-003 decision 8, invariants that need terminology take a
    /// `&TerminologyService`. Trivially `true` when `purpose` is `Void`
    /// (`None`); otherwise the purpose's `defining_code` must be a code under
    /// the openEHR "term mapping purpose" grouper.
    ///
    /// PORT NOTE: `has_code_for_group_id` matches on the `code_string` alone
    /// (see `openehr_terminology::BundledTerminologyAccess`), so the
    /// [`TerminologyCode`] carrier is built with the openEHR terminology id
    /// and the purpose's own code string.
    pub fn invariant_purpose_valid(&self, terminology: &TerminologyService) -> bool {
        match &self.purpose {
            None => true,
            Some(purpose) => {
                let code = TerminologyCode::new(
                    OpenehrTerminologyGroupIdentifiers::TERMINOLOGY_ID_OPENEHR,
                    purpose.defining_code.code_string.clone(),
                );
                terminology
                    .terminology(OpenehrTerminologyGroupIdentifiers::TERMINOLOGY_ID_OPENEHR)
                    .is_some_and(|openehr| {
                        openehr.has_code_for_group_id(
                            OpenehrTerminologyGroupIdentifiers::GROUP_ID_TERM_MAPPING_PURPOSE,
                            &code,
                        )
                    })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `CODE_PHRASE` built from JSON so the test does not hard-code the
    /// `TerminologyId` field shape; a missing `_type` is tolerated on
    /// concrete slots per ADR-002.
    fn code_phrase(code: &str) -> CodePhrase {
        serde_json::from_value(serde_json::json!({
            "terminology_id": { "value": "openehr" },
            "code_string": code,
        }))
        .unwrap()
    }

    fn coded_text(code: &str) -> DvCodedText {
        serde_json::from_value(serde_json::json!({
            "value": "purpose",
            "defining_code": { "terminology_id": { "value": "openehr" }, "code_string": code },
        }))
        .unwrap()
    }

    fn mapping(match_: MatchKind, purpose: Option<DvCodedText>) -> TermMapping {
        TermMapping {
            type_tag: TypeTag::new(),
            match_,
            purpose,
            target: code_phrase("target"),
        }
    }

    /// The four match predicates each read the closed `MatchKind`.
    #[test]
    fn match_predicates_follow_the_match_kind() {
        assert!(mapping(MatchKind::Narrower, None).narrower());
        assert!(mapping(MatchKind::Broader, None).broader());
        assert!(mapping(MatchKind::Equivalent, None).equivalent());
        assert!(mapping(MatchKind::Unknown, None).unknown());
        assert!(!mapping(MatchKind::Equivalent, None).narrower());
        // Match_valid is structurally guaranteed by the closed enum.
        assert!(mapping(MatchKind::Equivalent, None).invariant_match_valid());
    }

    /// `is_valid_match_code` accepts exactly the four legal characters.
    #[test]
    fn is_valid_match_code_accepts_only_the_four_legal_chars() {
        for c in ['>', '=', '<', '?'] {
            assert!(MatchKind::is_valid_match_code(c));
        }
        for c in ['!', 'x', ' ', '≈'] {
            assert!(!MatchKind::is_valid_match_code(c));
        }
    }

    /// `Purpose_valid` checks membership in the openEHR "term mapping
    /// purpose" grouper: `Void` is trivially valid, 669 ("public health") is
    /// in the group, a valid-but-off-group code (249 = "creation") fails.
    #[test]
    fn purpose_valid_checks_the_term_mapping_purpose_group() {
        let terminology = TerminologyService::bundled().expect("bundled terminology");
        assert!(mapping(MatchKind::Equivalent, None).invariant_purpose_valid(terminology));
        assert!(
            mapping(MatchKind::Equivalent, Some(coded_text("669")))
                .invariant_purpose_valid(terminology)
        );
        assert!(
            !mapping(MatchKind::Equivalent, Some(coded_text("249")))
                .invariant_purpose_valid(terminology)
        );
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.text — docs/research/spec-cache/RM-1.1.0/uml_classes/term_mapping.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-text_package.adoc §Class Descriptions / term_mapping.adoc §TERM_MAPPING Class
//   confidence: high
//   todos: 0
//   note: `match` char narrowed to a closed MatchKind enum (four legal values, is_valid_match_code recast as TryFrom/matches!); match_ field renamed for the Rust keyword collision. Purpose_valid now implemented per ADR-003 decision 8: takes a &TerminologyService and checks the purpose's defining_code against the openEHR "term mapping purpose" grouper (in-file test pins Void/in-group/off-group). P4/ADR-002: TermMapping self-tags via TypeTag<Self> first field + TypeName ("TERM_MAPPING"); MatchKind carries no _type and keeps its one-character-JSON-string wire form via #[serde(into/try_from = "String")] bridging as_char()/TryFrom<char>; match_ carries #[serde(rename = "match")]; purpose skips when None.
// ─────────────────────────────────────────────
