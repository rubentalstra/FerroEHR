//! Authorization configuration ([`AuthzConfig`]) — the `[authz]` section of the
//! one server configuration tree; it carries
//! **no loader of its own** — the whole tree is assembled once by
//! `ferroehr::config` and this struct is deserialized as a field of it.
//!
//! Authorization is spec-silent: no openEHR spec governs RBAC/ABAC (the SM
//! places it out of band), so this is our own enterprise design.
//!
//! Two sections: the coarse RBAC keys (`rbac.*`, always evaluated when auth is
//! enabled) and the opt-in ABAC keys (`abac.*`, master switch `abac.enabled`,
//! default `false`). Every field has a default, so an all-defaults
//! [`AuthzConfig`] is valid (RBAC on, `ADMIN`/`USER` roles, admin-only
//! management access, ABAC off).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Management-surface access level — the `rbac.management_access` tri-state
/// (§5.2). `admin_only` is the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ManagementAccess {
    /// Requires `rbac.admin_role`.
    #[default]
    AdminOnly,
    /// Any authenticated principal with a role.
    Private,
    /// No authorization check.
    Public,
}

/// The coarse role-based access-control settings (§5.2, §8).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RbacConfig {
    /// Master switch; the coarse role gate is active only when auth is enabled
    /// (`FERROEHR_AUTHZ_RBAC__ENABLED`, default `true`). Disabling restores
    /// authentication-only behaviour.
    #[serde(default = "defaults::yes")]
    pub enabled: bool,
    /// The role required for Admin-class operations (default `ADMIN`).
    #[serde(default = "defaults::admin_role")]
    pub admin_role: String,
    /// The baseline clinical role (default `USER`).
    #[serde(default = "defaults::user_role")]
    pub user_role: String,
    /// The role marking a principal as read-only (default `READONLY`): a caller
    /// carrying it is refused on every write operation even when it also holds
    /// granting roles (a restriction overrides a grant). Supports the CNF
    /// SEC-BASIC authorization-separation profile.
    #[serde(default = "defaults::readonly_role")]
    pub readonly_role: String,
    /// JWT claim paths mined for roles (default `["realm_access.roles","scope"]`).
    #[serde(default = "defaults::role_claims")]
    pub role_claims: Vec<String>,
    /// Access level for the management surface (default `admin_only`).
    #[serde(default)]
    pub management_access: ManagementAccess,
}

impl Default for RbacConfig {
    fn default() -> Self {
        Self {
            enabled: defaults::yes(),
            admin_role: defaults::admin_role(),
            user_role: defaults::user_role(),
            readonly_role: defaults::readonly_role(),
            role_claims: defaults::role_claims(),
            management_access: ManagementAccess::default(),
        }
    }
}

/// The ABAC policy engine selector (`abac.engine`, §5.6/§5.5). `cedar` is the
/// embedded default; `remote` is the v1-compatible external PDP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AbacEngineKind {
    /// Embedded Cedar (`cedar-policy`), policies from `abac.cedar.policy_dir`.
    #[default]
    Cedar,
    /// The v1-compatible remote policy-decision-point (`abac.remote.server`).
    Remote,
}

/// One flat parameter a remote PDP body may carry (§5.5): the resolved
/// attribute keys, exactly `organization` / `patient` / `template` on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbacParam {
    /// The caller's organization (from `abac.organization_claim`).
    Organization,
    /// The patient the resource is about (from `abac.patient_claim` / EHR subject).
    Patient,
    /// The resource's template id.
    Template,
}

impl AbacParam {
    /// The exact wire key v1 uses in the flat PDP request body (§2.1).
    #[must_use]
    pub const fn wire_key(self) -> &'static str {
        match self {
            AbacParam::Organization => "organization",
            AbacParam::Patient => "patient",
            AbacParam::Template => "template",
        }
    }
}

/// A per-resource-kind policy binding (§5.5): the policy name appended to the
/// remote PDP base URL, and which resolved parameters its request body carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyRule {
    /// The policy name (the path segment appended to `abac.remote.server`).
    pub name: String,
    /// The parameters the PDP body carries; empty = an unparameterized policy.
    #[serde(default)]
    pub parameters: Vec<AbacParam>,
}

/// The embedded-Cedar engine settings (§5.6).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CedarConfig {
    /// Directory of `*.cedar` policy files (required when `engine = cedar`).
    #[serde(default)]
    pub policy_dir: Option<PathBuf>,
    /// Optional periodic hot-reload interval (seconds); off when unset.
    #[serde(default)]
    pub reload_secs: Option<u64>,
}

