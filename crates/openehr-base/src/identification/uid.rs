//! `UID` — abstract parent of unique identifier classes.
//!
//! openEHR class: `UID` (abstract), package `base.base_types.identification`.
//!
//! Abstract parent of classes representing unique identifiers which
//! identify information entities in a durable way. UIDs only ever identify
//! one IE in time or space and are never re-used.
use super::internet_id::InternetId;
use super::iso_oid::IsoOid;
use super::uuid::Uuid;
use openehr_foundation::serde_support::TypeTag;
use std::str::FromStr;

/// Shared attribute state of `UID` and its descendants.
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct + marker
/// trait), every concrete `UID` subtype (`IsoOid`, `Uuid`, `InternetId`)
/// embeds this struct rather than inheriting from it, since Rust has no
/// class inheritance. None of the three concrete subtypes adds any
/// attribute or function of its own beyond what `UID` declares, so each
/// concrete file wraps `UidData` directly.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct UidData {
    /// `value`: the value of the id.
    ///
    /// Invariant `Value_valid`: `not value.empty`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl; recorded here as a doc note pending the RM invariant framework
    /// (`.claude/rules/rm-transcription.md` "Invariants").
    pub value: String,
}

/// `UID` is abstract in the spec and is used polymorphically wherever an
/// attribute is declared of type `UID` (e.g. `UID_BASED_ID.root()`,
/// `OBJECT_VERSION_ID.object_id()`/`creating_system_id()`). Per ADR-001 §4
/// (closed subtype set → enum), the three concrete subtypes `ISO_OID`,
/// `UUID`, and `INTERNET_ID` are collected into this closed `enum` so a
/// field or return type can be declared `Uid` exactly where the spec
/// declares it `UID`.
///
/// The spec notes (BASE 1.2.0 identification package, "Primitive
/// Identifiers") that the three subtypes have "mutually exclusive string
/// patterns" and so can always be distinguished by inspecting the string
/// form alone — justifying the closed, exhaustively-matchable enum shape
/// used here rather than a trait object.
///
/// PORT NOTE: `#[serde(untagged)]` per ADR-002 — the `_type` discriminator
/// is not emitted by this enum but by each variant payload's own
/// self-tagging `TypeTag` field (`IsoOid`/`Uuid`/`InternetId` each carry
/// `#[serde(rename = "_type")] type_tag`), so serialization still yields
/// the canonical `{"_type": "<NAME>", "value": "..."}` UID shape, and
/// deserialization dispatch is tag-driven: a payload's `TypeTag` fails on
/// a mismatched `_type` string, so untagged variant probing selects exactly
/// the variant whose class name matches. The three payloads are otherwise
/// structure-identical (`{value}`), so input *missing* `_type` (invalid in
/// an abstract `UID` slot per ITS-JSON) falls back to the first declared
/// variant.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(untagged)]
pub enum Uid {
    /// `ISO_OID`.
    IsoOid(IsoOid),
    /// `UUID`.
    ///
    /// PORT NOTE: named `Uuid` (PascalCase of the spec's `UUID`), which is a
    /// distinct type from the `uuid` crate's `Uuid` — see the doc comment on
    /// `uuid::Uuid` in `uuid.rs` for the disambiguation. No external `uuid`
    /// crate dependency is introduced by this transcription.
    Uuid(Uuid),
    /// `INTERNET_ID`.
    InternetId(InternetId),
}

/// Marker/accessor trait shared by every `UID` descendant, exposing the
/// abstract class's sole attribute uniformly whether the caller holds a
/// concrete type or a `Uid` enum value.
pub trait UidApi {
    /// `value`: the value of the id.
    fn value(&self) -> &str;
}

impl UidApi for Uid {
    fn value(&self) -> &str {
        match self {
            Uid::IsoOid(v) => v.value(),
            Uid::Uuid(v) => v.value(),
            Uid::InternetId(v) => v.value(),
        }
    }
}

/// Error returned when a raw string does not match any BASE `UID`
/// concrete lexical form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseUidError {
    value: String,
}

impl ParseUidError {
    /// Raw value that failed the BASE `uid = iso_oid | uuid | internet_id`
    /// grammar.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl std::fmt::Display for ParseUidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid openEHR UID value {:?}", self.value)
    }
}

impl std::error::Error for ParseUidError {}

impl FromStr for Uid {
    type Err = ParseUidError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Uid::parse_value(value)
    }
}

impl Uid {
    /// Parse a string value into the concrete UID subtype selected by the
    /// BASE identification grammar.
    ///
    /// BASE 1.2.0 states that `ISO_OID`, `UUID`, and `INTERNET_ID` have
    /// mutually exclusive string patterns, so this classification is
    /// deterministic for conforming values.
    pub fn parse_value(value: &str) -> Result<Self, ParseUidError> {
        parse_uid_value(value).ok_or_else(|| ParseUidError {
            value: value.to_string(),
        })
    }

