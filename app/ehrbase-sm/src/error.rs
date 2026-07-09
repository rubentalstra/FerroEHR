//! The SM call-status error model (`platform.common`).
//!
//! Mirrors `CALL_STATUS` / `CALL_STATUS_TYPE` and the service-specific
//! descendant enumerations of the openEHR Platform Service Model
//! (`docs/specs/openehr/SM/docs/UML/classes/{call_status,call_status_type,
//! ehr_call_status_type,definition_call_status_type}.adoc`;
//! `master03-common_package.adoc` §Representing Call Status).
//!
//! The SM's stateful `I_STATUS.last_call_failed()`/`last_call_status()`
//! protocol maps onto our stateless typed-error style — a mapping the spec
//! explicitly sanctions (`master02-overview.adoc` §Functional Style: results
//! as return values with status in the response envelope, "Either style can
//! be used, and can be trivially mapped from one to the other").
//!
//! [`CallStatusType::api_error`] is the single SM → HTTP table
//! (`docs/design/sm-platform/08-target-architecture.md` §5). ITS-REST 1.0.3 +
//! the CNF/ECC schedule remain the wire oracle (ADR-010): where the SM name
//! and the wire disagree, the wire's status code wins here.

use openehr_its::rest::runtime::ApiError;

/// `CALL_STATUS_TYPE` and its service-specific descendants, as one Rust enum.
///
/// The SM models the extension as inheritance ("Particular services may add
/// more codes by inheriting from this class", `master03-common_package.adoc`
/// §Representing Call Status); a single flat Rust enum with the provenance
/// documented per variant is the idiomatic equivalent — every abstract error
/// name used by an SM interface has exactly one variant here.
///
/// Variants marked *(prose-only)* appear in SM interface `.Errors` blocks but
/// in no vendored enumeration — a catalogued spec gap
/// (`docs/design/sm-platform/02-ehr-service.md` §9); this enum is their one
/// concrete home.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CallStatusType {
    // ── CALL_STATUS_TYPE (base; `call_status_type.adoc`) ────────────────────
    /// Call succeeded.
    Success,
    /// Authorisation failure.
    AuthFailure,
    /// Precondition violation occurred.
    PreconditionViolation,
    /// Referenced Object version of a Versioned Object does not exist.
    ObjectVersionDoesNotExist,
    /// No Versioned Object with referenced identifier found.
    VersionedObjectDoesNotExist,
    /// Exception other than precondition violation occurred.
    Exception,
    /// EHR with provided id not found.
    EhrIdDoesNotExist,
    /// Party with provided id not found.
    PartyIdDoesNotExist,
    /// File system locator cannot be written to.
    FileNotWritable,
    /// (Meaning blank in the source.) The committed `preceding_version_uid`
    /// does not match the current version — the optimistic-concurrency
    /// failure (ITS-REST `If-Match` → `412`).
    VersionMismatch,

    // ── EHR_CALL_STATUS_TYPE (`ehr_call_status_type.adoc`) ──────────────────
    /// COMPOSITION not found (per-variant meanings are blank in the source).
    CompositionDoesNotExist,
    /// CONTRIBUTION not found.
    ContributionDoesNotExist,
    /// COMPOSITION fails archetype/template conformance.
    CompositionArchetypeInvalid,
    /// `create_ehr_with_id` with an already-used EHR id.
    EhrCreateFailDuplicateId,
    /// COMPOSITION with this id already exists.
    CompositionAlreadyExists,
    /// An EHR for this subject already exists.
    EhrForSubjectAlreadyExists,

    // ── DEFINITION_CALL_STATUS_TYPE (`definition_call_status_type.adoc`) ────
    /// An invalid archetype was provided as a parameter.
    InvalidArchetype,
    /// An invalid template was provided as a parameter.
    InvalidTemplate,
    /// An invalid artefact was provided (meaning blank in the source; added
    /// by SM amendment 0.9.5 / SPECPR-293).
    InvalidArtefact,
    /// An invalid query was provided as a parameter.
    InvalidQuery,
    /// An invalid archetype identifier regex pattern was provided.
    InvalidIdPattern,
    /// A provided archetype identifier does not exist.
    ArtefactDoesNotExist,
    /// A provided template identifier does not exist.
    TemplateDoesNotExist,

    // ── prose-only error names (no vendored enum; see the type docs) ────────
    /// *(prose-only)* Referenced archetype/template identifiers unknown to
    /// the definitions service (`I_VALIDITY_CHECKER.definitions_valid`
    /// precondition failures on create/update calls).
    DefinitionUnknown,
    /// *(prose-only)* Content is not a valid RM instance
    /// (`I_VALIDITY_CHECKER.content_valid` precondition failures).
    ContentInvalid,
    /// *(prose-only)* A referenced VERSION does not exist
    /// (`I_EHR_DIRECTORY.get_directory_at_version`).
    VersionDoesNotExist,
    /// *(prose-only)* Subject identifier not found (`I_EHR_INDEX`).
    SubjectIdDoesNotExist,
    /// *(prose-only)* `VERSIONED_COMPOSITION` not found
    /// (`I_EHR_COMPOSITION.get_versioned_composition`).
    VersionedCompositionDoesNotExist,
}

