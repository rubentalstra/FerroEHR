//! openEHR RM (Reference Model), generated from the BMM meta-model.
//!
//! @generated module tree by openehr-codegen. The type files
//! are generated; hand-written spec behaviour lives in sibling `*_impl.rs`.

#![allow(
    clippy::doc_markdown,
    clippy::doc_link_with_quotes,
    clippy::tabs_in_doc_comments,
    clippy::doc_lazy_continuation,
    clippy::doc_overindented_list_items,
    clippy::struct_excessive_bools,
    clippy::struct_field_names,
    clippy::module_inception,
    clippy::large_enum_variant,
    clippy::enum_variant_names,
    reason = "inherent to faithful openEHR spec generation: verbatim spec prose in doc comments, and spec-owned class/variant/field names (a field name IS the normative BMM attribute name)"
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
