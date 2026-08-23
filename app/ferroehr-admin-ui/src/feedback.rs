// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Mutation feedback: the console's one error-feedback rule and the
//! actionable copy behind it.
//!
//! The rule (recorded in the crate `CLAUDE.md`): **a mutation toasts on
//! success AND on failure**, a detailed inline `MessageBar` may stay beside
//! the failure toast where the diagnostic is worth reading, and **pure reads
//! render inline errors only**. A screen that quietly leaves a failed write's
//! diagnostic below the fold looks like nothing happened — which on a
//! clinical repository is the worst possible feedback.
//!
//! No openEHR spec governs an admin UI's feedback — our own design / product
//! extension. The copy shape follows
//! [`delete_failure_copy`](crate::admin::delete_failure_copy) (the
//! destructive-operation sibling): name the object, name what went wrong
//! (carrying the CDR's own diagnostic verbatim), name the next action.

use crate::error::AdminUiError;

/// Actionable copy for a failed write: names `object`, what went wrong, and
/// the next action.
///
/// `object` is a noun phrase for the thing being written ("the operational
/// template", "the new EHR", "the stored query"), so every arm reads as one
/// sentence about that object.
///
/// The CDR's own diagnostic travels verbatim — a `422` body naming the
/// offending path is exactly what the user needs.
#[must_use]
pub fn write_failure_copy(object: &str, error: &AdminUiError) -> String {
    match error {
        AdminUiError::Invalid(message) => {
            format!("{object} was not sent: {message}. Correct the input and retry.")
        }
        // 412 and 409 are different failures on the wire, so they get
        // different next actions: `412` is "one or more conditions given in
        // the request header fields evaluated to false" (the stale `If-Match`
        // version), `409` is "the request could not be processed because it
        // might generate a duplicate or a conflict" — ITS-REST
        // `specifications/docs/overview/Requests_and_responses.md`
        // §status codes.
        AdminUiError::Cdr { status, message } => match error.status_code() {
            Some(http::StatusCode::BAD_REQUEST | http::StatusCode::UNPROCESSABLE_ENTITY) => {
                format!("The CDR rejected {object}: {message}. Fix the reported problem and retry.")
            }
            Some(http::StatusCode::PRECONDITION_FAILED) => format!(
                "{object} changed in the CDR since this screen loaded ({message}). Reload the \
                 latest version, then reapply your change."
            ),
            Some(http::StatusCode::CONFLICT) => format!(
                "The CDR already holds a conflicting object, so {object} was not created \
                 ({message}). Open the existing one, or change the identifier and retry."
            ),
            Some(http::StatusCode::NOT_FOUND) => format!(
                "{object} is not in the CDR ({message}) — it may have been deleted meanwhile. \
                 Reload this screen and retry."
            ),
            _ => format!("The CDR answered {status} to {object}: {message}. Nothing was saved."),
        },
        AdminUiError::Forbidden(message) => format!(
            "This session may not write {object} ({message}). Sign in with an account that \
             carries the required role and retry."
        ),
        AdminUiError::Unauthenticated => format!(
            "The console session expired before {object} was saved — sign in again and retry; \
             your input is still on screen."
        ),
        AdminUiError::CdrUnreachable(message) => format!(
            "The CDR is unreachable ({message}), so {object} was not saved. Check the CDR, then \
             retry."
        ),
        AdminUiError::Internal(message) => format!(
            "{object} could not be saved: {message}. Retry; if it persists, check the console \
             logs."
        ),
    }
}

/// Actionable copy for a refused openEHR **logical delete**.
///
/// The versioned delete of a COMPOSITION or a directory, which is a normal
/// write on the versioned object (not the destructive admin delete
/// [`delete_failure_copy`](crate::admin::delete_failure_copy) covers).
///
/// The concurrency family gets its own next action: a `409` is returned "when
/// supplied `uid_based_id` doesn't match the latest version"
/// (`docs/specs/openehr/ITS-REST/specifications/responses/409_COMPOSITION_with_uid_based_id.yaml`)
/// and a `412` is the `If-Match` precondition on the preceding version
/// evaluating to false
/// (`docs/specs/openehr/ITS-REST/specifications/docs/overview/Requests_and_responses.md`
/// §"If-Match and accidental overwrites") — the same cause, so one next
/// action: reload the history and delete the version that is now latest.
/// Everything else falls through to [`write_failure_copy`].
#[must_use]
pub fn logical_delete_failure_copy(object: &str, error: &AdminUiError) -> String {
    match error {
        AdminUiError::Cdr { message, .. } => match error.status_code() {
            Some(http::StatusCode::CONFLICT | http::StatusCode::PRECONDITION_FAILED) => format!(
                "{object} moved on in the CDR since this screen loaded, so nothing was deleted \
                 ({message}). Reload the version history and delete the version that is latest \
                 now."
            ),
            // `400_already_deleted` on the COMPOSITION delete: the version is
            // already logically deleted, so there is nothing left to do.
            Some(http::StatusCode::BAD_REQUEST) => format!(
                "The CDR refused to delete {object}: {message}. It may already be deleted — \
                 reload this screen to see the current history."
            ),
            Some(http::StatusCode::NOT_FOUND) => format!(
                "{object} is not in the CDR ({message}) — it may have been deleted meanwhile. \
                 Reload this screen."
            ),
            _ => write_failure_copy(object, error),
        },
        AdminUiError::Forbidden(message) => format!(
            "This session may not delete {object} ({message}). Sign in with an account that \
             carries the required role and retry."
        ),
        other => write_failure_copy(object, other),
    }
}

