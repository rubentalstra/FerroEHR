//! `EXTERNAL_ENVIRONMENT_ACCESS` — mixin providing access to external
//! services.
//!
//! openEHR class: `EXTERNAL_ENVIRONMENT_ACCESS` (abstract, mixin), package
//! `rm.support`.
//!
//! A mixin class providing access to services in the external environment.
//! Its spec `Inherit` clause names two parents:
//! `TERMINOLOGY_SERVICE, MEASUREMENT_SERVICE`.
//!
//! # Flagged mismatch — not resolved here
//!
//! Per ADR-001 §2 (multiple inheritance → supertrait composition), the
//! textbook transcription of `EXTERNAL_ENVIRONMENT_ACCESS` would be:
//!
//! ```ignore
//! pub trait ExternalEnvironmentAccess: TerminologyServiceTrait + MeasurementService {}
//! ```
//!
//! That shape is **not achievable as written** for this specific class,
//! because its two parents were transcribed at different times by
//! different phases into incompatible shapes:
//!
//! - `MEASUREMENT_SERVICE` (this task, `measurement_service.rs`) is an
//!   abstract interface with `Functions` only and no `Attributes`, so per
//!   ADR-001 §1 it became a plain Rust **trait**,
//!   `crate::support::measurement_service::MeasurementService`.
//! - `TERMINOLOGY_SERVICE` was transcribed in **P2**, directly into the
//!   `openehr-terminology` crate, as
//!   [`openehr_terminology::TerminologyService`] — a **concrete struct**
//!   backed by the bundled TERM Release-3.0.0 XML assets
//!   (`crates/openehr-terminology/src/terminology_service.rs`), with
//!   inherent (non-trait) methods and a process-wide `bundled()`
//!   singleton accessor. It is not a trait, and no
//!   `TerminologyServiceTrait`-shaped abstraction over it exists anywhere
//!   in the repository as of this transcription.
//!
//! A `trait ExternalEnvironmentAccess: TerminologyService + MeasurementService`
//! supertrait bound is therefore **not well-formed**: `TerminologyService`
//! is a struct, not a trait, and cannot appear as a supertrait bound. Two
//! genuinely different resolutions are possible and neither is invented
//! here, since choosing one is a judgement call beyond this transcription's
//! scope (it would mean either reshaping the already-compiling,
//! already-tested `openehr-terminology` crate, or reshaping this abstract
//! mixin's contract to hold a concrete dependency instead of a trait
//! bound):
//!
//! 1. Retrofit `openehr-terminology::TerminologyService` behind a trait
//!    (e.g. extract its public inherent-method surface into a
//!    `TerminologyServiceApi` trait, `impl` it for the concrete struct),
//!    then `EXTERNAL_ENVIRONMENT_ACCESS` becomes
//!    `trait ExternalEnvironmentAccess: TerminologyServiceApi + MeasurementService {}`
//!    — symmetric with every other MI case in this port, but requires
//!    touching a crate this transcription is not scoped to touch.
//! 2. Model `EXTERNAL_ENVIRONMENT_ACCESS` asymmetrically: a
//!    `MeasurementService` trait bound (behaviour) plus an *owned or
//!    borrowed* `&'static openehr_terminology::TerminologyService`
//!    field/accessor (state — since it is a concrete singleton-style
//!    service, not a per-instance implementor), which matches how
//!    `TerminologyService::bundled()` is actually consumed today but
//!    departs from the "supertrait composition" MI pattern used
//!    everywhere else.
//!
//! Neither option is applied below. `ExternalEnvironmentAccess` is left as
//! a trait extending only the `MeasurementService` half, with the
//! `TERMINOLOGY_SERVICE` half recorded as a `TODO(port)` rather than
//! silently dropped or silently forced into an ill-typed supertrait bound.
use super::measurement_service::MeasurementService;

/// `EXTERNAL_ENVIRONMENT_ACCESS` — see the module doc comment for the
/// flagged `TERMINOLOGY_SERVICE`-half mismatch this trait does **not**
/// resolve.
///
/// TODO(port): add the `TERMINOLOGY_SERVICE` half of this mixin's
/// inheritance once a decision is made between (1) a
/// `TerminologyServiceApi` trait extracted from
/// `openehr_terminology::TerminologyService`, or (2) a concrete
/// dependency field/accessor. Do not add either speculatively here.
pub trait ExternalEnvironmentAccess: MeasurementService {}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 support §Class Definitions EXTERNAL_ENVIRONMENT_ACCESS — docs/research/spec-cache/RM-1.1.0/uml_classes/external_environment_access.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-support_package.adoc §Class Definitions / uml_classes/external_environment_access.adoc §EXTERNAL_ENVIRONMENT_ACCESS Class
//   confidence: low
//   todos: 2
//   note: TERMINOLOGY_SERVICE half of the spec's declared multiple inheritance is deliberately NOT encoded — it is a concrete struct in openehr-terminology (P2), not a trait, so it cannot be a supertrait bound; two resolutions are documented but neither applied, pending a decision outside this transcription's scope.
// ─────────────────────────────────────────────
