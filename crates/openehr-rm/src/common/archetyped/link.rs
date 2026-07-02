//! `LINK` — a logical relationship between two archetyped structures.
//!
//! openEHR class: `LINK` (concrete), package `common.archetyped`.
//!
//! The `LINK` type defines a logical relationship between two items, such
//! as two `ENTRY`s or an `ENTRY` and a `COMPOSITION`. Links can be used
//! across compositions, and across EHRs. Links can potentially be used
//! between interior (i.e. non archetype root) nodes, although this
//! probably should be prevented in archetypes. Multiple `LINK`s can be
//! attached to the root object of any archetyped structure to give the
//! effect of a 1→N link.
//!
//! 1:1 and 1:N relationships between archetyped content elements (e.g.
//! `ENTRY`s) can be expressed by using one, or more than one,
//! respectively, `LINK`s. Chains of links can be used to see "problem
//! threads" or other logical groupings of items.
//!
//! Links should be between archetyped structures only, i.e. between
//! objects representing complete domain concepts because relationships
//! between sub-elements of whole concepts are not necessarily meaningful,
//! and may be downright confusing. Sensible links only exist between whole
//! `ENTRY`s, `SECTION`s, `COMPOSITION`s and so on.

// TODO(port): `DV_TEXT` and `DV_EHR_URI` are RM 1.1.0 `data_types.text`
// and `data_types.uri` respectively, transcribed by a sibling agent in
// this same phase but not yet landed in this worktree. Forward-references
// to their eventual module paths.
use crate::data_types::text::dv_text::DvText;
use crate::data_types::uri::dv_ehr_uri::DvEhrUri;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class in serialized
/// form. Single-sources the [`TypeName`] impl below (ADR-002).
pub const TYPE_NAME: &str = "LINK";

/// `LINK` declares no `Inherit` row in the spec table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    /// Canonical `_type` discriminator (`"LINK"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// `meaning`: `DV_TEXT`, cardinality `1..1`.
    ///
    /// Used to describe the relationship, usually in clinical terms, such
    /// as "in response to" (the relationship between test results and an
    /// order), "follow-up to" and so on. Such relationships can represent
    /// any clinically meaningful connection between pieces of information.
    /// Values for meaning include those described in Annex C, ENV 13606
    /// pt 2 under the categories of "generic", "documenting and
    /// reporting", "organisational", "clinical", "circumstantial", and
    /// "view management".
    pub meaning: DvText,

    /// `type`: `DV_TEXT`, cardinality `1..1`.
    ///
    /// The type attribute is used to indicate a clinical or domain-level
    /// meaning for the kind of link, for example "problem" or "issue". If
    /// type values are designed appropriately, they can be used by the
    /// requestor of EHR extracts to categorise links which must be
    /// followed and which can be broken when the extract is created.
    ///
    /// PORT NOTE: named `r#type` because `type` is a Rust reserved
    /// keyword, matching the same convention used for `OBJECT_REF.type`
    /// (`openehr_base::identification::object_ref::ObjectRef::r#type`).
    /// The eventual serde attribute (P4) should carry
    /// `#[serde(rename = "type")]` to restore the spec's exact snake_case
    /// attribute name on the wire, per `PORT_MASTER_PLAN.md` §14.4
    /// ("serde: snake_case attribute names").
    #[serde(rename = "type")]
    pub r#type: DvText,

    /// `target`: `DV_EHR_URI`, cardinality `1..1`.
    ///
    /// The logical "to" object in the link relation, as per the
    /// linguistic sense of the meaning attribute.
    pub target: DvEhrUri,
}

impl TypeName for Link {
    const NAME: &'static str = TYPE_NAME;
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.archetyped — docs/research/spec-cache/RM-1.1.0/uml_classes/link.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master03-archetyped_package.adoc §Class Definitions / uml_classes/link.adoc §LINK Class
//   confidence: high
//   todos: 0
//   note: type field named r#type per the reserved-keyword convention, with serde(rename = "type") restoring the spec attribute name on the wire. Forward-refs DvText and DvEhrUri (data_types, sibling-agent territory, not yet landed). No invariants published for this class. P4/ADR-002: self-tags via TypeName + first-field TypeTag<Self> (_type = "LINK").
// ─────────────────────────────────────────────
