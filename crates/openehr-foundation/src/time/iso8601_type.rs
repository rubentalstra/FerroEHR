//! `Iso8601_type` — abstract ancestor type of ISO 8601 types.
//!
//! openEHR class: `Iso8601_type` (abstract), package
//! `base.foundation_types.time`.
//! Inherits: `Temporal`, `Time_Definitions`.
//!
//! Abstract ancestor type of ISO 8601 types, defining interface for
//! 'extended' and 'partial' concepts from ISO 8601.
//!
//! # Multiple inheritance (ADR-001 §2 worked example)
//!
//! This is the class ADR-001 §2 and `docs/ROSETTA.md` name explicitly as the
//! multiple-inheritance worked example alongside `Ordered_Numeric`. Unlike
//! `Ordered_Numeric` (whose two parents, `Ordered` and `Numeric`, are both
//! pure-behaviour traits combined by a blanket-implemented supertrait), this
//! class's two parents are *not* symmetric:
//!
//! * `Temporal` (see `temporal.rs`) is a pure-behaviour marker trait —
//!   itself just `Ordered` with nothing added — so it composes as an
//!   ordinary Rust supertrait bound with no special handling.
//! * `Time_Definitions` (see `time_definitions.rs`) is transcribed as a
//!   zero-sized struct of associated `const`s and `fn`s, not a trait — it
//!   has no instance-level behaviour to abstract over (every member is
//!   either a compile-time constant or a pure function of explicit
//!   arguments). A struct cannot be a Rust supertrait, so this half of the
//!   spec's `Inherit` relation is transcribed as direct calls to
//!   `TimeDefinitions::*` from concrete `Iso8601_type` descendants (e.g.
//!   `Iso8601_date`'s invariants calling `TimeDefinitions::valid_year`),
//!   rather than as a second bound on the `Iso8601Type` trait below. This is
//!   a genuine judgement call, not a settled precedent from
//!   `Ordered_Numeric` — recorded here loudly since a later transcriber
//!   hitting another "inherits a constants-only class" case should treat
//!   this reasoning, not the `OrderedNumeric` blanket-impl pattern, as the
//!   template.
//!
//! Per the RM transcription rule for multiple inheritance where a parent
//! carries attributes ("composition of fields from all parents plus one
//! trait per parent behaviour", ADR-001 §3), the spec attribute `value:
//! String` is embedded directly as state (via `Iso8601TypeCore` below,
//! which every concrete class in this module holds), while the `Iso8601Type`
//! trait supplies the shared abstract behaviour (`as_string`, `is_partial`,
//! `is_extended`) as methods every concrete type must implement or inherit
//! a default for.
//!
//! # String-value representation, not a resolved instant
//!
//! These classes model *ISO 8601 string values with partial precision*
//! (e.g. `"2007-04"`, `"10:30"`), not resolved instants. Each concrete
//! `Iso8601_*` type in this module (`Iso8601Date`, `Iso8601Time`,
//! `Iso8601DateTime`, `Iso8601Duration`, `Iso8601Timezone`) carries its
//! `value: String` representation and exposes the spec's declared accessor/
//! arithmetic functions as methods, with `todo!()` bodies wherever string
//! parsing is required and deferred.
//!
//! PERF(port) / PORT NOTE: the internal engine backing these string-parsing
//! and arithmetic bodies is expected to bridge to `jiff`'s calendar/duration
//! types once wired in at implementation time (P17: make-it-compile / the
//! parity phases). Do NOT add a `jiff` dependency to `openehr-foundation`
//! now — Phase A transcription captures the interface shape only, per the
//! invoking task's explicit instruction.
use crate::time::temporal::Temporal;
use serde::{Deserialize, Serialize};
// PORT NOTE: `Any` and `Ordered` are not imported here even though the
// module doc discusses them — `Iso8601Type: Temporal` already pulls in
// `Temporal: Ordered` and (transitively) `Ordered: Any` as supertrait
// bounds without needing their names in scope in this file; they are only
// named directly by concrete implementors (`iso8601_date.rs` etc.), which
// `impl Any for ...` / `impl Ordered for ...` directly.

/// Embedded parent state for the `Iso8601_type` attribute `value: String`.
///
/// Every concrete `Iso8601_*` struct in this module holds one of these
/// (conceptually "flattened" — see the ADR-001 §3 note above; `#[serde
/// (flatten)]` itself is deferred to the JSON-serialization phase, P4/P5,
/// since serde derives are out of scope for Phase A transcription).
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Iso8601TypeCore {
    /// `value`: `String` (`1..1`).
    ///
    /// Representation of all descendants is a single String.
    pub value: String,
}

