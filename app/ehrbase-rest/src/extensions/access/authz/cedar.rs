//! [`CedarEngine`] — the embedded, default ABAC engine, built on `cedar-policy` 4.x.
//!
//! A typed Cedar **schema** (shipped, built from the [`ResourceKind`] ×
//! [`AccessMode`] enums so it can never drift from the op classification) is
//! validated at load; the operator's `*.cedar` **policies** are parsed and
//! schema-validated at boot — an invalid policy set refuses to start
//! (fail-closed at the earliest moment). Optional periodic reload swaps the
//! policy set on validity ([`arc_swap`]). Multi-valued attributes fan out per
//! [`AuthzRequest::combinations`] with the same all-must-permit / short-circuit
//! semantics as the remote PDP, so the two engines are behaviourally identical
//! (the §9.4 differential test).
//!
//! Cedar is deny-by-default with `forbid` overriding `permit`; the shipped
//! example policies document that.
//!
//! ## Attribute model
//! - **principal** `User { organization?, patient?, roles, scopes }` — the
//!   caller. `roles`/`scopes` are declared for advanced policies but left empty
//!   here (the [`AuthzRequest`] carries only the v1 wire attributes, so the
//!   example policies stay wire-equivalent with the remote PDP).
//! - **resource** (one entity type per [`ResourceKind`]) carries the
//!   per-combination candidate `patient?` and `template?`.
//! - **action** `"<kind>.<mode>"` (e.g. `composition.create`, `query.execute`).

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::str::FromStr as _;
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use async_trait::async_trait;
use cedar_policy::{
    Authorizer, Context, Decision as CedarDecision, Entities, Entity, EntityUid, PolicySet,
    Request, RestrictedExpression, Schema, ValidationMode, Validator,
};

use crate::extensions::access::authz::engine::{AuthzError, PolicyEngine};
use crate::extensions::access::authz::request::{
    AccessMode, AuthzRequest, Combination, Decision, ResourceKind,
};

/// Every resource kind, for schema construction.
const KINDS: [ResourceKind; 6] = [
    ResourceKind::Ehr,
    ResourceKind::EhrStatus,
    ResourceKind::Composition,
    ResourceKind::Contribution,
    ResourceKind::Query,
    ResourceKind::Directory,
];

/// Every access mode, for schema construction.
const MODES: [AccessMode; 5] = [
    AccessMode::Create,
    AccessMode::Read,
    AccessMode::Update,
    AccessMode::Delete,
    AccessMode::Execute,
];

/// The Cedar entity-type name for a resource kind (`PascalCase`).
const fn entity_type(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Ehr => "Ehr",
        ResourceKind::EhrStatus => "EhrStatus",
        ResourceKind::Composition => "Composition",
        ResourceKind::Contribution => "Contribution",
        ResourceKind::Query => "Query",
        ResourceKind::Directory => "Directory",
    }
}

/// The Cedar action id for a (kind, mode) pair, e.g. `composition.create`.
fn action_id(kind: ResourceKind, mode: AccessMode) -> String {
    format!("{}.{}", kind.config_key(), mode.as_str())
}

/// Build the shipped Cedar schema source from the kind × mode enums, so the
/// declared entity types and actions always match the runtime mapping.
fn schema_src() -> String {
    let mut s = String::new();
    s.push_str(
        "entity User {\n  organization?: String,\n  patient?: String,\n  \
         roles: Set<String>,\n  scopes: Set<String>,\n};\n",
    );
    for kind in KINDS {
        let _ = writeln!(
            s,
            "entity {} {{\n  patient?: String,\n  template?: String,\n}};",
            entity_type(kind)
        );
    }
    for kind in KINDS {
        for mode in MODES {
            let _ = writeln!(
                s,
                "action \"{}\" appliesTo {{ principal: [User], resource: [{}] }};",
                action_id(kind, mode),
                entity_type(kind)
            );
        }
    }
    s
}

