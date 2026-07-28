//! The IXIT (`ixit.json`) — the SUT topology the runner drives: one or more
//! named instances (base URL + auth + overrides) plus the environment block.
//!
//! ISO/IEC 9646 names this artifact the IXIT (implementation extra
//! information for testing); the schedule's party-artifact contract makes it
//! the single file that drives any runner against any SUT topology.
//! Single-instance platform cases use the default instance `sut`;
//! multi-instance cases and the security principals address ixit-declared
//! instances via the flow `on:` selector.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ids::InstanceName;

/// Authentication mode of an instance. Credentials are REFERENCES (env-var
/// names or, for the SMART lane, the party's declared test-issuer key file),
/// never inline secrets — the ixit file is committed/shared.
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
    /// A SMART *Application* principal: the runner MINTS a fresh RS256 access
    /// token per step against the party's declared test issuer
    /// ([`Ixit::smart`]), carrying exactly the scopes that step declares.
    ///
    /// This exists because the CDR is a SMART **resource server**, never an
    /// Authorization Server (ITS-REST
    /// `docs/smart_app_launch/master06-authentication.adoc` §Supported
    /// Authentication Flows: token issuance is the AS's duty), so the
    /// conformance stack runs no AS and no other principal can carry a CHOSEN
    /// `scope` claim. An instance may only declare this mode when the ixit
    /// declares a `smart` block; otherwise the cases that need it are
    /// not-applicable with that citation (ISO/IEC 9646 test selection).
    BearerMint {
        /// The `sub` claim for THIS principal; falls back to the lane mint's.
        #[serde(default)]
        subject: Option<String>,
        /// `realm_access.roles` for THIS principal (the RBAC identity the
        /// minted token carries — USER / ADMIN / READONLY per the SUT's role
        /// model); falls back to the lane mint's roles.
        #[serde(default)]
        roles: Option<Vec<String>>,
        /// The `scope` claim minted when the driven step declares none — the
        /// standing grant this principal holds for the general catalogue
        /// (master08 resource scopes; a step-level `scopes:` always wins).
        #[serde(default)]
        default_scopes: Vec<String>,
    },
}

/// The party's SMART App Launch lane declaration — a deployment fact no
/// released operation discloses, so it is an IXIT declaration exactly like
/// [`Ixit::system_id`] and [`Ixit::signing`].
///
/// Present => this deployment runs the CDR in the SMART resource-server role
/// (ITS-REST `docs/smart_app_launch/master02-overview.adoc` §Glossary: the
/// CDR is the Platform's `org.openehr.rest` service) and trusts the declared
/// static test issuer, so the runner may mint per-step scoped access tokens.
/// Absent => every SMART case is not-applicable with that citation.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmartLane {
    /// The ixit instance whose `base_url` is the SMART **Platform** base URL.
    /// master04 §Service Discovery serves `/.well-known/smart-configuration`
    /// "relative to the _Platform_ base URL" — which is NOT the openEHR REST
    /// base the other instances address, so the topology needs its own entry.
    pub platform_instance: InstanceName,
    /// The token mint the `bearer_mint` instances sign with.
    pub mint: BearerMint,
}

