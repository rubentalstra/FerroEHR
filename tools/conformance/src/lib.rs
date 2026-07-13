//! The ehrbase-rs Conformance Catalogue (ECC) runner — W-10 redesign.
//!
//! A spec-first openEHR conformance instrument able to assess **any** ITS-REST
//! CDR: the case universe derives from the CNF Platform Conformance Test
//! Schedule (`docs/specs/openehr/CNF/docs/platform_test_schedule/`), profiles
//! and claims from `CNF/docs/profiles/master03-profiles.adoc`, and the result
//! artefacts from `CNF/docs/certificate/master03-certificate.adoc`. Design:
//! `docs/design/conformance/` (registers 01–13, 80 data sets, 90 target).
//!
//! Framework law (owner rulings, carried):
//! - **Own identity**: every case is an ECC case (`ECC-<AREA>-<NNN>`,
//!   allocated once in `inventory/ecc-catalog.tsv`, never reused). Official
//!   schedule ids are trace references ([`model::case::ScheduleTrace`]),
//!   never the key system; no Robot/legacy mapping machinery exists.
//! - **Multi-SUT**: one case universe drives every SUT
//!   ([`sut::SutDescriptor`] — ehrbase-rs, upstream `EHRbase`, or any
//!   bring-your-own endpoint by URL). Per-SUT facts (system id, template-id
//!   format, admin mount) come from the descriptor, never from literals.
//! - **Edition ladder**: assertions separate their normative core from
//!   edition-specific wire forms; the runner tries the highest edition first
//!   and steps down, recording the satisfied level as an edition finding
//!   ([`edition`]). CNF backing: `master03-overview.adoc` §API Conformance
//!   (supported RM versions are stated in the Conformance Statement).
//! - **Honesty invariants**: spec identity from provenance
//!   ([`model::versions`]); spec-contradicting cases are adjudicated
//!   ([`model::adjudication`]), the SUT is never bent to a wrong case;
//!   every coverage bound is logged ([`engine::harness::DataSetReport`]);
//!   profile verdicts are machine-computed ([`model::profile`]); foreign
//!   runs get fairness triage ([`model::fairness`]). Every SUT receives the
//!   full artefact set incl. the Certificate — always a framework
//!   self-assessment, never an official openEHR certification.

pub mod edition;
pub mod engine;
pub mod model;
pub mod reporting;
pub mod suites;
pub mod sut;
pub mod testdata;
pub mod ts;
pub mod wire;

pub use engine::{assert, harness, registry, run, transport};
pub use model::{adjudication, case, catalog, fairness, profile, versions};
