//! Authorization for the `FerroEHR` CDR — the two composable layers,
//! folded into the protocol adapter
//! (authorization is an adapter concern by design; the former `ferroehr-authz`
//! crate is dissolved here).
//!
//! **No openEHR spec governs this.** The SM places authorization out of band
//! (SM `openehr_platform/master02-overview.adoc` §General Assumptions) and no
//! CNF profile carries an RBAC/ABAC requirement — everything here is our own
//! enterprise design, flagged as such rather than presented as conformance. The
//! spec-grounded access authority is the sibling `EHR_ACCESS` gate
//! ([`crate::extensions::access::ehr_access`]); these layers compose on top of it.
//! The layers:
//!
//! 1. **RBAC** (coarse, always on when auth is enabled): every generated
//!    ITS-REST operation is classified ([`classify`]) and gated by a role model
//!    ([`roles`]) driven by `ferroehr::config::authz::AuthzConfig`.
//! 2. **ABAC** (fine-grained, opt-in `abac.enabled`): a policy-decision-point
//!    seam ([`engine::PolicyEngine`]) consulted per clinical operation with
//!    resolved attributes (organization/patient/template), behind two
//!    interchangeable engines — an embedded Cedar engine ([`cedar`], the
//!    default) and the v1-compatible [`remote::RemotePdp`].
//!
//! This module also carries the per-server authorization handle wired onto
//! [`AppState`](crate::state::AppState) (the RBAC + ABAC gates), built by the binary
//! from `ferroehr::config::authz::AuthzConfig`.
//!
//! ## Module map (§4.1)
//! - `config` — the `ferroehr::config::authz::AuthzConfig` serde struct (the `[authz]`
//!   section of the one server config tree) + boot validation.
//! - [`roles`] — the role model, JWT-claim role extraction, and the RBAC gate
//!   decision.
//! - [`classify`] — `operation_id → OperationClass` / `ResourceKind` /
//!   `AccessMode` (total-coverage-guarded).
//! - [`request`] — the ABAC [`request::AuthzRequest`] + multi-valued fan-out.
//! - [`engine`] — the [`engine::PolicyEngine`] PDP seam + [`engine::AuthzError`].
//! - [`cedar`] — the embedded Cedar engine (schema, policy loading, reload).
//! - [`remote`] — the v1-compatible remote PDP client.

pub mod cedar;
pub mod classify;
pub mod engine;
pub mod remote;
pub mod request;
pub mod roles;

use crate::extensions::access::authz::classify::{OperationClass, class_of, is_write};
use crate::extensions::access::authz::engine::{AuthzError, PolicyEngine};
use crate::extensions::access::authz::roles::{RbacDecision, authorize, authorize_readonly};
use ferroehr::config::authz::{AbacConfig, AbacEngineKind, AuthzConfig, RbacConfig};

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use http::Method;

use self::cedar::CedarEngine;
use self::remote::RemotePdp;

/// An attribute-resolution failure (a DB lookup error). Fail-closed at the PEP
/// (§5.7): a resolution failure denies (→ 403/500), never silently permits.
#[derive(Debug, thiserror::Error)]
#[error("attribute resolution failed: {0}")]
pub struct ResolveError(pub String);

/// The future a resolver closure returns.
pub type ResolverFuture<T> = Pin<Box<dyn Future<Output = Result<T, ResolveError>> + Send>>;

/// `ehr_id → EHR subject external-ref id` (the promoted `ehr.subject_id`
/// column; the audit `SubjectResolver` query). Ids are passed as strings so the
/// REST layer stays `uuid`-free (the binary parses).
pub type SubjectFn = Arc<dyn Fn(String) -> ResolverFuture<Option<String>> + Send + Sync>;

/// `(vo_id, version) → template_id` (`vo_version.template_id`, read back via
/// `ferroehr::service` in the binary). `version` is the `VERSION_TREE_ID` lexical
/// form (`N` or `N.B.V` — trunk or branch, RM common master06 §The 'Virtual Version Tree');
/// `None` = the current version.
pub type TemplateOfVersionFn =
    Arc<dyn Fn(String, Option<String>) -> ResolverFuture<Option<String>> + Send + Sync>;

/// The DB-backed attribute resolvers the ABAC PEP calls (§6). Defined here so
/// the REST layer can hold them; the closures are built in the binary (which
/// owns the pool + service), the audit-`SubjectResolver` precedent.
#[derive(Clone)]
pub struct AuthzResolvers {
    /// EHR subject external-ref id lookup.
    pub subject: SubjectFn,
    /// COMPOSITION version → template id lookup.
    pub template_of_version: TemplateOfVersionFn,
}

impl std::fmt::Debug for AuthzResolvers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthzResolvers").finish_non_exhaustive()
    }
}

