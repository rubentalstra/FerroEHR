//! Mapping note: `rm.support.terminology` → `openehr-terminology` (P2).
//!
//! This file intentionally transcribes **no types**. It exists as a
//! doc-only pointer so a reader of `crates/openehr-rm/src/support/` finds
//! the terminology package's disposition explicitly recorded here, rather
//! than concluding it was skipped.
//!
//! # Where the classes actually are
//!
//! The `rm.support.terminology` package (RM 1.1.0 §support,
//! `master05-terminology_package.adoc`) defines the interface/constants
//! cluster:
//!
//! | Spec class | Kind | Landed as |
//! |---|---|---|
//! | `TERMINOLOGY_SERVICE` | class (mixin-inherits two constants classes) | [`openehr_term::TerminologyService`] — concrete struct, `crates/openehr-term/src/terminology_service.rs` |
//! | `TERMINOLOGY_ACCESS` | interface | [`openehr_term::TerminologyAccess`] (trait) + [`openehr_term::BundledTerminologyAccess`] (impl), `crates/openehr-term/src/terminology_access.rs` |
//! | `CODE_SET_ACCESS` | interface | [`openehr_term::CodeSetAccess`] (trait) + [`openehr_term::BundledCodeSetAccess`] (impl), `crates/openehr-term/src/code_set_access.rs` |
//! | `OPENEHR_TERMINOLOGY_GROUP_IDENTIFIERS` | constants class | [`openehr_term::OpenehrTerminologyGroupIdentifiers`], `crates/openehr-term/src/openehr_terminology_group_identifiers.rs` |
//! | `OPENEHR_CODE_SET_IDENTIFIERS` | constants class | [`openehr_term::OpenehrCodeSetIdentifiers`], `crates/openehr-term/src/openehr_code_set_identifiers.rs` |
//!
//! This was a deliberate P2 decision (not made by this transcription pass):
//! `openehr-terminology` is a dependency leaf that is already wired and
//! compiling (unlike the rest of `openehr-rm`, which is Phase A / does not
//! need to compile until P17), so the terminology-service surface was
//! transcribed directly into its own crate rather than staged here first.
//! `openehr-rm`'s `Cargo.toml` already declares
//! `openehr-terminology = { path = "../openehr-terminology" }` as a
//! dependency, so these types are already reachable from `openehr-rm` code
//! via `openehr_term::...` — see, for example, the mismatch
//! documented in `external_environment_access.rs`, which depends on this
//! fact directly.
//!
//! # Relationship to `EXTERNAL_ENVIRONMENT_ACCESS`
//!
//! `EXTERNAL_ENVIRONMENT_ACCESS` (this same `rm.support` package,
//! `external_environment_access.rs`) spec-inherits `TERMINOLOGY_SERVICE`
//! alongside `MEASUREMENT_SERVICE`. Because `TerminologyService` above is a
//! concrete struct and not a trait, that inheritance could not be encoded
//! as a supertrait bound the way `MEASUREMENT_SERVICE` was — see that
//! file's doc comment for the full account; this file only records the
//! terminology-side half of that story.
//!
//! TODO(port): at P17 (make-it-compile), if `openehr-rm` code wants to
//! refer to the terminology surface under an `rm::support::terminology`-
//! shaped path for symmetry with `measurement_service`, add a thin
//! re-export here, e.g.:
//!
//! ```ignore
//! pub use openehr_term::{
//!     BundledCodeSetAccess, BundledTerminologyAccess, CodeSetAccess,
//!     OpenehrCodeSetIdentifiers, OpenehrTerminologyGroupIdentifiers,
//!     TerminologyAccess, TerminologyService,
//! };
//! ```
//!
//! Not added speculatively now: whether `openehr-rm::support` should
//! re-export these types, versus callers importing
//! `openehr_term::*` directly, is a P17 crate-API-surface decision,
//! not a transcription one.
//!
//! # `support.assumed_types`
//!
//! Separately, the `support.assumed_types` pseudo-package
//! (`master03-assumed_types.adoc`) is explicitly noted in the spec itself
//! as relocated: "These sections have been removed to a separate
//! specification in the BASE component." Its three subsections (Inbuilt
//! Primitive Types, Assumed Library Types, Date/Time Types) map to the
//! BASE Foundation Types specification's Primitive Types, Structure Types,
//! and Time Types sections respectively — i.e. the **P1** work already
//! done in `crates/openehr-foundation/src/{primitive_types,structure_types,time}/`
//! (see the `project-time-types-precedent` memory for the Time Types
//! cluster specifically). Nothing in `support.assumed_types` requires
//! transcription in `openehr-rm` at all; this paragraph is the record of
//! that fact for anyone looking for it under `rm.support`.

// This file transcribes no types (it is a doc-only mapping note, per the
// task that produced it); the PORT STATUS trailer is retained anyway for
// grep-consistency across the crate.

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 support (mapping note only, no types transcribed) — docs/research/spec-cache/RM-1.1.0/support/master05-terminology_package.adoc + master03-assumed_types.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-terminology_package.adoc (whole file) / master03-assumed_types.adoc (whole file)
//   confidence: high
//   todos: 2
//   note: doc-only file recording that TERMINOLOGY_SERVICE/TERMINOLOGY_ACCESS/CODE_SET_ACCESS/identifier classes landed in openehr-terminology at P2, and that support.assumed_types maps to openehr-foundation from P1; TODO(port) re-export deferred to P17.
// ─────────────────────────────────────────────
