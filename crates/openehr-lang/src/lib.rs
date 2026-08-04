//! openEHR LANG: the BMM object model in BOTH its extant generations, generated from the BMM meta-model — the stable v2.x model (`bmm`, its `bmm_persistence` P_BMM form and the `beom` expression model) and the v3 development line (`bmm3`, with the `EL_*` expression and `BMM_STATEMENT*` families). Each generation is emitted completely at its own source-package path; the prelude exports one type per Rust name (the v3 twin where both declare a name). The generator's own BMM reader lives in openehr-codegen (tooling, not spec); the hand-written ODIN reader and BEL parser live beside this generated tree.
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
// A vendored BMM model is a deep, mutually-recursive type graph (the LANG // BMM-3 expression/statement families reach several hundred levels), so // auto-trait inference — `Send`/`Sync`/`RefUnwindSafe`, which rustdoc // evaluates for every item — overflows the default limit of 128. Raising // the limit is exactly what rustc prescribes for that overflow // (<https://doc.rust-lang.org/reference/attributes/limits.html>); it // changes no emitted type.
#![recursion_limit = "512"]

pub mod beom;
pub mod bmm;
pub mod bmm3;
pub mod bmm_persistence;
pub mod prelude;

/// The openEHR specification version this crate implements.
///
/// The pin is emitted by `openehr-codegen` from the vendored inputs and is
/// deliberately independent of the crates.io package version, which is the
/// crate's own SemVer line and moves only with this implementation's code.
pub const SPEC_VERSION: &str = "1.0.0";

// hand-written modules (spec behaviour), auto-declared:
pub mod bel;
pub mod escape;
pub mod lexer;
pub mod odin;
pub mod position;

// canonical-JSON `serde` impls (openehr-codegen -- emit-json), auto-declared:
mod json_serde;
