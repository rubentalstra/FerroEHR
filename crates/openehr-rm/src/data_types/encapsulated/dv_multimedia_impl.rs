//! Hand-written RM class invariants for `DV_MULTIMEDIA`.
//!
//! Mirrors archie `DvMultimedia` (the non-terminology invariants):
//! - `Not_empty`: `data` (inline) or `uri` (external) must be present.
//! - `Integrity_check_validity`: an integrity check implies an integrity-check
//!   algorithm.
//! - `Size_valid`: `size >= 0`.
//!
//! NOTE: openEHR's `Size_valid` is `size >= 0`; archie implements it as
//! `size > 0` (a known reference quirk that rejects a legitimately empty
//! multimedia). We follow the **spec** (`>= 0`) — by design the openEHR
//! spec, not a specific reference implementation, the conformance target.
//!
//! NOTE: the terminology-bound invariants (`Media_type_valid`,
//! `Compression_algorithm_valid`, `Integrity_check_algorithm_validity`,
//! `Charset_valid`, `Language_valid`) are deferred to the composition validator
//! + `openehr-term`.

use crate::data_types::encapsulated::dv_multimedia::DvMultimedia;
use crate::validate::{InvariantViolation, Validate};

impl Validate for DvMultimedia {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if self.data.is_none() && self.uri.is_none() {
            out.push(InvariantViolation::here(
                "Invariant Not_empty failed on type DV_MULTIMEDIA",
            ));
        }
        if self.integrity_check.is_some() && self.integrity_check_algorithm.is_none() {
            out.push(InvariantViolation::here(
                "Invariant Integrity_check_validity failed on type DV_MULTIMEDIA",
            ));
        }
        if self.size < 0 {
            out.push(InvariantViolation::here(
                "Invariant Size_valid failed on type DV_MULTIMEDIA",
            ));
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;
    use crate::data_types::text::code_phrase::CodePhrase;
    use openehr_base::prelude::TerminologyId;

    fn media_type() -> CodePhrase {
        CodePhrase {
            terminology_id: TerminologyId {
                value: "IANA_media-types".to_owned(),
            },
            code_string: "image/png".to_owned(),
            preferred_term: None,
        }
    }

    fn valid() -> DvMultimedia {
        DvMultimedia {
            charset: None,
            language: None,
            alternate_text: None,
            uri: None,
            data: Some("AAA=".to_owned()),
            media_type: media_type(),
            compression_algorithm: None,
            integrity_check: None,
            integrity_check_algorithm: None,
            thumbnail: None,
            size: 1,
        }
    }

    #[test]
    fn valid_multimedia() {
        assert!(valid().invariants().is_empty());
    }

    #[test]
    fn empty_invalid() {
        let mut m = valid();
        m.data = None;
        let v = m.invariants();
        assert!(
            v.iter()
                .any(|x| x.message == "Invariant Not_empty failed on type DV_MULTIMEDIA")
        );
    }

    #[test]
    fn integrity_check_without_algorithm() {
        let mut m = valid();
        m.integrity_check = Some("deadbeef".to_owned());
        let v = m.invariants();
        assert!(v.iter().any(
            |x| x.message == "Invariant Integrity_check_validity failed on type DV_MULTIMEDIA"
        ));
    }

    #[test]
    fn negative_size_invalid() {
        let mut m = valid();
        m.size = -10;
        let v = m.invariants();
        assert!(
            v.iter()
                .any(|x| x.message == "Invariant Size_valid failed on type DV_MULTIMEDIA")
        );
    }

    #[test]
    fn zero_size_valid_per_spec() {
        // openEHR spec permits size == 0 (archie's `> 0` quirk would reject it).
        let mut m = valid();
        m.size = 0;
        assert!(m.invariants().is_empty());
    }
}
