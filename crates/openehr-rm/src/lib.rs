//! openEHR RM (Reference Model), generated from the BMM meta-model.
//!
//! @generated module tree by openehr-codegen. The type files
//! are generated; hand-written spec behaviour lives in sibling `*_impl.rs`.

#![allow(
    clippy::doc_markdown,
    clippy::doc_link_with_quotes,
    clippy::tabs_in_doc_comments,
    clippy::doc_lazy_continuation,
    clippy::struct_excessive_bools,
    clippy::module_inception,
    clippy::large_enum_variant
)]

pub mod common;
pub mod composition;
pub mod data_structures;
pub mod data_types;
pub mod demographic;
pub mod ehr;
pub mod ehr_extract;
pub mod integration;
pub mod prelude;
pub mod support;

/// The openEHR specification version this crate implements — the crate
/// version itself: the spec crates are versioned by the specification they
/// implement (`docs/VERSIONS.md` §Product and crate versioning), so
/// consumers read the pin from the package, never from a hand-typed literal.
pub const SPEC_VERSION: &str = env!("CARGO_PKG_VERSION");
pub mod model;

// hand-written modules (spec behaviour), auto-declared:
pub mod paths;
pub mod validate;
