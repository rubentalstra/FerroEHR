//! `String` — minimal interface of the built-in String type.
//!
//! openEHR class: `String`, package `base.foundation_types.primitive_types`.
//! Inherits: `Ordered`.
//!
//! Type representing the minimal interface of a built-in String type, as
//! used to represent textual data in any natural or formal language. Per the
//! chapter overview: "It is assumed in the openEHR specifications that
//! Unicode is supported by the type `String` ... In openEHR, UTF-8 encoding
//! is assumed."
use super::any::Any;
use super::integer::Integer;
use super::ordered::Ordered;
use serde::{Deserialize, Serialize};

/// Rust type name: `OpenEhrString`, **not** `String`.
///
/// PORT NOTE: the openEHR class is literally named `String`, but that name
/// collides with `std::string::String`, which every ordinary RM attribute of
/// spec type `String` maps directly onto per `docs/PORTING.md` Section 14.2
/// (`String` → Rust `String`, owned). This type is specifically the
/// *foundation-types class* transcription of `base.foundation_types
/// .primitive_types.String` — the abstract-operations interface
/// (`is_empty`, `is_integer`, `as_integer`, `append`, `contains`,
/// `less_than`) — which is a distinct concern from "a struct field typed
/// `String`" elsewhere in the RM (those fields keep using `std::string
/// ::String` directly, unwrapped, as the existing PORTING.md mapping
/// already directs). Recorded in `docs/ROSETTA.md` so this naming decision
/// is not relitigated by a later transcriber.
///
/// Implemented as a transparent newtype over `std::string::String` so the
/// wrapped value is the same owned, UTF-8, growable string Rust already
/// provides — matching the spec's own Unicode/UTF-8 assumption exactly,
/// with no reinterpretation needed.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct OpenEhrString(pub String);

impl OpenEhrString {
    /// `is_empty(): Boolean`.
    ///
    /// True if the string is empty, i.e. equal to `""`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// `is_integer(): Boolean`.
    ///
    /// True if the string can be parsed as an integer.
    ///
    /// PORT NOTE: "an integer" is read as the spec's 32-bit `Integer` class,
    /// so parseability is exactly Rust's `str::parse::<i32>` (optional
    /// leading `+`/`-` sign, decimal digits, value within `i32` range; no
    /// surrounding whitespace).
    #[must_use]
    pub fn is_integer(&self) -> bool {
        self.0.parse::<i32>().is_ok()
    }

    /// `as_integer(): Integer`.
    ///
    /// Return the integer corresponding to the integer value represented in
    /// this string.
    ///
    /// PORT NOTE: the spec does not state the behaviour when the string
    /// does not represent a valid integer (`is_integer()` returning false) —
    /// a faithful Eiffel-style precondition would be `require is_integer`.
    /// Widened to `Option<Integer>` (`None` exactly when `is_integer()` is
    /// false), matching the crate's established "or Void" treatment of
    /// spec-silent partial functions (`List::first`/`last`,
    /// `Container::select`), with the same `str::parse::<i32>` semantics as
    /// `is_integer` above.
    #[must_use]
    pub fn as_integer(&self) -> Option<Integer> {
        self.0.parse::<i32>().ok().map(Integer)
    }

    /// `append` __alias__ `"+"` `(other: String) -> String`.
    ///
    /// Concatenation operator — causes `other` to be appended to this
    /// string.
    #[must_use]
    pub fn append(&self, other: &OpenEhrString) -> OpenEhrString {
        let mut result = self.0.clone();
        result.push_str(&other.0);
        OpenEhrString(result)
    }

    /// `contains(other: String) -> Boolean`.
    ///
    /// Return `true` if this String contains `other` (case-sensitive).
    #[must_use]
    pub fn contains(&self, other: &OpenEhrString) -> bool {
        self.0.contains(other.0.as_str())
    }
}

impl Any for OpenEhrString {
    fn is_equal(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn type_of(&self) -> String {
        "String".to_string()
    }
}

impl Ordered for OpenEhrString {
    /// `less_than` __alias__ `"<"` `(other: String) -> Boolean` (effected).
    ///
    /// Lexical comparison of string content based on ordering in the
    /// relevant character set.
    fn less_than(&self, other: &Self) -> bool {
        self.0 < other.0
    }
}

#[cfg(test)]
mod tests {
    use super::OpenEhrString;
    use crate::primitive_types::integer::Integer;
    use crate::primitive_types::ordered::Ordered;

    fn s(text: &str) -> OpenEhrString {
        OpenEhrString(text.to_string())
    }

    // Spec: is_empty "True if string is empty, i.e. equal to ''".
    #[test]
    fn is_empty_matches_the_empty_string_only() {
        assert!(s("").is_empty());
        assert!(!s(" ").is_empty());
        assert!(!s("a").is_empty());
    }

    // Spec: is_integer "True if string can be parsed as an integer";
    // as_integer "Return the integer corresponding to the integer value
    // represented in this string" (None when is_integer is false).
    #[test]
    fn is_integer_and_as_integer_agree_on_i32_parse_semantics() {
        assert!(s("123").is_integer());
        assert_eq!(s("123").as_integer(), Some(Integer(123)));
        assert!(s("-45").is_integer());
        assert_eq!(s("-45").as_integer(), Some(Integer(-45)));
        assert!(s("+7").is_integer());
        assert_eq!(s("+7").as_integer(), Some(Integer(7)));
        for not_an_integer in ["", "abc", "1.5", " 12", "12 ", "2147483648"] {
            assert!(!s(not_an_integer).is_integer(), "{not_an_integer:?}");
            assert_eq!(s(not_an_integer).as_integer(), None, "{not_an_integer:?}");
        }
    }

    // Spec: append (alias "+") "causes other to be appended to this string".
    #[test]
    fn append_concatenates() {
        assert_eq!(s("foo").append(&s("bar")), s("foobar"));
        assert_eq!(s("").append(&s("x")), s("x"));
    }

    // Spec: contains "Return True if this String contains other
    // (case-sensitive)".
    #[test]
    fn contains_is_case_sensitive() {
        assert!(s("Hello world").contains(&s("world")));
        assert!(!s("Hello world").contains(&s("World")));
    }

    // Spec: less_than (alias "<", effected) "Lexical comparison of string
    // content".
    #[test]
    fn less_than_is_lexical() {
        assert!(s("abc").less_than(&s("abd")));
        assert!(!s("abd").less_than(&s("abc")));
        assert!(s("ab").less_than(&s("abc")));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/string.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / string.adoc §String Class
//   confidence: high
//   todos: 0
//   note: Rust type named OpenEhrString to avoid colliding with std::string::String; recorded in ROSETTA. as_integer widened to Option<Integer> (spec-silent not-an-integer case, "or Void" convention), with str::parse::<i32> defining integer-parseability for both is_integer and as_integer.
// ─────────────────────────────────────────────
