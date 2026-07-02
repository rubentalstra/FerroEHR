//! `INTERVAL_EVENT<T>` — a single interval event in a series.
//!
//! openEHR class: `INTERVAL_EVENT<T>`, package `rm.data_structures.history`.
//!
//! Defines a single interval event in a series.

use super::event::{EventApi, EventData};
// PORT NOTE: `CODE_PHRASE`/`DV_CODED_TEXT`/`DV_DATE_TIME`/`DV_DURATION`
// belong to `rm.data_types`, transcribed concurrently by a sibling agent;
// see `representation/element.rs` for the identical forward-reference
// rationale and assumed module paths.
use crate::data_types::date_time::dv_date_time::DvDateTime;
use crate::data_types::date_time::dv_duration::DvDuration;
use crate::data_types::text::dv_coded_text::DvCodedText;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// `INTERVAL_EVENT<T>` class.
///
/// Embeds the shared `EVENT<T>` state (per ADR-001 §3/§5) plus its own
/// attributes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntervalEvent<T> {
    /// Canonical `_type` discriminator (`"INTERVAL_EVENT"`), always
    /// serialized first; tolerated-absent and validated-if-present on input
    /// (ADR-002). Spelled `TypeTag<IntervalEvent<T>>` (not `TypeTag<Self>`)
    /// and paired with the mandatory function-path
    /// `default = "TypeTag::new"` so serde's derive adds no spurious
    /// `T: Default` bound.
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<IntervalEvent<T>>,

    /// Inherited `EVENT<T>` (and transitively `LOCATABLE`) state.
    #[serde(flatten)]
    pub event: EventData<T>,

    /// `width`: duration of the time interval during which the values
    /// recorded under `data` are true and, if set, the values recorded
    /// under `state` are true. Void if an instantaneous event.
    ///
    /// Cardinality `1..1` per the spec table, despite the description's
    /// "Void if an instantaneous event" wording — transcribed literally as
    /// non-optional per the table's own cardinality column, not per the
    /// prose. Flagged as an apparent table/prose inconsistency: an
    /// instantaneous event is exactly what `POINT_EVENT<T>` (a sibling,
    /// not a further `INTERVAL_EVENT<T>` case) already models, so the
    /// "Void if instantaneous" prose may be describing a degenerate case
    /// that in practice never arises for a `1..1` field on this
    /// (necessarily non-instantaneous) class specifically.
    pub width: DvDuration,

    /// `sample_count`: optional count of original samples to which this
    /// event corresponds.
    ///
    /// Cardinality `0..1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_count: Option<i32>,

    /// `math_function`: mathematical function of the data of this event,
    /// e.g. maximum, mean etc. Coded using the openEHR vocabulary `event
    /// math function`. Default value `640|actual|`, meaning 'actual
    /// value'.
    ///
    /// Cardinality `1..1`.
    ///
    /// TODO(port): the spec-stated default value (`640|actual|`) is not
    /// encoded as a Rust `Default` impl here, since `DvCodedText` itself
    /// is a forward reference and constructing a `CODE_PHRASE` for
    /// `640|actual|` needs the terminology binding machinery (see
    /// `openehr-terminology`); revisit once `DvCodedText`/`CodePhrase` land.
    pub math_function: DvCodedText,
}

impl<T> TypeName for IntervalEvent<T> {
    const NAME: &'static str = TYPE_NAME;
}

impl<T> EventApi<T> for IntervalEvent<T> {
    fn event_data(&self) -> &EventData<T> {
        &self.event
    }
}

impl<T> IntervalEvent<T> {
    /// `interval_start_time`: start time of the interval of this event.
    ///
    /// Invariant `Interval_start_time_valid`:
    /// `interval_start_time = time - width`. This invariant is exactly the
    /// function's own defining contract, so the implementation is
    /// definitional (matching the `is_null()` precedent in `element.rs`):
    /// subtract `width` from the inherited `EVENT.time`.
    pub fn interval_start_time(&self) -> DvDateTime {
        // TODO(port): needs `DvDateTime - DvDuration` subtraction, which
        // depends on the not-yet-landed `data_types.date_time` package's
        // arithmetic operations.
        todo!(
            "interval_start_time(): needs DvDateTime - DvDuration subtraction from the data_types package"
        )
    }

    // TODO(port): invariant `Math_function_validity`:
    // `terminology(Terminology_id_openehr).has_code_for_group_id(
    // Group_id_event_math_function, math_function.defining_code)` —
    // requires a `TERMINOLOGY_SERVICE` lookup (see `openehr-terminology`)
    // to verify `math_function.defining_code` is a member of the openEHR
    // `event math function` terminology group, analogous to `ELEMENT`'s
    // `Inv_null_flavour_valid` (see `representation/element.rs`).
}

pub const TYPE_NAME: &str = "INTERVAL_EVENT";

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures.history §INTERVAL_EVENT — docs/research/spec-cache/RM-1.1.0/uml_classes/interval_event.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-history_package.adoc §Class Descriptions / interval_event.adoc §INTERVAL_EVENT Class
//   confidence: medium
//   todos: 3
//   note: width is transcribed as 1..1 per the table's cardinality column despite the description's "Void if instantaneous" wording (flagged); interval_start_time() and the Math_function_validity invariant both block on data_types/terminology dependencies not yet landed. P4/ADR-002: self-tag added (generic form TypeTag<IntervalEvent<T>>, bound-free TypeName impl).
// ─────────────────────────────────────────────
