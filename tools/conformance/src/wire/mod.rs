//! The spec-grade wire layer — the single place response headers, `ETags`, and
//! openEHR identifiers are parsed.
//!
//! Founding evidence: the ITS-REST overview made
//! `ETags` weak-type (`W/"…"`), and because case setups scraped ids out of
//! headers with ad-hoc helpers, that single client-side wire change silently
//! corrupted 22 case setups into empty-body 404s that looked like server
//! failures. The cure is structural: suites obtain wire ids ONLY through
//! this module; every parse classifies the observed form on the edition
//! ladder ([`crate::edition`]) so a wire-form change surfaces as an explicit
//! edition finding, never as silent rot.

pub mod headers;
pub mod ids;
pub mod negotiate;
