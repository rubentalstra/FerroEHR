//! FHIR **outbound** emitter — event-driven emission of
//! mapped FHIR resources.
//!
//! A background drainer (wired like the E1 outbox publisher) walks
//! committed `event_outbox` rows through its own cursor
//! (`fhir_outbound_cursor.last_seq`, migration 0006), and for every COMPOSITION
//! version whose template matches an enabled `fhir_mapping` it loads the version
//! via the versioned read seam, reverse-maps it
//! ([`EhrbaseService::fhir_outbound_messages`](crate::service::EhrbaseService::fhir_outbound_messages)),
//! and publishes the FHIR resource JSON to a broker with confirms.
//!
//! ## PHI
//! Unlike the E1 event envelopes (PHI-free by design), the payload
//! here IS the mapped FHIR resource — clinical content by design. It is
//! therefore off by default behind its own [`FhirOutboundConfig::enabled`] flag
//! and published to a **separate** exchange (default `ehrbase.fhir`) so
//! broker-level access control can restrict the PHI stream independently of the
//! PHI-free envelope stream (see [`config`]).
//!
//! ## Module map
//! - [`config`] — the `figment` [`FhirOutboundConfig`].
//! - [`publisher`] — the drainer task + [`FhirOutboundHandle`]. Reuses the
//!   [`EventPublisher`](crate::events::EventPublisher) /
//!   [`AmqpPublisher`](crate::events::AmqpPublisher) broker seam from `events`.

pub mod config;

mod publisher;

pub use config::FhirOutboundConfig;
pub use publisher::{FhirOutboundHandle, start, start_with_publisher};
