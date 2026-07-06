//! openEHR RM (Reference Model), generated from the BMM meta-model.
//!
//! @generated module tree by openehr-codegen (ADR-004). The type files
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

// hand-written modules (ADR-003 spec behaviour), auto-declared:
pub mod validate;
