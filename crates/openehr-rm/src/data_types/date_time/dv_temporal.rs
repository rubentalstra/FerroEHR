//! `DV_TEMPORAL` — specialised temporal variant of `DV_ABSOLUTE_QUANTITY`.
//!
//! openEHR class: `DV_TEMPORAL` (abstract), package
//! `rm.data_types.quantity.date_time`.
//! Inherits: `DV_ABSOLUTE_QUANTITY`.
//!
//! Specialised temporal variant of `DV_ABSOLUTE_QUANTITY` whose diff type is
//! `DV_DURATION`. This is the shared abstract ancestor of `DV_DATE`,
//! `DV_TIME`, and `DV_DATE_TIME` — it fixes the previously-open `DV_AMOUNT`
//! diff/accuracy type of `DV_ABSOLUTE_QUANTITY` down to `DV_DURATION`
//! specifically, and declares the `add`/`subtract`/`diff` signatures the
//! three concrete date/time classes each redefine with their own return
//! type.
//!
//! # Forward references
//!
//! `DV_ABSOLUTE_QUANTITY` is transcribed in the sibling `quantity` package
//! cluster (not yet landed as of this file), so its embeddable state struct
//! is forward-referenced by path per the invoking task's instruction. The
//! quantity cluster and this date_time cluster are being transcribed by
//! concurrent agents; do not attempt to resolve or stub the quantity types
//! here.
use crate::data_types::date_time::dv_duration::DvDuration;
use crate::data_types::quantity::dv_absolute_quantity::DvAbsoluteQuantityData;
use crate::data_types::quantity::dv_ordered::DvOrderedApi;

/// Embedded parent state for `DV_TEMPORAL`'s attributes.
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct + marker
/// trait), every concrete `DV_TEMPORAL` subtype (`DvDate`, `DvTime`,
/// `DvDateTime`) embeds this struct rather than inheriting from it.
///
/// `DV_TEMPORAL` itself embeds `DV_ABSOLUTE_QUANTITY`'s state (via
/// `DvAbsoluteQuantityData`) and narrows/redefines the inherited `accuracy`
/// attribute from the parent's open `DV_AMOUNT` type down to `DV_DURATION`
/// specifically — an ADR-001 §6 covariant redefinition one level up the
/// hierarchy from the concrete classes.
///
/// PORT NOTE: `T: DvOrderedApi` continues the same F-bounded self-type
/// threading used throughout the `quantity` cluster
/// (`DvOrderedData<T>`/`DvQuantifiedData<T>`/`DvAbsoluteQuantityData<T>`,
/// see `dv_ordered.rs`): each concrete leaf (`DvDate`, `DvTime`,
/// `DvDateTime`) instantiates `DvTemporalData<Self>` so its inherited
/// `normal_range`/`other_reference_ranges` narrow to `DV_INTERVAL<Self>` /
/// `REFERENCE_RANGE<Self>` exactly as each leaf's own spec table requires.
/// The parameter was missing in the original concurrent-worktree
/// transcription (the quantity cluster had not landed); supplying the
/// self-type — rather than a fixed carrier such as the accuracy type — is
/// the only choice that keeps the range attributes spec-typed per leaf.
#[derive(Debug, Clone, PartialEq)]
pub struct DvTemporalData<T: DvOrderedApi> {
    /// Embedded `DV_ABSOLUTE_QUANTITY` state (in turn embedding
    /// `DV_QUANTIFIED` → `DV_ORDERED` → `DV_ORDERED` → `DATA_VALUE`, per that
    /// cluster's own transcription).
    ///
    /// PORT NOTE: the spec's own `accuracy` attribute (declared abstract on
    /// `DV_ABSOLUTE_QUANTITY` as `DV_AMOUNT`, `0..1`, `(redefined)`) is
    /// re-redefined here to `DV_DURATION` per the ADR-001 §6 covariant
    /// redefinition rule — see `accuracy` below rather than inside the
    /// embedded `DvAbsoluteQuantityData`, since Rust field embedding cannot
    /// itself narrow a field's declared type without shadowing.
    pub quantified: DvAbsoluteQuantityData<T>,

    /// `accuracy`: `DV_DURATION` (`0..1`, redefined from
    /// `DV_ABSOLUTE_QUANTITY.accuracy: DV_AMOUNT`).
    ///
    /// Time accuracy, expressed as a duration.
    ///
    /// This is the covariant narrowing named in the class table
    /// (`0..1 (redefined)`): the parent's `accuracy: DV_AMOUNT` is fixed down
    /// to `accuracy: DV_DURATION` for every `DV_TEMPORAL` descendant,
    /// encoded directly on this struct per ADR-001 §6 rather than via
    /// generic parameterization.
    pub accuracy: Option<DvDuration>,
}

