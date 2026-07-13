//! The system under test (design §3.1, §6): a named identity plus the
//! conformance crate's [`SutClient`], so the request path driving ehrbase-rs is
//! **provably identical** to the one driving `EHRbase` Java — the core fairness
//! guarantee (design §5).

use conformance::transport::{Credential, SutClient};
use conformance::harness::{HttpRequest, HttpResponse, Transport, TransportError};

/// Which implementation a target points at — recorded in the report so a reader
/// always knows which server produced a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Implementation {
    /// This project.
    EhrbaseRs,
    /// The reference implementation (`ehrbase/ehrbase`, Java/Spring Boot).
    EhrbaseJava,
}

impl Implementation {
    /// A short label for reports/CLI.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Implementation::EhrbaseRs => "ehrbase-rs",
            Implementation::EhrbaseJava => "ehrbase-java",
        }
    }
}

impl std::str::FromStr for Implementation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().replace(['_', ' '], "-").as_str() {
            "ehrbase-rs" | "rs" | "rust" => Ok(Implementation::EhrbaseRs),
            "ehrbase-java" | "java" | "ehrbase" => Ok(Implementation::EhrbaseJava),
            other => Err(format!(
                "unknown implementation {other:?} (expected ehrbase-rs | ehrbase-java)"
            )),
        }
    }
}

/// A benchmark target: an implementation identity + a client reaching it.
#[derive(Debug, Clone)]
pub struct Target {
    /// Which implementation this is.
    pub implementation: Implementation,
    /// The ITS-REST base URL (e.g. `http://host:8080/ehrbase/rest/openehr/v1`).
    pub base_url: String,
    client: SutClient,
}

impl Target {
    /// Build a target against `base_url` with an optional regular + admin
    /// credential.
    ///
    /// # Errors
    /// [`TransportError`] if the HTTP client cannot be constructed.
    pub fn new(
        implementation: Implementation,
        base_url: impl Into<String>,
        regular: Option<Credential>,
        admin: Option<Credential>,
    ) -> Result<Self, TransportError> {
        let base_url = base_url.into();
        let client = SutClient::new(base_url.clone(), regular, admin)?;
        Ok(Self {
            implementation,
            base_url: base_url.trim_end_matches('/').to_owned(),
            client,
        })
    }

    /// Send a request to this target (the identical path for both servers).
    ///
    /// # Errors
    /// [`TransportError`] on a transport-level failure.
    pub async fn send(&self, request: HttpRequest) -> Result<HttpResponse, TransportError> {
        self.client.send(request).await
    }

    /// A short identity label.
    #[must_use]
    pub fn label(&self) -> &'static str {
        self.implementation.label()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_implementation_aliases() {
        assert_eq!(
            "ehrbase-rs".parse::<Implementation>().unwrap(),
            Implementation::EhrbaseRs
        );
        assert_eq!(
            "java".parse::<Implementation>().unwrap(),
            Implementation::EhrbaseJava
        );
        assert!("mysql".parse::<Implementation>().is_err());
    }

    #[test]
    fn trims_trailing_slash() {
        let t = Target::new(
            Implementation::EhrbaseRs,
            "http://localhost:8080/ehrbase/rest/openehr/v1/",
            None,
            None,
        )
        .unwrap();
        assert!(!t.base_url.ends_with('/'));
    }
}
