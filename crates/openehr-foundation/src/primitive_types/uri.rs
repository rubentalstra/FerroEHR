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
use serde::de::Error as _;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

/// Error raised when constructing a [`Uri`] from text that does not obey
/// RFC 3986 syntax (the class's sole spec constraint).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UriError {
    /// The value is neither a valid RFC 3986 URI nor a valid RFC 3986
    /// relative reference.
    #[error("value does not obey RFC 3986 syntax: {value:?}")]
    InvalidSyntax {
        /// The offending raw text, unmodified.
        value: String,
    },
}

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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Uri(pub OpenEhrString);

impl Uri {
    /// Construct a `Uri` from a raw string, enforcing the class's RFC 3986
    /// syntax constraint (the RM transcription rule's standing exception:
    /// "a constructor that throws" → `fn new(...) -> Result<Self, E>`).
    ///
    /// The stored text is kept exactly as supplied — never normalized,
    /// percent-encoded, or otherwise rewritten — per ADR-003 decision 5
    /// (RM canonical-form round-trips must not rewrite user URIs).
    pub fn new(value: impl Into<String>) -> Result<Self, UriError> {
        let value = value.into();
        if Self::text_obeys_rfc3986(&value) {
            Ok(Uri(OpenEhrString(value)))
        } else {
            Err(UriError::InvalidSyntax { value })
        }
    }

    /// Construct a `Uri` from a raw string, without validating RFC 3986
    /// syntax. Kept for deserialization and for legacy data whose URI
    /// values predate validation; a value built this way can be checked
    /// after the fact with [`Uri::is_valid`].
    pub fn new_unchecked(value: impl Into<String>) -> Self {
        Uri(OpenEhrString(value.into()))
    }

