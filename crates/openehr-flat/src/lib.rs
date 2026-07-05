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
pub mod webtemplate;

pub use error::FlatError;
pub use webtemplate::{WebTemplate, build_web_template};
