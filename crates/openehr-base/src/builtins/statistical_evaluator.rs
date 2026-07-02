//! `Statistical_evaluator` — common statistical functions on collections of
//! numbers.
//!
//! openEHR class: `Statistical_evaluator` (interface), package
//! `base.base_types.builtins`.
//!
//! A basic statistical evaluator class providing common functions on
//! collections of numbers.
// TODO(port): `Container<T>` belongs to `base.foundation_types.structures`
// and `Numeric` to `base.foundation_types.primitive_types::numeric`; the
// `openehr-foundation` copy of `Numeric` exists (`primitive_types::numeric`)
// but `Container<T>` has not been transcribed yet. The `use` path below
// names where it is expected to land per the crate layout
// (PORT_MASTER_PLAN.md Section 9); update once that file exists.
use openehr_foundation::primitive_types::numeric::Numeric;
use openehr_foundation::structure_types::container::Container;

/// `Statistical_evaluator` is a pure function interface (no attributes, no
/// state), so it is transcribed as a Rust trait per ADR-001 §1, generic
/// over the numeric element type per the spec's own `Container<Numeric>`
/// parameter, mirroring the constrained-generic transcription rule
/// (`.claude/rules/rm-transcription.md`).
///
/// # Covariant/open-result-type note (PORT NOTE)
///
/// The spec types `median`, `mode`, `max`, `min`, and `count` as returning
/// the abstract `Numeric` type itself, not the container's own element
/// type `T` — the same open-result-type pattern flagged on the `Numeric`
/// trait's own `add`/`subtract`/etc. in
/// `openehr-foundation::primitive_types::numeric` (see that file's
/// trait-level PORT NOTE). Since every value actually produced by these
/// functions is drawn from (or derived from) the input container's own
/// element type, this trait narrows the abstract `Numeric` result to the
/// closed, same-type case `T`, consistent with how the `Numeric` trait
/// itself narrows its own abstract signatures.
pub trait StatisticalEvaluator<T: Numeric> {
    /// `sum` (vals: `Container<Numeric>[1]`): `Double`.
    ///
    /// Sum of a container of values.
    ///
    /// TODO(port): return type is spec-declared `Double`
    /// (`openehr_foundation::primitive_types::double::Double`), not yet
    /// transcribed in this crate's dependency graph at time of writing;
    /// left as `f64` pending that type's existence, per the primitive-type
    /// std mapping (ADR-001 §7).
    fn sum<C: Container<T>>(&self, vals: &C) -> f64;

    /// `avg` (vals: `Container<Numeric>[1]`): `Double`.
    ///
    /// Synonym for `mean()`.
    fn avg<C: Container<T>>(&self, vals: &C) -> f64 {
        self.mean(vals)
    }

    /// `mean` (vals: `Container<Numeric>[1]`): `Double`.
    ///
    /// Mean (arithmetic average) of a container of values.
    fn mean<C: Container<T>>(&self, vals: &C) -> f64;

    /// `median` (vals: `Container<Numeric>[1]`): `Numeric` (narrowed to
    /// `T`, see trait-level PORT NOTE).
    ///
    /// Return numerically centre value in ordered form of container
    /// contents.
    fn median<C: Container<T>>(&self, vals: &C) -> T;

    /// `mode` (vals: `Container<Numeric>[1]`): `Numeric` (narrowed to `T`).
    ///
    /// Mode (most frequent) of a container of values.
    fn mode<C: Container<T>>(&self, vals: &C) -> T;

    /// `max` (vals: `Container<Numeric>[1]`): `Numeric` (narrowed to `T`).
    ///
    /// Maximum of a container of values.
    fn max<C: Container<T>>(&self, vals: &C) -> T;

    /// `min` (vals: `Container<Numeric>[1]`): `Numeric` (narrowed to `T`).
    ///
    /// Minimum of a container of values.
    fn min<C: Container<T>>(&self, vals: &C) -> T;

    /// `count` (vals: `Container<Numeric>[1]`): `Numeric` (narrowed to `T`,
    /// see trait-level PORT NOTE).
    ///
    /// Return the number of items in `vals`, i.e. `vals.count`.
    ///
    /// PORT NOTE: the spec text itself calls out `count` as a synonym for
    /// `vals.count` (the container's own item count), which is ordinarily
    /// an `Integer`, not the container's element type `T` — an apparent
    /// further inconsistency in the spec's declared `Numeric` return type
    /// for this function beyond the general open-result-type note above.
    /// Kept as `T` here for trait-signature uniformity with `median`/
    /// `mode`/`max`/`min`; a call site may need an explicit conversion.
    fn count<C: Container<T>>(&self, vals: &C) -> T;

    /// `std_dev` (vals: `Container<Numeric>[1]`): `Double`.
    ///
    /// Compute standard deviation of a container of values.
    fn std_dev<C: Container<T>>(&self, vals: &C) -> f64;
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.builtins — docs/research/spec-cache/BASE-1.2.0/uml_classes/statistical_evaluator.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master04-builtins_package.adoc §Class Definitions / statistical_evaluator.adoc §Statistical_evaluator Interface
//   confidence: medium
//   todos: 2
//   note: forward-references Container<T>, not transcribed yet; Double return narrowed to f64 pending that type's transcription; median/mode/max/min/count narrow the spec's abstract Numeric result to the trait's own generic T, and count's Numeric-vs-Integer inconsistency in the spec text is flagged rather than silently resolved.
// ─────────────────────────────────────────────