    /// True if this `Uri`'s stored value obeys the syntax of RFC 3986 — the
    /// class's single spec invariant, exposed as a working validity method
    /// per ADR-003 decision 8 (`new_unchecked` can bypass `new`'s check).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        Self::text_obeys_rfc3986(&self.0.0)
    }

    /// Conservative RFC 3986 syntax check per ADR-003 decision 5: a value
    /// with a scheme (an absolute URI) is validated with the pinned `url`
    /// crate; a value without one (an RFC 3986 relative reference, which
    /// `url::Url::parse` cannot represent on its own) is checked against
    /// RFC 3986's generic-syntax grammar componentwise.
    fn text_obeys_rfc3986(value: &str) -> bool {
        if Self::split_scheme(value).is_some() {
            url::Url::parse(value).is_ok()
        } else {
            Self::is_valid_relative_reference(value)
        }
    }

    /// If `value` starts with an RFC 3986 `scheme ":"` (ALPHA followed by
    /// ALPHA/DIGIT/`+`/`-`/`.`, terminated by the first `:` that occurs
    /// before any `/`, `?` or `#`), return the scheme and the rest.
    fn split_scheme(value: &str) -> Option<(&str, &str)> {
        let colon = value.find([':', '/', '?', '#'])?;
        if value.as_bytes()[colon] != b':' {
            return None;
        }
        let scheme = &value[..colon];
        let mut chars = scheme.chars();
        let first_is_alpha = chars.next().is_some_and(|c| c.is_ascii_alphabetic());
        let rest_ok = chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
        (first_is_alpha && rest_ok).then_some((scheme, &value[colon + 1..]))
    }

    /// RFC 3986 §4.2 `relative-ref = relative-part [ "?" query ] [ "#"
    /// fragment ]`, checked componentwise: every character must belong to
    /// the component's allowed set (unreserved / sub-delims plus the
    /// component's extra delimiters) or be a valid `%XX` percent-encoding.
    /// The empty string is a valid same-document reference.
    fn is_valid_relative_reference(value: &str) -> bool {
        let (before_fragment, fragment) = match value.split_once('#') {
            Some((head, fragment)) => (head, Some(fragment)),
            None => (value, None),
        };
        let (relative_part, query) = match before_fragment.split_once('?') {
            Some((head, query)) => (head, Some(query)),
            None => (before_fragment, None),
        };
        // relative-part: allow pchar + "/" throughout; "[" and "]" only
        // appear in an IP-literal host, i.e. when the part starts "//".
        let authority_form = relative_part.starts_with("//");
        Self::component_chars_ok(relative_part, |c| {
            Self::is_pchar_char(c) || c == '/' || (authority_form && matches!(c, '[' | ']'))
        }) && query.is_none_or(|q| {
            Self::component_chars_ok(q, |c| Self::is_pchar_char(c) || matches!(c, '/' | '?'))
        }) && fragment.is_none_or(|f| {
            Self::component_chars_ok(f, |c| Self::is_pchar_char(c) || matches!(c, '/' | '?'))
        })
    }

    /// `pchar = unreserved / pct-encoded / sub-delims / ":" / "@"` — the
    /// non-`%` single characters of that set (`%` is handled by
    /// `component_chars_ok`).
    fn is_pchar_char(c: char) -> bool {
        c.is_ascii_alphanumeric()
            || matches!(
                c,
                '-' | '.'
                    | '_'
                    | '~'
                    | '!'
                    | '$'
                    | '&'
                    | '\''
                    | '('
                    | ')'
                    | '*'
                    | '+'
                    | ','
                    | ';'
                    | '='
                    | ':'
                    | '@'
            )
    }

    /// True if every byte of `component` is either an allowed character or
    /// part of a well-formed `%XX` percent-encoding.
    fn component_chars_ok(component: &str, allowed: impl Fn(char) -> bool) -> bool {
        let bytes = component.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' {
                let hex_ok = i + 2 < bytes.len()
                    && bytes[i + 1].is_ascii_hexdigit()
                    && bytes[i + 2].is_ascii_hexdigit();
                if !hex_ok {
                    return false;
                }
                i += 3;
            } else {
                // Multi-byte UTF-8 is outside RFC 3986's ASCII grammar.
                let Some(c) = component[i..].chars().next() else {
                    return false;
                };
                if !c.is_ascii() || !allowed(c) {
                    return false;
                }
                i += c.len_utf8();
            }
        }
        true
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

    /// Inherited `String::as_integer(): Integer` (widened to
    /// `Option<Integer>` — see `OpenEhrString::as_integer`).
    #[must_use]
    pub fn as_integer(&self) -> Option<Integer> {
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

impl Serialize for Uri {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("URI", 2)?;
        state.serialize_field("_type", "URI")?;
        state.serialize_field("value", &self.0)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Uri {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Object {
                #[serde(rename = "_type")]
                type_name: Option<String>,
                #[serde(default)]
                value: OpenEhrString,
            },
            Bare(OpenEhrString),
        }

        match Wire::deserialize(deserializer)? {
            Wire::Object { type_name, value } => {
                if type_name.as_deref().is_some_and(|name| name != "URI") {
                    return Err(D::Error::custom("expected _type \"URI\""));
                }
                Ok(Uri(value))
            }
            Wire::Bare(value) => Ok(Uri(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Uri, UriError};

    // Spec: "A kind of String constrained to obey the syntax of RFC 3986"
    // — absolute URIs are validated via the url crate (ADR-003 decision 5).
    #[test]
    fn new_accepts_valid_absolute_uris() {
        for valid in [
            "https://example.com/path?q=1#frag",
            "http://openehr.org",
            "urn:uuid:6ba7b810-9dad-11d1-80b4-00c04fd430c8",
            "mailto:someone@example.com",
            "file:///etc/hosts",
            "ehr://system/eb95f471-0f21-4e94-a2cb-3b6c1e3d1e0a",
        ] {
            assert!(Uri::new(valid).is_ok(), "{valid:?}");
        }
    }

    // RFC 3986 §4.2 relative references are also valid URI-reference
    // syntax; checked componentwise (ADR-003 decision 5).
    #[test]
    fn new_accepts_valid_relative_references() {
        for valid in [
            "",
            "../relative/path",
            "path/to/resource?query=1#frag",
            "//host.example/abs-path",
            "#fragment-only",
            "with%20escaped/space",
        ] {
            assert!(Uri::new(valid).is_ok(), "{valid:?}");
        }
    }

    #[test]
    fn new_rejects_rfc3986_syntax_violations() {
        for invalid in [
            "has space/in path",
            "http://exa mple.com/",
            "bad%2xescape",
            "trailing%2",
            "<angle>brackets",
            "back\\slash",
            "curly{brace}",
            "caf\u{e9}/unencoded-non-ascii",
        ] {
            let result = Uri::new(invalid);
            assert_eq!(
                result,
                Err(UriError::InvalidSyntax {
                    value: invalid.to_string()
                }),
                "{invalid:?}"
            );
        }
    }

    // Invariant method direction test: new_unchecked can hold an invalid
    // value, and is_valid reports it; a checked value reports valid.
    #[test]
    fn is_valid_flags_unchecked_values_both_ways() {
        assert!(Uri::new_unchecked("https://example.com").is_valid());
        assert!(!Uri::new_unchecked("not a uri").is_valid());
    }

    // ADR-003 decision 5: accessors return the un-normalized stored text —
    // construction must not rewrite the value.
    #[test]
    fn stored_text_is_not_normalized() {
        let uri = Uri::new("HTTPS://Example.COM/A%2Fb/../c").expect("valid URI");
        assert_eq!(uri.0.0, "HTTPS://Example.COM/A%2Fb/../c");
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/uri.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / uri.adoc §Uri Class
//   confidence: high
//   todos: 0
//   note: wraps OpenEhrString (not std::string::String) to reflect the spec's actual String-class inheritance; RFC 3986 invariant enforced by Uri::new (url crate for absolute URIs, componentwise generic-syntax check for relative references, per ADR-003 decision 5) with new_unchecked + is_valid for the bypass path; stored text stays un-normalized. P4: canonical JSON emits object form `{_type:"URI",value}` to satisfy the pinned ITS-JSON schema while preserving the string payload.
// ─────────────────────────────────────────────
