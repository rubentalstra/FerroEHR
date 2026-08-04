//! openEHR AM (Archetype Model): am14 (AM 1.4.0, for ADL 1.4) and am24 (AM 2.4.0, for ADL 2) — both generated from BMM. Both ADL versions are in use.
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

pub mod am14;
pub mod am24;

/// The openEHR specification version this crate implements.
///
/// The pin is emitted by `openehr-codegen` from the vendored inputs and is
/// deliberately independent of the crates.io package version, which is the
/// crate's own `SemVer` line and moves only with this implementation's code.
pub const SPEC_VERSION: &str = "2.4.0";

// canonical-JSON `serde` impls (openehr-codegen -- emit-json), auto-declared:
mod json_serde;
