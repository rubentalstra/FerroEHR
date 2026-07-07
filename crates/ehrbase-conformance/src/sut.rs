//! SUT lifecycle (design §4.3): the two modes both required by the framework.
//!
//! - **External** — a pure API client against a deployed real system (the
//!   guide's own model; certification-grade). Two credential slots.
//! - **`SelfHosted`** (`self-host` feature) — boots testcontainers PG18 +
//!   `EhrbaseService` + the axum app in-process on an ephemeral port; the fast
//!   inner loop for development and PR-time CI.
//!
//! Both expose a [`Transport`](crate::harness::Transport) so a case runs against
//! either unchanged.

use crate::client::{Credential, SutClient};
use crate::harness::{Transport, TransportError};

/// Errors raised setting up a SUT.
#[derive(Debug, thiserror::Error)]
pub enum SutError {
    /// The HTTP client could not be built.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// The self-hosted SUT failed to boot.
    #[error("self-hosted SUT boot failed: {0}")]
    Boot(String),
}

/// A SUT the runner drives. Owns any in-process lifecycle (container + server
/// task) for the self-hosted mode; a no-op for external.
pub struct Sut {
    client: SutClient,
    _keep_alive: KeepAlive,
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
    /// # Errors
    /// [`SutError::Transport`] if the HTTP client cannot be built.
    pub fn external(
        base_url: impl Into<String>,
        regular: Option<Credential>,
        admin: Option<Credential>,
    ) -> Result<Self, SutError> {
        Ok(Self {
            client: SutClient::new(base_url, regular, admin)?,
            _keep_alive: KeepAlive::External,
        })
    }

    /// Boot a self-hosted SUT: testcontainers PG18 + the real app in-process.
    ///
    /// # Errors
    /// [`SutError::Boot`] if the container, database, or server cannot be started.
    #[cfg(feature = "self-host")]
    pub async fn self_hosted() -> Result<Self, SutError> {
        self_host::boot().await
    }
}

/// Keeps in-process resources alive for the lifetime of the [`Sut`]. The
/// self-hosted variant's state is held only for its `Drop` (it stops the server
/// task and the container).
enum KeepAlive {
    External,
    // Held only to run its `Drop` (stops the server task + testcontainer);
    // never read, by design. Boxed so the enum isn't sized by the (large)
    // self-host state in every build.
    #[cfg(feature = "self-host")]
    SelfHosted(#[allow(dead_code)] Box<self_host::SelfHostState>),
}

#[cfg(feature = "self-host")]
mod self_host {
    use std::net::SocketAddr;
    use std::sync::Arc;

    use testcontainers::runners::AsyncRunner;
    use testcontainers::{ContainerAsync, ImageExt};
    use testcontainers_modules::postgres::Postgres;
    use tower::Layer;
    use tower_http::normalize_path::NormalizePathLayer;

    use ehrbase::db::{self, DbSettings};
    use ehrbase::service::EhrbaseService;
    use ehrbase_rest::{Backend, RestConfig};

    use super::{KeepAlive, Sut, SutError};
    use crate::client::{Credential, SutClient};

    /// The dev Basic credential the self-hosted app is configured with (mirrors
    /// `docker/ehrbase.dev.toml`): username `ehrbase`, password `ehrbase`.
    const DEV_USER: &str = "ehrbase";
    const DEV_PASS: &str = "ehrbase";
    /// Argon2id PHC hash of `ehrbase` (from `docker/ehrbase.dev.toml`).
    const DEV_PASS_HASH: &str = "$argon2id$v=19$m=4096,t=2,p=1$ZWhyYmFzZURldlNhbHQ$4Cf1W/JiP800r2sbj8/y0HbNAcwMb/YseM1fTINO3Dc";

    /// The in-process resources kept alive for the SUT's lifetime.
    pub(super) struct SelfHostState {
        _container: ContainerAsync<Postgres>,
        server: tokio::task::JoinHandle<()>,
    }

