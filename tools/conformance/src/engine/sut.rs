//! SUT lifecycle (design §4.3): a pure API client against a deployed real
//! system (the guide's own model; certification-grade), with two credential
//! slots.
//!
//! There is deliberately **no in-process self-hosted mode**: the runner always
//! drives the real deployable artefact — the Docker-composed server brought up
//! by `scripts/conformance.sh` (or any externally deployed SUT) — so the wire
//! under test is the production `serve_full` stack, never a re-wired
//! approximation that can drift from the binary (owner ruling 2026-07-09; the
//! removed `self-host` feature did exactly that during the crate-layout rebuild).
//!
//! The client exposes a [`Transport`](crate::harness::Transport) so a case runs
//! against any SUT unchanged.

use crate::client::{Credential, SutClient};
use crate::harness::{Transport, TransportError};

/// Errors raised setting up a SUT.
#[derive(Debug, thiserror::Error)]
pub enum SutError {
    /// The HTTP client could not be built.
    #[error(transparent)]
    Transport(#[from] TransportError),
}

/// A SUT the runner drives over HTTP.
pub struct Sut {
    client: SutClient,
}

impl std::fmt::Debug for Sut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sut")
            .field("base_url", &self.client.describe())
            .finish_non_exhaustive()
    }
}

impl Sut {
    /// The transport reaching this SUT.
    #[must_use]
    pub fn transport(&self) -> &dyn Transport {
        &self.client
    }

    /// The SUT base URL.
    #[must_use]
    pub fn base_url(&self) -> String {
        self.client.describe()
    }

    /// An external SUT at `base_url` with the given credential slots.
    ///
    /// `admin_base_url`, when `Some`, routes `/admin/*` requests to a sibling
    /// admin mount (e.g. upstream `EHRbase`'s `/ehrbase/rest/admin`, §3a.5); `None`
    /// nests admin under `base_url` as every existing run does.
    ///
    /// # Errors
    /// [`SutError::Transport`] if the HTTP client cannot be built.
    pub fn external(
        base_url: impl Into<String>,
        regular: Option<Credential>,
        admin: Option<Credential>,
        admin_base_url: Option<String>,
    ) -> Result<Self, SutError> {
        Ok(Self {
            client: SutClient::new(base_url, regular, admin)?.with_admin_base_url(admin_base_url),
        })
    }
}