/// The v1-compatible remote-PDP client settings (§5.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RemoteConfig {
    /// The PDP base URL; the policy name is appended, so it must end with `/`
    /// (required when `engine = remote`).
    #[serde(default)]
    pub server: Option<String>,
    /// TCP connect timeout in milliseconds (v1 defect #4 fixed; default 2000).
    #[serde(default = "defaults::connect_timeout_ms")]
    pub connect_timeout_ms: u64,
    /// Whole-request timeout in milliseconds (default 5000).
    #[serde(default = "defaults::request_timeout_ms")]
    pub request_timeout_ms: u64,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            server: None,
            connect_timeout_ms: defaults::connect_timeout_ms(),
            request_timeout_ms: defaults::request_timeout_ms(),
        }
    }
}

/// The fine-grained attribute-based access-control settings (§5.3–5.7, §8).
/// Master switch `enabled` defaults to `false`, so an all-defaults config
/// preserves today's authentication-plus-RBAC behaviour until an operator
/// opts in.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AbacConfig {
    /// Master ABAC switch (default `false`).
    #[serde(default)]
    pub enabled: bool,
    /// Which PDP engine to use (default `cedar`).
    #[serde(default)]
    pub engine: AbacEngineKind,
    /// The JWT claim carrying the caller's organization (default
    /// `organization_id`); resolved opportunistically (absent → no attribute).
    #[serde(default = "defaults::organization_claim")]
    pub organization_claim: String,
    /// The JWT claim carrying the patient id (default `patient_id`); its presence
    /// enables the local subject gate (§5.7). Set to empty to disable the gate.
    #[serde(default = "defaults::patient_claim")]
    pub patient_claim: String,
    /// Embedded-Cedar settings.
    #[serde(default)]
    pub cedar: CedarConfig,
    /// Remote-PDP settings.
    #[serde(default)]
    pub remote: RemoteConfig,
    /// Per-resource-kind policy bindings, keyed by the canonical kind name
    /// (`ehr`, `ehr_status`, `composition`, `contribution`, `query`,
    /// `directory`). An absent kind is unchecked (v1 parity for `directory`).
    #[serde(default)]
    pub policy: BTreeMap<String, PolicyRule>,
}

impl Default for AbacConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            engine: AbacEngineKind::default(),
            organization_claim: defaults::organization_claim(),
            patient_claim: defaults::patient_claim(),
            cedar: CedarConfig::default(),
            remote: RemoteConfig::default(),
            policy: BTreeMap::new(),
        }
    }
}

impl AbacConfig {
    /// The patient claim, or `None` when blank (the subject gate is then off).
    #[must_use]
    pub fn patient_claim(&self) -> Option<&str> {
        let c = self.patient_claim.trim();
        (!c.is_empty()).then_some(c)
    }

    /// The organization claim, or `None` when blank.
    #[must_use]
    pub fn organization_claim(&self) -> Option<&str> {
        let c = self.organization_claim.trim();
        (!c.is_empty()).then_some(c)
    }
}

/// Authorization configuration (§8): the coarse RBAC gate plus the opt-in ABAC
/// policy layer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuthzConfig {
    /// Role-based access control.
    #[serde(default)]
    pub rbac: RbacConfig,
    /// Attribute-based access control (opt-in).
    #[serde(default)]
    pub abac: AbacConfig,
}

/// A boot-time authorization-configuration error (§8 — hard errors that must
/// abort startup rather than silently mis-gate).
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AuthzConfigError {
    /// A required role name was blank.
    #[error("authz.rbac.{0} must not be empty")]
    EmptyRole(&'static str),
    /// The role-claim path list was empty or contained a blank entry.
    #[error("authz.rbac.role_claims must be non-empty and contain no blank paths")]
    RoleClaims,
    /// A policy key does not name a known resource kind.
    #[error(
        "authz.abac.policy.{0} is not a known resource kind \
         (ehr, ehr_status, composition, contribution, query, directory)"
    )]
    UnknownPolicyKind(String),
    /// A `template` parameter was configured for `ehr`/`ehr_status` — illegal
    /// (v1 made this a runtime 500; §5.5 makes it a boot error).
    #[error("authz.abac.policy.{0} must not take the `template` parameter")]
    TemplateParamIllegal(&'static str),
    /// `engine = remote` without a configured base URL.
    #[error("authz.abac.remote.server is required when abac.engine = remote")]
    RemoteServerMissing,
    /// The remote base URL does not end with `/` (the policy name is appended).
    #[error("authz.abac.remote.server must end with '/' (got {0:?})")]
    RemoteServerTrailingSlash(String),
    /// `engine = cedar` without a configured policy directory.
    #[error("authz.abac.cedar.policy_dir is required when abac.engine = cedar")]
    CedarDirMissing,
}

