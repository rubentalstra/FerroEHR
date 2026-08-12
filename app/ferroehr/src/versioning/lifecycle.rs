// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

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
//! spec does not sanction.

use crate::service::error::{ServiceError, Violation};
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
            ServiceError::content_invalid(
                Violation::new(format!(
                    "{token:?} is not a code in the openEHR version_lifecycle_state group"
                ))
                .with_path("lifecycle_state")
                .with_invariant("ORIGINAL_VERSION.Lifecycle_state_valid"),
            )
        }),
        None => Ok(state::COMPLETE.to_owned()),
    }
}

/// Refuse `523|deleted|` on a commit that carries `data`.
///
/// master06 §Logical Deletion states deletion as ONE indivisible procedure —
/// "create a new Version in the normal way; delete its `_data_` (which will by
/// default be a copy of the data of the previous Version); set the
/// `_lifecycle_state_` value to the code for `deleted`; commit in the normal
/// way". The state and the data-Void are two steps of the same act, so a
/// data-carrying version in the `deleted` state is not producible by the
/// spec's own procedure: it would leave the repository serving `204` for the
/// resource (the version says the item is gone) while its content stays
/// stored and AQL-queryable. `ORIGINAL_VERSION.data` is typed `0..1`
/// (`RM/docs/UML/classes/org.openehr.rm.common.original_version.adoc`
/// §Attributes) precisely so the deleted version can carry none.
///
/// The mirror of the CONTRIBUTION `classify` rule that already couples change
/// type `523` to data-absence, applied to the *lifecycle* axis so both the
/// CONTRIBUTION member (`UPDATE_VERSION.lifecycle_state`) and the direct
/// route's `openehr-version: lifecycle_state.code_string="523"` header reach
/// the same refusal. The code is the ITS-REST overview 422 row
/// (`Requests_and_responses.md` §HTTP status codes: "The request was
/// well-formed but was unable to be followed due to semantic errors").
///
/// # Errors
/// [`ServiceError::Unprocessable`] when `lifecycle` is `523|deleted|`.
pub(crate) fn reject_deleted_with_data(lifecycle: &str) -> Result<(), ServiceError> {
    if lifecycle != state::DELETED {
        return Ok(());
    }
    Err(ServiceError::content_invalid(
        Violation::new(
            "523|deleted| is invalid on a version that carries data — logical deletion \
             deletes the version's data and sets the deleted state in one act",
        )
        .with_path("lifecycle_state")
        .with_invariant("RM common master06 §Logical Deletion"),
    ))
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
    Err(ServiceError::content_invalid(
        Violation::new(format!(
            "illegal version lifecycle transition {from_code}|{from_rubric}| -> {to}|{}|",
            lifecycle_rubric(to),
        ))
        .with_invariant("RM common master06 §Version Lifecycle state machine"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every named constant is a real group member, and the constants cover the
    /// COMPLETE `version_lifecycle_state` group — a code added to the TERM
    /// bundle without a constant here fails this test. (The same guard the
    /// `audit_change_type` and `composition_category` constant sets carry;
    /// master06 §Version Lifecycle names exactly these five values.)
    #[test]
    fn lifecycle_constants_are_the_complete_group() {
        let all = [
            state::COMPLETE,
            state::INCOMPLETE,
            state::DELETED,
            state::INACTIVE,
            state::ABANDONED,
        ];
        let t = openehr();
        for code in all {
            assert!(t.is_valid_version_lifecycle_state(code), "code {code}");
            // `code_string` must be numeric (the group's wire form).
            assert!(code.chars().all(|c| c.is_ascii_digit()), "code {code}");
        }
        let mut group: Vec<String> = t
            .concepts_in_group(VERSION_LIFECYCLE_STATE)
            .iter()
            .map(|c| c.id.clone())
            .collect();
        group.sort();
        let mut named: Vec<String> = all.iter().map(|c| (*c).to_owned()).collect();
        named.sort();
        assert_eq!(group, named, "constants must mirror the full TERM group");
    }

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
        // The refusal is asserted as DATA: the path and the named invariant,
        // not a substring of the rendered sentence.
        match resolve_lifecycle(Some("nonsense".into())) {
            Err(ServiceError::Unprocessable { violation: v, .. }) => {
                assert_eq!(v.path(), Some("lifecycle_state"));
                assert_eq!(
                    v.invariant(),
                    Some("ORIGINAL_VERSION.Lifecycle_state_valid")
                );
                assert!(v.detail().contains("\"nonsense\""), "{v}");
            }
            other => panic!("an out-of-group lifecycle token must be 422, got {other:?}"),
        }
    }

    /// master06 §Logical Deletion couples the `deleted` state to the data-Void
    /// in ONE act, so `523` is refused on a data-carrying commit — twinned
    /// against every other state, which passes unchanged.
    #[test]
    fn deleted_state_is_refused_on_a_data_carrying_commit() {
        // The refusal, asserted as DATA (path + named spec rule).
        match reject_deleted_with_data(state::DELETED) {
            Err(ServiceError::Unprocessable { violation: v, .. }) => {
                assert_eq!(v.path(), Some("lifecycle_state"));
                assert_eq!(v.invariant(), Some("RM common master06 §Logical Deletion"));
            }
            other => panic!("523 on a data-carrying version must be 422, got {other:?}"),
        }
        // The accepting twins: every other lifecycle state carries data.
        for code in [
            state::COMPLETE,
            state::INCOMPLETE,
            state::INACTIVE,
            state::ABANDONED,
        ] {
            assert!(
                reject_deleted_with_data(code).is_ok(),
                "state {code} carries data legitimately"
            );
        }
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

        // Illegal transitions. The first is asserted as DATA — the refusal
        // names the state machine it enforces, readable off the value.
        match validate_transition(Some(COMPLETE), ABANDONED) {
            Err(ServiceError::Unprocessable { violation: v, .. }) => assert_eq!(
                v.invariant(),
                Some("RM common master06 §Version Lifecycle state machine")
            ),
            other => panic!("complete -> abandoned must be refused, got {other:?}"),
        }
        assert!(validate_transition(Some(COMPLETE), ABANDONED).is_err());
        assert!(validate_transition(Some(COMPLETE), INCOMPLETE).is_err());
        assert!(validate_transition(Some(ABANDONED), COMPLETE).is_err());
        assert!(validate_transition(Some(ABANDONED), INACTIVE).is_err());
        assert!(validate_transition(Some(INACTIVE), ABANDONED).is_err());
    }
}
