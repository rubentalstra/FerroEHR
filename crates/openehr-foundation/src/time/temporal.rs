//! `Temporal` — abstract ancestor of time-related classes.
//!
//! openEHR class: `Temporal` (abstract), package `base.foundation_types.time`.
//! Inherits: `Ordered`.
//!
//! The per-class table declares no attributes and no functions of its own;
//! it exists purely to name the "time-related, and therefore ordered"
//! capability that `Iso8601_type` and its descendants build on.
use crate::primitive_types::ordered::Ordered;

/// `Temporal` is modelled as a Rust trait with `Ordered` as a supertrait,
/// mirroring the spec's single-parent inheritance (`Temporal` inherits
/// `Ordered`). Like `Ordered_Numeric` in `primitive_types::ordered_numeric`,
/// this class declares nothing beyond its parent, so the trait body is
/// empty — it is a pure marker naming the combined "time-related and
/// ordered" capability for `Iso8601_type` (see `iso8601_type.rs`) to build
/// on.
///
/// Unlike `OrderedNumeric`, this trait is *not* blanket-implemented for
/// every `T: Ordered` — `Temporal` specifically means "time-related", a
/// distinction the spec draws by declaring it as its own abstract class
/// rather than reusing `Ordered` directly, even though it adds no members.
/// Concrete `Iso8601_type` descendants implement it explicitly (via the
/// `Iso8601Type` trait in `iso8601_type.rs`, which requires `Temporal` as a
/// supertrait) rather than getting it for free.
pub trait Temporal: Ordered {}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.time — docs/research/spec-cache/BASE-1.2.0/uml_classes/temporal.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master06-time_types.adoc §Class Definitions / temporal.adoc §Temporal Class
//   confidence: high
//   todos: 0
//   note: pure marker trait, no attributes/functions declared by the spec; not blanket-implemented (contrast OrderedNumeric) since Temporal names a distinct semantic category, not a mechanically-derivable combination.
// ─────────────────────────────────────────────
