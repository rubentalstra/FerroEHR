//! `PARTY_IDENTIFIED` — proxy data for an identified party other than the
//! subject of the record.
//!
//! openEHR class: `PARTY_IDENTIFIED` (concrete), package `common.generic`.
//! Inherits: `PARTY_PROXY`.
//!
//! Proxy data for an identified party other than the subject of the
//! record, minimally consisting of human-readable identifier(s), such as
//! name, formal (and possibly computable) identifiers such as NHS number,
//! and an optional link to external data. There must be at least one of
//! name, identifier or external_ref present.
//!
//! Used to describe parties where only identifiers may be known, and there
//! is no entry at all in the demographic system (or even no demographic
//! system). Typically for health care providers, e.g. name and provider
//! number of an institution.
//!
//! Should not be used to include patient identifying information.
use openehr_base::identification::party_ref::PartyRef;
use serde::{Deserialize, Serialize};

// TODO(port): `DV_IDENTIFIER` is RM 1.1.0 `data_types.basic`, transcribed
// by a sibling agent in this same phase but not yet landed in this
// worktree. Forward-reference to its eventual module path.
use crate::data_types::basic::dv_identifier::DvIdentifier;
use openehr_foundation::serde_support::{TypeName, TypeTag};

use super::party_proxy::{PartyProxyApi, PartyProxyData};

/// Canonical `_type` discriminator string for this class in serialized
/// form. Single-sources the [`TypeName`] impl below (ADR-002).
pub const TYPE_NAME: &str = "PARTY_IDENTIFIED";

/// Shared attribute state of `PARTY_IDENTIFIED` and its descendant
/// `PARTY_RELATED`.
///
/// PORT NOTE (ADR-002 restructure): `PARTY_IDENTIFIED` is a *concrete*
/// class that is also inherited by `PARTY_RELATED`. Under ADR-002 the
/// concrete [`PartyIdentified`] struct self-tags with a `TypeTag`; if
/// `PartyRelated` still `#[serde(flatten)]`ed the full self-tagged
/// `PartyIdentified` struct (as it did pre-P4), serializing a
/// `PARTY_RELATED` would emit a second, wrong `_type: "PARTY_IDENTIFIED"`
/// key from the embedded parent. The shared attribute set is therefore
/// split into this untagged `*Data` struct (per ADR-002 §3, embedded
/// `*Data` structs carry no tag): [`PartyIdentified`] wraps it with its
/// own tag, and [`super::party_related::PartyRelated`] flattens it
/// directly, so exactly one `_type` appears on the wire for either class.
// PartialOrd/Ord dropped from the derive set: `identifiers` carries
// `DV_IDENTIFIER`, which derives no ordering (the spec defines none).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PartyIdentifiedData {
    /// Embedded `PARTY_PROXY` state (`external_ref`).
    #[serde(flatten)]
    pub party_proxy: PartyProxyData,

    /// `name`: `String`, cardinality `0..1`.
    ///
    /// Optional human-readable name (in String form).
    ///
    /// Invariant `Name_valid`: `name /= Void implies not name.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl; recorded here as a doc note pending the RM invariant
    /// framework (`.claude/rules/rm-transcription.md` "Invariants").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// `identifiers`: `List<DV_IDENTIFIER>`, cardinality `0..1`.
    ///
    /// One or more formal identifiers (possibly computable).
    ///
    /// Invariant `Identifiers_valid`: `identifiers /= Void implies not
    /// identifiers.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifiers: Option<Vec<DvIdentifier>>,
}

/// `PARTY_IDENTIFIED` inherits `PARTY_PROXY` and adds `name` and
/// `identifiers`. Per ADR-001 §3 the inherited `external_ref` attribute is
/// carried via the embedded [`PartyProxyData`] inside
/// [`PartyIdentifiedData`]; per ADR-002 this concrete struct carries the
/// `_type` tag while the shared field set lives in the flattened, untagged
/// `*Data` struct (see that struct's PORT NOTE for why the split exists).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PartyIdentified {
    /// Canonical `_type` discriminator (`"PARTY_IDENTIFIED"`), always
    /// serialized first; tolerated-absent and validated-if-present on
    /// input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// The class's full attribute set (`external_ref`, `name`,
    /// `identifiers`), shared with `PARTY_RELATED` — see
    /// [`PartyIdentifiedData`].
    #[serde(flatten)]
    pub data: PartyIdentifiedData,
}

impl TypeName for PartyIdentified {
    const NAME: &'static str = TYPE_NAME;
}

impl PartyIdentifiedData {
    /// Invariant `Basic_validity`: `name /= Void or identifiers /= Void or
    /// external_ref /= Void`.
    ///
    /// At least one of a human-readable name, a formal identifier list, or
    /// an external-system reference must be present.
    ///
    /// TODO(port): not yet wired into a constructor or the RM `Validate`
    /// framework; this method lets a future `Validate` impl call the check
    /// directly once that framework lands.
    pub fn is_basic_validity_satisfied(&self) -> bool {
        self.name.is_some() || self.identifiers.is_some() || self.party_proxy.external_ref.is_some()
    }

    /// Invariant `Name_valid`: `name /= Void implies not name.is_empty`.
    ///
    /// TODO(port): not yet wired into a constructor or the RM `Validate`
    /// framework.
    pub fn is_name_valid(&self) -> bool {
        match &self.name {
            Some(n) => !n.is_empty(),
            None => true,
        }
    }

    /// Invariant `Identifiers_valid`: `identifiers /= Void implies not
    /// identifiers.is_empty`.
    ///
    /// TODO(port): not yet wired into a constructor or the RM `Validate`
    /// framework.
    pub fn are_identifiers_valid(&self) -> bool {
        match &self.identifiers {
            Some(ids) => !ids.is_empty(),
            None => true,
        }
    }
}

impl PartyIdentified {
    /// Invariant `Basic_validity` — delegates to
    /// [`PartyIdentifiedData::is_basic_validity_satisfied`].
    pub fn is_basic_validity_satisfied(&self) -> bool {
        self.data.is_basic_validity_satisfied()
    }

    /// Invariant `Name_valid` — delegates to
    /// [`PartyIdentifiedData::is_name_valid`].
    pub fn is_name_valid(&self) -> bool {
        self.data.is_name_valid()
    }

    /// Invariant `Identifiers_valid` — delegates to
    /// [`PartyIdentifiedData::are_identifiers_valid`].
    pub fn are_identifiers_valid(&self) -> bool {
        self.data.are_identifiers_valid()
    }
}

impl PartyProxyApi for PartyIdentifiedData {
    fn external_ref(&self) -> Option<&PartyRef> {
        self.party_proxy.external_ref.as_ref()
    }
}

impl PartyProxyApi for PartyIdentified {
    fn external_ref(&self) -> Option<&PartyRef> {
        self.data.party_proxy.external_ref.as_ref()
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.generic — docs/research/spec-cache/RM-1.1.0/uml_classes/party_identified.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master04-generic_package.adoc §Referring to Demographic Entities / uml_classes/party_identified.adoc §PARTY_IDENTIFIED Class
//   confidence: high
//   todos: 3
//   note: Three invariants (Basic_validity, Name_valid, Identifiers_valid) recorded as boolean-check methods but not yet Validate-enforced. Forward-refs DvIdentifier (data_types.basic, sibling-agent territory, not yet landed). P4/ADR-002: split into untagged PartyIdentifiedData (shared with PartyRelated, which flattens it) + self-tagged PartyIdentified wrapper (TypeName + first-field TypeTag<Self>) so the parent's _type never leaks into PARTY_RELATED output.
// ─────────────────────────────────────────────
