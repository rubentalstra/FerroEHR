//! `DV_EHR_URI` — a `DV_URI` scoped to the `ehr` scheme.
//!
//! openEHR class: `DV_EHR_URI`, package `rm.data_types.uri`.
//! Inherits: `DV_URI`.
//!
//! A `DV_EHR_URI` is a `DV_URI` which has the scheme name `'ehr'`, and
//! which can only reference items in EHRs.
//!
//! Used to reference items in an EHR, which may be the same as the current
//! EHR (containing this link), or another.
//!
//! # Syntax
//!
//! Per `master10-uri_package.adoc` §Syntaxes, a `DV_EHR_URI`'s value is an
//! openEHR path inside the `'ehr'` URI scheme-space, one of the forms:
//!
//! ```text
//! ehr://system_id/ehr_id/top_level_structure_locator/path_inside_top_level_structure
//! ehr:/ehr_id
//! ehr:/ehr_id/top_level_structure_locator
//! ehr:/ehr_id/top_level_structure_locator/path_inside_top_level_structure
//! ```
use super::dv_uri::DvUri;
use crate::data_types::data_value::DataValueApi;

/// Canonical `_type` discriminator string for this class in serialized
/// form (ADR-001 Refinements: serde derives wait until P4).
pub const TYPE_NAME: &str = "DV_EHR_URI";

/// `_Ehr_scheme_`: `String` = `"ehr"`.
///
/// Symbolic definition from `master10-uri_package.adoc` §Definitions, used
/// by the `Scheme_valid` invariant below.
pub const EHR_SCHEME: &str = "ehr";

/// `DV_EHR_URI` inherits `DV_URI` (a concrete class) and declares no
/// attributes of its own — only the `Scheme_valid` invariant narrows its
/// legal values. Per the embedding shape established for `DV_TEXT`/
/// `DV_CODED_TEXT` and `DV_URI` itself, this struct embeds [`DvUri`]
/// directly (composition), rather than duplicating its `uri` state.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DvEhrUri {
    /// Embedded `DV_URI` state and behaviour.
    pub uri: DvUri,
}

impl DvEhrUri {
    /// `Scheme_valid`: `scheme.is_equal(Ehr_scheme)`.
    ///
    /// TODO(port): depends on [`DvUri::scheme`], itself `todo!()` pending
    /// an RFC-3986 parser; cannot be evaluated until that lands.
    pub fn invariant_scheme_valid(&self) -> bool {
        self.uri.scheme() == EHR_SCHEME
    }
}

impl DataValueApi for DvEhrUri {
    fn type_name(&self) -> &'static str {
        TYPE_NAME
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.uri — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_ehr_uri.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master10-uri_package.adoc §Class Descriptions / dv_ehr_uri.adoc §DV_EHR_URI Class; §Syntaxes for the ehr:// path grammar
//   confidence: high
//   todos: 1
//   note: embeds DvUri by composition (single `uri` field); Scheme_valid invariant transitively depends on DvUri::scheme(), which is itself a todo!() pending RFC-3986 parsing.
// ─────────────────────────────────────────────
