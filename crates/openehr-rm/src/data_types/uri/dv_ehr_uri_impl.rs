//! Hand-written RM class invariants for `DV_EHR_URI`.
//!
//! Mirrors archie `DvEHRURI` (which extends `DvURI`):
//! - `Scheme_valid`: the URI scheme must be `ehr` (case-insensitive).
//! - `Value_valid`: inherited from `DV_URI` — value non-empty.
//!
//! An empty value fails both (as in archie `DvEhrUriInvariantTest.invalid2`).

use crate::data_types::uri::dv_ehr_uri::DvEhrUri;
use crate::validate::{InvariantViolation, Validate};

/// The scheme is the substring before the first `:` (RFC-3986 `scheme:` prefix).
fn scheme(value: &str) -> Option<&str> {
    value.split_once(':').map(|(s, _)| s)
}

impl Validate for DvEhrUri {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if !scheme(&self.value).is_some_and(|s| s.eq_ignore_ascii_case("ehr")) {
            out.push(InvariantViolation::here(
                "Invariant Scheme_valid failed on type DV_EHR_URI",
            ));
        }
        if self.value.is_empty() {
            out.push(InvariantViolation::here(
                "Invariant Value_valid failed on type DV_EHR_URI",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ehr_uri(value: &str) -> Vec<InvariantViolation> {
        DvEhrUri {
            value: value.to_owned(),
        }
        .invariants()
    }

    #[test]
    fn valid() {
        assert!(ehr_uri("ehr://something/something").is_empty());
    }

    #[test]
    fn wrong_scheme() {
        let v = ehr_uri("https://something/something");
        assert_eq!(v.len(), 1);
        assert_eq!(
            v[0].message,
            "Invariant Scheme_valid failed on type DV_EHR_URI"
        );
    }

    #[test]
    fn no_scheme() {
        let v = ehr_uri("target1");
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Scheme_valid failed on type DV_EHR_URI")
        );
    }

    #[test]
    fn empty_fails_both() {
        let msgs: Vec<_> = ehr_uri("").into_iter().map(|m| m.message).collect();
        assert!(msgs.contains(&"Invariant Scheme_valid failed on type DV_EHR_URI".to_owned()));
        assert!(msgs.contains(&"Invariant Value_valid failed on type DV_EHR_URI".to_owned()));
    }
}
