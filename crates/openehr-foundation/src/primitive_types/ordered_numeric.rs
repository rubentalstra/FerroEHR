//! `Ordered_Numeric` — abstract notional parent class of ordered, numeric
//! types.
//!
//! openEHR class: `Ordered_Numeric` (abstract), package
//! `base.foundation_types.primitive_types`.
//! Inherits: `Ordered`, `Numeric`.
//!
//! Abstract notional parent class of ordered, numeric types, which are types
//! with both the `less_than()` and arithmetic functions defined. Declares no
//! attributes or functions of its own beyond those inherited.
use super::numeric::Numeric;
use super::ordered::Ordered;

/// `Ordered_Numeric` is the multiple-inheritance case named explicitly in
/// PORT_MASTER_PLAN.md Section 7.2 and `.claude/rules/rm-transcription.md`:
/// modelled as a Rust supertrait composition (`Ordered + Numeric`) rather
/// than a struct, since the spec class itself declares no attributes and no
/// functions — it exists purely to name the combined capability.
///
/// Per the RM transcription rule for multiple inheritance ("composition of
/// fields from all parents plus one trait per parent behaviour"), the two
/// parent behaviours already exist as their own traits (`Ordered`,
/// `Numeric`); this trait adds nothing beyond requiring both, and is
/// blanket-implemented for any type that already satisfies both parents so
/// no concrete type needs a separate `impl OrderedNumeric for ...` block.
pub trait OrderedNumeric: Ordered + Numeric {}

impl<T: Ordered + Numeric> OrderedNumeric for T {}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/ordered_numeric.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / ordered_numeric.adoc §Ordered_Numeric Class
//   confidence: high
//   todos: 0
//   note: blanket impl means Integer/Integer64/Real/Double automatically satisfy OrderedNumeric once they implement Ordered and Numeric; no separate impl block needed per concrete type.
// ─────────────────────────────────────────────
