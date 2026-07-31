//! The ITS-REST **Simplified Formats** wire adapter (STABLE —
//! `docs/specs/openehr/ITS-REST/docs/simplified_formats/`): FLAT + STRUCTURED
//! data-instance JSON and their media types
//! (`application/openehr.wt.flat+json`, `application/openehr.wt.structured+json`,
//! `master02 §MIME Types`).
//!
//! The conversion engine lives in `openehr_its::flat`; this module is the
//! wire seam ([`dispatch`]: COMPOSITION FLAT/STRUCTURED I/O, the CONTRIBUTION
//! envelope rule, and the uniform reject for spec-silent resources), composed
//! with the negotiation core in [`crate::overview::negotiate`].

pub mod dispatch;
