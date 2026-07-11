//! openEHR TERM (Terminology) data model, generated from the BMM meta-model. The vendored terminology XML content lives in `assets/` (data, not generated); an XML→model loader is added when composition validation needs it.
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

pub mod prelude;
pub mod terminology;

// hand-written modules (ADR-003 spec behaviour), auto-declared:
pub mod bundle;
pub mod measurement;