impl CallStatusType {
    /// The SM abstract name of this status, exactly as the spec text spells
    /// it (`snake_case` enumeration literals).
    #[must_use]
    pub fn sm_name(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::AuthFailure => "auth_failure",
            Self::PreconditionViolation => "precondition_violation",
            Self::ObjectVersionDoesNotExist => "object_version_does_not_exist",
            Self::VersionedObjectDoesNotExist => "versioned_object_does_not_exist",
            Self::Exception => "exception",
            Self::EhrIdDoesNotExist => "ehr_id_does_not_exist",
            Self::PartyIdDoesNotExist => "party_id_does_not_exist",
            Self::FileNotWritable => "file_not_writable",
            Self::VersionMismatch => "version_mismatch",
            Self::CompositionDoesNotExist => "composition_does_not_exist",
            Self::ContributionDoesNotExist => "contribution_does_not_exist",
            Self::CompositionArchetypeInvalid => "composition_archetype_invalid",
            Self::EhrCreateFailDuplicateId => "ehr_create_fail_duplicate_id",
            Self::CompositionAlreadyExists => "composition_already_exists",
            Self::EhrForSubjectAlreadyExists => "ehr_for_subject_already_exists",
            Self::InvalidArchetype => "invalid_archetype",
            Self::InvalidTemplate => "invalid_template",
            Self::InvalidArtefact => "invalid_artefact",
            Self::InvalidQuery => "invalid_query",
            Self::InvalidIdPattern => "invalid_id_pattern",
            Self::ArtefactDoesNotExist => "artefact_does_not_exist",
            Self::TemplateDoesNotExist => "template_does_not_exist",
            Self::DefinitionUnknown => "definition_unknown",
            Self::ContentInvalid => "content_invalid",
            Self::VersionDoesNotExist => "version_does_not_exist",
            Self::SubjectIdDoesNotExist => "subject_id_does_not_exist",
            Self::VersionedCompositionDoesNotExist => "versioned_composition_does_not_exist",
        }
    }

    /// The one SM → HTTP mapping (design 08 §5); the wire oracle (ITS-REST
    /// 1.0.3 status codes carried by [`ApiError`]) decides each row.
    #[must_use]
    pub fn api_error(self, message: impl Into<String>) -> ApiError {
        let message = message.into();
        match self {
            Self::AuthFailure => ApiError::Forbidden(message),
            Self::PreconditionViolation | Self::InvalidIdPattern => ApiError::BadRequest(message),
            Self::ObjectVersionDoesNotExist
            | Self::VersionedObjectDoesNotExist
            | Self::EhrIdDoesNotExist
            | Self::PartyIdDoesNotExist
            | Self::CompositionDoesNotExist
            | Self::ContributionDoesNotExist
            | Self::ArtefactDoesNotExist
            | Self::TemplateDoesNotExist
            | Self::VersionDoesNotExist
            | Self::SubjectIdDoesNotExist
            | Self::VersionedCompositionDoesNotExist => ApiError::NotFound(message),
            Self::VersionMismatch => ApiError::PreconditionFailed(message),
            Self::EhrCreateFailDuplicateId
            | Self::CompositionAlreadyExists
            | Self::EhrForSubjectAlreadyExists => ApiError::Conflict(message),
            Self::CompositionArchetypeInvalid
            | Self::InvalidArchetype
            | Self::InvalidTemplate
            | Self::InvalidArtefact
            | Self::InvalidQuery
            | Self::DefinitionUnknown
            | Self::ContentInvalid => ApiError::Unprocessable(message),
            // `success` is not an error; mapping it is defensively a 500.
            Self::Success | Self::FileNotWritable | Self::Exception => ApiError::Internal(message),
        }
    }
}

