//! `Array<T>` — container whose storage is assumed to be contiguous.
//!
//! openEHR class: `Array<T>`, package `base.foundation_types.structures`.
//! Inherits: `Container<T>`.
//!
//! Container whose storage is assumed to be contiguous. Adds a single
//! keyed-access function, `item`.
use super::super::primitive_types::any::Any;
use super::container::Container;

/// Per `docs/PORTING.md` §14.2 and the structure_types chapter's own
/// cross-reference table (`Array<T>` → contiguous array), transcribed as a
/// transparent newtype over `std::vec::Vec<T>`. Rust's native fixed-size
/// `[T; N]` array cannot carry a runtime-determined length, and `Vec<T>`'s
/// own storage guarantee is contiguous heap allocation, matching the spec's
/// stated assumption directly.
///
/// PORT NOTE: this makes `Array<T>` and `List<T>` (`super::list::List`)
/// backed by the same underlying Rust type (`Vec<T>`). This mirrors the
/// spec's own type-cross-reference table, which lists both `Array<T>` and
/// `List<T>` against `sequence`/`Array<T>`/`List<T>` in XML/Java/C# with no
/// further structural distinction beyond their respective function sets —
/// the two remain distinct Rust newtypes here (not a type alias to one
/// another) because they are declared as separate classes with separate
/// abstract-operations interfaces (`Array` gets `item`; `List` gets
/// `first`/`last`), even though the storage representation coincides.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Array<T>(pub Vec<T>);

impl<T> Array<T> {
    /// `item` __alias__ `"[]"` `(a_key: Integer[1]) -> T`.
    ///
    /// Return item for key `a_key`.
    ///
    /// PORT NOTE: the spec types `a_key` as `Integer` (signed, 32-bit) while
    /// Rust's native indexing takes `usize`; transcribed with an `i32`
    /// parameter to match the spec signature literally, converting
    /// internally. TODO(port) below covers the unspecified out-of-range
    /// behaviour.
    ///
    /// TODO(port): the spec does not state what happens for a key outside
    /// the array's bounds (no precondition/postcondition given in the
    /// per-class table, unlike e.g. `List`'s explicit `First_validity`/
    /// `Last_validity` invariants). A negative `a_key` or one `>= count()`
    /// is left as `todo!()` rather than guessing between a panic, `Option`,
    /// or `Result` contract the spec does not itself state.
    pub fn item(&self, a_key: i32) -> &T {
        match usize::try_from(a_key) {
            Ok(index) if index < self.0.len() => &self.0[index],
            // TODO(port): spec is silent on out-of-range/negative key
            // behaviour for `item`; see doc comment above.
            _ => todo!("Array::item: a_key out of range, spec does not define this behaviour"),
        }
    }
}

impl<T: PartialEq> Container<T> for Array<T> {
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

impl<T: PartialEq> Any for Array<T> {
    fn is_equal(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn type_of(&self) -> String {
        "Array".to_string()
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.structures §Class Definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/array.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master04-structure_types.adoc §Class Definitions / array.adoc §Array Class
//   confidence: medium
//   todos: 3
//   note: item()'s out-of-range/negative-key behaviour is unspecified by the spec, left as todo!() with an accompanying TODO(port) comment (two markers for that one gap); count()'s i32 cast shares List's unspecified-overflow gap (third marker). Backed by Vec<T> like List<T> (spec's contiguous-storage assumption), kept as a separate newtype since Array and List are separate classes with distinct function sets.
// ─────────────────────────────────────────────
