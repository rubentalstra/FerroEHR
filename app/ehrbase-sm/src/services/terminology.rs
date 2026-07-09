//! The SM `I_TERMINOLOGY_SERVICE` interface + its extract model.
//!
//! Spec (`docs/specs/openehr/SM/docs/openehr_platform/
//! master12-terminology_service.adoc` and the UML classes it includes):
//!
//! - `i_terminology_service.adoc` — the 9 calls (`get_terminology_ids`,
//!   `has_terminology`, `get_terminology_description`, `has_term`, `get_term`,
//!   `subsumes`, `value_set_validate`, `has_value_set`, `get_value_set`) and
//!   their preconditions (`Pre_has_terminology`, `Pre_has_term`,
//!   `Pre_has_value_set`).
//! - `terminology_description.adoc`, `terminology_extract.adoc`,
//!   `term_code.adoc`, `defined_term.adoc`, `term_relationship.adoc`,
//!   `terminology_relation.adoc` — the extract data model.
//!
//! PORT NOTE (temporal): the SM `at_date` parameter (an `Iso8601_date`) selects
//! the terminology as it stood on a date. Our provider is the compile-time,
//! spec-pinned `openehr-term` bundle (a single version — TERM 3.1.0), so
//! `at_date` is accepted and validated in shape by the caller but does not
//! change the answer; it is modelled here as `Option<String>` (the ISO date
//! text) rather than a strong date type because the native API never
//! date-resolves against multiple versions.

use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{CallStatusType, SmError};

// ─── extract data model (`terminology_extract` package) ──────────────────────

/// `Terminology_description` (`terminology_description.adoc`): "Descriptor for a
/// terminology as it is known in a particular terminology service."
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminologyDescription {
    /// `publisher` (1..1) — publisher organisation name.
    pub publisher: String,
    /// `available_versions` (0..1) — identifiers of available versions of this
    /// terminology in this service.
    pub available_versions: Option<Vec<String>>,
    /// `attributes` (0..1) — meta-model attributes that may be requested within
    /// extract requests.
    pub attributes: Option<Vec<String>>,
    /// `uri` (1..1) — published and/or standardised identifying URI for the
    /// terminology.
    pub uri: String,
}

/// `Term_code` (`term_code.adoc`): "Pure terminology concept within the scope of
/// the terminology of the owning extract."
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermCode {
    /// `code` (1..1) — a terminology code or post-coordinated code expression.
    pub code: String,
}

/// `Defined_term` (`defined_term.adoc`): "Fully defined term within the scope of
/// the terminology of the owning extract." Inherits `Term_code` (the `code`
/// attribute is flattened in).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinedTerm {
    /// `Term_code.code` (inherited, 1..1).
    pub code: String,
    /// `text` (1..1) — text of term.
    pub text: String,
    /// `language` (0..1) — code representing the language (ISO 639 / IETF RFC
    /// 5646). PORT NOTE: the SM types this as `Terminology_code`; we carry the
    /// bare code string (the native API resolves rubrics per language directly
    /// against the `openehr-term` bundle).
    pub language: Option<String>,
    /// `is_preferred_term` (0..1) — true if this term is the preferred term
    /// among alternatives, if supported within the scoping terminology.
    pub is_preferred_term: Option<bool>,
}

/// A `Terminology_extract._terms_` value: either a bare `Term_code` or a fully
/// defined `Defined_term`.
///
/// The SM types `_terms_` as `Hash<String, Term_code>` where "each [term] may be
/// a bare code, or have displayable text included, via the `Term` subtype
/// [`Defined_term`]" (`terminology_extract.adoc`). This closed enum is the
/// faithful Rust encoding of that `Term_code`/`Defined_term` subtype choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TermEntry {
    /// A fully defined term (`Defined_term`) — has displayable text.
    Defined(DefinedTerm),
    /// A bare code (`Term_code`).
    Bare(TermCode),
}

/// `Term_relationship` (`term_relationship.adoc`): "Term relationship,
/// represented as a 1:N code map in the scope of the terminology identified by
/// the owning extract."
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermRelationship {
    /// `origin_code` (1..1) — code of origin ('left-hand') concept.
    pub origin_code: String,
    /// `relation_name` (1..1) — name of the relation; must match a key in the
    /// owning `Terminology_extract._relations_`.
    pub relation_name: String,
    /// `target_codes` (0..1) — codes of target ('right-hand') concept(s).
    pub target_codes: Option<Vec<String>>,
}