impl AuthzConfig {
    /// Validate the configuration at boot (§8). All defaults are valid.
    ///
    /// # Errors
    /// [`AuthzConfigError`] when a role name is blank or the role-claim path list
    /// is empty / contains a blank path.
    pub fn validate(&self) -> Result<(), AuthzConfigError> {
        if self.rbac.admin_role.trim().is_empty() {
            return Err(AuthzConfigError::EmptyRole("admin_role"));
        }
        if self.rbac.user_role.trim().is_empty() {
            return Err(AuthzConfigError::EmptyRole("user_role"));
        }
        if self.rbac.readonly_role.trim().is_empty() {
            return Err(AuthzConfigError::EmptyRole("readonly_role"));
        }
        if self.rbac.role_claims.is_empty()
            || self.rbac.role_claims.iter().any(|c| c.trim().is_empty())
        {
            return Err(AuthzConfigError::RoleClaims);
        }
        if self.abac.enabled {
            self.abac.validate()?;
        }
        Ok(())
    }
}

/// The canonical resource-kind keys accepted in the `abac.policy` map, and the
/// two on which a `template` parameter is illegal (§5.5).
const POLICY_KINDS: [&str; 6] = [
    "ehr",
    "ehr_status",
    "composition",
    "contribution",
    "query",
    "directory",
];

impl AbacConfig {
    /// Validate the ABAC section at boot (§8 hard errors). Only called when
    /// `enabled`.
    fn validate(&self) -> Result<(), AuthzConfigError> {
        for (kind, rule) in &self.policy {
            if !POLICY_KINDS.contains(&kind.as_str()) {
                return Err(AuthzConfigError::UnknownPolicyKind(kind.clone()));
            }
            if rule.parameters.contains(&AbacParam::Template) {
                match kind.as_str() {
                    "ehr" => return Err(AuthzConfigError::TemplateParamIllegal("ehr")),
                    "ehr_status" => {
                        return Err(AuthzConfigError::TemplateParamIllegal("ehr_status"));
                    }
                    _ => {}
                }
            }
        }
        match self.engine {
            AbacEngineKind::Remote => {
                let server = self
                    .remote
                    .server
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
                    .ok_or(AuthzConfigError::RemoteServerMissing)?;
                if !server.ends_with('/') {
                    return Err(AuthzConfigError::RemoteServerTrailingSlash(
                        server.to_owned(),
                    ));
                }
            }
            AbacEngineKind::Cedar => {
                if self.cedar.policy_dir.is_none() {
                    return Err(AuthzConfigError::CedarDirMissing);
                }
            }
        }
        Ok(())
    }
}

mod defaults {
    pub(super) const fn yes() -> bool {
        true
    }
    pub(super) fn admin_role() -> String {
        "ADMIN".to_owned()
    }
    pub(super) fn user_role() -> String {
        "USER".to_owned()
    }
    pub(super) fn readonly_role() -> String {
        "READONLY".to_owned()
    }
    pub(super) fn role_claims() -> Vec<String> {
        vec!["realm_access.roles".to_owned(), "scope".to_owned()]
    }
    pub(super) fn organization_claim() -> String {
        "organization_id".to_owned()
    }
    pub(super) fn patient_claim() -> String {
        "patient_id".to_owned()
    }
    pub(super) const fn connect_timeout_ms() -> u64 {
        2000
    }
    pub(super) const fn request_timeout_ms() -> u64 {
        5000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        let c = AuthzConfig::default();
        assert!(c.rbac.enabled);
        assert_eq!(c.rbac.admin_role, "ADMIN");
        assert_eq!(c.rbac.user_role, "USER");
        assert_eq!(c.rbac.readonly_role, "READONLY");
        assert_eq!(c.rbac.management_access, ManagementAccess::AdminOnly);
        assert_eq!(
            c.rbac.role_claims,
            vec!["realm_access.roles".to_owned(), "scope".to_owned()]
        );
        assert!(c.validate().is_ok());
    }

    #[test]
    fn blank_admin_role_rejected() {
        let c = AuthzConfig {
            rbac: RbacConfig {
                admin_role: "  ".to_owned(),
                ..RbacConfig::default()
            },
            ..AuthzConfig::default()
        };
        assert_eq!(c.validate(), Err(AuthzConfigError::EmptyRole("admin_role")));
    }

    #[test]
    fn blank_readonly_role_rejected() {
        let c = AuthzConfig {
            rbac: RbacConfig {
                readonly_role: "  ".to_owned(),
                ..RbacConfig::default()
            },
            ..AuthzConfig::default()
        };
        assert_eq!(
            c.validate(),
            Err(AuthzConfigError::EmptyRole("readonly_role"))
        );
    }

