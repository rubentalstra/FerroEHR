//! Version lifecycle: the `version_lifecycle_state` codes and the state-machine
//! that governs transitions between them.
//!
//! Spec: RM common `master06-change_control_package.adoc` §Version Lifecycle.
//! `ORIGINAL_VERSION.lifecycle_state` is coded from the openEHR
//! `version lifecycle state` group; "the possible values are `532|complete|`,
//! `553|incomplete|`, `523|deleted|`, `800|inactive|` and `801|abandoned|`"
//! (master06 §Version Lifecycle). "One basic rule … is that any transition
//! requires the commit of a _new version_, even if the content is otherwise
//! unchanged" — realized by the storage layer (a new `vo_version` row per
//! commit). This module owns the **legality** of each transition: reading
//! the current state from the preceding version and rejecting a transition the
//! spec does not sanction (previously any target state was accepted).

use crate::service::error::ServiceError;
use openehr_term::bundle::openehr;

/// The `version_lifecycle_state` openEHR terminology group id.
const VERSION_LIFECYCLE_STATE: &str = "version_lifecycle_state";

/// `version_lifecycle_state` group codes (master06 §Version Lifecycle).
pub(crate) mod state {
    /// `532|complete|` — a fully authored version.
    pub(crate) const COMPLETE: &str = "532";
    /// `553|incomplete|` — a partial/unreviewed version committed with relaxed
    /// validation (master06 §Incomplete Content).
    pub(crate) const INCOMPLETE: &str = "553";
    /// `523|deleted|` — a logically deleted version (master06 §Logical Deletion).
    pub(crate) const DELETED: &str = "523";
    /// `800|inactive|` — content marked no longer valid for use (master06
    /// §Abandoned and Inactive States).
    pub(crate) const INACTIVE: &str = "800";
    /// `801|abandoned|` — incomplete content that lost relevance before
    /// completion (master06 §Abandoned and Inactive States).
    pub(crate) const ABANDONED: &str = "801";
}

/// Resolve an inbound `lifecycle_state` token — a numeric group code (`"553"`)
/// or a rubric (`"incomplete"`) — to its canonical numeric
/// `version_lifecycle_state` code. `None` when the token is not a member of the
/// group (`ORIGINAL_VERSION.Lifecycle_state_valid`; master06 §Version
/// Lifecycle: the five values `532/553/523/800/801`).
pub(crate) fn lifecycle_state_code(token: &str) -> Option<String> {
    let t = openehr();
    if t.is_valid_version_lifecycle_state(token) {
        return Some(token.to_owned());
    }
    t.concepts_in_group(VERSION_LIFECYCLE_STATE)
        .iter()
        .find(|c| c.rubric.eq_ignore_ascii_case(token))
        .map(|c| c.id.clone())
}

/// The rubric (English display text) for a `version_lifecycle_state` code;
/// falls back to the code itself if unknown. Note the SPECPR-51 quirk: `532` is
/// `complete` in this group (`completed` only in `instruction_states`) — the
/// group-scoped lookup resolves it correctly.
pub(crate) fn lifecycle_rubric(code: &str) -> String {
    openehr()
        .rubric(VERSION_LIFECYCLE_STATE, code, "en")
        .unwrap_or(code)
        .to_owned()
}

/// Resolve a client-supplied `version_lifecycle_state` token into its canonical
/// numeric code, defaulting to `532|complete|` when absent (master06 §Version
/// Lifecycle). An out-of-group token is a `422`
/// (`ORIGINAL_VERSION.Lifecycle_state_valid`), naming the terminology group.
pub(crate) fn resolve_lifecycle(token: Option<String>) -> Result<String, ServiceError> {
    match token {
        Some(token) => lifecycle_state_code(&token).ok_or_else(|| {
            ServiceError::Unprocessable(format!(
                "lifecycle_state {token:?} is not a code in the openEHR \
                 version_lifecycle_state group (ORIGINAL_VERSION.Lifecycle_state_valid)"
            ))
        }),
        None => Ok(state::COMPLETE.to_owned()),
    }
}

