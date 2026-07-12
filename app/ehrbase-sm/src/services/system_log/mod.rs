//! The SM **System Log** component (`I_SYSTEM_LOG`).
//!
//! The vendored SM defines this component almost entirely by one normative
//! line — "System Log | IHE ATNA-compliant system log"
//! (`master02-overview.adoc` §openEHR Platform Model) — and an **empty**
//! interface stub (`i_system_log.adoc` names `I_SYSTEM_LOG` with no methods;
//! a recorded spec gap). The event model and emit contract in [`service`] are
//! therefore our own designed realization of that mandate: the
//! transport-agnostic audit event ([`AuditEvent`]) and the [`SystemLog`]
//! seam the protocol adapter's audit middleware hands events to. The IHE
//! ATNA rendering (DICOM `AuditMessage`, syslog framing, transports) is the
//! platform crate's concern (`ehrbase::system_log`).

pub mod service;

pub use service::{AuditEvent, EmitOutcome, EventActionCode, EventOutcome, ObjectClass, SystemLog};