/// Build the configured ABAC [`PolicyEngine`] (§5.5/§5.6), or `None` when ABAC
/// is disabled. Called by the binary at boot after [`AuthzConfig::validate`].
///
/// # Errors
/// [`AuthzError`] if the Cedar schema/policies fail to load/validate, or the
/// remote-PDP client cannot be built.
pub fn build_engine(config: &AbacConfig) -> Result<Option<Arc<dyn PolicyEngine>>, AuthzError> {
    if !config.enabled {
        return Ok(None);
    }
    let engine: Arc<dyn PolicyEngine> = match config.engine {
        AbacEngineKind::Cedar => {
            let dir = config.cedar.policy_dir.as_deref().ok_or_else(|| {
                AuthzError::PolicyLoad("abac.cedar.policy_dir is not set".to_owned())
            })?;
            Arc::new(CedarEngine::new(dir, config.cedar.reload_secs)?)
        }
        AbacEngineKind::Remote => Arc::new(RemotePdp::new(config)?),
    };
    Ok(Some(engine))
}

/// The per-server authorization handle. `None` on [`AppState`](crate::state::AppState)
/// means authorization is off (authentication-only behaviour). Carries the
/// coarse RBAC gate and/or the fine-grained ABAC gate.
#[derive(Debug)]
pub struct AuthzHandle {
    rbac: Option<RbacGate>,
    abac: Option<AbacGate>,
}

impl AuthzHandle {
    /// Build an RBAC-only handle from config + the REST base path. `None` when
    /// RBAC is disabled — the caller then leaves the
    /// [`AppState`](crate::state::AppState) slot empty, preserving auth-only behaviour.
    /// (The binary uses [`AuthzHandle::build`] to attach ABAC.)
    #[must_use]
    pub fn from_config(config: &AuthzConfig, base_path: &str) -> Option<Self> {
        if !config.rbac.enabled {
            return None;
        }
        Some(Self {
            rbac: Some(RbacGate::new(config.rbac.clone(), base_path)),
            abac: None,
        })
    }

    /// Build the full handle (the binary): the RBAC gate (when enabled) plus the
    /// ABAC gate (when `engine` is `Some`, i.e. `abac.enabled`). `None` when
    /// neither layer is active, so `AppState` stays empty (auth-only).
    #[must_use]
    pub fn build(
        config: &AuthzConfig,
        base_path: &str,
        engine: Option<Arc<dyn PolicyEngine>>,
        resolvers: AuthzResolvers,
    ) -> Option<Self> {
        let rbac = config
            .rbac
            .enabled
            .then(|| RbacGate::new(config.rbac.clone(), base_path));
        let abac = engine.map(|engine| AbacGate::new(&config.abac, engine, resolvers));
        if rbac.is_none() && abac.is_none() {
            return None;
        }
        Some(Self { rbac, abac })
    }

    /// The coarse RBAC gate, if enabled.
    pub(crate) fn rbac(&self) -> Option<&RbacGate> {
        self.rbac.as_ref()
    }

    /// The fine-grained ABAC gate, if enabled.
    pub(crate) fn abac(&self) -> Option<&AbacGate> {
        self.abac.as_ref()
    }

    /// The configured JWT role-claim paths (fed into the [`Authenticator`] so
    /// Bearer role extraction uses them); defaults when RBAC is off.
    ///
    /// [`Authenticator`]: crate::extensions::access::authn::Authenticator
    #[must_use]
    pub fn role_claims(&self) -> Vec<String> {
        self.rbac
            .as_ref()
            .map_or_else(roles::default_role_claims, |r| r.rules.role_claims.clone())
    }
}

/// The fine-grained ABAC gate: the PDP engine, the DB-backed attribute
/// resolvers, and the resolved claim names + directory opt-in. The PEP
/// ([`crate::extensions::access::pep`]) drives it.
pub(crate) struct AbacGate {
    pub(crate) engine: Arc<dyn PolicyEngine>,
    pub(crate) resolvers: AuthzResolvers,
    /// The JWT claim carrying the caller's organization (blank → unused).
    pub(crate) organization_claim: Option<String>,
    /// The JWT claim carrying the patient id; its presence enables the subject
    /// gate (§5.7).
    pub(crate) patient_claim: Option<String>,
    /// Whether a `directory` policy is configured (v1 parity: DIRECTORY is
    /// unchecked unless opted in).
    pub(crate) directory_checked: bool,
}

impl std::fmt::Debug for AbacGate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AbacGate")
            .field("organization_claim", &self.organization_claim)
            .field("patient_claim", &self.patient_claim)
            .field("directory_checked", &self.directory_checked)
            .finish_non_exhaustive()
    }
}

impl AbacGate {
    fn new(config: &AbacConfig, engine: Arc<dyn PolicyEngine>, resolvers: AuthzResolvers) -> Self {
        Self {
            engine,
            resolvers,
            organization_claim: config.organization_claim().map(str::to_owned),
            patient_claim: config.patient_claim().map(str::to_owned),
            directory_checked: config.policy.contains_key("directory"),
        }
    }
}

