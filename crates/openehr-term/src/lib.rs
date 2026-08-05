//! openEHR TERM (Terminology) data model, generated from the BMM meta-model. The vendored terminology XML content lives in `assets/` (data, not generated); an XML→model loader is added when composition validation needs it.
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

pub mod prelude;
pub mod v3_1;

/// The openEHR specification version this crate implements.
///
/// The pin is emitted by `openehr-codegen` from the vendored inputs and is
/// deliberately independent of the crates.io package version, which is the
/// crate's own `SemVer` line and moves only with this implementation's code.
pub const SPEC_VERSION: &str = "3.1.0";

/// The BMM generations this crate emits, one variant per version module,
/// oldest first.
///
/// Generated from the openehr-codegen composition table — the single
/// authority for which generations exist. [`std::fmt::Display`] and
/// [`std::str::FromStr`] round-trip the generation-module name (`"v3_1"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Generation {
    /// The `v3_1` generation — openEHR specification version 3.1.0.
    V3_1,
}

impl Generation {
    /// The crate's CURRENT generation — the one `crate::prelude` re-exports.
    pub const CURRENT: Self = Self::V3_1;

    /// Every generation this crate emits, oldest first.
    pub const ALL: &'static [Self] = &[Self::V3_1];

    /// The openEHR specification version this generation implements.
    #[must_use]
    pub const fn spec_version(self) -> &'static str {
        match self {
            Self::V3_1 => "3.1.0",
        }
    }

    /// The generation-module name (`"v1_2"`) — the
    /// [`std::fmt::Display`]/[`std::str::FromStr`] token.
    #[must_use]
    pub const fn module(self) -> &'static str {
        match self {
            Self::V3_1 => "v3_1",
        }
    }
}

impl std::fmt::Display for Generation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.module())
    }
}

/// Error returned when parsing a [`Generation`] from an unknown token.
///
/// The valid tokens are the generation-module names (`v3_1`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationParseError {
    unrecognized: String,
}

impl std::fmt::Display for GenerationParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown generation {:?} (valid: `v3_1`)",
            self.unrecognized
        )
    }
}

impl std::error::Error for GenerationParseError {}

impl std::str::FromStr for Generation {
    type Err = GenerationParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "v3_1" => Ok(Self::V3_1),
            other => Err(GenerationParseError {
                unrecognized: other.to_owned(),
            }),
        }
    }
}

// hand-written modules (spec behaviour), auto-declared:
pub mod bundle;
pub mod measurement;

// canonical-JSON `serde` impls (openehr-codegen -- emit-json), auto-declared:
mod json_serde;
