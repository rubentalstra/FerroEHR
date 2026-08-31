// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The **Validity Checking** component of the platform crate: the SM
//! `I_VALIDITY_CHECKER` interface realized on [`FerroEhrService`]'s existing
//! validation choke points.
//!
//! Spec: `docs/specs/openehr/SM/docs/openehr_platform/
//! master03-common_package.adoc` §Class Definitions and
//! `UML/classes/i_validity_checker.adoc` (the two calls `definitions_valid` and
//! `content_valid`, both over a `LOCATABLE`). The SM keeps `I_VALIDITY_CHECKER`
//! in its `common` package rather than among the platform services, so its impl
//! sits as the peer file `service/validity.rs`.
//!
//! Validation itself is owned by the validation register (`src/validation/`);
//! this file is only the SM interface adapter over the shared choke points
//! `FerroEhrService::web_template_for` and
//! `FerroEhrService::validate_for_commit`.
//!
//! NOTE: where the spec says "archetype and template identifiers",
//! `definitions_valid` checks template identifiers only, so content declaring no
//! template resolves `true`. `content_valid` runs the same per-`Kind` structural
//! validation every commit runs, and an unrecognized root `_type` is `false`.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 8): genuinely open operational JSON (config \
              dump, management env, validity-checker input, OpenAPI schema literals)"
)]

use serde_json::Value;

use crate::service::FerroEhrService;
use crate::service::error::ServiceError;
use crate::service::status::SmError;
use crate::versioning::Kind;

impl FerroEhrService {
    /// `definitions_valid` (`i_validity_checker.adoc`): "Return `True` if the
    /// definition identifiers (i.e. archetype and template identifiers) are
    /// known in the local `definitions` service."
    ///
    /// Content declaring no `archetype_details.template_id` resolves `true` (the
    /// module NOTE); otherwise the answer is whether the declared template
    /// resolves to a stored OPT.
    ///
    /// # Errors
    ///
    /// Infallible in the current realization — a template that fails to
    /// resolve answers `Ok(false)`, never an error; the `Result` is the SM
    /// call shape.
    pub async fn definitions_valid(&self, a_content: &Value) -> Result<bool, SmError> {
        let template_id = a_content
            .pointer("/archetype_details/template_id/value")
            .and_then(Value::as_str);
        match template_id {
            None => Ok(true),
            Some(id) => Ok(self.web_template_for(id).await.is_ok()),
        }
    }

    /// `content_valid` (`i_validity_checker.adoc`): "Return `True` if the
    /// content structure is a valid instance of the relevant RM classes."
    /// Runs the same per-`Kind` structural validation every commit runs; an
    /// unrecognized root `_type` is `false`. A bare validity check has no
    /// lifecycle context → full strictness.
    ///
    /// # Errors
    ///
    /// A validation verdict is never an error (`ValidationFailed` /
    /// `Unprocessable` answer `Ok(false)`); any other service failure from
    /// the validation path (e.g. a template-store/database fault) propagates
    /// as its [`SmError`] mapping.
    pub async fn content_valid(&self, a_content: &Value) -> Result<bool, SmError> {
        let rm_type = a_content.get("_type").and_then(Value::as_str).unwrap_or("");
        let Some(kind) = Kind::from_type(rm_type) else {
            return Ok(false);
        };
        Ok(self.commit_rejection(kind, a_content).await?.is_none())
    }

    /// The diagnostics-bearing sibling of [`Self::content_valid`]: the same
    /// commit-path validation, answering `None` for valid content and
    /// `Some(rejection)` with the refusal text VERBATIM — the seam a dry-run
    /// caller (the FHIR `$validate` door) previews the commit's own verdict
    /// through. Full strictness, exactly like a bare validity check.
    ///
    /// # Errors
    /// A validation verdict is never an error; any other service failure from
    /// the validation path (e.g. a template-store/database fault) propagates
    /// as its [`SmError`] mapping.
    pub(crate) async fn commit_rejection(
        &self,
        a_kind: Kind,
        a_content: &Value,
    ) -> Result<Option<String>, SmError> {
        match self.validate_for_commit(a_kind, a_content, false).await {
            Ok(()) => Ok(None),
            // Rendered through the SM bridge — where the per-path violation
            // list joins into one message — so the text matches what the
            // committing routes serve, verbatim.
            Err(
                rejection
                @ (ServiceError::ValidationFailed(_) | ServiceError::Unprocessable { .. }),
            ) => Ok(Some(SmError::from(rejection).message)),
            Err(other) => Err(other.into()),
        }
    }
}