/// Toast a failed write with [`write_failure_copy`] — the failure half of the
/// rule this module records.
///
/// `title` is the short outcome ("Upload failed", "Commit failed"); `object`
/// is the noun phrase the copy is built around.
pub fn toast_write_failure(
    toaster: thaw::ToasterInjection,
    title: &str,
    object: &str,
    error: &AdminUiError,
) {
    crate::components::toast::toast_error(toaster, title, &write_failure_copy(object, error));
}

#[cfg(test)]
mod tests {
    use super::{logical_delete_failure_copy, write_failure_copy};
    use crate::error::AdminUiError;

    #[test]
    fn a_logical_delete_conflict_says_reload_the_history() {
        // 409 (path uid is not the latest version) and 412 (If-Match failed)
        // are the same cause and share one next action.
        for status in [409_u16, 412] {
            let copy = logical_delete_failure_copy(
                "this composition version",
                &AdminUiError::Cdr {
                    status,
                    message: "latest is ::3".to_owned(),
                },
            );
            assert!(copy.contains("this composition version"), "{copy}");
            assert!(copy.contains("latest is ::3"), "{copy}");
            assert!(copy.contains("Reload the version history"), "{copy}");
            assert!(copy.contains("nothing was deleted"), "{copy}");
        }
    }

    #[test]
    fn an_already_deleted_version_and_a_refusal_read_as_deletes() {
        let already = logical_delete_failure_copy(
            "this composition version",
            &AdminUiError::Cdr {
                status: 400,
                message: "already deleted".to_owned(),
            },
        );
        assert!(
            already.contains("already deleted") && already.contains("reload"),
            "{already}"
        );

        let refused = logical_delete_failure_copy(
            "this composition version",
            &AdminUiError::Forbidden("insufficient scope".to_owned()),
        );
        assert!(
            refused.contains("may not delete") && refused.contains("insufficient scope"),
            "{refused}"
        );

        // Anything outside the delete-specific family keeps the shared write
        // copy rather than inventing a second vocabulary.
        let down = logical_delete_failure_copy(
            "this composition version",
            &AdminUiError::CdrUnreachable("connection refused".to_owned()),
        );
        assert_eq!(
            down,
            write_failure_copy(
                "this composition version",
                &AdminUiError::CdrUnreachable("connection refused".to_owned())
            )
        );
    }

    #[test]
    fn cdr_validation_rejection_names_the_object_the_diagnostic_and_the_next_action() {
        let copy = write_failure_copy(
            "the operational template",
            &AdminUiError::Cdr {
                status: 422,
                message: "template id already registered".to_owned(),
            },
        );
        assert!(copy.contains("the operational template"), "{copy}");
        assert!(copy.contains("template id already registered"), "{copy}");
        assert!(copy.contains("retry"), "{copy}");
    }

    #[test]
    fn concurrency_duplication_and_absence_get_their_own_next_action() {
        let stale = write_failure_copy(
            "the directory",
            &AdminUiError::Cdr {
                status: 412,
                message: "If-Match did not hold".to_owned(),
            },
        );
        assert!(stale.contains("Reload the latest version"), "{stale}");

        // A 409 is a duplicate/conflict, NOT a stale version: reloading a
        // version the user does not have is the wrong next action.
        let duplicate = write_failure_copy(
            "the new EHR",
            &AdminUiError::Cdr {
                status: 409,
                message: "an EHR for that subject already exists".to_owned(),
            },
        );
        assert!(
            duplicate.contains("an EHR for that subject already exists")
                && duplicate.contains("Open the existing one"),
            "{duplicate}"
        );
        assert!(
            !duplicate.contains("Reload the latest version"),
            "{duplicate}"
        );

        let gone = write_failure_copy(
            "the composition",
            &AdminUiError::Cdr {
                status: 404,
                message: "HTTP 404".to_owned(),
            },
        );
        assert!(
            gone.contains("the composition") && gone.contains("Reload this screen"),
            "{gone}"
        );
    }

    #[test]
    fn transport_auth_and_input_failures_all_name_an_action() {
        let down = write_failure_copy(
            "the new EHR",
            &AdminUiError::CdrUnreachable("connection refused".to_owned()),
        );
        assert!(
            down.contains("the new EHR")
                && down.contains("connection refused")
                && down.contains("retry"),
            "{down}"
        );

        let refused = write_failure_copy(
            "the stored query",
            &AdminUiError::Forbidden("insufficient scope".to_owned()),
        );
        assert!(
            refused.contains("insufficient scope") && refused.contains("required role"),
            "{refused}"
        );

        let expired = write_failure_copy("the directory", &AdminUiError::Unauthenticated);
        assert!(expired.contains("sign in again"), "{expired}");

        let invalid = write_failure_copy(
            "the composition",
            &AdminUiError::Invalid("the body is not valid JSON".to_owned()),
        );
        assert!(
            invalid.contains("the body is not valid JSON") && invalid.contains("Correct the input"),
            "{invalid}"
        );

        let other = write_failure_copy(
            "the stored query",
            &AdminUiError::Cdr {
                status: 500,
                message: "internal".to_owned(),
            },
        );
        assert!(
            other.contains("500") && other.contains("Nothing was saved"),
            "{other}"
        );
    }
}
