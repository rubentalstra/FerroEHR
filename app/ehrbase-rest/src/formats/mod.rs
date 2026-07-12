//! The ITS-REST **Simplified Formats** specification (development edition,
//! STABLE) — `docs/specs/openehr/ITS-REST/docs/simplified_formats/`
//! (FLAT + STRUCTURED web-template JSON) and its media types
//! (`application/openehr.wt.flat+json`, `application/openehr.wt.structured+json`,
//! master02 §MIME types).
//!
//! Register: `docs/design/its-rest/formats.md`. The converters themselves
//! live in `crates/openehr-flat` (the engine); this module is the wire seam
//! ([`dispatch`]: FLAT/STRUCTURED composition I/O + the template example
//! endpoint), composed with the negotiation in
//! [`crate::overview::negotiate`].

pub mod dispatch;
