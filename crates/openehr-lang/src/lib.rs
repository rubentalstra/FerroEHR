//! openEHR LANG: the BMM / P_BMM object model, generated from the BMM meta-model. The generator's own BMM reader lives in openehr-codegen (tooling, not spec); the runtime ODIN and EL parsers are future hand-written work (P8/P9).
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

pub mod beom;
pub mod bmm;
pub mod bmm3;
pub mod bmm_persistence;
pub mod prelude;

// hand-written modules (spec behaviour), auto-declared:
pub mod bel;
pub mod odin;
