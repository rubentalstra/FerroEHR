//! [`RemotePdp`] — the v1-compatible external policy-server client
//! (`docs/enterprise/access-control.md` §5.5, wire contract §2.1).
//!
//! Byte-compatible with `EHRbase` v1: `POST {server}{policy-name}` with a **flat**
//! JSON body carrying only the configured, resolved keys
//! (`organization`/`patient`/`template`); HTTP **200 = permit**, any other
//! status = deny; the response body is ignored. Connection/IO failure is
//! fail-closed ([`AuthzError::Unreachable`] → 500 at the PEP). Multi-valued
//! attributes fan out as the full cartesian product with all-must-permit and
//! short-circuit deny (§5.4). Unlike v1, the client has explicit connect/request
//! timeouts (v1 defect #4).

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Map, Value};

use ehrbase::config::authz::{AbacConfig, AbacParam, PolicyRule};
use crate::extensions::access::authz::engine::{AuthzError, PolicyEngine};
use crate::extensions::access::authz::request::{AuthzRequest, Combination, Decision};

/// `metrics` counter incremented once per PDP HTTP call (`result` = permit/deny).
pub const METRIC_REMOTE_CALLS: &str = "authz_remote_pdp_calls_total";

/// The v1-compatible remote policy-decision-point client.
#[derive(Debug)]
pub struct RemotePdp {
    client: reqwest::Client,
    /// The base URL, guaranteed to end with `/` (config boot validation).
    server: String,
    /// Per-resource-kind policy bindings, keyed by [`ResourceKind::config_key`].
    ///
    /// [`ResourceKind::config_key`]: crate::extensions::access::authz::request::ResourceKind::config_key
    policies: BTreeMap<String, PolicyRule>,
}

impl RemotePdp {
    /// Build the client from the ABAC config (already boot-validated: `server`
    /// present and slash-terminated).
    ///
    /// # Errors
    /// [`AuthzError::PolicyLoad`] if the reqwest client cannot be built or the
    /// server URL is absent (defensive — config validation catches this first).
    pub fn new(config: &AbacConfig) -> Result<Self, AuthzError> {
        let server = config
            .remote
            .server
            .clone()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| AuthzError::PolicyLoad("abac.remote.server is not set".to_owned()))?;
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(config.remote.connect_timeout_ms))
            .timeout(Duration::from_millis(config.remote.request_timeout_ms))
            .build()
            .map_err(|e| AuthzError::PolicyLoad(format!("building remote PDP client: {e}")))?;
        Ok(Self {
            client,
            server,
            policies: config.policy.clone(),
        })
    }

    /// Build the flat request body for one combination: only the configured
    /// parameters that have a resolved value (§2.1).
    fn body(rule: &PolicyRule, combo: &Combination<'_>) -> Value {
        let mut map = Map::new();
        for param in &rule.parameters {
            let value = match param {
                AbacParam::Organization => combo.organization,
                AbacParam::Patient => combo.patient,
                AbacParam::Template => combo.template,
            };
            if let Some(v) = value {
                map.insert(param.wire_key().to_owned(), Value::String(v.to_owned()));
            }
        }
        Value::Object(map)
    }

    /// POST one combination and return whether the PDP permitted it (HTTP 200).
    async fn permits(
        &self,
        rule: &PolicyRule,
        combo: &Combination<'_>,
    ) -> Result<bool, AuthzError> {
        let url = format!("{}{}", self.server, rule.name);
        let body = Self::body(rule, combo);
        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AuthzError::Unreachable(format!("POST {url}: {e}")))?;
        Ok(response.status().as_u16() == 200)
    }
}

#[async_trait]
impl PolicyEngine for RemotePdp {
    async fn decide(&self, req: &AuthzRequest<'_>) -> Result<Decision, AuthzError> {
        // An unconfigured kind is unchecked (v1 parity for `directory`, §2.3).
        let Some(rule) = self.policies.get(req.kind.config_key()) else {
            return Ok(Decision::Permit);
        };
        for combo in req.combinations() {
            if !self.permits(rule, &combo).await? {
                metrics::counter!(METRIC_REMOTE_CALLS, "result" => "deny").increment(1);
                return Ok(Decision::Deny);
            }
            metrics::counter!(METRIC_REMOTE_CALLS, "result" => "permit").increment(1);
        }
        Ok(Decision::Permit)
    }
}
