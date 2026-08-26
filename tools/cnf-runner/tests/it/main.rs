// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for `cnf-runner`, the CNF 2.0 conformance runner: the
//! committed catalogue's artifact gates and schema-drift guards, the
//! claim/coverage completeness gates, defect-fixture rejection, the
//! measured-performance driver, and the self-verification pack.
//!
//! One binary per crate, split into topic modules
//! (`.claude/rules/testing.md` §One integration-test binary per crate).

#![expect(
    clippy::disallowed_types,
    reason = "test fixtures and wire assertions are raw JSON by the testing rule \
              (.claude/rules/testing.md §Test-fixture construction)"
)]

mod artifact_gates;
mod claim_completeness;
mod defect_rejection;
mod perf_driver;
mod schema_drift;
mod verification_pack;