/// `Iso8601_type` is modelled as a Rust trait requiring `Temporal` (and, by
/// extension, `Ordered` and `Any`) as a supertrait, plus an accessor for the
/// embedded `Iso8601TypeCore` state. See the module-level doc comment above
/// for the full multiple-inheritance reasoning, including why
/// `Time_Definitions` is *not* a second supertrait here.
pub trait Iso8601Type: Temporal {
    /// Access to the embedded `value: String` attribute.
    ///
    /// Not itself a member of the spec's `Iso8601_type` per-class table
    /// (the table declares `value` as a plain attribute, not a function);
    /// exposed as a trait method here so code can stay polymorphic over any
    /// `Iso8601Type` implementor without downcasting to a concrete struct
    /// first.
    fn core(&self) -> &Iso8601TypeCore;

    /// `as_string(): String`.
    ///
    /// Return the string value in extended format.
    ///
    /// PORT NOTE: the per-class table for `Iso8601_type` itself does not
    /// declare `as_string` — it appears individually on each concrete
    /// descendant's own table (`Iso8601_date.as_string`,
    /// `Iso8601_time.as_string`, etc., all with the identical signature and
    /// near-identical wording "Return \[the\] string value in extended
    /// format"). Hoisted to this shared trait as a default method delegating
    /// to the embedded `value` field, since every concrete effector's
    /// documented behaviour is the same "return my string representation"
    /// operation; concrete types may override if their internal
    /// representation ever diverges from the raw stored string (e.g. once
    /// compact-vs-extended re-formatting is implemented, see the TODO
    /// below).
    ///
    /// TODO(port): this default simply returns the stored `value` verbatim.
    /// The spec's actual contract is "in extended format" specifically —
    /// i.e. a value stored in *compact* form (`is_extended() == false`, see
    /// below) should be reformatted with `-`/`:` separators before being
    /// returned. That reformatting is string-parsing work deferred to the
    /// internal engine (see the module doc's jiff-bridging plan); until
    /// then this returns the raw value unconditionally, which is only
    /// correct when the stored value is already extended.
    fn as_string(&self) -> String {
        self.core().value.clone()
    }

    /// `is_partial(): Boolean` (abstract).
    ///
    /// True if this date time is partial, i.e. if trailing end (right hand)
    /// value(s) is/are missing.
    fn is_partial(&self) -> bool;

    /// `is_extended(): Boolean` (abstract).
    ///
    /// True if this ISO8601 string is in the 'extended' form, i.e. uses `-`
    /// and / or `:` separators. This is the preferred format.
    fn is_extended(&self) -> bool;
}

// PORT NOTE: `Iso8601Type: Temporal` requires every implementor to also
// satisfy `Ordered` and `Any` transitively (`Temporal: Ordered`, `Ordered:
// Any`), matching the spec's full inheritance chain
// `Iso8601_type -> Temporal -> Ordered -> Any`. Concrete types in this
// module (`iso8601_date.rs`, `iso8601_time.rs`, `iso8601_date_time.rs`,
// `iso8601_duration.rs`, `iso8601_timezone.rs`) must implement `Any`,
// `Ordered`, `Temporal`, and `Iso8601Type` individually — none of those four
// traits is blanket-implemented here, unlike `OrderedNumeric` in
// `primitive_types::ordered_numeric`, because `Temporal` and `Iso8601Type`
// each name a distinct semantic category rather than a mechanically
// combinable capability (see the PORT NOTE on `Temporal` itself in
// `temporal.rs`).

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.time — docs/research/spec-cache/BASE-1.2.0/uml_classes/iso8601_type.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master06-time_types.adoc §Class Definitions / iso8601_type.adoc §Iso8601_type Class
//   confidence: medium
//   todos: 1
//   note: multiple-inheritance worked example (ADR-001 §2) — Temporal composes as an ordinary supertrait, but Time_Definitions is a constants/free-fn struct with no trait shape, so that half of the Inherit relation is transcribed as direct TimeDefinitions::* calls from concrete descendants rather than a second supertrait bound; as_string's default returns the raw stored value verbatim pending the jiff-backed compact->extended reformatting engine at P17.
// ─────────────────────────────────────────────