/// The static test issuer the runner signs access tokens with.
///
/// The key is a FILE reference (resolved relative to the ixit document), never
/// inline: the ixit is committed and shared, and a PEM pasted into it would
/// read as a credential rather than as the deliberately-public test material
/// it is.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BearerMint {
    /// The `iss` claim; must equal the SUT's configured `auth.oidc.issuer`.
    pub issuer: String,
    /// The `aud` claim. Omitted from the token when absent (a deployment that
    /// configures no accepted audience does not check one).
    #[serde(default)]
    pub audience: Option<String>,
    /// The `sub` claim — the SMART Application's authenticated user.
    pub subject: String,
    /// Roles minted into `realm_access.roles`, the RBAC claim path the CDR
    /// mines by default. The SMART gate AND-composes onto RBAC, so a token
    /// that carries the right scopes but no role would be refused one layer
    /// earlier and the case would prove nothing about SMART.
    #[serde(default)]
    pub roles: Vec<String>,
    /// The RSA private key (PEM). A relative path is resolved against the ixit
    /// document's own directory by [`Ixit::rebase_paths`], so a party artifact
    /// set is relocatable and never depends on the runner's working directory.
    pub key_file: PathBuf,
    /// The JWKS `kid` the SUT resolves the verifying key by.
    pub kid: String,
    /// Token lifetime in seconds (`exp` = `iat` + this).
    pub ttl_seconds: u64,
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
    /// THIS instance's version-signing posture, when it differs from the
    /// party default ([`Ixit::signing`]).
    ///
    /// RM common `master06-change_control_package.adoc` §Digital Signature
    /// defines digest and openPGP as alternative depths of ONE mechanism, and
    /// a running deployment realizes exactly one of them — so the posture is
    /// a property of the *deployment*, not of the party. A party that claims
    /// both modes therefore declares two deployments as two instances, each
    /// carrying its own block, and every signature check resolves
    /// instance-first (see [`Ixit::signing_of`]). Absent => the top-level
    /// default applies, so every single-posture ixit parses unchanged.
    #[serde(default)]
    pub signing: Option<crate::exec::signature::SigningMode>,
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
    /// The SUT's OWN CONFIGURED system identifier — the value it stamps into
    /// data it authors: `AUDIT_DETAILS.system_id` when the client supplies
    /// none (ITS-REST `Requests_and_responses.md` §"openehr-version and
    /// openehr-audit-details": "when `system_id` is not provided by the
    /// client, the server MUST set it to its own configured system
    /// identifier") and the `creating_system_id` of every
    /// `OBJECT_VERSION_ID` it mints (RM common `master06` §Change Control).
    ///
    /// It is an IXIT fact because no released operation discloses it: the
    /// value half of that MUST is not derivable from the wire, so a case that
    /// asserts it must be told what the deployment is configured with.
    /// Absent => the party makes no such declaration, and the cases that
    /// reference `${ixit:system_id}` are not-applicable with that citation
    /// rather than guessing.
    #[serde(default)]
    pub system_id: Option<String>,
    /// The party's DEFAULT version-signing posture (RM common master06
    /// §Digital Signature). Present => the SUT claims the Signing capability
    /// and this block declares the mode (digest | pgp) every instance runs
    /// unless it declares its own ([`Instance::signing`]) so the SIG-VERSION
    /// `verifiable` check knows how to verify; absent => no Signing
    /// capability, and the SIG-VERSION cases N/A on their guard.
    #[serde(default)]
    pub signing: Option<crate::exec::signature::SigningMode>,
    /// The SMART App Launch lane (ITS-REST `docs/smart_app_launch`). Present
    /// => the deployment runs the CDR's SMART resource-server role and trusts
    /// the declared test issuer, so the SMART cases are drivable; absent =>
    /// they are not-applicable with that citation.
    #[serde(default)]
    pub smart: Option<SmartLane>,
}

impl Ixit {
    /// Resolve relative file references in the document against `base` — the
    /// directory the ixit itself was read from.
    pub fn rebase_paths(&mut self, base: &Path) {
        if let Some(smart) = &mut self.smart
            && smart.mint.key_file.is_relative()
        {
            smart.mint.key_file = base.join(&smart.mint.key_file);
        }
    }

