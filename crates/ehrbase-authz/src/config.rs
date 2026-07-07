//! Authorization configuration ([`AuthzConfig`]) — a `figment`-loaded serde
//! struct with its own `EHRBASE_AUTHZ_` prefix (the [`AuditConfig`] precedent),
//! §8 of `docs/enterprise/access-control.md`.
//!
//! This PR ships the RBAC keys (`rbac.*`); the ABAC section (`abac.*`) is added
//! in the ABAC PR. Every field has a default, so an all-defaults [`AuthzConfig`]
//! is valid (RBAC on, `ADMIN`/`USER` roles, admin-only management access).
//!
//! [`AuditConfig`]: https://docs.rs/ehrbase-audit

use figment::Figment;
use figment::providers::{Env, Format, Serialized, Toml};
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
pub struct RbacConfig {
    /// Master switch; the coarse role gate is active only when auth is enabled
    /// (`EHRBASE_AUTHZ_RBAC__ENABLED`, default `true`). Disabling restores
    /// authentication-only behaviour.
    #[serde(default = "defaults::yes")]
    pub enabled: bool,
    /// The role required for Admin-class operations (default `ADMIN`).
    #[serde(default = "defaults::admin_role")]
    pub admin_role: String,
    /// The baseline clinical role (default `USER`).
    #[serde(default = "defaults::user_role")]
    pub user_role: String,
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
            role_claims: defaults::role_claims(),
            management_access: ManagementAccess::default(),
        }
    }
}

/// Authorization configuration (§8). RBAC only for now; ABAC lands in the
/// follow-up PR.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthzConfig {
    /// Role-based access control.
    #[serde(default)]
    pub rbac: RbacConfig,
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
}

impl AuthzConfig {
    /// Load configuration: defaults, then an optional TOML file (path in
    /// `EHRBASE_AUTHZ_CONFIG`), then `EHRBASE_AUTHZ_`-prefixed environment
    /// variables (nested keys use `__`).
    ///
    /// # Errors
    /// Returns a [`figment::Error`] if a value fails to parse.
    #[allow(clippy::result_large_err)] // figment::Error is large by design
    pub fn load() -> Result<Self, figment::Error> {
        let mut fig = Figment::from(Serialized::defaults(AuthzConfig::default()));
        if let Ok(path) = std::env::var("EHRBASE_AUTHZ_CONFIG") {
            fig = fig.merge(Toml::file(path));
        }
        fig.merge(Env::prefixed("EHRBASE_AUTHZ_").split("__"))
            .extract()
    }

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
        if self.rbac.role_claims.is_empty()
            || self.rbac.role_claims.iter().any(|c| c.trim().is_empty())
        {
            return Err(AuthzConfigError::RoleClaims);
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
    pub(super) fn role_claims() -> Vec<String> {
        crate::roles::default_role_claims()
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
        };
        assert_eq!(c.validate(), Err(AuthzConfigError::EmptyRole("admin_role")));
    }

    #[test]
    fn empty_role_claims_rejected() {
        let c = AuthzConfig {
            rbac: RbacConfig {
                role_claims: vec![],
                ..RbacConfig::default()
            },
        };
        assert_eq!(c.validate(), Err(AuthzConfigError::RoleClaims));
        let c = AuthzConfig {
            rbac: RbacConfig {
                role_claims: vec!["ok".to_owned(), "  ".to_owned()],
                ..RbacConfig::default()
            },
        };
        assert_eq!(c.validate(), Err(AuthzConfigError::RoleClaims));
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn env_overrides_apply() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("EHRBASE_AUTHZ_RBAC__ENABLED", "false");
            jail.set_env("EHRBASE_AUTHZ_RBAC__ADMIN_ROLE", "root");
            jail.set_env("EHRBASE_AUTHZ_RBAC__MANAGEMENT_ACCESS", "public");
            let c = AuthzConfig::load().expect("load");
            assert!(!c.rbac.enabled);
            assert_eq!(c.rbac.admin_role, "root");
            assert_eq!(c.rbac.management_access, ManagementAccess::Public);
            Ok(())
        });
    }
}
