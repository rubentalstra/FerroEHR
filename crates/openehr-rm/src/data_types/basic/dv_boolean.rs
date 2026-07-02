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

/// Canonical `_type` discriminator string for this class in serialized
/// form (ADR-001 Refinements: serde derives wait until P4).
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DvBoolean {
    /// `value`: `Boolean` (`1..1`).
    ///
    /// Boolean value of this item. Actual values may be language or
    /// implementation dependent.
    pub value: bool,
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
//   note: leaf struct, single attribute, no invariants published; `value` transcribed as `bool`, not the foundation-types `Boolean` newtype (see doc note).
// ─────────────────────────────────────────────