/// The embedded Cedar engine: a validated schema + a hot-swappable, validated
/// policy set.
#[derive(Debug)]
pub struct CedarEngine {
    schema: Arc<Schema>,
    policies: Arc<ArcSwap<PolicySet>>,
}

impl CedarEngine {
    /// Build the engine from a policy directory. The schema is validated, then
    /// every `*.cedar` file in `policy_dir` is parsed and validated against it;
    /// any failure is an [`AuthzError::PolicyLoad`] (refuse to start). When
    /// `reload_secs` is `Some`, a background task periodically reloads and swaps
    /// the policy set on validity (invalid reloads are logged and ignored).
    ///
    /// # Errors
    /// [`AuthzError::PolicyLoad`] if the shipped schema, or the operator policy
    /// set, fails to parse or validate.
    pub fn new(policy_dir: &Path, reload_secs: Option<u64>) -> Result<Self, AuthzError> {
        let schema = build_schema()?;
        let schema = Arc::new(schema);
        let policies = load_and_validate(policy_dir, &schema)?;
        let policies = Arc::new(ArcSwap::from_pointee(policies));
        let engine = Self {
            schema: Arc::clone(&schema),
            policies: Arc::clone(&policies),
        };
        if let Some(secs) = reload_secs.filter(|s| *s > 0) {
            spawn_reload(policy_dir.to_owned(), schema, policies, secs);
        }
        Ok(engine)
    }

    /// Build directly from an in-memory policy-set source (tests).
    ///
    /// # Errors
    /// [`AuthzError::PolicyLoad`] if the schema or the policy source is invalid.
    pub fn from_policy_src(policies_src: &str) -> Result<Self, AuthzError> {
        let schema = Arc::new(build_schema()?);
        let policies = parse_and_validate(policies_src, &schema)?;
        Ok(Self {
            schema,
            policies: Arc::new(ArcSwap::from_pointee(policies)),
        })
    }

    /// Evaluate one combination; `Ok(true)` = permit.
    fn permits(&self, req: &AuthzRequest<'_>, combo: &Combination<'_>) -> Result<bool, AuthzError> {
        let principal = build_principal(combo)?;
        let resource = build_resource(req.kind, combo)?;
        let action: EntityUid = format!("Action::\"{}\"", action_id(req.kind, req.access))
            .parse()
            .map_err(|e| AuthzError::Evaluation(format!("action uid: {e}")))?;
        let principal_uid = principal.uid();
        let resource_uid = resource.uid();
        let entities = Entities::from_entities([principal, resource], Some(&self.schema))
            .map_err(|e| AuthzError::Evaluation(format!("entities: {e}")))?;
        let request = Request::new(
            principal_uid,
            action,
            resource_uid,
            Context::empty(),
            Some(&self.schema),
        )
        .map_err(|e| AuthzError::Evaluation(format!("request: {e}")))?;
        let policies = self.policies.load();
        let response = Authorizer::new().is_authorized(&request, &policies, &entities);
        Ok(response.decision() == CedarDecision::Allow)
    }
}

#[async_trait]
impl PolicyEngine for CedarEngine {
    async fn decide(&self, req: &AuthzRequest<'_>) -> Result<Decision, AuthzError> {
        for combo in req.combinations() {
            if self.permits(req, &combo)? {
                metrics::counter!(
                    ehrbase::telemetry::prometheus::AUTHZ_CEDAR_DECISIONS,
                    "result" => "permit"
                )
                .increment(1);
            } else {
                metrics::counter!(
                    ehrbase::telemetry::prometheus::AUTHZ_CEDAR_DECISIONS,
                    "result" => "deny"
                )
                .increment(1);
                return Ok(Decision::Deny);
            }
        }
        Ok(Decision::Permit)
    }
}

/// Parse + validate the shipped schema.
fn build_schema() -> Result<Schema, AuthzError> {
    let (schema, _warnings) = Schema::from_cedarschema_str(&schema_src())
        .map_err(|e| AuthzError::PolicyLoad(format!("cedar schema: {e}")))?;
    Ok(schema)
}

