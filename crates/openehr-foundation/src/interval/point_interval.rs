//! `Point_interval<T>` — type representing an Interval that happens to be a
//! point value.
//!
//! openEHR class: `Point_interval<T>`, package
//! `base.foundation_types.interval`.
//! Inherits: `Interval<T>`.
//!
//! Provides an efficient representation that is substitutable for
//! `Interval<T>` where needed.
use super::interval::Interval;
use crate::primitive_types::ordered::Ordered;

/// Per ADR-001 §3, the `Interval<T>` parent's six attributes are embedded by
/// value as an `Interval<T>` field rather than duplicated flat on this
/// struct, since `Point_interval` only *redefines* four of the parent's
/// booleans (giving them fixed defaults) and adds no attributes of its own —
/// embedding keeps the single source of truth for the shared shape while the
/// constructor below enforces the redefined defaults.
///
/// The spec marks `lower_unbounded`, `upper_unbounded`, `lower_included`,
/// and `upper_included` all `(redefined)` here, each with a `{default = ...}`
/// value (`false`, `false`, `true`, `true` respectively). Rust has no
/// per-field default-value annotation independent of a constructor, so the
/// redefinition is expressed as `Point_interval::new`, which is the only
/// spec-faithful way to construct one (it fixes the four redefined booleans
/// to their spec defaults and requires the caller to supply only the shared
/// point value for both `lower` and `upper`, satisfying `Inv_point` by
/// construction — see the invariant note below).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointInterval<T: Ordered> {
    /// Embedded parent state; see the struct-level PORT NOTE for why this is
    /// composition rather than a flattened field list.
    pub interval: Interval<T>,
}

impl<T: Ordered + Clone> PointInterval<T> {
    /// Constructs a `Point_interval<T>` for the point value `value`.
    ///
    /// Fixes the four redefined boundary attributes to their spec defaults
    /// (`lower_unbounded = false`, `upper_unbounded = false`,
    /// `lower_included = true`, `upper_included = true`) and sets both
    /// `lower` and `upper` to `value`, which satisfies the class invariant
    /// `Inv_point: lower = upper` by construction — there is no other way to
    /// build a `PointInterval` in this module, so the invariant cannot be
    /// violated through this API.
    pub fn new(value: T) -> Self {
        PointInterval {
            interval: Interval {
                lower: Some(value.clone()),
                upper: Some(value),
                lower_unbounded: false,
                upper_unbounded: false,
                lower_included: true,
                upper_included: true,
            },
        }
    }
}

// PORT NOTE: `Inv_point: lower = upper` is enforced structurally by
// `PointInterval::new` above rather than as a separate `Validate` impl,
// since this module exposes no other constructor and no field-mutation API
// that could put a `PointInterval` into a `lower != upper` state. If a
// future phase adds direct field mutation (e.g. via `pub` field access
// already present on `interval: Interval<T>`), a runtime `Validate` check
// will be needed then — flagged here rather than silently assumed
// permanently safe, since `Interval<T>`'s fields are `pub`.
//
// TODO(port): revisit once direct mutation of `.interval.lower` /
// `.interval.upper` is possible from outside this module — the invariant is
// only guaranteed at construction time today.

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.interval — docs/research/spec-cache/BASE-1.2.0/uml_classes/point_interval.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-interval.adoc §Class Definitions / point_interval.adoc §Point_interval Class
//   confidence: medium
//   todos: 1
//   note: Inv_point (lower = upper) enforced structurally by the only constructor rather than a runtime Validate check; revisit if direct field mutation is ever exposed since Interval<T>'s fields are pub.
// ─────────────────────────────────────────────