/// `Terminology_relation` (`terminology_relation.adoc`): "Definition of a
/// relationship within the terminology meta-model."
///
/// Invariant `Inv_valid_definition`: `local_code /= Void xor external_code /=
/// Void` — enforced by the [`TerminologyRelation::new`] constructor (exactly one
/// of `local_code`/`external_code` is `Some`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminologyRelation {
    /// `name` (1..1) — name of this relation from relevant meta-model.
    pub name: String,
    /// `local_code` (0..1) — local code defining this relation.
    pub local_code: Option<String>,
    /// `external_code` (0..1) — code from another terminology that defines a
    /// relation used by this terminology. PORT NOTE: the SM types this as
    /// `Terminology_code`; carried here as the bare code string.
    pub external_code: Option<String>,
}

/// The `Terminology_relation` invariant violation (`Inv_valid_definition`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TerminologyRelationError {
    /// Neither `local_code` nor `external_code` was supplied, or both were —
    /// the `local_code /= Void xor external_code /= Void` invariant.
    #[error(
        "Terminology_relation.Inv_valid_definition: exactly one of local_code/external_code must be set"
    )]
    XorViolation,
}

impl TerminologyRelation {
    /// Construct a `Terminology_relation`, enforcing `Inv_valid_definition`
    /// (`local_code /= Void xor external_code /= Void`).
    ///
    /// # Errors
    ///
    /// [`TerminologyRelationError::XorViolation`] if neither or both of
    /// `local_code`/`external_code` are supplied.
    pub fn new(
        name: impl Into<String>,
        local_code: Option<String>,
        external_code: Option<String>,
    ) -> Result<Self, TerminologyRelationError> {
        if local_code.is_some() == external_code.is_some() {
            return Err(TerminologyRelationError::XorViolation);
        }
        Ok(Self {
            name: name.into(),
            local_code,
            external_code,
        })
    }

    /// Construct a relation defined by a `local_code` (from this terminology).
    #[must_use]
    pub fn local(name: impl Into<String>, local_code: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            local_code: Some(local_code.into()),
            external_code: None,
        }
    }

    /// Construct a relation defined by an `external_code` (from another
    /// terminology).
    #[must_use]
    pub fn external(name: impl Into<String>, external_code: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            local_code: None,
            external_code: Some(external_code.into()),
        }
    }
}

/// `Terminology_extract` (`terminology_extract.adoc`): "Root object of a
/// collection of items extracted from a single version or release of one
/// terminology." May represent a flat value-set, a structured value-set, or a
/// subsumption hierarchy.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TerminologyExtract {
    /// `terminology_id` (1..1) — the namespace identifier of the terminology.
    pub terminology_id: String,
    /// `terminology_version` (0..1) — terminology version (date or dotted
    /// numeric).
    pub terminology_version: Option<String>,
    /// `terms` (0..1) — the terms in the extract, keyed by code; each is a bare
    /// code or a fully defined term ([`TermEntry`]).
    pub terms: Option<BTreeMap<String, TermEntry>>,
    /// `relationships` (0..1) — relationships according to the specification
    /// generating the extract.
    pub relationships: Option<Vec<TermRelationship>>,
    /// `relations` (0..1) — definitions of relations used in this extract, keyed
    /// by `_name_`.
    pub relations: Option<BTreeMap<String, TerminologyRelation>>,
}

impl TerminologyExtract {
    /// `create_terminology_code` (`terminology_extract.adoc`) — the standalone
    /// form of a terminology code within this extract's terminology: a
    /// [`TerminologyCode`](openehr_base::prelude::TerminologyCode) whose
    /// `terminology_id` is this extract's.
    #[must_use]
    pub fn create_terminology_code(
        &self,
        code: impl Into<String>,
    ) -> openehr_base::prelude::TerminologyCode {
        openehr_base::prelude::TerminologyCode {
            terminology_id: self.terminology_id.clone(),
            terminology_version: self.terminology_version.clone(),
            code_string: code.into(),
            uri: None,
        }
    }
}

// ─── the service interface (`i_terminology_service`) ─────────────────────────