    /// Look up an instance by name.
    #[must_use]
    pub fn instance(&self, name: &InstanceName) -> Option<&Instance> {
        self.instances
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, i)| i)
    }

    /// The version-signing posture in force for `instance`: its own
    /// declaration wins, the party default ([`Ixit::signing`]) fills in.
    ///
    /// RM common `master06-change_control_package.adoc` §Digital Signature —
    /// the mode is a deployment fact, so a party exercising both modes runs
    /// two deployments and the verification posture follows the instance the
    /// step addressed, never the party-wide default.
    #[must_use]
    pub fn signing_of<'i>(
        &'i self,
        instance: &'i Instance,
    ) -> Option<&'i crate::exec::signature::SigningMode> {
        instance.signing.as_ref().or(self.signing.as_ref())
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
    fn declared_system_id_is_optional_and_parses() {
        let bare: Ixit = serde_json::from_value(serde_json::json!({
            "instances": { "sut": { "base_url": "http://x", "auth": { "mode": "none" } } }
        }))
        .unwrap();
        assert!(bare.system_id.is_none());

        let declared: Ixit = serde_json::from_value(serde_json::json!({
            "instances": { "sut": { "base_url": "http://x", "auth": { "mode": "none" } } },
            "system_id": "ehrbase-rs.local"
        }))
        .unwrap();
        assert_eq!(declared.system_id.as_deref(), Some("ehrbase-rs.local"));
    }

    #[test]
    fn smart_lane_is_optional_and_parses() {
        let bare: Ixit = serde_json::from_value(serde_json::json!({
            "instances": { "sut": { "base_url": "http://x", "auth": { "mode": "none" } } }
        }))
        .unwrap();
        assert!(bare.smart.is_none());

        let mut declared: Ixit = serde_json::from_value(serde_json::json!({
            "instances": {
                "sut": { "base_url": "http://x/openehr/v1", "auth": { "mode": "none" } },
                "smart_app": { "base_url": "http://x/openehr/v1", "auth": { "mode": "bearer_mint" } },
                "smart_platform": { "base_url": "http://x", "auth": { "mode": "none" } }
            },
            "smart": {
                "platform_instance": "smart_platform",
                "mint": {
                    "issuer": "https://as.example.test",
                    "audience": "cnf-smart-sut",
                    "subject": "cnf-smart-app",
                    "roles": ["USER"],
                    "key_file": "../smart/cnf-smart-test.key.pem",
                    "kid": "cnf-smart-test",
                    "ttl_seconds": 300
                }
            }
        }))
        .unwrap();
        let lane = declared.smart.as_ref().unwrap();
        assert_eq!(lane.platform_instance.as_str(), "smart_platform");
        assert_eq!(lane.mint.kid, "cnf-smart-test");
        assert_eq!(lane.mint.audience.as_deref(), Some("cnf-smart-sut"));
        assert!(matches!(
            declared
                .instance(&InstanceName::parse("smart_app").unwrap())
                .unwrap()
                .auth,
            AuthMode::BearerMint { .. }
        ));

        // A relative key file resolves against the ixit document's directory,
        // never the runner's working directory.
        declared.rebase_paths(Path::new("/party/ehrbase-rs"));
        assert_eq!(
            declared.smart.as_ref().unwrap().mint.key_file,
            PathBuf::from("/party/ehrbase-rs/../smart/cnf-smart-test.key.pem")
        );
        // Rebasing is idempotent for an already-absolute path.
        declared.rebase_paths(Path::new("/elsewhere"));
        assert_eq!(
            declared.smart.as_ref().unwrap().mint.key_file,
            PathBuf::from("/party/ehrbase-rs/../smart/cnf-smart-test.key.pem")
        );
    }

    #[test]
    fn instance_signing_overrides_the_party_default() {
        // Two deployments of one product, one per signing mode (RM common
        // master06 §Digital Signature: digest and openPGP are alternative
        // depths of one mechanism, and a deployment runs one).
        let ixit: Ixit = serde_json::from_value(serde_json::json!({
            "instances": {
                "sut": { "base_url": "http://localhost:8080", "auth": { "mode": "none" } },
                "sut_pgp": {
                    "base_url": "http://localhost:8081",
                    "auth": { "mode": "none" },
                    "signing": { "mode": "pgp", "public_key": "-----BEGIN PGP PUBLIC KEY BLOCK-----\n" }
                }
            },
            "signing": { "mode": "digest", "algorithm": "sha256", "encoding": "base64", "prefix": "sha256:" }
        }))
        .unwrap();

        let default = ixit.default_instance().unwrap();
        assert!(default.signing.is_none());
        assert!(matches!(
            ixit.signing_of(default),
            Some(crate::exec::signature::SigningMode::Digest { .. })
        ));

        let pgp = ixit
            .instance(&InstanceName::parse("sut_pgp").unwrap())
            .unwrap();
        assert!(matches!(
            ixit.signing_of(pgp),
            Some(crate::exec::signature::SigningMode::Pgp { .. })
        ));
    }

    #[test]
    fn instance_signing_is_absent_without_any_declaration() {
        let ixit: Ixit = serde_json::from_value(serde_json::json!({
            "instances": { "sut": { "base_url": "http://x", "auth": { "mode": "none" } } }
        }))
        .unwrap();
        assert!(ixit.signing_of(ixit.default_instance().unwrap()).is_none());
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
