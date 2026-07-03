//! `CONTACT` — a means of contact of a Party.
//!
//! openEHR class: `CONTACT` (concrete), package `rm.demographic`.
//!
//! Description of a means of contact of a Party. Actual structure is
//! archetyped.
use super::address::Address;
use crate::common::archetyped::locatable::LocatableData;
use crate::data_types::quantity::dv_interval::DvInterval;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// `pub const TYPE_NAME`: the canonical `_type` discriminator string for
/// this concrete class, single-sourcing the [`TypeName`] impl below
/// (ADR-002).
pub const TYPE_NAME: &str = "CONTACT";

/// `CONTACT` inherits `LOCATABLE` directly. `#[serde(flatten)]` folds
/// `LocatableData` into `CONTACT`'s own JSON object; per ADR-002 the class
/// self-tags via its first field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contact {
    /// Canonical `_type` discriminator (`"CONTACT"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Inherited `LOCATABLE` state.
    #[serde(flatten)]
    pub locatable: LocatableData,

    /// `addresses`: `List<ADDRESS>` `[1..1]` — a set of address
    /// alternatives for this contact purpose and time validity combination.
    pub addresses: Vec<Address>,

    /// `time_validity`: `DV_INTERVAL<DV_DATE>` `[0..1]` — valid time
    /// interval for this contact descriptor.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub time_validity: Option<DvInterval<crate::data_types::date_time::dv_date::DvDate>>,
}

impl TypeName for Contact {
    const NAME: &'static str = TYPE_NAME;
}

impl Contact {
    /// Spec function `purpose(): DV_TEXT` — purpose for which this contact
    /// is used, e.g. "mail", "daytime phone", etc. Taken from value of the
    /// inherited `name` attribute.
    ///
    /// Invariant `Purpose_valid`: `purpose = name` — see
    /// [`Contact::invariant_purpose_valid`].
    #[must_use]
    pub fn purpose(&self) -> crate::data_types::text::dv_text::DvText {
        self.locatable.name.clone()
    }

    /// Invariant `Purpose_valid`: `purpose = name` (ADR-003 §8).
    /// Structurally guaranteed by [`Contact::purpose`] (it clones `name`),
    /// evaluated literally here.
    #[must_use]
    pub fn invariant_purpose_valid(&self) -> bool {
        self.purpose() == self.locatable.name
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions CONTACT — docs/research/spec-cache/RM-1.1.0/uml_classes/contact.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/contact.adoc §CONTACT Class
//   confidence: high
//   todos: 0
//   note: addresses is REQUIRED (1..1, a List that per spec text is "a set of alternatives" — still typed List<ADDRESS> not Set<ADDRESS> in the table, transcribed literally as Vec). P4/ADR-002: self-tags via TypeTag<Self> first field (TypeName from TYPE_NAME); no-op struct-level rename deleted.
// ─────────────────────────────────────────────
