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

// TODO(port): `DV_IDENTIFIER` is RM 1.1.0 `data_types.basic`, transcribed
// by a sibling agent in this same phase but not yet landed in this
// worktree. Forward-reference to its eventual module path.
use crate::data_types::basic::dv_identifier::DvIdentifier;

use super::party_proxy::{PartyProxyApi, PartyProxyData};

/// Canonical `_type` discriminator string for this class in serialized
/// form. Per ADR-001 refinements ("serde derives wait until P4"), a
/// `const` stands in for `#[serde(rename = ...)]` until serde lands as a
/// dependency of this crate.
pub const TYPE_NAME: &str = "PARTY_IDENTIFIED";

/// `PARTY_IDENTIFIED` inherits `PARTY_PROXY` and adds `name` and
/// `identifiers`. Per ADR-001 §3, the inherited `external_ref` attribute
/// is carried via an embedded [`PartyProxyData`] field, with the two new
/// attributes declared directly on this struct.
///
/// This struct is itself embedded (rather than referenced) by
/// [`super::party_related::PartyRelated`] per that class's `Inherit` row —
/// see `party_related.rs`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartyIdentified {
    /// Embedded `PARTY_PROXY` state (`external_ref`).
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
    pub identifiers: Option<Vec<DvIdentifier>>,
}

impl PartyIdentified {
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

impl PartyProxyApi for PartyIdentified {
    fn external_ref(&self) -> Option<&PartyRef> {
        self.party_proxy.external_ref.as_ref()
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.generic — docs/research/spec-cache/RM-1.1.0/uml_classes/party_identified.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master04-generic_package.adoc §Referring to Demographic Entities / uml_classes/party_identified.adoc §PARTY_IDENTIFIED Class
//   confidence: high
//   todos: 3
//   note: Three invariants (Basic_validity, Name_valid, Identifiers_valid) recorded as boolean-check methods but not yet Validate-enforced. Forward-refs DvIdentifier (data_types.basic, sibling-agent territory, not yet landed). Embedded (not referenced) by PartyRelated per its Inherit row.
// ─────────────────────────────────────────────
