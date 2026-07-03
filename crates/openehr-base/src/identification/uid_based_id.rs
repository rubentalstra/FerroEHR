//! `UID_BASED_ID` — abstract model of UID-based identifiers.
//!
//! openEHR class: `UID_BASED_ID` (abstract), package
//! `base.base_types.identification`.
//! Inherits: `OBJECT_ID`.
//!
//! Abstract model of UID-based identifiers consisting of a root part and an
//! optional extension; lexical form: `root '::' extension`.
//!
//! Lexical form (Syntaxes, BASE 1.2.0 identification package):
//! `uid_based_id = root, [ '::', extension ] ; root = uid ; extension = ? any string ? ;`
use super::hier_object_id::HierObjectId;
use super::object_version_id::ObjectVersionId;
use super::uid::{Uid, uid_from_value_or_unvalidated_internet_id};

/// Shared attribute state of `UID_BASED_ID` and its descendants.
///
/// `UID_BASED_ID` adds no new attribute beyond the inherited `value: String`
/// from `OBJECT_ID` — see `object_id.rs::ObjectIdData` — but is transcribed
/// with its own copy here (rather than embedding `ObjectIdData` by
/// composition) because `UID_BASED_ID` is itself the layer that defines the
/// `root`/`extension`/`has_extension` parsing behaviour on top of that one
/// attribute; both `HIER_OBJECT_ID` and `OBJECT_VERSION_ID` embed this
/// struct so both automatically gain the parsing functions via
/// [`UidBasedIdApi`]'s default methods.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct UidBasedIdData {
    /// `value`: the value of the id, in the form `root [ '::' extension ]`.
    ///
    /// Invariant `Has_extension_valid`: `extension.is_empty xor
    /// has_extension` — see [`UidBasedIdApi::has_extension`]. Since
    /// `has_extension()` is *defined* as `not extension().is_empty()`, that
    /// invariant holds by construction for any value; what a constructor
    /// can usefully enforce is the lexical form itself (`root = uid` per
    /// the identification package's Syntaxes section), which
    /// [`UidBasedIdData::new`] does (ADR-003 decision 8). Struct-literal
    /// construction remains possible for unchecked wire data.
    pub value: String,
}

/// Error raised by [`UidBasedIdData::new`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UidBasedIdError {
    /// The value is empty (inherited `OBJECT_ID`/`UID` non-emptiness).
    #[error("UID_BASED_ID value must not be empty")]
    Empty,
    /// The `root` part (left of the first `::`) does not match any BASE
    /// `UID` concrete lexical form (`iso_oid | uuid | internet_id`).
    #[error("invalid UID_BASED_ID root {0:?}: must be an ISO_OID, UUID, or INTERNET_ID")]
    InvalidRoot(String),
}

impl UidBasedIdData {
    /// Fallible constructor enforcing the `root [ '::' extension ]`
    /// lexical form: the value is non-empty and its `root` part parses as
    /// one of the BASE `UID` concrete grammars.
    pub fn new(value: impl Into<String>) -> Result<Self, UidBasedIdError> {
        let value = value.into();
        if value.is_empty() {
            return Err(UidBasedIdError::Empty);
        }
        let root = uid_based_root_value(&value);
        if !Uid::is_valid_value(root) {
            return Err(UidBasedIdError::InvalidRoot(root.to_string()));
        }
        Ok(Self { value })
    }
}

/// `UID_BASED_ID` is abstract and used polymorphically wherever an
/// attribute is declared of that type (e.g. `LOCATABLE_REF.id`, a covariant
/// redefinition narrowing `OBJECT_ID` — see `locatable_ref.rs`). Per
/// ADR-001 §4, its two concrete descendants `HIER_OBJECT_ID` and
/// `OBJECT_VERSION_ID` are collected into this closed `enum`.
///
/// PORT NOTE: `#[serde(untagged)]` per ADR-002 — the `_type` discriminator
/// is not emitted by this enum but by each variant payload's own
/// self-tagging `TypeTag` field (`HierObjectId`/`ObjectVersionId` each
/// carry `#[serde(rename = "_type")] type_tag`), so serialization still
/// yields `{"_type": "<NAME>", "value": "..."}`, and deserialization
/// dispatch is tag-driven: a payload's `TypeTag` fails on a mismatched
/// `_type` string, so untagged variant probing selects exactly the variant
/// whose class name matches. The two payloads are otherwise
/// structure-identical (`{value}`), so input *missing* `_type` (invalid in
/// an abstract `UID_BASED_ID` slot per ITS-JSON) falls back to the first
/// declared variant.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(untagged)]
pub enum UidBasedId {
    /// `HIER_OBJECT_ID`.
    HierObjectId(HierObjectId),
    /// `OBJECT_VERSION_ID`.
    ObjectVersionId(ObjectVersionId),
}