/// The coarse RBAC gate: a route-template → [`OperationClass`] map (built once
/// from the generated `ROUTES` tables) plus the role rules.
#[derive(Debug)]
pub(crate) struct RbacGate {
    rules: RbacConfig,
    /// `(method, full-path-template)` → (class, operation id), keyed by
    /// `base_path` + [`crate::api::normalize_path`] so a request's `MatchedPath`
    /// matches. The op id feeds the read-only write classifier
    /// ([`classify::is_write`]).
    routes: HashMap<(Method, String), (OperationClass, &'static str)>,
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
                let full = format!("{base_path}{}", crate::api::normalize_path(path));
                routes.insert((method, full), (class, *op));
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
                .map_or_else(
                    || {
                        if mp.contains("/admin/") {
                            OperationClass::Admin
                        } else {
                            OperationClass::Clinical
                        }
                    },
                    |(class, _op)| *class,
                ),
        }
    }

    /// Whether the matched request is a **write** operation, for the read-only
    /// gate. A mapped route classifies via its op id ([`classify::is_write`]); an
    /// unmapped route (an extension surface outside the generated tables) falls
    /// back to its HTTP method — only `GET`/`HEAD` are reads, every mutating verb
    /// is a write so the read-only restriction cannot be bypassed there. A
    /// request with no matched path (a 404) is not a write to gate.
    pub(crate) fn is_write_for(&self, method: &Method, matched: Option<&str>) -> bool {
        match matched {
            None => false,
            Some(mp) => self
                .routes
                .get(&(method.clone(), mp.to_owned()))
                .map_or_else(
                    || !matches!(*method, Method::GET | Method::HEAD),
                    |(_class, op)| is_write(op),
                ),
        }
    }

    /// Apply the RBAC decision for a class + the caller's roles.
    pub(crate) fn decide(&self, class: OperationClass, principal_roles: &[String]) -> RbacDecision {
        authorize(class, principal_roles, &self.rules)
    }

    /// Apply the read-only restriction for a write flag + the caller's roles: a
    /// principal carrying the configured read-only role is refused on writes.
    pub(crate) fn decide_readonly(
        &self,
        is_write: bool,
        principal_roles: &[String],
    ) -> RbacDecision {
        authorize_readonly(is_write, principal_roles, &self.rules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "/ferroehr/rest/openehr/v1";

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
    fn is_write_for_maps_generated_ops() {
        let g = gate();
        // Writes on the clinical + definition surface.
        assert!(g.is_write_for(&Method::POST, Some(&format!("{BASE}/ehr"))));
        assert!(g.is_write_for(
            &Method::POST,
            Some(&format!("{BASE}/ehr/{{ehr_id}}/composition"))
        ));
        assert!(g.is_write_for(
            &Method::POST,
            Some(&format!("{BASE}/definition/template/adl1.4"))
        ));
        // Reads: a GET, and AQL execution (POST-but-read).
        assert!(!g.is_write_for(&Method::GET, Some(&format!("{BASE}/ehr/{{ehr_id}}"))));
        assert!(!g.is_write_for(&Method::POST, Some(&format!("{BASE}/query/aql"))));
        // Unmapped route: method-based fail-safe.
        assert!(g.is_write_for(&Method::POST, Some("/x/extension/unknown")));
        assert!(!g.is_write_for(&Method::GET, Some("/x/extension/unknown")));
        // A 404 (no matched path) is not a write to gate.
        assert!(!g.is_write_for(&Method::POST, None));
    }

    #[test]
    fn decide_readonly_denies_writes() {
        let g = gate();
        assert!(matches!(
            g.decide_readonly(true, &["READONLY".to_owned()]),
            RbacDecision::Deny(_)
        ));
        assert_eq!(
            g.decide_readonly(false, &["READONLY".to_owned()]),
            RbacDecision::Allow
        );
        assert_eq!(
            g.decide_readonly(true, &["USER".to_owned()]),
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

    #[test]
    fn build_engine_none_when_abac_disabled() {
        // ABAC off (the default) → no engine, current behaviour preserved.
        let engine = build_engine(&AbacConfig::default()).expect("build");
        assert!(engine.is_none());
    }

    #[tokio::test]
    async fn resolvers_invoke_their_closures() {
        // The AuthzResolvers type carries async DB closures; exercise the shape
        // with in-memory stand-ins (the binary supplies pool-backed ones).
        let resolvers = AuthzResolvers {
            subject: Arc::new(|ehr_id: String| {
                Box::pin(async move { Ok((ehr_id == "known").then(|| "subject-1".to_owned())) })
            }),
            template_of_version: Arc::new(|_vo: String, version: Option<String>| {
                Box::pin(async move { Ok(version.map(|v| format!("t.v{v}"))) })
            }),
        };
        assert_eq!(
            (resolvers.subject)("known".to_owned()).await.unwrap(),
            Some("subject-1".to_owned())
        );
        assert_eq!((resolvers.subject)("other".to_owned()).await.unwrap(), None);
        assert_eq!(
            (resolvers.template_of_version)("vo".to_owned(), Some("3".to_owned()))
                .await
                .unwrap(),
            Some("t.v3".to_owned())
        );
    }
}
