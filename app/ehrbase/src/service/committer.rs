//! The authenticated-committer request context.
//!
//! Every CONTRIBUTION carries a committer (`AUDIT_DETAILS.committer` `1..1`,
//! RM common `master04-generic_package` §Audit Details): when a request is
//! authenticated, the audit is attributed to that principal; an internal or
//! unauthenticated write falls back to the system identity. The protocol
//! adapter publishes the identity for the request's task scope via
//! [`with_committer`] and the platform reads it via [`current_committer`] —
//! the transport (a task-local) is spec-silent: no openEHR spec governs this
//! — our own design.

use tokio::task_local;

/// The authenticated identity a protocol adapter attributes commits to.
#[derive(Debug, Clone)]
pub struct CommitterIdentity {
    /// The authenticated subject (user name / token subject).
    pub subject: String,
    /// The `DV_IDENTIFIER.type` discriminator for the identity's origin
    /// (`"basic"` / `"oauth2"`).
    pub id_type: &'static str,
}

task_local! {
    static REQUEST_COMMITTER: Option<CommitterIdentity>;
}

/// The committer identity published for the current request, if any.
///
/// Outside a request scope (background tasks, tests) this is `None` and the
/// audit falls back to the system identity.
#[must_use]
pub fn current_committer() -> Option<CommitterIdentity> {
    REQUEST_COMMITTER.try_with(Clone::clone).ok().flatten()
}

/// Run `fut` with `identity` published as the request's committer.
pub async fn with_committer<F>(identity: Option<CommitterIdentity>, fut: F) -> F::Output
where
    F: Future,
{
    REQUEST_COMMITTER.scope(identity, fut).await
}
