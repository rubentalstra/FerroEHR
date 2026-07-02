//! `DV_BOOLEAN` — items which are truly boolean data.
//!
//! openEHR class: `DV_BOOLEAN`, package `rm.data_types.basic`.
//! Inherits: `DATA_VALUE`.
//!
//! Items which are truly boolean data, such as true/false or yes/no
//! answers. For such data, it is important to devise the meanings (usually
//! questions in subjective data) carefully, so that the only allowed
//! results are in fact true or false.
//!
//! Misuse: `DV_BOOLEAN` should not be used as a replacement for naively
//! modelled enumerated types such as male/female etc. Such values should be
//! coded, and in any case the enumeration often has more than two values.
use crate::data_types::data_value::DataValueApi;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class, single-sourced
/// into its [`TypeName`] impl (ADR-002).
pub const TYPE_NAME: &str = "DV_BOOLEAN";

/// `DV_BOOLEAN` is a leaf, non-abstract class with a single attribute, so
/// it is transcribed as a plain struct embedding `DATA_VALUE`'s (empty)
/// state.
///
/// The spec's `value` attribute is declared of foundation-types `Boolean`;
/// per `docs/PORTING.md` §14.2 and the `AuthoredResource.is_controlled`
/// precedent (`crates/openehr-base/src/resource/authored_resource.rs`), an
/// ordinary RM/BASE attribute of spec type `Boolean` maps directly to
/// `std::primitive::bool`, not the foundation-types `Boolean` newtype
/// (which is reserved for the foundation-types class itself, e.g. when one
/// foundation-types class embeds another).
///
/// PORT NOTE (ADR-002): this class self-tags. The former struct-level
/// `#[serde(rename = "DV_BOOLEAN")]` was verified by direct experiment to
/// be a **no-op on the wire** and is deleted per ADR-002; the canonical
/// `_type: "DV_BOOLEAN"` property is instead emitted by the [`TypeTag`]
/// first field below (tolerated-absent and validated-if-present on input),
/// which also drives `#[serde(untagged)]` dispatch in the enclosing
/// `DataValue` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DvBoolean {
    /// Canonical `_type` discriminator (`"DV_BOOLEAN"`), always serialized
    /// first (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// `value`: `Boolean` (`1..1`).
    ///
    /// Boolean value of this item. Actual values may be language or
    /// implementation dependent.
    pub value: bool,
}

impl TypeName for DvBoolean {
    const NAME: &'static str = TYPE_NAME;
}

impl DataValueApi for DvBoolean {
    fn type_name(&self) -> &'static str {
        TYPE_NAME
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.basic — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_boolean.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-basic_package.adoc §Class Descriptions / dv_boolean.adoc §DV_BOOLEAN Class
//   confidence: high
//   todos: 0
//   note: leaf struct, single attribute, no invariants published; `value` transcribed as `bool`, not the foundation-types `Boolean` newtype (see doc note). P4/ADR-002: self-tags via TypeTag<Self> first field + TypeName ("DV_BOOLEAN"); inert struct-level #[serde(rename)] deleted.
// ─────────────────────────────────────────────