/// `DV_TEMPORAL` is modelled as a Rust trait, requiring the embedded-state
/// accessor plus the three abstract-turned-effected temporal arithmetic
/// functions every concrete descendant redefines with its own return type
/// (`DvDate::add` returns `DvDate`, not `DvTemporal`, per that class's own
/// `(redefined)` marker — see `dv_date.rs`).
///
/// PORT NOTE: `DvOrderedApi` is a supertrait per the spec's own inheritance
/// chain (`DV_TEMPORAL` → `DV_ABSOLUTE_QUANTITY` → `DV_QUANTIFIED` →
/// `DV_ORDERED`); the intermediate `DvAbsoluteQuantityApi<T>` /
/// `DvQuantifiedApi<T>` traits are *not* supertraits here because their
/// magnitude-carrier parameter `T: OrderedNumeric` differs per concrete
/// descendant (`DV_DATE.magnitude(): Integer` vs `DV_TIME`/`DV_DATE_TIME`'s
/// `Real`) and cannot be fixed at this level without picking one carrier
/// for all three.
pub trait DvTemporal: DvOrderedApi {
    /// Access to the embedded `DV_TEMPORAL` state (which itself embeds
    /// `DV_ABSOLUTE_QUANTITY`'s state), self-typed per the F-bounded
    /// threading documented on [`DvTemporalData`].
    fn temporal_data(&self) -> &DvTemporalData<Self>
    where
        Self: Sized;

    /// `add` __alias__ `"+"` `(a_diff: DV_DURATION[1]): DV_TEMPORAL` (effected
    /// at this level, further redefined by each concrete descendant).
    ///
    /// Addition of a Duration to this temporal entity.
    ///
    /// TODO(port): the class table marks this `(effected)` at the
    /// `DV_TEMPORAL` level (i.e. it has a concrete body here, not just an
    /// abstract declaration), but the description gives no arithmetic
    /// detail beyond the concrete `DvDate`/`DvTime`/`DvDateTime` overrides.
    /// Left as a default trait method delegating to `todo!()` since a
    /// faithful shared implementation would require the ISO 8601
    /// calendar/clock arithmetic engine deferred to the jiff-bridging pass
    /// (see `openehr_foundation::time::iso8601_type` module doc, P17).
    fn add(&self, a_diff: &DvDuration) -> Self
    where
        Self: Sized,
    {
        let _ = a_diff;
        todo!(
            "DV_TEMPORAL.add: ISO 8601 temporal arithmetic deferred to the jiff-backed engine at P17"
        )
    }

    /// `subtract` __alias__ `"-"` `(a_diff: DV_DURATION[1]): DV_TEMPORAL`
    /// (effected).
    ///
    /// Subtract a Duration from this temporal entity.
    ///
    /// TODO(port): see `add` above — deferred to the jiff-backed engine.
    fn subtract(&self, a_diff: &DvDuration) -> Self
    where
        Self: Sized,
    {
        let _ = a_diff;
        todo!(
            "DV_TEMPORAL.subtract: ISO 8601 temporal arithmetic deferred to the jiff-backed engine at P17"
        )
    }

    /// `diff` __alias__ `"-"` `(other: DV_TEMPORAL[1]): DV_DURATION`
    /// (effected).
    ///
    /// Difference between this temporal entity and `other`.
    ///
    /// TODO(port): see `add` above — deferred to the jiff-backed engine.
    fn diff(&self, other: &Self) -> DvDuration
    where
        Self: Sized,
    {
        let _ = other;
        todo!(
            "DV_TEMPORAL.diff: ISO 8601 temporal arithmetic deferred to the jiff-backed engine at P17"
        )
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.date_time — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_temporal.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master07-date_time_package.adoc §Class Descriptions / dv_temporal.adoc §DV_TEMPORAL Class
//   confidence: medium
//   todos: 3
//   note: abstract class, embedded-Data + trait per ADR-001 §3; accuracy narrowed DV_AMOUNT->DV_DURATION is a covariant redefinition one level above the concrete DvDate/DvTime/DvDateTime classes (ADR-001 §6); add/subtract/diff are stubbed todo!() pending the jiff-backed ISO 8601 arithmetic engine (P17); DvTemporalData<T> now threads the quantity cluster's F-bounded self-type and DvTemporal requires DvOrderedApi per the spec inheritance chain.
// ─────────────────────────────────────────────
