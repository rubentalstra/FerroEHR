//! Terminology-server integration support for the ECC (B4): the hermetic
//! `wiremock` FHIR-tx [`fixture`] the runner spins up, and the [`TxServer`]
//! descriptor threaded into a case's [`crate::harness::RunContext`] so a case
//! knows which terminology server the harness has available (the CI fixture or
//! a real server via `--tx-server-url`).
//!
//! Design: `docs/design/terminology-server-integration.md` §5 (two modes:
//! hermetic `wiremock` by default, real server on demand).

pub mod fixture;

pub use fixture::{Fault, FhirTxFixture};

/// Which kind of terminology server the harness has available for the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxMode {
    /// The hermetic `wiremock` FHIR-tx fixture the runner spun up (the CI
    /// default).
    Fixture,
    /// A real FHIR R4 terminology server named by `--tx-server-url`.
    Real,
}

impl TxMode {
    /// A stable lowercase label for the report / results record.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            TxMode::Fixture => "fixture",
            TxMode::Real => "real",
        }
    }
}

/// The terminology server available to a run: its FHIR base URL and whether it
/// is the hermetic fixture or a real server. A case reads this (via
/// [`crate::harness::RunContext::tx`]) to phrase its skip reason and to record
/// the server it targeted; it never reconfigures the SUT, which is reached only
/// over its own ITS-REST transport.
#[derive(Debug, Clone)]
pub struct TxServer {
    /// The FHIR R4 base URL.
    pub base_url: String,
    /// Fixture or real.
    pub mode: TxMode,
}

impl TxServer {
    /// A real-server descriptor (`--tx-server-url`).
    #[must_use]
    pub fn real(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            mode: TxMode::Real,
        }
    }

    /// A fixture descriptor for `base_url` (the spun-up [`FhirTxFixture`]'s URL).
    #[must_use]
    pub fn fixture(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            mode: TxMode::Fixture,
        }
    }
}
