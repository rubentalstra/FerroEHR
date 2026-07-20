//! Terminology-server integration support for the ECC: the hermetic
//! `wiremock` FHIR-tx [`fixture`] the runner spins up, and the [`TxServer`]
//! descriptor threaded into a case's [`crate::harness::RunContext`] so a case
//! knows which terminology server the harness has available (the CI fixture or
//! a real server via `--tx-server-url`).
//!
//! Two modes: hermetic `wiremock` by default, real server on demand.

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
    /// Whether the **SUT** is configured with a FHIR terminology provider
    /// pointing at this server (the composed-run wiring: the fixture bound to a
    /// fixed host port + the SUT's `[terminology.external]` provider aimed at it
    /// via `host.docker.internal`). When `true`, the FHIR-provider + fault cases
    /// (`ECC-TS-006…009`) can drive the SUT end to end; when `false` (a bare
    /// `nextest` run with no SUT wiring, or a `--tx-server-url` the SUT does not
    /// use) they report `SKIPPED(SutConfig)`.
    pub wired: bool,
}

impl TxServer {
    /// A real-server descriptor (`--tx-server-url`). Not wired to the SUT — a
    /// real server named on the CLI is a server the *harness* can reach, not one
    /// the SUT's provider is pointed at.
    #[must_use]
    pub fn real(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            mode: TxMode::Real,
            wired: false,
        }
    }

    /// A fixture descriptor for `base_url` (the spun-up [`FhirTxFixture`]'s URL),
    /// not wired to any SUT (the in-process `nextest` default).
    #[must_use]
    pub fn fixture(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            mode: TxMode::Fixture,
            wired: false,
        }
    }

    /// Mark this server as wired into the SUT (the composed conformance run):
    /// the SUT's FHIR terminology provider is configured to reach it.
    #[must_use]
    pub fn wired(mut self) -> Self {
        self.wired = true;
        self
    }
}