/// `CALL_STATUS` — "Object representing a call status" (`call_status.adoc`).
///
/// All five attributes are mandatory in the SM. In our stateless mapping this
/// is the structured error payload a failed call surfaces (the SM obtains it
/// via `I_STATUS.last_call_status()` after the fact).
#[derive(Debug, Clone)]
pub struct CallStatus {
    /// Call status code for last call.
    pub code: CallStatusType,
    /// Name of call that this status documents.
    pub call_name: String,
    /// Full call, in stringified form, including arguments.
    pub call_string: String,
    /// Meaning of the result status.
    pub meaning: String,
    /// Error message text.
    pub message: String,
}

impl CallStatus {
    /// Build a status for a failed call; `meaning` defaults to the SM name of
    /// the code.
    #[must_use]
    pub fn new(
        code: CallStatusType,
        call_name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let call_name = call_name.into();
        Self {
            code,
            call_string: call_name.clone(),
            call_name,
            meaning: code.sm_name().to_owned(),
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_mismatch_maps_to_precondition_failed() {
        // ITS-REST `If-Match` failure → 412 (design 08 §5 row).
        assert!(matches!(
            CallStatusType::VersionMismatch.api_error("stale"),
            ApiError::PreconditionFailed(_)
        ));
    }

    #[test]
    fn every_status_has_a_distinct_sm_name() {
        let all = [
            CallStatusType::Success,
            CallStatusType::AuthFailure,
            CallStatusType::PreconditionViolation,
            CallStatusType::ObjectVersionDoesNotExist,
            CallStatusType::VersionedObjectDoesNotExist,
            CallStatusType::Exception,
            CallStatusType::EhrIdDoesNotExist,
            CallStatusType::PartyIdDoesNotExist,
            CallStatusType::FileNotWritable,
            CallStatusType::VersionMismatch,
            CallStatusType::CompositionDoesNotExist,
            CallStatusType::ContributionDoesNotExist,
            CallStatusType::CompositionArchetypeInvalid,
            CallStatusType::EhrCreateFailDuplicateId,
            CallStatusType::CompositionAlreadyExists,
            CallStatusType::EhrForSubjectAlreadyExists,
            CallStatusType::InvalidArchetype,
            CallStatusType::InvalidTemplate,
            CallStatusType::InvalidArtefact,
            CallStatusType::InvalidQuery,
            CallStatusType::InvalidIdPattern,
            CallStatusType::ArtefactDoesNotExist,
            CallStatusType::TemplateDoesNotExist,
            CallStatusType::DefinitionUnknown,
            CallStatusType::ContentInvalid,
            CallStatusType::VersionDoesNotExist,
            CallStatusType::SubjectIdDoesNotExist,
            CallStatusType::VersionedCompositionDoesNotExist,
        ];
        let names: std::collections::BTreeSet<_> = all.iter().map(|s| s.sm_name()).collect();
        assert_eq!(names.len(), all.len());
    }
}
