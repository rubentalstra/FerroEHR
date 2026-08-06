#![recursion_limit = "256"]
//! CNF 2.0 reference runner — the typed schedule-artifact model, validator,
//! and JSON-Schema emission for the openEHR conformance framework.
//!
//! The conformance oracle is the vendored openEHR CNF component
//! (`docs/specs/openehr/CNF/`); the artifact families this crate models are
//! the machine-readable normative form of the Platform Conformance Test
//! Schedule (case cores, operation bindings, vocabularies incl. the
//! capability matrix, corpus manifest, ambiguity register). Every closed
//! vocabulary is a Rust enum/newtype so illegal states are unrepresentable.

// Doctests are copy-paste templates: they must use `?`, never unwrap
// (C-QUESTION-MARK, https://rust-lang.github.io/api-guidelines/documentation.html#c-question-mark).
#![doc(test(attr(deny(warnings))))]
pub mod artifacts;
pub mod badges;
pub mod conf_assets;
pub mod exec;
pub mod ids;
pub mod ixit;
pub mod literal;
pub mod load;
pub mod model;
pub mod party;
pub mod perf;
pub mod perf_assets;
pub mod perf_run;
pub mod probe;
pub mod refgrammar;
pub mod render;
pub mod run;
pub mod schema;
pub mod stress;
pub mod validate;
pub mod verdict;
pub mod vocab;
