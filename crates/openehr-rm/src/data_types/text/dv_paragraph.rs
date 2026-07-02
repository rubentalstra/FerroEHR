//! `DV_PARAGRAPH` — a logical composite text value made of `DV_TEXT` items.
//!
//! openEHR class: `DV_PARAGRAPH`, package `rm.data_types.text`.
//! Inherits: `DATA_VALUE`.
//!
//! DEPRECATED: use markdown formatted `DV_TEXT` instead.
//!
//! Original definition: a logical composite text value consisting of a
//! series of `DV_TEXT`s, i.e. plain text (optionally coded) potentially
//! with simple formatting, to form a larger tract of prose, which may be
//! interpreted for display purposes as a paragraph.
//!
//! `DV_PARAGRAPH` is the standard way for constructing longer text items in
//! summaries, reports and so on.
//!
//! WARNING (`master05-text_package.adoc` §Formatting and Hyperlinking):
//! `DV_PARAGRAPH` is deprecated as of RM Release 1.0.4, in favour of plain
//! `DV_TEXT`/`DV_CODED_TEXT` with markdown formatting and inline links.
//! For legacy reasons it remains legal, and should be supported in at
//! least a basic way.
use crate::data_types::data_value::DataValueApi;
use crate::data_types::text::dv_text::DvText;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class, single-sourced
/// into its [`TypeName`] impl (ADR-002).
pub const TYPE_NAME: &str = "DV_PARAGRAPH";

/// `DV_PARAGRAPH` is a leaf, non-abstract class with one attribute.
///
/// `items` is declared `List<DV_TEXT>` in the spec, and per the "Design"
/// narrative in `master05-text_package.adoc` ("i.e. plain text (optionally
/// coded)"), each element may genuinely be either a bare `DV_TEXT` or a
/// `DV_CODED_TEXT` — this is the one load-bearing use site in this whole
/// cluster for the `DV_TEXT`/`DV_CODED_TEXT` substitutability the
/// [`crate::data_types::text::dv_text::DvText`] enum exists to encode.
/// Transcribed as `Vec<DvText>`, not `Vec<DvTextData>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DvParagraph {
    /// Canonical `_type` discriminator (`"DV_PARAGRAPH"`), always
    /// serialized first (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// `items`: `List<DV_TEXT>` (`1..1`).
    ///
    /// Items making up the paragraph, each of which is a text item (which
    /// may have its own formatting, and/or have hyperlinks).
    ///
    /// Invariant `Items_valid`: `not items.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl; recorded here as a doc note pending the RM invariant
    /// framework.
    pub items: Vec<DvText>,
}

impl TypeName for DvParagraph {
    const NAME: &'static str = TYPE_NAME;
}

impl DvParagraph {
    /// `Items_valid`: `not items.is_empty`.
    ///
    /// TODO(port): wire into a `Validate` impl once the RM invariant
    /// framework lands.
    pub fn invariant_items_valid(&self) -> bool {
        !self.items.is_empty()
    }
}

impl DataValueApi for DvParagraph {
    fn type_name(&self) -> &'static str {
        TYPE_NAME
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.text — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_paragraph.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-text_package.adoc §Class Descriptions / dv_paragraph.adoc §DV_PARAGRAPH Class
//   confidence: high
//   todos: 2
//   note: deprecated-but-still-legal class; `items` typed Vec<DvText> (the enum) to preserve DV_TEXT/DV_CODED_TEXT mixed-list substitutability, not Vec<DvTextData>; Items_valid invariant mentioned on both the field doc and the invariant method doc. P4/ADR-002: self-tags via TypeTag<Self> first field + TypeName ("DV_PARAGRAPH"); inert struct-level #[serde(rename)] deleted; each items element carries its own _type via the DvText variants' tags.
// ─────────────────────────────────────────────