/// Whether a `version_lifecycle_state` transition `from -> to` is sanctioned by
/// the master06 §Version Lifecycle state machine.
///
/// `from = None` is a **first** version (no preceding state): only the two
/// commit-time states `complete`/`incomplete` are reachable
/// (master06 §Incomplete Content: "content will be committed in the `complete`
/// state … it may be committed as `incomplete`"). A subsequent transition is
/// checked against the state table (master06 §Abandoned and Inactive States):
///
/// | from | allowed to |
/// |---|---|
/// | `complete`   | `complete` (edit), `inactive` (deactivate), `deleted` (delete) |
/// | `incomplete` | `incomplete` (edit), `complete` (complete), `abandoned` (abandon), `deleted` (delete) |
/// | `inactive`   | `inactive`, `complete` (reactivate), `incomplete` (retrieve), `deleted` (delete) |
/// | `abandoned`  | `abandoned`, `incomplete` (retrieve), `deleted` (delete) |
/// | `deleted`    | `complete`/`incomplete` (restoration, `816`) |
///
/// A transition outside this table is a `422` naming the state machine — e.g.
/// `complete -> abandoned` (must pass through `incomplete`), `abandoned ->
/// complete` (must `retrieve` to `incomplete` first). The import replay does
/// **not** call this (it preserves source history verbatim — master06
/// §Copying), and the logical-delete path targets `deleted` from any live state
/// (master06 §Logical Deletion).
pub(crate) fn validate_transition(from: Option<&str>, to: &str) -> Result<(), ServiceError> {
    use state::{ABANDONED, COMPLETE, DELETED, INACTIVE, INCOMPLETE};

    let allowed = match from {
        // A first version can only be authored `complete` or `incomplete`.
        None => matches!(to, COMPLETE | INCOMPLETE),
        Some(COMPLETE) => matches!(to, COMPLETE | INACTIVE | DELETED),
        Some(INCOMPLETE) => matches!(to, INCOMPLETE | COMPLETE | ABANDONED | DELETED),
        Some(INACTIVE) => matches!(to, INACTIVE | COMPLETE | INCOMPLETE | DELETED),
        Some(ABANDONED) => matches!(to, ABANDONED | INCOMPLETE | DELETED),
        // Restoration of a logically-deleted version (change type `816`).
        Some(DELETED) => matches!(to, COMPLETE | INCOMPLETE),
        // An unknown stored state (should be impossible past the CHECK) is
        // treated permissively — the terminology guard is the real gate.
        Some(_) => true,
    };
    if allowed {
        return Ok(());
    }
    let (from_code, from_rubric) = match from {
        Some(c) => (c, lifecycle_rubric(c)),
        None => ("(new)", "new".to_owned()),
    };
    Err(ServiceError::Unprocessable(format!(
        "illegal version lifecycle transition {from_code}|{from_rubric}| -> {to}|{}| \
         (RM common master06 §Version Lifecycle state machine)",
        lifecycle_rubric(to),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_state_code_accepts_code_or_rubric_and_rejects_non_members() {
        assert_eq!(lifecycle_state_code("532").as_deref(), Some("532"));
        assert_eq!(lifecycle_state_code("complete").as_deref(), Some("532"));
        assert_eq!(lifecycle_state_code("incomplete").as_deref(), Some("553"));
        assert_eq!(lifecycle_state_code("Deleted").as_deref(), Some("523"));
        assert_eq!(lifecycle_state_code("inactive").as_deref(), Some("800"));
        assert_eq!(lifecycle_state_code("abandoned").as_deref(), Some("801"));
        // Out-of-group tokens rejected (ORIGINAL_VERSION.Lifecycle_state_valid).
        assert_eq!(lifecycle_state_code("not-a-state"), None);
        assert_eq!(lifecycle_state_code("249"), None); // change-type code, wrong group
    }

    #[test]
    fn lifecycle_rubric_specpr51() {
        // SPECPR-51: version lifecycle 532 → "complete" (not "completed").
        assert_eq!(lifecycle_rubric(state::COMPLETE), "complete");
        assert_eq!(lifecycle_rubric(state::DELETED), "deleted");
    }

    #[test]
    fn resolve_defaults_to_complete() {
        assert_eq!(resolve_lifecycle(None).unwrap(), state::COMPLETE);
        assert_eq!(
            resolve_lifecycle(Some("incomplete".into())).unwrap(),
            state::INCOMPLETE
        );
        assert!(resolve_lifecycle(Some("nonsense".into())).is_err());
    }

    /// the master06 §Version Lifecycle state machine — legal transitions
    /// pass, illegal ones are rejected.
    #[test]
    fn transition_state_machine() {
        use state::{ABANDONED, COMPLETE, DELETED, INACTIVE, INCOMPLETE};

        // First version: only complete / incomplete.
        assert!(validate_transition(None, COMPLETE).is_ok());
        assert!(validate_transition(None, INCOMPLETE).is_ok());
        assert!(validate_transition(None, INACTIVE).is_err());
        assert!(validate_transition(None, ABANDONED).is_err());
        assert!(validate_transition(None, DELETED).is_err());

        // The named transitions of the spec table.
        assert!(validate_transition(Some(COMPLETE), INACTIVE).is_ok()); // deactivate
        assert!(validate_transition(Some(INACTIVE), COMPLETE).is_ok()); // reactivate
        assert!(validate_transition(Some(INACTIVE), INCOMPLETE).is_ok()); // retrieve
        assert!(validate_transition(Some(INCOMPLETE), COMPLETE).is_ok()); // complete
        assert!(validate_transition(Some(INCOMPLETE), ABANDONED).is_ok()); // abandon
        assert!(validate_transition(Some(ABANDONED), INCOMPLETE).is_ok()); // retrieve
        assert!(validate_transition(Some(ABANDONED), DELETED).is_ok()); // delete
        assert!(validate_transition(Some(DELETED), COMPLETE).is_ok()); // restoration

        // Ordinary same-state re-commit (a new version, unchanged state).
        assert!(validate_transition(Some(COMPLETE), COMPLETE).is_ok());

        // Illegal transitions.
        assert!(validate_transition(Some(COMPLETE), ABANDONED).is_err());
        assert!(validate_transition(Some(COMPLETE), INCOMPLETE).is_err());
        assert!(validate_transition(Some(ABANDONED), COMPLETE).is_err());
        assert!(validate_transition(Some(ABANDONED), INACTIVE).is_err());
        assert!(validate_transition(Some(INACTIVE), ABANDONED).is_err());
    }
}