/// The SM `I_TERMINOLOGY_SERVICE` interface (`i_terminology_service.adoc`):
/// "Terminology service interface."
///
/// Every call defaults to `NotImplemented` (`501`) so [`StubBackend`] and any
/// partial backend inherit it until a real provider overrides it. The concrete
/// provider ([`ehrbase`'s bundle-backed impl]) enforces the SM preconditions:
/// a failed `Pre_has_terminology`/`Pre_has_term`/`Pre_has_value_set` surfaces as
/// [`SmError::NotFound`] (the natural HTTP reading of the abstract
/// `*_does_not_exist` errors; the terminology surface has no ITS-REST wire
/// contract, so no CNF status is prescribed — SM-4 phase doc §Wire).
///
/// [`StubBackend`]: crate::backend::StubBackend
/// [`ehrbase`'s bundle-backed impl]: <the `ehrbase` service crate>
#[async_trait]
pub trait TerminologyService: Send + Sync {
    /// `get_terminology_ids` (0..1) — the identifiers of all terminologies known
    /// to this server; the identifiers usable in subsequent calls.
    async fn get_terminology_ids(&self) -> Result<Vec<String>, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }

    /// `has_terminology` (1..1) — True if this server has a terminology with the
    /// identifier `terminology_id`.
    async fn has_terminology(&self, _terminology_id: &str) -> Result<bool, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }

    /// `get_terminology_description` (1..1) — the description object for the
    /// terminology `terminology_id`. `Pre_has_terminology`.
    async fn get_terminology_description(
        &self,
        _terminology_id: &str,
    ) -> Result<TerminologyDescription, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }

    /// `has_term` (1..1) — True if `code` is known in terminology
    /// `terminology_id`. If `at_date` is specified, the code was present at that
    /// date. `Pre_has_terminology`.
    async fn has_term(
        &self,
        _terminology_id: &str,
        _code: &str,
        _at_date: Option<String>,
    ) -> Result<bool, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }

    /// `get_term` (1..1) — a term definition from a terminology, optionally
    /// filtered by `attributes` (a meta-model attribute allow-list). If
    /// `at_date` is specified, the definition as it was on that date.
    /// `Pre_has_terminology` + `Pre_has_term`.
    async fn get_term(
        &self,
        _terminology_id: &str,
        _code: &str,
        _attributes: Option<BTreeMap<String, String>>,
        _at_date: Option<String>,
    ) -> Result<TerminologyExtract, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }

    /// `subsumes` (1..1) — True if `candidate_child_code` is in the strict
    /// subsumption of `ref_code`. `Pre_has_terminology`.
    async fn subsumes(
        &self,
        _terminology_id: &str,
        _ref_code: &str,
        _candidate_child_code: &str,
    ) -> Result<bool, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }

    /// `value_set_validate` (1..1) — True if `candidate_code` is in the value set
    /// `value_set_id`. `Pre_has_terminology`.
    async fn value_set_validate(
        &self,
        _terminology_id: &str,
        _value_set_id: &str,
        _candidate_code: &str,
        _at_date: Option<String>,
    ) -> Result<bool, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }

    /// `has_value_set` (1..1) — True if this service has a value-set identified
    /// by `value_set_code`.
    async fn has_value_set(
        &self,
        _terminology_id: &str,
        _value_set_code: &str,
    ) -> Result<bool, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }

    /// `get_value_set` (1..1) — the value-set `value_set_code`.
    /// `Pre_has_terminology` + `Pre_has_value_set`.
    async fn get_value_set(
        &self,
        _terminology_id: &str,
        _value_set_code: &str,
    ) -> Result<TerminologyExtract, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminology_relation_xor_invariant() {
        // local only → ok.
        assert!(TerminologyRelation::new("is_a", Some("123".into()), None).is_ok());
        // external only → ok.
        assert!(TerminologyRelation::new("is_a", None, Some("SCT:116680003".into())).is_ok());
        // neither → Inv_valid_definition violation.
        assert_eq!(
            TerminologyRelation::new("is_a", None, None),
            Err(TerminologyRelationError::XorViolation)
        );
        // both → Inv_valid_definition violation.
        assert_eq!(
            TerminologyRelation::new("is_a", Some("123".into()), Some("SCT:1".into())),
            Err(TerminologyRelationError::XorViolation)
        );
        // the infallible smart constructors uphold the invariant structurally.
        assert_eq!(
            TerminologyRelation::local("is_a", "123")
                .local_code
                .as_deref(),
            Some("123")
        );
        assert_eq!(
            TerminologyRelation::external("is_a", "SCT:1")
                .external_code
                .as_deref(),
            Some("SCT:1")
        );
    }

    #[test]
    fn create_terminology_code_uses_extract_terminology_id() {
        let extract = TerminologyExtract {
            terminology_id: "openehr".to_owned(),
            ..Default::default()
        };
        let tc = extract.create_terminology_code("249");
        assert_eq!(tc.terminology_id, "openehr");
        assert_eq!(tc.code_string, "249");
    }
}
