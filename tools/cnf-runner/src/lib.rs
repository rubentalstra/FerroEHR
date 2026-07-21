//! CNF 2.0 reference runner — the typed schedule-artifact model, validator,
//! and JSON-Schema emission for the openEHR conformance framework.
//!
//! The conformance oracle is the vendored openEHR CNF component
//! (`docs/specs/openehr/CNF/`); the artifact families this crate models are
//! the machine-readable normative form of the Platform Conformance Test
//! Schedule (case cores, operation bindings, vocabularies incl. the
//! capability matrix, corpus manifest, ambiguity register). Every closed
//! vocabulary is a Rust enum/newtype so illegal states are unrepresentable.

pub mod artifacts;
pub mod compare;
pub mod exec;
pub mod ids;
pub mod ixit;
pub mod literal;
pub mod load;
pub mod model;
pub mod refgrammar;
pub mod schema;
pub mod validate;
pub mod vocab;
