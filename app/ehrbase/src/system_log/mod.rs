//! IHE **ATNA** (Audit Trail and Node Authentication) audit trail for the
//! ehrbase-rs CDR — the platform-crate implementation of the SM
//! [`SystemLog`](crate::service::SystemLog) component (`I_SYSTEM_LOG`).
//!
//! The one normative openEHR statement for this component is a single line of
//! the SM platform component table — verbatim: "System Log | IHE
//! ATNA-compliant system log."
//! (`docs/specs/openehr/SM/docs/openehr_platform/master02-overview.adoc`); the
//! `I_SYSTEM_LOG` interface (`.../UML/classes/i_system_log.adoc`) is an empty
//! stub. Everything below therefore realizes that "IHE ATNA-compliant" mandate
//! against the external standards it pulls in, cited as external standards
//! (never as openEHR spec text).
//!
//! Emits one **DICOM Audit Message** (DICOM PS3.15 §A.5 — *not* openEHR
//! ITS-XML) per audited API operation, shipped to an Audit Record Repository
//! over **syslog** (RFC 5424 framing; RFC 5426 UDP or RFC 5425 TLS). This is
//! authorized defensive security-audit logging for a healthcare system.
//! (`docs/enterprise/atna-audit.md` is a non-normative design record — the
//! behaviour is governed by the standards cited above, not by that document.)
//!
//! ## Scope boundary (read/operation audit vs write/change-control audit)
//! This ATNA system log is the *security surveillance* record of API access
//! (who did what to which resource, with what outcome). It is **orthogonal to**
//! the RM change-control audit: every VERSION/CONTRIBUTION write records its
//! own authorship in `AUDIT_DETAILS` in the versioning path — "every write
//! access of any kind … is logged with the user identification, time, reason"
//! (BASE `architecture_overview/master07-security.adoc` §Integrity). That
//! write-audit is **not** implemented here; do not duplicate it in this module.
//!
//! The transport-agnostic event model ([`AuditEvent`](crate::system_log::event::AuditEvent))
//! and the [`SystemLog`](crate::service::SystemLog) trait live in the SM native-API
//! (the SM's `I_SYSTEM_LOG` is an empty stub); this module is
//! the ATNA *rendering* — the DICOM `AuditMessage`, syslog framing, transports,
//! and the non-blocking sender — plus the [`SystemLog`](crate::service::SystemLog)
//! implementation on [`EhrbaseService`](crate::service::EhrbaseService). The
//! ITS-REST operation → classification mapping is the protocol adapter's
//! concern (`ehrbase-rest::audit_table`). The `ehrbase-rest` layer builds an
//! `AuditEvent` and emits it through the platform (`Platform: … + SystemLog`);
//! the binary (`ehrbase`) boots the [`AuditSender`] and supplies the DB-backed
//! [`SubjectResolver`].
//!
//! ## Module map
//! - [`message`] — the DICOM `AuditMessage` model + `quick-xml` serializer.
//! - [`codes`] — DCM / RFC-3881 code constants + the ATNA rendering of the SM
//!   event enums ([`codes::AtnaCodes`]).
//! - [`syslog`] — RFC 5424 assembly + RFC 5426 UDP / RFC 5425 TLS transports.
//! - [`sender`] — the bounded-mpsc sender + background drain + fail modes.
//! - [`config`] — the `figment` [`AuditConfig`].

pub mod codes;
pub mod config;
pub mod message;
pub mod sender;
pub mod syslog;

pub mod event;

use event::{AuditEvent, EmitOutcome};

pub use config::{AuditConfig, FailMode, Transport};
pub use message::{AuditContext, AuditMessage};
pub use sender::{AuditHandle, AuditSender, SubjectResolver, start};

use crate::service::EhrbaseService;

/// Errors raised while rendering or shipping an audit record.
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    /// XML serialization of the DICOM Audit Message failed.
    #[error("audit message serialization failed: {0}")]
    Xml(String),
    /// The syslog transport (UDP/TLS) could not be established.
    #[error("audit transport error: {0}")]
    Transport(String),
}

// quick-xml's `Writer` over an in-memory buffer surfaces write failures as
// `std::io::Error`; in the DICOM serializer these can only be a buffer fault.
impl From<std::io::Error> for AuditError {
    fn from(e: std::io::Error) -> Self {
        AuditError::Xml(e.to_string())
    }
}

/// The platform realizes the SM `I_SYSTEM_LOG` component: it emits resolved
/// audit events through the optional ATNA [`AuditSender`] the binary wires in
/// ([`EhrbaseService::with_audit`](crate::service::EhrbaseService::with_audit)).
/// With no sender wired, auditing is off and every emit is
/// [`EmitOutcome::Dropped`].
impl EhrbaseService {
    #[must_use]
    pub fn emit(&self, event: AuditEvent) -> EmitOutcome {
        // `map_or` (not `map(..).unwrap_or(..)`) keeps clippy happy; behaviour
        // is identical to `Dropped` when no sender is wired.
        self.audit
            .as_ref()
            .map_or(EmitOutcome::Dropped, |s| s.emit(event))
    }

    pub fn audit_enabled(&self) -> bool {
        self.audit.as_ref().is_some_and(AuditSender::enabled)
    }

    pub fn suppress_login_events(&self) -> bool {
        self.audit
            .as_ref()
            .is_some_and(AuditSender::suppress_login_events)
    }
}
