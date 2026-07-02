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
use super::dv_uri::DvUriData;
use crate::data_types::data_value::DataValueApi;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class, single-sourced
/// into its [`TypeName`] impl (ADR-002).
pub const TYPE_NAME: &str = "DV_EHR_URI";

/// `_Ehr_scheme_`: `String` = `"ehr"`.
///
/// Symbolic definition from `master10-uri_package.adoc` §Definitions, used
/// by the `Scheme_valid` invariant below.
pub const EHR_SCHEME: &str = "ehr";

/// `DV_EHR_URI` inherits `DV_URI` (a concrete class) and declares no
/// attributes of its own — only the `Scheme_valid` invariant narrows its
/// legal values. Per the embedding shape established for `DV_TEXT`/
/// `DV_CODED_TEXT`, this struct embeds the shared [`DvUriData`] state
/// (composition), rather than duplicating its `value` attribute.
///
/// PORT NOTE (ADR-002): this previously embedded the concrete [`super::
/// dv_uri::DvUri`] wrapper itself. Once `DvUri` self-tags (its own
/// `TypeTag<DvUri>` serializing `"DV_URI"`), flattening it here would emit
/// a duplicate, contradictory `_type` key beside this class's own
/// `"DV_EHR_URI"` tag. Reshaped to flatten the *untagged* `DvUriData`
/// instead — the exact `DvCodedText`-flattens-`DvTextData` pattern (only
/// embedded `*Data` structs are flattened; concrete self-tagged classes
/// never are). The wire shape is unchanged and schema-verified against
/// `openehr_rm_1.1.0_all.json`'s `DV_EHR_URI` definition: `value` sits
/// directly alongside `_type` with no nested `"uri"` object.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DvEhrUri {
    /// Canonical `_type` discriminator (`"DV_EHR_URI"`), always serialized
    /// first (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `DV_URI` state (the single `value` attribute) and
    /// behaviour, via the untagged shared-state struct — see the
    /// struct-level PORT NOTE.
    #[serde(flatten)]
    pub uri: DvUriData,
}

impl TypeName for DvEhrUri {
    const NAME: &'static str = TYPE_NAME;
}

impl DvEhrUri {
    /// `Scheme_valid`: `scheme.is_equal(Ehr_scheme)`.
    pub fn invariant_scheme_valid(&self) -> bool {
        self.uri.scheme() == EHR_SCHEME
    }
}

impl DataValueApi for DvEhrUri {
    fn type_name(&self) -> &'static str {
        TYPE_NAME
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ehr_uri_scheme_invariant_uses_dv_uri_component_parser() {
        let ehr_uri = DvEhrUri {
            type_tag: TypeTag::new(),
            uri: DvUriData {
                value: "ehr:/ehr_id/composition/path".to_string(),
            },
        };
        let http_uri = DvEhrUri {
            type_tag: TypeTag::new(),
            uri: DvUriData {
                value: "https://example.org/ehr_id".to_string(),
            },
        };

        assert!(ehr_uri.invariant_scheme_valid());
        assert!(!http_uri.invariant_scheme_valid());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.uri — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_ehr_uri.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master10-uri_package.adoc §Class Descriptions / dv_ehr_uri.adoc §DV_EHR_URI Class; §Syntaxes for the ehr:// path grammar
//   confidence: high
//   todos: 0
//   note: embeds DvUriData by composition (single `uri` field); Scheme_valid now delegates to DvUriData::scheme(), which parses RFC3986 components while preserving the spec's plain-text URI allowance. P4/ADR-002: self-tags via TypeTag<Self> first field + TypeName ("DV_EHR_URI"); embedded field reshaped from the concrete DvUri to the untagged DvUriData so the flatten cannot emit a duplicate _type key beside this class's own tag (see the struct-level PORT NOTE) — wire shape unchanged, schema-verified.
// ─────────────────────────────────────────────