/// The principal entity for a combination.
fn build_principal(combo: &Combination<'_>) -> Result<Entity, AuthzError> {
    let uid: EntityUid = "User::\"caller\""
        .parse()
        .map_err(|e| AuthzError::Evaluation(format!("principal uid: {e}")))?;
    let mut attrs: HashMap<String, RestrictedExpression> = HashMap::new();
    if let Some(org) = combo.organization {
        attrs.insert(
            "organization".to_owned(),
            RestrictedExpression::new_string(org.to_owned()),
        );
    }
    if let Some(patient) = combo.patient {
        attrs.insert(
            "patient".to_owned(),
            RestrictedExpression::new_string(patient.to_owned()),
        );
    }
    // Declared for advanced policies; empty under the v1 wire attribute model.
    attrs.insert(
        "roles".to_owned(),
        RestrictedExpression::new_set(std::iter::empty()),
    );
    attrs.insert(
        "scopes".to_owned(),
        RestrictedExpression::new_set(std::iter::empty()),
    );
    Entity::new(uid, attrs, HashSet::new())
        .map_err(|e| AuthzError::Evaluation(format!("principal entity: {e}")))
}

/// The resource entity for a combination.
fn build_resource(kind: ResourceKind, combo: &Combination<'_>) -> Result<Entity, AuthzError> {
    let uid: EntityUid = format!("{}::\"resource\"", entity_type(kind))
        .parse()
        .map_err(|e| AuthzError::Evaluation(format!("resource uid: {e}")))?;
    let mut attrs: HashMap<String, RestrictedExpression> = HashMap::new();
    if let Some(patient) = combo.patient {
        attrs.insert(
            "patient".to_owned(),
            RestrictedExpression::new_string(patient.to_owned()),
        );
    }
    if let Some(template) = combo.template {
        attrs.insert(
            "template".to_owned(),
            RestrictedExpression::new_string(template.to_owned()),
        );
    }
    Entity::new(uid, attrs, HashSet::new())
        .map_err(|e| AuthzError::Evaluation(format!("resource entity: {e}")))
}

/// Read + concatenate every `*.cedar` file in `dir`, then parse and validate.
fn load_and_validate(dir: &Path, schema: &Schema) -> Result<PolicySet, AuthzError> {
    let mut sources = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| {
        AuthzError::PolicyLoad(format!("reading policy dir {}: {e}", dir.display()))
    })?;
    for entry in entries {
        let path = entry
            .map_err(|e| AuthzError::PolicyLoad(format!("policy dir entry: {e}")))?
            .path();
        if path.extension().and_then(|e| e.to_str()) == Some("cedar") {
            let text = std::fs::read_to_string(&path).map_err(|e| {
                AuthzError::PolicyLoad(format!("reading policy {}: {e}", path.display()))
            })?;
            sources.push(text);
        }
    }
    if sources.is_empty() {
        return Err(AuthzError::PolicyLoad(format!(
            "no *.cedar policy files found in {}",
            dir.display()
        )));
    }
    parse_and_validate(&sources.join("\n"), schema)
}

/// Parse a policy-set source and strict-validate it against the schema.
fn parse_and_validate(src: &str, schema: &Schema) -> Result<PolicySet, AuthzError> {
    let policies = PolicySet::from_str(src)
        .map_err(|e| AuthzError::PolicyLoad(format!("parsing policies: {e}")))?;
    let validator = Validator::new(schema.clone());
    let result = validator.validate(&policies, ValidationMode::Strict);
    if result.validation_passed() {
        Ok(policies)
    } else {
        let errors: Vec<String> = result
            .validation_errors()
            .map(ToString::to_string)
            .collect();
        Err(AuthzError::PolicyLoad(format!(
            "policy set failed schema validation: {}",
            errors.join("; ")
        )))
    }
}

