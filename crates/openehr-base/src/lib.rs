//! openEHR BASE (foundation + base types), generated from the BMM meta-model.
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

pub mod base_types;
pub mod foundation_types;
pub mod prelude;
pub mod resource;

// hand-written modules (spec behaviour), auto-declared:
pub mod validate;