    #[test]
    fn empty_role_claims_rejected() {
        let c = AuthzConfig {
            rbac: RbacConfig {
                role_claims: vec![],
                ..RbacConfig::default()
            },
            ..AuthzConfig::default()
        };
        assert_eq!(c.validate(), Err(AuthzConfigError::RoleClaims));
        let c = AuthzConfig {
            rbac: RbacConfig {
                role_claims: vec!["ok".to_owned(), "  ".to_owned()],
                ..RbacConfig::default()
            },
            ..AuthzConfig::default()
        };
        assert_eq!(c.validate(), Err(AuthzConfigError::RoleClaims));
    }

    #[test]
    fn abac_disabled_by_default_and_skips_validation() {
        let c = AuthzConfig::default();
        assert!(!c.abac.enabled);
        assert_eq!(c.abac.engine, AbacEngineKind::Cedar);
        // A misconfigured-but-disabled ABAC section is not validated.
        let c = AuthzConfig {
            abac: AbacConfig {
                enabled: false,
                engine: AbacEngineKind::Remote,
                remote: RemoteConfig {
                    server: None,
                    ..RemoteConfig::default()
                },
                ..AbacConfig::default()
            },
            ..AuthzConfig::default()
        };
        assert!(c.validate().is_ok());
    }

    #[test]
    fn template_param_illegal_on_ehr_and_ehr_status() {
        for kind in ["ehr", "ehr_status"] {
            let mut policy = BTreeMap::new();
            policy.insert(
                kind.to_owned(),
                PolicyRule {
                    name: "p".to_owned(),
                    parameters: vec![AbacParam::Template],
                },
            );
            let c = AuthzConfig {
                abac: AbacConfig {
                    enabled: true,
                    engine: AbacEngineKind::Cedar,
                    cedar: CedarConfig {
                        policy_dir: Some(PathBuf::from("/tmp/policies")),
                        reload_secs: None,
                    },
                    policy,
                    ..AbacConfig::default()
                },
                ..AuthzConfig::default()
            };
            assert!(matches!(
                c.validate(),
                Err(AuthzConfigError::TemplateParamIllegal(_))
            ));
        }
    }

    #[test]
    fn unknown_policy_kind_rejected() {
        let mut policy = BTreeMap::new();
        policy.insert(
            "nonsense".to_owned(),
            PolicyRule {
                name: "p".to_owned(),
                parameters: vec![],
            },
        );
        let c = AuthzConfig {
            abac: AbacConfig {
                enabled: true,
                cedar: CedarConfig {
                    policy_dir: Some(PathBuf::from("/tmp/p")),
                    reload_secs: None,
                },
                policy,
                ..AbacConfig::default()
            },
            ..AuthzConfig::default()
        };
        assert_eq!(
            c.validate(),
            Err(AuthzConfigError::UnknownPolicyKind("nonsense".to_owned()))
        );
    }

    #[test]
    fn remote_server_must_be_present_and_slash_terminated() {
        let base = AbacConfig {
            enabled: true,
            engine: AbacEngineKind::Remote,
            ..AbacConfig::default()
        };
        let missing = AuthzConfig {
            abac: base.clone(),
            ..AuthzConfig::default()
        };
        assert_eq!(
            missing.validate(),
            Err(AuthzConfigError::RemoteServerMissing)
        );

        let no_slash = AuthzConfig {
            abac: AbacConfig {
                remote: RemoteConfig {
                    server: Some("http://pdp:3001/exec".to_owned()),
                    ..RemoteConfig::default()
                },
                ..base.clone()
            },
            ..AuthzConfig::default()
        };
        assert!(matches!(
            no_slash.validate(),
            Err(AuthzConfigError::RemoteServerTrailingSlash(_))
        ));

        let ok = AuthzConfig {
            abac: AbacConfig {
                remote: RemoteConfig {
                    server: Some("http://pdp:3001/exec/".to_owned()),
                    ..RemoteConfig::default()
                },
                ..base
            },
            ..AuthzConfig::default()
        };
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn cedar_requires_policy_dir() {
        let c = AuthzConfig {
            abac: AbacConfig {
                enabled: true,
                engine: AbacEngineKind::Cedar,
                ..AbacConfig::default()
            },
            ..AuthzConfig::default()
        };
        assert_eq!(c.validate(), Err(AuthzConfigError::CedarDirMissing));
    }

    #[test]
    fn remote_timeouts_default() {
        let c = AbacConfig::default();
        assert_eq!(c.remote.connect_timeout_ms, 2000);
        assert_eq!(c.remote.request_timeout_ms, 5000);
    }
}
