//! The IXIT (`ixit.json`) — the SUT topology the runner drives: one or more
//! named instances (base URL + auth + overrides) plus the environment block.
//!
//! ISO/IEC 9646 names this artifact the IXIT (implementation extra
//! information for testing); the schedule's party-artifact contract makes it
//! the single file that drives any runner against any SUT topology.
//! Single-instance platform cases use the default instance `sut`;
//! multi-instance cases and the security principals address ixit-declared
//! instances via the flow `on:` selector.

use serde::{Deserialize, Serialize};

use crate::ids::InstanceName;

/// Authentication mode of an instance. Credentials are REFERENCES (env-var
/// names), never inline secrets — the ixit file is committed/shared.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum AuthMode {
    /// No Authorization header at all (the `unauthenticated` principal).
    None,
    /// HTTP Basic; user/password resolved from the named environment-variable
    /// pair at run time.
    Basic {
        user_env: String,
        password_env: String,
    },
    /// `OAuth2` bearer token resolved from the named environment variable.
    Bearer { token_env: String },
}

/// One named SUT instance.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Instance {
    /// The openEHR REST base (up to and including the API version segment,
    /// e.g. `http://localhost:8080/ehrbase/rest/openehr/v1`).
    pub base_url: String,
    pub auth: AuthMode,
    /// Extra headers stamped on every request to this instance.
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub headers: Option<Vec<(String, String)>>,
}

/// The environment block — mandatory for performance runs, informative
/// otherwise. `Serialize` because every measurement record embeds the
/// environment it was taken in (an earned class is reported WITH its
/// environment, never bare).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    /// Whether the SUT instance is exclusively owned by this run — the
    /// precondition for `requires.server: exclusive` cases (global-state
    /// grounds like an empty template list). Defaults to `false`: a shared
    /// instance N/As those cases.
    #[serde(default)]
    pub exclusive_server: bool,
    pub hardware_class: String,
    pub cores: u32,
    pub memory_gb: u32,
    pub storage_class: String,
    pub topology: String,
}

/// The container-runtime identities of the composed SUT — topology facts,
/// exactly what the ixit is for. Presence enables resource sampling on
/// measured runs; absence records no `resources` block and never fails a
/// run (a BYO SUT has no reachable containers).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Containers {
    /// The SUT process container name.
    pub sut: String,
    /// The database container name (also the disk-anchor probe target).
    pub db: String,
}

/// The whole IXIT document.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ixit {
    /// Named instances; `sut` is the default the flow addresses when no
    /// `on:` selector is present.
    #[serde(deserialize_with = "crate::model::de::ordered_map")]
    pub instances: Vec<(InstanceName, Instance)>,
    #[serde(default)]
    pub environment: Option<Environment>,
    /// The composed SUT's container identities (optional by capability —
    /// see [`Containers`]).
    #[serde(default)]
    pub containers: Option<Containers>,
    /// The SUT's version-signing posture (RM common master06 §Digital
    /// Signature). Present => the SUT claims the Signing capability and this
    /// block declares its mode (digest | pgp) so the SIG-VERSION `verifiable`
    /// check knows how to verify; absent => no Signing capability, and the
    /// SIG-VERSION cases N/A on their guard.
    #[serde(default)]
    pub signing: Option<crate::exec::signature::SigningMode>,
}

impl Ixit {
    /// Look up an instance by name.
    #[must_use]
    pub fn instance(&self, name: &InstanceName) -> Option<&Instance> {
        self.instances
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, i)| i)
    }

    /// The default instance (`sut`) — required for every run.
    ///
    /// # Errors
    /// Returns a message when no `sut` instance is declared.
    pub fn default_instance(&self) -> Result<&Instance, String> {
        let sut = InstanceName::parse("sut").map_err(|e| e.to_string())?;
        self.instance(&sut)
            .ok_or_else(|| "ixit declares no `sut` instance".to_owned())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod tests {
    use super::*;

    #[test]
    fn ixit_parses_with_principals() {
        let ixit: Ixit = serde_json::from_value(serde_json::json!({
            "instances": {
                "sut": { "base_url": "http://localhost:8080/ehrbase/rest/openehr/v1",
                          "auth": { "mode": "basic", "user_env": "SUT_USER", "password_env": "SUT_PASS" } },
                "unauthenticated": { "base_url": "http://localhost:8080/ehrbase/rest/openehr/v1",
                          "auth": { "mode": "none" } },
                "readonly": { "base_url": "http://localhost:8080/ehrbase/rest/openehr/v1",
                          "auth": { "mode": "bearer", "token_env": "SUT_RO_TOKEN" } }
            },
            "environment": { "hardware_class": "consumer-laptop", "cores": 8,
                              "memory_gb": 16, "storage_class": "nvme", "topology": "single-node" }
        }))
        .unwrap();
        assert!(ixit.default_instance().is_ok());
        assert!(
            ixit.instance(&InstanceName::parse("readonly").unwrap())
                .is_some()
        );
        assert!(matches!(
            ixit.instance(&InstanceName::parse("unauthenticated").unwrap())
                .unwrap()
                .auth,
            AuthMode::None
        ));
    }

    #[test]
    fn containers_block_is_optional_and_parses() {
        let bare: Ixit = serde_json::from_value(serde_json::json!({
            "instances": { "sut": { "base_url": "http://x", "auth": { "mode": "none" } } }
        }))
        .unwrap();
        assert!(bare.containers.is_none());

        let with: Ixit = serde_json::from_value(serde_json::json!({
            "instances": { "sut": { "base_url": "http://x", "auth": { "mode": "none" } } },
            "containers": { "sut": "ehrbase-rs-ehrbase-1", "db": "ehrbase-rs-ehrbase-postgres-1" }
        }))
        .unwrap();
        let containers = with.containers.unwrap();
        assert_eq!(containers.sut, "ehrbase-rs-ehrbase-1");
        assert_eq!(containers.db, "ehrbase-rs-ehrbase-postgres-1");
    }

    #[test]
    fn missing_sut_is_an_error() {
        let ixit: Ixit = serde_json::from_value(serde_json::json!({
            "instances": { "primary": { "base_url": "http://x", "auth": { "mode": "none" } } }
        }))
        .unwrap();
        assert!(ixit.default_instance().is_err());
    }
}
