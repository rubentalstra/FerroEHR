//! IHE **ATNA** (Audit Trail and Node Authentication) audit trail for the
//! ehrbase-rs CDR.
//!
//! Emits one **DICOM Audit Message** (DICOM PS3.15 §A.5 — *not* openEHR
//! ITS-XML) per audited API operation, shipped to an Audit Record Repository
//! over **syslog** (RFC 5424 framing; RFC 5426 UDP or RFC 5425 TLS). This is
//! authorized defensive security-audit logging for a healthcare system; see
//! `docs/enterprise/atna-audit.md` for the behavioural spec (§1–7) and the
//! implementation binding (§8, which governs this crate).
//!
//! The crate is a pure leaf: it depends only on `quick-xml`, `tokio`,
//! `tokio-rustls`/`rustls`, `jiff`, `serde`/`figment`, and the observability
//! crates — **no dependency on any `ehrbase-*` or `openehr-*` crate**. The REST
//! layer (`ehrbase-rest`) builds an [`AuditEvent`] and [`emit`](AuditSender::emit)s
//! it; the binary (`ehrbase`) boots the [`AuditSender`] and supplies the
//! DB-backed [`SubjectResolver`].
//!
//! ## Module map (§8.1)
//! - [`message`] — the DICOM `AuditMessage` model + `quick-xml` serializer.
//! - [`codes`] — DCM / RFC-3881 code constants.
//! - [`event`] — the transport-agnostic [`AuditEvent`].
//! - [`table`] — operation id → audit classification (+ total-coverage guard).
//! - [`syslog`] — RFC 5424 assembly + RFC 5426 UDP / RFC 5425 TLS transports.
//! - [`sender`] — the bounded-mpsc sender + background drain + fail modes.
//! - [`config`] — the `figment` [`AuditConfig`].

pub mod codes;
pub mod config;
pub mod event;
pub mod message;
pub mod sender;
pub mod syslog;
pub mod table;

pub use config::{AuditConfig, FailMode, Transport};
pub use event::{AuditEvent, EventActionCode, EventOutcome, ObjectClass};
pub use message::{AuditContext, AuditMessage};
pub use sender::{AuditHandle, AuditSender, EmitOutcome, SubjectResolver, start};
pub use table::{Classification, audit_for, classify};

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