/// Spawn the periodic policy-reload task (swap-on-valid).
fn spawn_reload(dir: PathBuf, schema: Arc<Schema>, policies: Arc<ArcSwap<PolicySet>>, secs: u64) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(secs));
        ticker.tick().await; // consume the immediate first tick
        loop {
            ticker.tick().await;
            match load_and_validate(&dir, &schema) {
                Ok(next) => {
                    policies.store(Arc::new(next));
                    tracing::debug!("cedar policy set reloaded");
                }
                Err(e) => tracing::warn!("cedar policy reload skipped (invalid): {e}"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::access::authz::request::Attr;

    fn req<'a>(patient: Option<Attr>, template: Option<Attr>) -> AuthzRequest<'a> {
        AuthzRequest {
            operation_id: "composition_create",
            kind: ResourceKind::Composition,
            access: AccessMode::Create,
            organization: Some("org1".to_owned()),
            patient,
            template,
        }
    }

    #[test]
    fn shipped_schema_is_valid() {
        assert!(build_schema().is_ok());
    }

    #[test]
    fn empty_policy_source_is_rejected() {
        // No `*.cedar` content → deny-by-default engine would still build from a
        // non-empty parse, but a syntactically invalid policy must be rejected.
        let bad = CedarEngine::from_policy_src("permit(this is not cedar);");
        assert!(matches!(bad, Err(AuthzError::PolicyLoad(_))));
    }

    #[test]
    fn policy_referencing_unknown_attribute_fails_validation() {
        // `resource.nonsense` is not in the schema → validation error at load.
        let src = r#"permit(principal, action, resource) when { resource.nonsense == "x" };"#;
        let e = CedarEngine::from_policy_src(src).expect_err("must reject");
        assert!(matches!(e, AuthzError::PolicyLoad(_)), "got {e:?}");
    }

    #[tokio::test]
    async fn deny_by_default_when_no_policy_permits() {
        // A forbid-only / unrelated policy set → deny.
        let src = r#"forbid(principal, action, resource) when { resource has patient && resource.patient == "blocked" };"#;
        let engine = CedarEngine::from_policy_src(src).expect("build");
        let r = req(Some(Attr::One("p1".to_owned())), None);
        assert_eq!(engine.decide(&r).await.unwrap(), Decision::Deny);
    }

    #[tokio::test]
    async fn permit_then_forbid_overrides() {
        let src = r#"
permit(principal, action, resource);
forbid(principal, action, resource) when { resource has patient && resource.patient == "blocked" };
"#;
        let engine = CedarEngine::from_policy_src(src).expect("build");
        // Not blocked → permit.
        assert_eq!(
            engine
                .decide(&req(Some(Attr::One("ok".to_owned())), None))
                .await
                .unwrap(),
            Decision::Permit
        );
        // Blocked → forbid overrides permit.
        assert_eq!(
            engine
                .decide(&req(Some(Attr::One("blocked".to_owned())), None))
                .await
                .unwrap(),
            Decision::Deny
        );
    }

    #[tokio::test]
    async fn fan_out_all_must_permit_and_empty_permits() {
        // Permit iff patient == "ok"; a set with one non-"ok" element denies.
        let src = r#"permit(principal, action, resource) when { resource has patient && resource.patient == "ok" };"#;
        let engine = CedarEngine::from_policy_src(src).expect("build");
        assert_eq!(
            engine
                .decide(&req(
                    Some(Attr::Set(vec!["ok".to_owned(), "ok".to_owned()])),
                    None
                ))
                .await
                .unwrap(),
            Decision::Permit
        );
        assert_eq!(
            engine
                .decide(&req(
                    Some(Attr::Set(vec!["ok".to_owned(), "bad".to_owned()])),
                    None
                ))
                .await
                .unwrap(),
            Decision::Deny
        );
        // Empty set → no combinations → vacuous permit.
        assert_eq!(
            engine
                .decide(&req(Some(Attr::Set(vec![])), None))
                .await
                .unwrap(),
            Decision::Permit
        );
    }
}