/// Behaviour trait for `UID_BASED_ID` and its descendants, providing the
/// spec's `root()`/`extension()`/`has_extension()` functions as default
/// methods derived uniformly from the single `value: String` attribute —
/// implementors need only provide [`UidBasedIdApi::value`].
pub trait UidBasedIdApi {
    /// `value`: the raw `root [ '::' extension ]` string.
    fn value(&self) -> &str;

    /// `root(): UID`.
    ///
    /// The identifier of the conceptual namespace in which the object
    /// exists, within the identification scheme. Returns the part to the
    /// left of the first `::` separator, if any, or else the whole string.
    ///
    /// PORT NOTE: this mirrors the spec function for valid `UID_BASED_ID`
    /// values. For unchecked raw input, call [`UidBasedIdApi::try_root`]
    /// first to detect a root that does not match the BASE `UID` grammar.
    fn root(&self) -> Uid {
        uid_from_value_or_unvalidated_internet_id(uid_based_root_value(self.value()))
    }

    /// Fallible Rust entrypoint for the same `root` string when the
    /// instance may have been constructed from unchecked raw data.
    fn try_root(&self) -> Option<Uid> {
        uid_based_root_value(self.value()).parse().ok()
    }

    /// `extension(): String`.
    ///
    /// Optional local identifier of the object within the context of the
    /// root identifier. Returns the part to the right of the first `::`
    /// separator if any, or else an empty `String`.
    fn extension(&self) -> String {
        match self.value().split_once("::") {
            Some((_root, extension)) => extension.to_string(),
            None => String::new(),
        }
    }

    /// `has_extension(): Boolean`.
    ///
    /// True if not `extension().is_empty()`.
    fn has_extension(&self) -> bool {
        !self.extension().is_empty()
    }
}

impl UidBasedIdApi for UidBasedId {
    fn value(&self) -> &str {
        match self {
            UidBasedId::HierObjectId(v) => v.value(),
            UidBasedId::ObjectVersionId(v) => v.value(),
        }
    }
}

fn uid_based_root_value(value: &str) -> &str {
    value
        .split_once("::")
        .map_or(value, |(root, _extension)| root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openehr_foundation::serde_support::TypeTag;

    #[test]
    fn uid_based_id_root_and_extension_follow_base_grammar() {
        let id = HierObjectId {
            type_tag: TypeTag::new(),
            uid_based_id: UidBasedIdData {
                value: "uk.nhs.ehr1::local value".to_string(),
            },
        };

        assert!(matches!(id.root(), Uid::InternetId(_)));
        assert!(matches!(id.try_root(), Some(Uid::InternetId(_))));
        assert_eq!(id.extension(), "local value");
        assert!(id.has_extension());
    }

    #[test]
    fn uid_based_id_try_root_rejects_unchecked_invalid_root() {
        let id = HierObjectId {
            type_tag: TypeTag::new(),
            uid_based_id: UidBasedIdData {
                value: "not a uid::local value".to_string(),
            },
        };

        assert!(id.try_root().is_none());
    }

    #[test]
    fn uid_based_id_data_new_enforces_the_lexical_form() {
        // Bare root, UUID root with extension, OBJECT_VERSION_ID-shaped
        // three-part value — all valid (root parses as a UID).
        assert!(UidBasedIdData::new("uk.nhs.ehr1").is_ok());
        assert!(
            UidBasedIdData::new("8849182c-82ad-4088-a07f-48ead4180515::ehrbase.org::1").is_ok()
        );
        assert!(UidBasedIdData::new("1.2.840.10008::extension text").is_ok());

        assert_eq!(UidBasedIdData::new(""), Err(UidBasedIdError::Empty));
        assert_eq!(
            UidBasedIdData::new("not a uid::local value"),
            Err(UidBasedIdError::InvalidRoot("not a uid".to_string()))
        );
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §UID_BASED_ID — docs/research/spec-cache/BASE-1.2.0/uml_classes/uid_based_id.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / uid_based_id.adoc §UID_BASED_ID Class
//   confidence: high
//   todos: 0
//   note: root() now classifies the root string via the official BASE UID grammar (ISO_OID vs UUID vs INTERNET_ID); UidBasedIdData::new enforces the root-is-a-UID lexical form (ADR-003 §8), Has_extension_valid holds by construction. P4/ADR-002: UidBasedId enum is #[serde(untagged)], _type dispatch comes from each concrete payload's TypeTag; UidBasedIdData stays untagged (embedded abstract-parent state).
// ─────────────────────────────────────────────