    /// `true` when `value` matches one of the concrete BASE `UID`
    /// grammars.
    #[must_use]
    pub fn is_valid_value(value: &str) -> bool {
        parse_uid_value(value).is_some()
    }
}

pub(crate) fn parse_uid_value(value: &str) -> Option<Uid> {
    if value.is_empty() {
        return None;
    }
    if is_uuid_value(value) {
        return Some(Uid::Uuid(Uuid {
            type_tag: TypeTag::new(),
            uid: UidData {
                value: value.to_string(),
            },
        }));
    }
    if is_iso_oid_value(value) {
        return Some(Uid::IsoOid(IsoOid {
            type_tag: TypeTag::new(),
            uid: UidData {
                value: value.to_string(),
            },
        }));
    }
    if is_internet_id_value(value) {
        return Some(Uid::InternetId(InternetId {
            type_tag: TypeTag::new(),
            uid: UidData {
                value: value.to_string(),
            },
        }));
    }
    None
}

pub(crate) fn uid_from_value_or_unvalidated_internet_id(value: &str) -> Uid {
    parse_uid_value(value).unwrap_or_else(|| {
        // Valid UID-bearing openEHR objects cannot reach this path. Public
        // raw-string structs are still constructible before the Validate
        // framework lands, so preserve the original bytes for those
        // invariant-violating intermediate values.
        Uid::InternetId(InternetId {
            type_tag: TypeTag::new(),
            uid: UidData {
                value: value.to_string(),
            },
        })
    })
}

fn is_uuid_value(value: &str) -> bool {
    let mut groups = value.split('-');
    let expected_lengths = [8, 4, 4, 4, 12];
    for expected_len in expected_lengths {
        let Some(group) = groups.next() else {
            return false;
        };
        if group.len() != expected_len || !group.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return false;
        }
    }
    groups.next().is_none() && ::uuid::Uuid::parse_str(value).is_ok()
}

fn is_iso_oid_value(value: &str) -> bool {
    let mut parts = value.split('.');
    let Some(first) = parts.next() else {
        return false;
    };
    is_number(first) && parts.all(is_number)
}

fn is_internet_id_value(value: &str) -> bool {
    !value.is_empty() && value.split('.').all(is_internet_label)
}

fn is_internet_label(value: &str) -> bool {
    let mut chars = value.chars().peekable();
    let Some(first) = chars.next() else {
        return false;
    };
    if chars.peek().is_none() {
        return is_ascii_alphanum(first);
    }

    if !first.is_ascii_alphabetic() {
        return false;
    }

    let mut last = first;
    for ch in chars {
        if !is_ascii_alphanum(ch) && ch != '_' && ch != '-' {
            return false;
        }
        last = ch;
    }
    is_ascii_alphanum(last)
}

fn is_number(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_ascii_alphanum(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uid_from_value_uses_official_mutually_exclusive_patterns() {
        assert!(matches!(
            Uid::parse_value("1.2.840.10008"),
            Ok(Uid::IsoOid(_))
        ));
        assert!(matches!(
            Uid::parse_value("87284370-2D4B-4e3d-A3F3-F303D2F4F34B"),
            Ok(Uid::Uuid(_))
        ));
        assert!(matches!(
            Uid::parse_value("uk.nhs.ehr1"),
            Ok(Uid::InternetId(_))
        ));
        assert!(Uid::parse_value("not a uid").is_err());
    }

    #[test]
    fn uid_parser_keeps_to_the_base_lexical_forms() {
        assert!(matches!(Uid::parse_value("123"), Ok(Uid::IsoOid(_))));
        assert!(Uid::parse_value("872843702D4B4e3dA3F3F303D2F4F34B").is_err());
        assert!(Uid::parse_value("3com.example").is_err());
        assert!(matches!(Uid::parse_value("x.9"), Ok(Uid::InternetId(_))));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §UID — docs/research/spec-cache/BASE-1.2.0/uml_classes/uid.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / uid.adoc §UID Class
//   confidence: high
//   todos: 1
//   note: Value_valid invariant (not value.empty) recorded but not yet enforced; awaits the RM Validate-trait framework. UID string classification is implemented from the official BASE grammar's mutually-exclusive ISO_OID/UUID/INTERNET_ID patterns. P4/ADR-002: Uid enum is #[serde(untagged)], _type dispatch comes from each concrete payload's TypeTag; UidData stays untagged (embedded abstract-parent state).
// ─────────────────────────────────────────────
