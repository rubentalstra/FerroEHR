//! `I_VALIDITY_CHECKER` (`i_validity_checker.adoc`, included by
//! `master03-common_package.adoc` §Class Definitions).

use async_trait::async_trait;
use serde_json::Value;

use super::status::SmError;

/// `I_VALIDITY_CHECKER` — "Utility functions for checking validity of use of
/// definitions within data" (`i_validity_checker.adoc`).
///
/// The `a_content` parameter is the canonical-JSON form of an RM `LOCATABLE`.
/// No default method bodies (compile-time completeness): a backend that does
/// not implement a check is a build error, not a silent runtime stub.
#[async_trait]
pub trait ValidityChecker: Send + Sync {
    /// `definitions_valid (a_content: LOCATABLE): Boolean` — "Return `True`
    /// if the definition identifiers (i.e. archetype and template
    /// identifiers) are known in the local `definitions` service."
    async fn definitions_valid(&self, a_content: &Value) -> Result<bool, SmError>;

    /// `content_valid (a_content: LOCATABLE): Boolean` — "Return `True` if
    /// the content structure is a valid instance of the relevant RM classes."
    /// (The SM precondition spelling `valid_content(…)` in the EHR interfaces
    /// names this same check — a source inconsistency, recorded.)
    async fn content_valid(&self, a_content: &Value) -> Result<bool, SmError>;
}
