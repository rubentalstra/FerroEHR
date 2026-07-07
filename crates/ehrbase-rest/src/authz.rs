//! The authorization handle wired onto [`AppState`](crate::AppState): the RBAC
//! gate (`docs/enterprise/access-control.md` §5.2). The ABAC policy-engine slot
//! is added in the ABAC PR (§4.1); for now this carries only the coarse role
//! gate, built by the binary from [`ehrbase_authz::AuthzConfig`].

use std::collections::HashMap;

use ehrbase_authz::{
    AuthzConfig, OperationClass, RbacConfig, RbacDecision, authorize, class_of, roles,
};
use http::Method;

/// The per-server authorization handle. `None` on [`AppState`](crate::AppState)
/// means authorization is off (authentication-only behaviour).
#[derive(Debug)]
pub struct AuthzHandle {
    pub(crate) rbac: RbacGate,
}

impl AuthzHandle {
    /// Build the handle from the loaded config and the REST base path (needed to
    /// key the route→class map by the same full templates axum's `MatchedPath`
    /// resolves to). `None` when RBAC is disabled — the caller then leaves the
    /// [`AppState`](crate::AppState) slot empty, preserving auth-only behaviour.
    #[must_use]
    pub fn from_config(config: &AuthzConfig, base_path: &str) -> Option<Self> {
        if !config.rbac.enabled {
            return None;
        }
        Some(Self {
            rbac: RbacGate::new(config.rbac.clone(), base_path),
        })
    }

    /// The configured JWT role-claim paths (fed into the [`Authenticator`] so
    /// Bearer role extraction uses them).
    ///
    /// [`Authenticator`]: crate::Authenticator
    #[must_use]
    pub fn role_claims(&self) -> Vec<String> {
        self.rbac.rules.role_claims.clone()
    }
}

/// The coarse RBAC gate: a route-template → [`OperationClass`] map (built once
/// from the generated `ROUTES` tables) plus the role rules.
#[derive(Debug)]
pub(crate) struct RbacGate {
    rules: RbacConfig,
    /// `(method, full-path-template)` → class, keyed by `base_path` +
    /// [`crate::dispatch::normalize_path`] so a request's `MatchedPath` matches.
    routes: HashMap<(Method, String), OperationClass>,
}

impl RbacGate {
    fn new(rules: RbacConfig, base_path: &str) -> Self {
        use openehr_its::rest::generated as g;
        let mut routes = HashMap::new();
        for table in [
            g::ehr::ROUTES,
            g::definition::ROUTES,
            g::demographic::ROUTES,
            g::query::ROUTES,
            g::admin::ROUTES,
        ] {
            for (method, path, op) in table {
                let (Ok(method), Some(class)) = (method.parse::<Method>(), class_of(op)) else {
                    // An unparsable method or unclassified op is impossible for the
                    // generated tables (the coverage guard proves classification);
                    // skip defensively rather than panic in a constructor.
                    continue;
                };
                let full = format!("{base_path}{}", crate::dispatch::normalize_path(path));
                routes.insert((method, full), class);
            }
        }
        Self { rules, routes }
    }

    /// The operation class for a request. `matched` is the axum `MatchedPath`
    /// template (the full nested path). A route not in the map defaults to
    /// [`OperationClass::Admin`] when its template is under `/admin/` (fail
    /// safe), else [`OperationClass::Clinical`]; a request with no matched path
    /// (a 404) is [`OperationClass::Public`] so routing still yields the 404.
    pub(crate) fn class_for(&self, method: &Method, matched: Option<&str>) -> OperationClass {
        match matched {
            None => OperationClass::Public,
            Some(mp) => self
                .routes
                .get(&(method.clone(), mp.to_owned()))
                .copied()
                .unwrap_or_else(|| {
                    if mp.contains("/admin/") {
                        OperationClass::Admin
                    } else {
                        OperationClass::Clinical
                    }
                }),
        }
    }

    /// Apply the RBAC decision for a class + the caller's roles.
    pub(crate) fn decide(&self, class: OperationClass, principal_roles: &[String]) -> RbacDecision {
        authorize(class, principal_roles, &self.rules)
    }
}

/// The default JWT role-claim paths, re-exported for the binary when no handle
/// is built (so the [`Authenticator`](crate::Authenticator) still extracts
/// roles under the default paths).
#[must_use]
pub fn default_role_claims() -> Vec<String> {
    roles::default_role_claims()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "/ehrbase/rest/openehr/v1";

    fn gate() -> RbacGate {
        RbacGate::new(RbacConfig::default(), BASE)
    }

    #[test]
    fn admin_template_is_admin_class() {
        let g = gate();
        assert_eq!(
            g.class_for(
                &Method::DELETE,
                Some(&format!("{BASE}/admin/ehr/{{ehr_id}}"))
            ),
            OperationClass::Admin
        );
    }

    #[test]
    fn clinical_template_is_clinical_class() {
        let g = gate();
        assert_eq!(
            g.class_for(
                &Method::POST,
                Some(&format!("{BASE}/ehr/{{ehr_id}}/composition"))
            ),
            OperationClass::Clinical
        );
        assert_eq!(
            g.class_for(&Method::GET, Some(&format!("{BASE}/ehr/{{ehr_id}}"))),
            OperationClass::Clinical
        );
    }

    #[test]
    fn unmatched_is_public_but_admin_substring_fails_safe() {
        let g = gate();
        // A genuine 404 (no MatchedPath) is not gated.
        assert_eq!(g.class_for(&Method::GET, None), OperationClass::Public);
        // A matched-but-unmapped admin-looking template still gates as Admin.
        assert_eq!(
            g.class_for(&Method::GET, Some("/x/admin/unknown")),
            OperationClass::Admin
        );
    }

    #[test]
    fn decide_delegates_to_rbac() {
        let g = gate();
        assert_eq!(
            g.decide(OperationClass::Admin, &["USER".to_owned()]),
            RbacDecision::Deny("operation requires the 'ADMIN' role".to_owned())
        );
        assert_eq!(
            g.decide(OperationClass::Clinical, &["USER".to_owned()]),
            RbacDecision::Allow
        );
    }

    #[test]
    fn handle_absent_when_rbac_disabled() {
        let mut cfg = AuthzConfig::default();
        cfg.rbac.enabled = false;
        assert!(AuthzHandle::from_config(&cfg, BASE).is_none());
        assert!(AuthzHandle::from_config(&AuthzConfig::default(), BASE).is_some());
    }
}
