//! `Uri` — a String constrained to RFC 3986 syntax.
//!
//! openEHR class: `Uri`, package `base.foundation_types.primitive_types`.
//! Inherits: `String` (the foundation-types `String` class transcribed in
//! `string.rs` as `OpenEhrString`, per the naming note there — not
//! `std::string::String` directly).
//!
//! A kind of String constrained to obey the syntax of RFC 3986. Declares no
//! functions or attributes of its own beyond those inherited from `String`;
//! the RFC 3986 constraint is an invariant, not a structural difference.
use super::any::Any;
use super::integer::Integer;
use super::ordered::Ordered;
use super::string::OpenEhrString;
use serde::{Deserialize, Serialize};

/// `Uri` is modelled as a newtype wrapping `OpenEhrString` — the transcribed
/// foundation-types `String` class — rather than `std::string::String`
/// directly, to reflect the spec's actual inheritance (`Uri` inherits
/// `String`, the abstract-operations class in this same module, not the raw
/// Rust primitive).
///
/// Per the spec's own description, this class adds a syntactic constraint
/// (RFC 3986) over its parent rather than any new attribute or function;
/// `Deref`-style forwarding of the inherited `String` operations
/// (`is_empty`, `is_integer`, `as_integer`, `append`, `contains`,
/// `less_than`) is provided as inherent methods below rather than a blanket
/// `Deref` impl, keeping the RFC 3986 invariant enforceable at every
/// construction site rather than allowing silent unchecked mutation through
/// a deref coercion.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct Uri(pub OpenEhrString);

impl Uri {
    /// Construct a `Uri` from a raw string, without validating RFC 3986
    /// syntax.
    ///
    /// TODO(port): the spec invariant that a `Uri`'s value "obey[s] the
    /// syntax of RFC 3986" is not yet enforced here. A syntax-checked
    /// constructor (the RM transcription rule's standing exception: "a
    /// constructor that throws" → `fn new(...) -> Result<Self, E>") belongs
    /// once this crate has an error type and an RFC 3986 validator
    /// dependency decision; left unvalidated for Phase A.
    pub fn new_unchecked(value: impl Into<String>) -> Self {
        Uri(OpenEhrString(value.into()))
    }

    /// Inherited `String::is_empty(): Boolean`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Inherited `String::is_integer(): Boolean`.
    #[must_use]
    pub fn is_integer(&self) -> bool {
        self.0.is_integer()
    }

    /// Inherited `String::as_integer(): Integer`.
    #[must_use]
    pub fn as_integer(&self) -> Integer {
        self.0.as_integer()
    }

    /// Inherited `String::append` __alias__ `"+"` `(other: String) -> String`.
    ///
    /// PORT NOTE: the spec declares `Uri` a kind of `String`, and does not
    /// separately re-declare `append` with a `Uri`-typed result; appending
    /// to a `Uri` does not generally produce a value that itself obeys RFC
    /// 3986 syntax, so this is transcribed as returning the parent
    /// `OpenEhrString` type, not `Uri`, avoiding a claim of validity the
    /// spec does not make.
    #[must_use]
    pub fn append(&self, other: &OpenEhrString) -> OpenEhrString {
        self.0.append(other)
    }

    /// Inherited `String::contains(other: String) -> Boolean`.
    #[must_use]
    pub fn contains(&self, other: &OpenEhrString) -> bool {
        self.0.contains(other)
    }
}

impl Any for Uri {
    fn is_equal(&self, other: &Self) -> bool {
        self.0.is_equal(&other.0)
    }

    fn type_of(&self) -> String {
        "Uri".to_string()
    }
}

impl Ordered for Uri {
    /// Inherited `String::less_than` __alias__ `"<"`.
    fn less_than(&self, other: &Self) -> bool {
        self.0.less_than(&other.0)
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/uri.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / uri.adoc §Uri Class
//   confidence: medium
//   todos: 1
//   note: wraps OpenEhrString (not std::string::String) to reflect the spec's actual String-class inheritance; RFC 3986 syntax invariant not yet enforced (new_unchecked only) pending an error type and validator dependency decision.
// ─────────────────────────────────────────────
