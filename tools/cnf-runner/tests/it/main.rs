//! Integration tests for `cnf-runner`, the CNF 2.0 conformance runner: the
//! committed catalogue's artifact gates and schema-drift guards, the
//! claim/coverage completeness gates, defect-fixture rejection, the
//! measured-performance driver, and the self-verification pack.
//!
//! One binary per crate, split into topic modules
//! (`.claude/rules/testing.md` §One integration-test binary per crate).

mod artifact_gates;
mod claim_completeness;
mod defect_rejection;
mod perf_driver;
mod schema_drift;
mod verification_pack;
