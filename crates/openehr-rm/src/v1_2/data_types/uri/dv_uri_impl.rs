// @generated-from-template templates/openehr-rm/data_types/uri/dv_uri_impl.rs — DO NOT EDIT; edit the source and re-run `openehr-codegen -- emit`.
//! Hand-written RM class invariant + functions for `DV_URI`.
//!
//! Spec: RM 1.2.0
//! `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.dv_uri.adoc`:
//! - `Value_valid`: the URI value must be non-empty.
//! - `scheme()` / `path()` / `query()` / `fragment_id()` accessor functions
//!   (RFC-3986 generic-syntax decomposition; each returns the empty string
//!   when the component is absent, matching the spec's `String` returns).

use crate::v1_2::data_types::uri::dv_uri::DvUriData;
use openehr_base::validate::{InvariantViolation, Validate};

/// `true` when `s` is a syntactically valid RFC-3986 scheme:
/// `ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`.
fn is_scheme(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// `true` when `value` is an absolute RFC-3986 reference — a scheme
/// (`ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`) followed by `:`.
fn has_scheme(value: &str) -> bool {
    matches!(value.split_once(':'), Some((s, _)) if is_scheme(s))
}

/// The DV_URI `Value_valid` (RM invariant) + scheme-presence (CNF conformance)
/// core over the projected value — one source for the typed impl and the
/// value-level fast path (`validate::fast`), mirroring the DV_EHR_URI sibling
/// (`dv_ehr_uri_impl::push_dv_ehr_uri_invariants`).
pub(crate) fn push_dv_uri_invariants(value: &str, out: &mut Vec<InvariantViolation>) {
    // RM invariant `Value_valid: not value.is_empty` (the only DV_URI invariant).
    crate::v1_2::validate::generated::dv_uri_core(value, out);
    // We therefore require an absolute reference (a scheme followed by `:`);
    // per the RM prose we do NOT enforce RFC-3986 encoding of the remainder
    // (plain-text URIs stay valid). The CNF content schedule is the conformance
    // oracle and is stricter than both the RM invariant set and RFC 3986:
    // `master17.7-content_tc_data_types-uri.adoc` (CONT-DV_URI-validate_open)
    // tabulates `xyz | rejected` vs `ftp://ftp.is.co.za/… | accepted`.
    // NOTE: scheme presence is NOT an RM class invariant — the only DV_URI
    // invariant is `Value_valid: not value.is_empty` (`…dv_uri.adoc`
    // §Invariants), and the class prose allows plain-text URIs.
    if !has_scheme(value) {
        out.push(InvariantViolation::here(
            "Invariant Scheme_valid failed on type DV_URI",
        ));
    }
}

impl DvUriData {
    /// RM `DV_URI.scheme()`: the URI scheme (e.g. `ftp`, `mailto`), or the
    /// empty string for a scheme-less (relative) value.
    #[must_use]
    pub fn scheme(&self) -> &str {
        match self.value.split_once(':') {
            Some((s, _)) if is_scheme(s) => s,
            _ => "",
        }
    }

    /// RM `DV_URI.path()`: the location part in scheme-space — the value
    /// after `scheme:` and any `//authority`, up to any `?query` or
    /// `#fragment`.
    #[must_use]
    pub fn path(&self) -> &str {
        let mut rest = self.value.as_str();
        if let Some((s, r)) = rest.split_once(':')
            && is_scheme(s)
        {
            rest = r;
        }
        if let Some(after) = rest.strip_prefix("//") {
            // Authority ends at the next '/', '?' or '#'.
            rest = after
                .find(['/', '?', '#'])
                .and_then(|i| after.get(i..))
                .unwrap_or_default();
        }
        rest.split(['?', '#']).next().unwrap_or("")
    }

    /// RM `DV_URI.query()`: the query string (between `?` and any `#`), or
    /// the empty string.
    #[must_use]
    pub fn query(&self) -> &str {
        self.value
            .split_once('?')
            .map_or("", |(_, q)| q.split('#').next().unwrap_or(""))
    }

    /// RM `DV_URI.fragment_id()`: the fragment (after `#`), or the empty
    /// string.
    #[must_use]
    pub fn fragment_id(&self) -> &str {
        self.value.split_once('#').map_or("", |(_, f)| f)
    }
}

impl Validate for DvUriData {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        push_dv_uri_invariants(&self.value, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uri(value: &str) -> DvUriData {
        DvUriData {
            value: value.to_owned(),
        }
    }

    fn messages(value: &str) -> Vec<String> {
        uri(value)
            .invariants()
            .into_iter()
            .map(|m| m.message)
            .collect()
    }

    #[test]
    fn value_valid() {
        assert!(uri("http://example.org/x").invariants().is_empty());
        // An empty value fails both `Value_valid` (non-empty) and `Scheme_valid`
        // (no scheme), mirroring the DV_EHR_URI sibling `empty_fails_both`.
        let msgs = messages("");
        assert!(msgs.contains(&"Invariant Value_valid failed on type DV_URI".to_owned()));
        assert!(msgs.contains(&"Invariant Scheme_valid failed on type DV_URI".to_owned()));
    }

    /// CNF `master17.7-content_tc_data_types-uri.adoc` (Test Case
    /// CONT-DV_URI-validate_open): "only invalid URIs should be rejected. Any
    /// RFC3986-compliant URI should be accepted"; the row `xyz | rejected`
    /// requires a bare relative reference (no scheme) to be rejected, while
    /// `ftp://ftp.is.co.za/rfc/rfc1808.txt | accepted`. A DV_URI value must
    /// therefore be an absolute reference (scheme present).
    #[test]
    fn scheme_required_for_absolute_reference() {
        let scheme_valid = "Invariant Scheme_valid failed on type DV_URI".to_owned();
        // Absolute references (scheme present) are accepted.
        assert!(
            uri("ftp://ftp.is.co.za/rfc/rfc1808.txt")
                .invariants()
                .is_empty()
        );
        assert!(
            uri("http://example.com/path/resource")
                .invariants()
                .is_empty()
        );
        assert!(uri("mailto:someone@example.org").invariants().is_empty());
        // RM prose allows "plain-text URIs" with RFC-3986-forbidden characters
        // (e.g. a space) — still accepted so long as a scheme is present.
        assert!(uri("http://example.org/a b").invariants().is_empty());
        // Bare relative references (no scheme) are rejected (CNF `xyz` row).
        assert!(messages("xyz").contains(&scheme_valid));
        assert!(messages("www.iana.org").contains(&scheme_valid));
        assert!(messages("content/items").contains(&scheme_valid));
    }

    #[test]
    fn accessors_decompose_uri() {
        let u = uri("ftp://ftp.example.org/pub/images/image_01?fmt=png#frag");
        assert_eq!(u.scheme(), "ftp");
        assert_eq!(u.path(), "/pub/images/image_01");
        assert_eq!(u.query(), "fmt=png");
        assert_eq!(u.fragment_id(), "frag");
    }

    #[test]
    fn accessors_handle_absent_components() {
        let u = uri("mailto:someone@example.org");
        assert_eq!(u.scheme(), "mailto");
        assert_eq!(u.path(), "someone@example.org");
        assert_eq!(u.query(), "");
        assert_eq!(u.fragment_id(), "");

        // A relative (scheme-less) reference.
        let r = uri("content/items");
        assert_eq!(r.scheme(), "");
        assert_eq!(r.path(), "content/items");

        // An authority with no path.
        let a = uri("https://example.org");
        assert_eq!(a.scheme(), "https");
        assert_eq!(a.path(), "");
    }
}
