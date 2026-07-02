//! `Set<T>` — unordered container that may not contain duplicates.
//!
//! openEHR class: `Set<T>`, package `base.foundation_types.structures`.
//! Inherits: `Container<T>`.
//!
//! Unordered container that may not contain duplicates. Declares no
//! functions or attributes of its own beyond those inherited from
//! `Container<T>`.
use super::super::primitive_types::any::Any;
use super::container::Container;
use std::collections::HashSet;
use std::hash::Hash;

/// Per `docs/PORTING.md` §14.2 (`Set<T>` → `HashSet<T>`/`BTreeSet<T>`),
/// transcribed as a transparent newtype over `std::collections::HashSet<T>`
/// — unordered, unique-membership, matching the spec's stated semantics
/// exactly. `BTreeSet` (ordered) would over-constrain the spec's explicit
/// "no order" description, so `HashSet` is the more faithful choice of the
/// two `docs/PORTING.md` options.
///
/// `T: Eq + Hash` is required by `HashSet` itself; this is a structural
/// requirement of the chosen backing container, not a spec-declared
/// constraint (the spec's own `Set<T>` has no bound on `T` at all).
#[derive(Debug, Clone, Default)]
#[repr(transparent)]
pub struct Set<T: Eq + Hash>(pub HashSet<T>);

impl<T: Eq + Hash> Container<T> for Set<T> {
    fn has(&self, v: &T) -> bool {
        self.0.contains(v)
    }

    fn count(&self) -> i32 {
        // TODO(port): see `List::count` PORT NOTE — same i32-cast overflow
        // gap, unaddressed by the spec.
        self.0.len() as i32
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T: Eq + Hash> Any for Set<T> {
    fn is_equal(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn type_of(&self) -> String {
        "Set".to_string()
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.structures §Class Definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/set.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master04-structure_types.adoc §Class Definitions / set.adoc §Set Class
//   confidence: high
//   todos: 1
//   note: T: Eq + Hash bound is a structural requirement of HashSet, not a spec-declared constraint (Set<T> itself is unconstrained); count()'s i32 cast shares List's unspecified-overflow gap.
// ─────────────────────────────────────────────
