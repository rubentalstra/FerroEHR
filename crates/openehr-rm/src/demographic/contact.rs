//! `CONTACT` — a means of contact of a Party.
//!
//! openEHR class: `CONTACT` (concrete), package `rm.demographic`.
//!
//! Description of a means of contact of a Party. Actual structure is
//! archetyped.
use super::address::Address;
use crate::common::archetyped::locatable::LocatableData;
use crate::data_types::quantity::dv_interval::DvInterval;

/// `pub const TYPE_NAME`: the canonical `_type` discriminator string for
/// this concrete class (serde derives deferred to P4/P5 per ADR-001
/// §Refinements).
pub const TYPE_NAME: &str = "CONTACT";

/// `CONTACT` inherits `LOCATABLE` directly.
#[derive(Debug, Clone, PartialEq)]
pub struct Contact {
    /// Inherited `LOCATABLE` state.
    pub locatable: LocatableData,

    /// `addresses`: `List<ADDRESS>` `[1..1]` — a set of address
    /// alternatives for this contact purpose and time validity combination.
    pub addresses: Vec<Address>,

    /// `time_validity`: `DV_INTERVAL<DV_DATE>` `[0..1]` — valid time
    /// interval for this contact descriptor.
    pub time_validity: Option<DvInterval<crate::data_types::date_time::dv_date::DvDate>>,
}

impl Contact {
    /// Spec function `purpose(): DV_TEXT` — purpose for which this contact
    /// is used, e.g. "mail", "daytime phone", etc. Taken from value of the
    /// inherited `name` attribute.
    ///
    /// Invariant `Purpose_valid`: `purpose = name`.
    ///
    /// TODO(port): implement once `LocatableData.name: DvText` is concrete;
    /// this should simply clone `self.locatable.name`.
    pub fn purpose(&self) -> crate::data_types::text::dv_text::DvText {
        todo!("CONTACT.purpose(): DV_TEXT — clone LocatableData.name once concrete")
    }
}

// TODO(port): invariant as a `Validate` impl:
//   - Purpose_valid: purpose = name

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions CONTACT — docs/research/spec-cache/RM-1.1.0/uml_classes/contact.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/contact.adoc §CONTACT Class
//   confidence: high
//   todos: 2
//   note: addresses is REQUIRED (1..1, a List that per spec text is "a set of alternatives" — still typed List<ADDRESS> not Set<ADDRESS> in the table, transcribed literally as Vec).
// ─────────────────────────────────────────────
