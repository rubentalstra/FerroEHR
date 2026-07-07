//! openEHR CNF conformance runner — the ADR-008 acceptance instrument.
//!
//! Implements the openEHR **Platform Conformance Test Schedule**
//! (`docs/specs/openehr/CNF/docs/platform_test_schedule/`, vendored at the
//! upstream `master` HEAD) as a native Rust test-case registry keyed on the
//! schedule's own case ids (`I_<SERVICE>.<operation>-<variant>` /
//! `CONT-<CLASS>-<variant>`), per `docs/design/conformance-framework.md`.
//!
//! The load-bearing property is **enforced total coverage** (design §4.2): a
//! guard test parses the vendored schedule text and asserts every identified
//! test case is either implemented in the registry or explicitly excluded
//! with a structural reason. New or changed upstream cases fail the build
//! until triaged; nothing is ever silently uncovered.

pub mod case;
pub mod registry;
pub mod schedule;
