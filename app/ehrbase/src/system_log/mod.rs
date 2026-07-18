//! IHE **ATNA** (Audit Trail and Node Authentication) audit trail — the
//! platform's realization of the SM **System Log** component (`I_SYSTEM_LOG`).
//!
//! The one normative openEHR statement for this component is a single line of
//! the SM platform component table — verbatim: "System Log | IHE
//! ATNA-compliant system log"
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
//! ## Seams
//! The ITS-REST operation → classification mapping is the protocol adapter's
//! concern (`ehrbase-rest::system_log::classify`); its audit middleware builds
//! an [`event::AuditEvent`] per request and hands it to the platform through
//! [`EhrbaseService::emit`]. The binary (`ehrbase-server`) boots the subsystem
//! via [`start`] and supplies the DB-backed [`SubjectResolver`]; the sender is
//! installed on the service with
//! [`EhrbaseService::with_audit`](crate::service::EhrbaseService::with_audit).
//!
//! ## Module map
//! - [`event`] — the transport-agnostic audit event model.
//! - [`codes`] — DCM / RFC-3881 code constants + the ATNA rendering of the
//!   event enums.
//! - [`message`] — the DICOM `AuditMessage` model + `quick-xml` serializer.
//! - [`fhir`] — the FHIR R4 `AuditEvent` rendering per the IHE BALP content
//!   profiles (the modern half of the dual format).
//! - [`syslog`] — RFC 5424 assembly + RFC 5426 UDP / RFC 5425 TLS transports.
//! - [`sender`] — the bounded-mpsc sender + background drain + fail modes.
//! - [`config`] — the `[atna]` section struct ([`config::AuditConfig`]).

pub mod codes;
pub mod config;
pub mod event;
pub mod fhir;
pub mod message;
pub mod sender;

use crate::system_log::sender::AuditSender;
pub mod syslog;

use event::{AuditEvent, EmitOutcome};

// The paths the binary and the config tree consume (`crate::system_log::sender::start`,
// `ehrbase::system_log::{AuditConfig, AuditHandle, AuditSender, SubjectResolver}`).

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
    /// Enqueue a resolved audit event on the system log (non-blocking).
    /// Returns [`EmitOutcome::Dropped`] when no sender is wired, and the
    /// sender's own outcome otherwise (see [`AuditSender::emit`]).
    #[must_use]
    pub fn emit(&self, event: AuditEvent) -> EmitOutcome {
        // `map_or` (not `map(..).unwrap_or(..)`) keeps clippy happy; behaviour
        // is identical to `Dropped` when no sender is wired.
        self.audit
            .as_ref()
            .map_or(EmitOutcome::Dropped, |s| s.emit(event))
    }

    /// Whether ATNA auditing is on (a sender is wired and its master switch set).
    pub fn audit_enabled(&self) -> bool {
        self.audit.as_ref().is_some_and(AuditSender::enabled)
    }

    /// Whether login / application-activity events are suppressed (the
    /// deployment default; see [`config::AuditConfig::suppress_login_events`]).
    pub fn suppress_login_events(&self) -> bool {
        self.audit
            .as_ref()
            .is_some_and(AuditSender::suppress_login_events)
    }
}
