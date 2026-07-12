//! openEHR `VERSION.signature` signing + verification for the ehrbase-rs CDR.
//!
//! Implements the RM common §"Digital Signature" mechanism
//! (`docs/specs/openehr/RM/docs/common/master06-change_control_package.adoc`;
//! `VERSION.signature` / `VERSION.canonical_form()` in
//! `org.openehr.rm.common.version`). Core change-control behaviour, so it lives
//! in the `ehrbase` platform crate as the `signing` module (the crate-layout
//! consolidation, 2026-07-09). Design: `docs/design/version-signing.md`.
//!
//! Two spec-blessed modes, both first-class (design §3.2):
//! - **digest** — `sha256:` + radix-64(SHA-256(canonical_form)): a data-integrity
//!   check that needs no key management (the default; makes the STANDARD
//!   "Signing" capability demonstrably met out of the box).
//! - **pgp** — an `OpenPGP` RFC 4880 detached signature (rPGP), ASCII-armored,
//!   with a server-held private key: authentication + non-repudiation.
//!
//! The bytes signed are the `VERSION.canonical_form()` produced by
//! `openehr-rm` (canonical openEHR JSON canonicalised per RFC 8785); this crate
//! is agnostic to how that string is produced — it only signs/verifies it.

mod config;
mod key;
mod signer;
mod verify;

pub use config::{Mode, SigningConfig, VerifyOnRead};
pub use key::{KeyError, PgpKey, PgpSignError};
pub use signer::{SignError, Signer, SigningError};
pub use verify::Verdict;
