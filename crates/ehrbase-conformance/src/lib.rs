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
//!
//! The runner ([`run`]) drives a SUT ([`sut`]: external or self-hosted) through a
//! [`harness::Transport`], executes the implemented [`registry`] cases, and
//! [`report`]s the machine- and human-readable result set. Even at the honest
//! zero state the report generates and shows `0/N` — the backlog is enforced and
//! visible.
//!
//! Two pedantic lints are allowed crate-wide because they fight the natural
//! shape of a data-heavy conformance registry, not any real defect:
//! `too_many_lines` (the per-chapter `entries()` functions are long, flat
//! `vec![]` case tables) and `needless_pass_by_value` (the case-builder helpers
//! take small owned payloads by value for call-site ergonomics — a consistent
//! idiom across every `suites/*` module).
#![allow(clippy::too_many_lines, clippy::needless_pass_by_value)]

pub mod assert;
pub mod case;
pub mod client;
pub mod fixtures;
pub mod harness;
pub mod registry;
pub mod report;
pub mod results;
pub mod run;
pub mod schedule;
pub mod sign;
pub mod suites;
pub mod sut;