    impl Drop for SelfHostState {
        fn drop(&mut self) {
            self.server.abort();
        }
    }

    /// Boot the container, migrate, build the app, and serve on an ephemeral port.
    pub(super) async fn boot() -> Result<Sut, SutError> {
        let container = Postgres::default()
            .with_tag("18")
            .start()
            .await
            .map_err(|e| SutError::Boot(format!("start postgres:18 (is Docker running?): {e}")))?;
        let host = container
            .get_host()
            .await
            .map_err(|e| SutError::Boot(e.to_string()))?
            .to_string();
        let port = container
            .get_host_port_ipv4(5432)
            .await
            .map_err(|e| SutError::Boot(e.to_string()))?;

        let dsn = format!("postgres://postgres:postgres@{host}:{port}/postgres");
        let pool = db::connect(&DbSettings::new(dsn))
            .await
            .map_err(|e| SutError::Boot(format!("connect: {e}")))?;
        db::run_migrations(&pool)
            .await
            .map_err(|e| SutError::Boot(format!("migrate: {e}")))?;

        let config = rest_config()?;
        let base_path = config.base_path.clone();
        let backend: Arc<dyn Backend> = Arc::new(EhrbaseService::new(pool));
        let router = ehrbase_rest::build_with(config, backend)
            .map_err(|e| SutError::Boot(format!("build router: {e}")))?;

        // Bind an ephemeral port ourselves so we know it (serve_with does not
        // surface the bound address), applying the same NormalizePathLayer +
        // ConnectInfo the production `run_server` uses.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| SutError::Boot(format!("bind: {e}")))?;
        let addr = listener
            .local_addr()
            .map_err(|e| SutError::Boot(e.to_string()))?;
        let app = NormalizePathLayer::trim_trailing_slash().layer(router);
        let make = axum::ServiceExt::<axum::extract::Request>::into_make_service_with_connect_info::<
            SocketAddr,
        >(app);
        let server = tokio::spawn(async move {
            let _ = axum::serve(listener, make).await;
        });

        let base_url = format!("http://{addr}{base_path}");
        let status_url = base_path.strip_suffix("/openehr/v1").map_or_else(
            || base_url.clone(),
            |root| format!("http://{addr}{root}/status"),
        );
        wait_ready(&status_url).await?;

        let cred = Credential::Basic {
            user: DEV_USER.to_owned(),
            pass: DEV_PASS.to_owned(),
        };
        let client = SutClient::new(base_url, Some(cred.clone()), Some(cred))?;
        Ok(Sut {
            client,
            _keep_alive: KeepAlive::SelfHosted(Box::new(SelfHostState {
                _container: container,
                server,
            })),
        })
    }

    /// The self-hosted REST config: the ITS-REST base path with the dev Basic
    /// user configured (built via serde to avoid importing the auth types).
    fn rest_config() -> Result<RestConfig, SutError> {
        serde_json::from_value(serde_json::json!({
            "bind": "127.0.0.1:0",
            "base_path": "/ehrbase/rest/openehr/v1",
            "swagger_ui": false,
            "auth": {
                "enabled": true,
                "basic": {
                    "users": [
                        {
                            "username": DEV_USER,
                            "password_hash": DEV_PASS_HASH,
                            "roles": ["USER", "ADMIN"],
                        }
                    ]
                }
            }
        }))
        .map_err(|e| SutError::Boot(format!("static self-host RestConfig invalid: {e}")))
    }

    /// Poll `status_url` until it answers 2xx (the app is up) or we give up.
    async fn wait_ready(status_url: &str) -> Result<(), SutError> {
        let http = reqwest::Client::new();
        for _ in 0..100 {
            if let Ok(resp) = http.get(status_url).send().await
                && resp.status().is_success()
            {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        Err(SutError::Boot(format!(
            "self-hosted app did not become ready at {status_url}"
        )))
    }
}
