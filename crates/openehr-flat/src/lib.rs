//! FLAT (simSDT), STRUCTURED (structSDT), and Web Template formats following
//! Better `web-template` semantics, with `EHRbase` deviations behind the
//! `ehrbase-quirks` feature.
//!
//! The **`WebTemplate` builder** ([`build_web_template`]) turns a parsed OPT 1.4
//! operational template ([`openehr_its::opt14::OperationalTemplate`]) into the
//! Better `web-template` JSON model (format version `"2.3"`). Better's reference
//! implementation (`github.com/better-care/web-template`, Apache-2.0) is the
//! interop oracle: field names, the `id`/`aqlPath` derivation, the RM-type →
//! `inputs` mapping, and the compaction/post-processing shape match it.
//!
//! FLAT / STRUCTURED (P17) build on the same [`webtemplate`] model.

pub mod cache;
pub mod error;
pub mod flat;
pub(crate) mod path;
pub mod structured;
pub mod validation;
pub mod webtemplate;

pub use error::FlatError;
pub use flat::{from_flat, to_flat};
pub use structured::{flat_to_structured, from_structured, structured_to_flat, to_structured};
pub use validation::{
    ValidationKind, ValidationMessage, validate_archetype_conformance, validate_composition,
    validate_rm_and_terminology,
};
pub use webtemplate::{WebTemplate, build_web_template};
