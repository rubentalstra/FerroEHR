//! The SM System Log component (`I_SYSTEM_LOG`).

/// The SM System Log component (`I_SYSTEM_LOG`, `i_system_log.adoc`) — an empty
/// stub in the vendored spec whose only normative statement is the platform
/// overview's "IHE ATNA-compliant system log" (`master02-overview.adoc`).
/// Realized by the `ehrbase-audit` crate (DICOM `AuditMessage` over syslog,
/// `docs/enterprise/atna-audit.md`); this marker names the component in the SM
/// map.
pub trait SystemLog: Send + Sync {}
