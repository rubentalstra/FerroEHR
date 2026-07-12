//! The **Validity Checking** component of the platform crate: the SM
//! `I_VALIDITY_CHECKER` interface ([`ehrbase_sm::ValidityChecker`]) realized on
//! [`EhrbaseService`]'s existing validation choke points.
//!
//! Spec: `docs/specs/openehr/SM/docs/openehr_platform/
//! master03-common_package.adoc` §Class Definitions →
//! `UML/classes/i_validity_checker.adoc` (the two calls `definitions_valid` /
//! `content_valid`, both over a `LOCATABLE`). SM keeps `I_VALIDITY_CHECKER` in
//! its `common` package (not among the platform *services*), so its impl sits as
//! a peer file `service/validity.rs`, mirroring the SM placement
//! (`ehrbase-sm/src/services/common/validity.rs`).
//!
//! Validation itself is owned by the validation register (`src/validation/`);
//! this file is only the SM interface adapter over the shared choke points
//! [`EhrbaseService::web_template_for`] and
//! [`EhrbaseService::validate_for_commit`].

use async_trait::async_trait;
use serde_json::Value;

use ehrbase_sm::{SmError, ValidityChecker};

use crate::service::{EhrbaseService, ServiceError};
use crate::versioning::Kind;

/// SM `I_VALIDITY_CHECKER` (`i_validity_checker.adoc`), realized on the existing
/// validation choke points.
///
/// PORT NOTE: `definitions_valid` checks **template** identifiers only — the
/// spec says "archetype and template identifiers", but there is no ADL2
/// archetype store to resolve bare archetype ids against, so content that
/// declares no template resolves `true` (nothing to look up). `content_valid`
/// runs the same per-`Kind` structural validation every commit runs
/// ([`EhrbaseService::validate_for_commit`]); an unrecognized root `_type` is
/// `false`.
#[async_trait]
impl ValidityChecker for EhrbaseService {
    async fn definitions_valid(&self, a_content: &Value) -> Result<bool, SmError> {
        let template_id = a_content
            .pointer("/archetype_details/template_id/value")
            .and_then(Value::as_str);
        match template_id {
            None => Ok(true),
            Some(id) => Ok(self.web_template_for(id).await.is_ok()),
        }
    }

    async fn content_valid(&self, a_content: &Value) -> Result<bool, SmError> {
        let rm_type = a_content.get("_type").and_then(Value::as_str).unwrap_or("");
        let Some(kind) = Kind::from_type(rm_type) else {
            return Ok(false);
        };
        // A bare validity check has no lifecycle context → full strictness.
        match self.validate_for_commit(kind, a_content, false).await {
            Ok(()) => Ok(true),
            Err(ServiceError::ValidationFailed(_) | ServiceError::Unprocessable(_)) => Ok(false),
            Err(other) => Err(other.into()),
        }
    }
}
