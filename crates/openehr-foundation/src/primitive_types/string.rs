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
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    #[must_use]
    pub fn is_integer(&self) -> bool {
        self.0.parse::<i32>().is_ok()
    }

    /// `as_integer(): Integer`.
    ///
    /// Return the integer corresponding to the integer value represented in
    /// this string.
    ///
    /// TODO(port): the spec does not state the behaviour when the string
    /// does not represent a valid integer (`is_integer()` returning false).
    /// A faithful Eiffel-style precondition would be `require is_integer`;
    /// modelled here with `todo!()` for the failure path pending a decision
    /// on whether this becomes a `Result`-returning method once call sites
    /// exist, rather than guessing a panic-vs-`Result` contract the spec
    /// does not itself state.
    #[must_use]
    pub fn as_integer(&self) -> Integer {
        match self.0.parse::<i32>() {
            Ok(value) => Integer(value),
            // TODO(port): spec is silent on the not-an-integer case; see
            // doc comment above.
            Err(_) => todo!("OpenEhrString::as_integer: input is not a valid Integer"),
        }
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

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/string.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / string.adoc §String Class
//   confidence: medium
//   todos: 1
//   note: Rust type named OpenEhrString to avoid colliding with std::string::String; recorded in ROSETTA. as_integer's not-an-integer behavior is unspecified by the spec and left as todo!().
// ─────────────────────────────────────────────
