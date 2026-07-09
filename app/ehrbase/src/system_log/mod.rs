//! IHE **ATNA** (Audit Trail and Node Authentication) audit trail for the
//! ehrbase-rs CDR — the platform-crate implementation of the SM
//! [`SystemLog`](ehrbase_sm::SystemLog) component (`I_SYSTEM_LOG`).
//!
//! Emits one **DICOM Audit Message** (DICOM PS3.15 §A.5 — *not* openEHR
//! ITS-XML) per audited API operation, shipped to an Audit Record Repository
//! over **syslog** (RFC 5424 framing; RFC 5426 UDP or RFC 5425 TLS). This is
//! authorized defensive security-audit logging for a healthcare system; see
//! `docs/enterprise/atna-audit.md` for the behavioural spec (§1–7) and the
//! implementation binding (§8, which governs this module).
//!
//! The transport-agnostic event model ([`AuditEvent`](ehrbase_sm::AuditEvent))
//! and the [`SystemLog`](ehrbase_sm::SystemLog) trait live in the SM native-API
//! crate (`ehrbase-sm`, the empty-stub `I_SYSTEM_LOG` component); this module is
//! the ATNA *rendering* — the DICOM `AuditMessage`, syslog framing, transports,
//! and the non-blocking sender — plus the [`SystemLog`](ehrbase_sm::SystemLog)
//! implementation on [`EhrbaseService`](crate::service::EhrbaseService). The
//! ITS-REST operation → classification mapping is the protocol adapter's
//! concern (`ehrbase-rest::audit_table`). The `ehrbase-rest` layer builds an
//! `AuditEvent` and emits it through the platform (`Platform: … + SystemLog`);
//! the binary (`ehrbase`) boots the [`AuditSender`] and supplies the DB-backed
//! [`SubjectResolver`].
//!
//! ## Module map (§8.1)
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

pub use config::{AuditConfig, FailMode, Transport};
pub use message::{AuditContext, AuditMessage};
pub use sender::{AuditHandle, AuditSender, SubjectResolver, start};

use ehrbase_sm::{AuditEvent, EmitOutcome, SystemLog};

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
impl SystemLog for EhrbaseService {
    fn emit(&self, event: AuditEvent) -> EmitOutcome {
        // `map_or` (not `map(..).unwrap_or(..)`) keeps clippy happy; behaviour
        // is identical to `Dropped` when no sender is wired.
        self.audit
            .as_ref()
            .map_or(EmitOutcome::Dropped, |s| s.emit(event))
    }

    fn audit_enabled(&self) -> bool {
        self.audit.as_ref().is_some_and(AuditSender::enabled)
    }

    fn suppress_login_events(&self) -> bool {
        self.audit
            .as_ref()
            .is_some_and(AuditSender::suppress_login_events)
    }
}
