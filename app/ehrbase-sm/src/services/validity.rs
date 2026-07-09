//! The SM `I_VALIDITY_CHECKER` interface — content/definition validity checks.

use async_trait::async_trait;
use serde_json::Value;

use crate::error::{CallStatusType, SmError};

/// The SM `I_VALIDITY_CHECKER` interface
/// (`docs/specs/openehr/SM/docs/UML/classes/i_validity_checker.adoc`): "Utility
/// functions for checking validity of use of definitions within data."
///
/// The `content` parameter is the canonical-JSON form of an RM `LOCATABLE`.
#[async_trait]
pub trait ValidityChecker: Send + Sync {
    /// `definitions_valid` — True if the definition identifiers (archetype
    /// and template identifiers) in `a_content` are known in the local
    /// definitions service.
    async fn definitions_valid(&self, _a_content: &Value) -> Result<bool, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }
    /// `content_valid` — True if the content structure is a valid instance
    /// of the relevant RM classes. (The SM precondition spelling
    /// `valid_content(...)` names the same check — source inconsistency,
    /// digest 01 §4.5.)
    async fn content_valid(&self, _a_content: &Value) -> Result<bool, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "not implemented",
        ))
    }
}
