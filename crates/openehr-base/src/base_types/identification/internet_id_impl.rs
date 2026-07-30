//! Hand-written RM/BASE class invariant for `INTERNET_ID`.
//!
//! `UID.Value_valid`: the identifier string must be non-empty (archie `UID`).

use super::internet_id::InternetId;
use crate::validate::{InvariantViolation, Validate};

impl Validate for InternetId {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if self.value.is_empty() {
            out.push(InvariantViolation::here(
                "Invariant Value_valid failed on type INTERNET_ID",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_valid() {
        assert!(
            InternetId {
                value: "org.openehr".to_owned()
            }
            .invariants()
            .is_empty()
        );
        assert_eq!(
            InternetId {
                value: String::new()
            }
            .invariants()[0]
                .message,
            "Invariant Value_valid failed on type INTERNET_ID"
        );
    }
}
