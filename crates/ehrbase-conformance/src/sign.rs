//! The runner-defined `SIGN-*` capability cases (design §4.6): the STANDARD
//! Signing capability's entire evidence base (upstream ships no Signing test
//! material). Specified against `docs/design/version-signing.md`. Not yet
//! transcribed — the digest cases (commit a composition, read the VERSION, assert
//! `signature` matches `sha256:<base64>` and recomputes from the served canonical
//! form) land with master07 (a composition commit is their precondition).

use crate::registry::CaseEntry;

/// The runner-defined SIGN-* case entries (none yet).
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    Vec::new()
}
