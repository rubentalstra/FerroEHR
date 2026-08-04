//! [`FhirTerminologyProvider`] — a FHIR R4 terminology-server client
//! realizing the SM `I_TERMINOLOGY_SERVICE` calls against a remote server,
//! over `reqwest` (rustls).
//!
//! The remote provider is one of the two the routing layer
//! (`super::routing`) selects among; the in-process `openehr-term` bundle
//! (`super::bundle`) is the enumerable local default. `arch-overview
//! master12-terminology.adoc` models this concrete backend as an external
//! "terminology query server", so it belongs with the interface realization.
//!
//! Design: our own — the client + the HAPI-FHIR/Snowstorm
//! server it points at. SM contract:
//! `docs/specs/openehr/SM/docs/UML/classes/i_terminology_service.adoc`.
//!
//! # SM call → FHIR operation mapping
//!
//! | SM call | FHIR R4 operation |
//! |---|---|
//! | `value_set_validate` | `ValueSet/$validate-code` (or `$expand` + membership) |
//! | `get_value_set` | `ValueSet/$expand` → [`TerminologyExtract`] |
//! | `subsumes` | `CodeSystem/$subsumes` (outcome `subsumes`) |
//! | `has_term` / `get_term` | `CodeSystem/$lookup` |
//!
//! NOTE (enumerating calls): `get_terminology_ids`,
//! `get_terminology_description` and `has_terminology` have no faithful FHIR
//! operation — a FHIR TS is a validation/expansion backend, not an enumerable
//! openEHR terminology bundle. The routing layer answers them from the
//! in-process `openehr-term` bundle, which remains the enumerable
//! terminology; this provider's [`FhirTerminologyProvider::get_terminology_description`]
//! is an explicit `NotImplemented`.
//!
//! NOTE (temporal): the SM `at_date` (`i_terminology_service.adoc`
//! `has_term`/`get_term`/`value_set_validate`, an `Iso8601_date`) selects the
//! terminology as it stood on a date. It is forwarded to the server as the
//! FHIR `date` parameter of `$lookup`/`$validate-code`/`$expand`.
//!
//! NOTE (hierarchy): a FHIR `ValueSet/$expand` may nest
//! members under `contains`. We keep the flat `Terminology_extract._terms_`
//! (the membership view) **and** preserve the tree in
//! `Terminology_extract._relationships_` as `Term_relationship`s under the
//! `CHILD_RELATION` name (`terminology_extract.adoc` §Structured value
//! set), defined in `_relations_` by the FHIR `child` concept property URI
//! (`FHIR_CHILD_PROPERTY`) — an `external_code` relation
//! (`terminology_relation.adoc` `Inv_valid_definition`).
//!
//! NOTE (errors): a value set / terminology / code the server does not
//! know (HTTP `404`, or `$validate-code result=false` with no membership) is
//! a `Pre_has_*` precondition failure →
//! [`CallStatusType::VersionedObjectDoesNotExist`] (the `404` reading,
//! matching the bundle provider in `super::bundle`). A transport fault
//! (connect/read timeout, `5xx`, malformed body) → [`SmError::exception`]
//! (`500`); the fail-open vs fail-closed decision belongs to the caller (the
//! composition-validation walker,
//! [`config::ExternalTerminologyConfig::fail_on_error`](super::config::ExternalTerminologyConfig::fail_on_error)), never to the raw
//! provider.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 6): FHIR resources are an external standard \
              with no RM type (typed-FHIR evaluation tracked separately)"
)]

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;

use crate::service::status::{CallStatusType, SmError};
use crate::service::terminology::types::{
    DefinedTerm, TermCode, TermEntry, TermRelationship, TerminologyDescription, TerminologyExtract,
    TerminologyRelation,
};

use super::config::{FhirOperation, FhirProviderConfig};
use super::oauth2::TokenSource;

/// The `Term_relationship.relation_name` under which a FHIR `$expand`
/// parent→child `contains` nesting is preserved (`terminology_extract.adoc`).
const CHILD_RELATION: &str = "child";
/// The FHIR R4 concept-property URI that defines the parent→child relation,
/// carried as the `Terminology_relation.external_code`
/// (`terminology_relation.adoc`).
const FHIR_CHILD_PROPERTY: &str = "http://hl7.org/fhir/concept-properties#child";

/// A FHIR R4 terminology-server client realizing the SM
/// `I_TERMINOLOGY_SERVICE` lookup/validation calls against a remote server.
#[derive(Debug, Clone)]
pub struct FhirTerminologyProvider {
    client: reqwest::Client,
    /// The FHIR base URL, trailing `/` stripped (e.g. `http://host:8090/fhir`).
    base: String,
    /// The membership operation used by `value_set_validate`.
    operation: FhirOperation,
    /// The configured provider name (for error/log context).
    name: String,
    /// TTL-bounded response cache keyed by the full operation URL (`None`
    /// when disabled). Caches the decoded JSON of successful responses —
    /// including the 404 "unknown resource" outcome — so a validation burst
    /// over the same codes costs one remote round trip per TTL window.
    cache: Option<moka::future::Cache<String, Option<serde_json::Value>>>,
    /// The `OAuth2` client-credentials token source whose bearer credential
    /// authenticates every request, when the provider configures one. `None` =
    /// unauthenticated requests.
    token: Option<Arc<TokenSource>>,
}

impl FhirTerminologyProvider {
    /// Build a provider from its configuration.
    ///
    /// # Errors
    ///
    /// [`SmError::exception`] if the URL is empty (after trimming), the
    /// configured TLS material is unusable ([`super::tls::TlsMaterialError`]),
    /// or the `reqwest` client cannot be built (e.g. TLS backend init
    /// failure).
    pub fn new(name: &str, cfg: &FhirProviderConfig) -> Result<Self, SmError> {
        let base = cfg.url.trim().trim_end_matches('/').to_owned();
        if base.is_empty() {
            return Err(SmError::exception(format!(
                "terminology provider '{name}' has an empty url"
            )));
        }
        let builder = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(cfg.connect_timeout_ms))
            .timeout(Duration::from_millis(cfg.request_timeout_ms));
        // Mutual TLS + the provider's trust anchors, when configured. Bad
        // material is a boot failure here, never a first-request surprise
        // ([`super::tls`]).
        let builder = super::tls::apply(builder, cfg)
            .map_err(|e| SmError::exception(format!("terminology provider '{name}': {e}")))?;
        let client = builder.build().map_err(|e| {
            SmError::exception(format!(
                "building terminology client for provider '{name}': {e}"
            ))
        })?;
        let cache = (cfg.cache_ttl_secs > 0).then(|| {
            moka::future::Cache::builder()
                .max_capacity(cfg.cache_capacity)
                .time_to_live(Duration::from_secs(cfg.cache_ttl_secs))
                .build()
        });
        Ok(Self {
            client,
            base,
            operation: cfg.operation,
            name: name.to_owned(),
            cache,
            token: None,
        })
    }

    /// Attach the `OAuth2` client-credentials token source whose bearer
    /// credential every request to this provider carries
    /// (`[terminology.external.providers.<name>] oauth2_client`).
    #[must_use]
    pub fn with_token(mut self, token: Arc<TokenSource>) -> Self {
        self.token = Some(token);
        self
    }

    /// The configured provider name (routing, logging, error context).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// GET a FHIR operation with query params, returning the parsed body.
    ///
    /// `404`/`410` → `Ok(None)` (the resource is unknown → a precondition the
    /// caller maps to `VersionedObjectDoesNotExist`).
    ///
    /// # Errors
    ///
    /// [`SmError::exception`] on an invalid operation URL, a transport fault
    /// (connect/read timeout), any other non-2xx status, or a body that does
    /// not parse as `T`.
    pub async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        op_path: &str,
        query: &[(&str, &str)],
    ) -> Result<Option<T>, SmError> {
        let mut url = reqwest::Url::parse(&format!("{}{op_path}", self.base))
            .map_err(|e| self.provider_fault(op_path, &format_args!("invalid url: {e}")))?;
        {
            let mut pairs = url.query_pairs_mut();
            for (k, v) in query {
                pairs.append_pair(k, v);
            }
        }
        if let Some(cache) = &self.cache
            && let Some(hit) = cache.get(url.as_str()).await
        {
            return match hit {
                Some(body) => serde_json::from_value(body).map(Some).map_err(|e| {
                    self.provider_fault(op_path, &format_args!("cached response decode: {e}"))
                }),
                None => Ok(None),
            };
        }
        let cache_key = url.as_str().to_owned();
        let mut request = self
            .client
            .get(url)
            .header(reqwest::header::ACCEPT, "application/fhir+json");
        // The bearer is resolved per request (the token source serves it from
        // its cache and re-grants only around expiry), so a rotated credential
        // takes effect without a restart.
        if let Some(token) = &self.token {
            request = request.bearer_auth(token.bearer().await?);
        }
        let response = request
            .send()
            .await
            .map_err(|e| self.transport_error(op_path, &e))?;
        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::GONE {
            if let Some(cache) = &self.cache {
                cache.insert(cache_key, None).await;
            }
            return Ok(None);
        }
        if !status.is_success() {
            return Err(self.provider_fault(
                op_path,
                &format_args!("upstream returned HTTP {}", status.as_u16()),
            ));
        }
        let body = response.json::<serde_json::Value>().await.map_err(|e| {
            self.provider_fault(op_path, &format_args!("malformed FHIR response: {e}"))
        })?;
        if let Some(cache) = &self.cache {
            cache.insert(cache_key, Some(body.clone())).await;
        }
        serde_json::from_value(body).map(Some).map_err(|e| {
            self.provider_fault(op_path, &format_args!("malformed FHIR response: {e}"))
        })
    }

    /// A classified transport-fault exception (timeout / connect / other).
    fn transport_error(&self, op_path: &str, e: &reqwest::Error) -> SmError {
        let kind = if e.is_timeout() {
            "timeout"
        } else if e.is_connect() {
            "connect error"
        } else {
            "transport error"
        };
        self.provider_fault(op_path, &format_args!("{kind}: {e}"))
    }

    /// A `500` for an upstream-terminology fault, with the OPERATOR detail
    /// (which provider, which operation, the upstream diagnostic) on the trace
    /// record and NOT on the wire.
    ///
    /// The detail here is the deployment's own configuration — the provider's
    /// configured name and the upstream server's error — which is exactly the
    /// class of fact a tenant's clients must not be able to read out of a
    /// response body, while the operator still needs it in full. The server
    /// carries no per-request diagnostic surface (`/management` mounts
    /// info/prometheus/metrics/env/loggers only), so the trace record IS the
    /// operator channel. (No openEHR spec governs the content of a `500` body
    /// — our own design/extension; the status itself is the ITS-REST overview
    /// `Requests_and_responses.md` §HTTP status codes row.)
    fn provider_fault(&self, op_path: &str, detail: &dyn std::fmt::Display) -> SmError {
        tracing::error!(
            provider = %self.name,
            operation = op_path,
            error = %detail,
            "terminology provider fault → 500"
        );
        SmError::exception(crate::service::error::INTERNAL_MESSAGE.to_owned())
    }

    /// A `Pre_has_*` precondition failure (`VersionedObjectDoesNotExist`).
    ///
    /// The body carries only what the CLIENT can act on — the kind and id it
    /// asked about; WHICH configured provider answered is deployment
    /// configuration and goes to the trace record only (the #1809 adjudication
    /// extended to the 4xx class, #1819).
    fn not_found(&self, what: &str, id: &str) -> SmError {
        tracing::debug!(provider = %self.name, what, id, "terminology lookup: not found");
        SmError::new(
            CallStatusType::VersionedObjectDoesNotExist,
            format!("{what} '{id}' not found"),
        )
    }

    /// `ValueSet/$expand` → an [`FhirValueSet`], or `None` when the value set
    /// is unknown (`404`). Forwards `at_date` as the FHIR `date` parameter
    ///.
    ///
    /// # Errors
    ///
    /// As [`FhirTerminologyProvider::get`].
    async fn expand(
        &self,
        value_set_url: &str,
        at_date: Option<&str>,
    ) -> Result<Option<FhirValueSet>, SmError> {
        let mut query = vec![("url", value_set_url)];
        if let Some(date) = at_date {
            query.push(("date", date));
        }
        self.get("/ValueSet/$expand", &query).await
    }

    /// `value_set_validate` → FHIR `ValueSet/$validate-code` (or `$expand` +
    /// membership when the provider is configured for `expand`).
    ///
    /// `terminology_id` is passed as the FHIR `system`; `value_set_id` as the
    /// value-set `url`; `at_date` as the FHIR `date`. A known value set
    /// with a non-member code → `Ok(false)`.
    ///
    /// # Errors
    ///
    /// Precondition on an empty `candidate_code`;
    /// `VersionedObjectDoesNotExist` on an unknown value set (`404`);
    /// exception on a transport fault or a `$validate-code` response with no
    /// `result` parameter.
    pub async fn value_set_validate(
        &self,
        terminology_id: &str,
        value_set_id: &str,
        candidate_code: &str,
        at_date: Option<String>,
    ) -> Result<bool, SmError> {
        if candidate_code.is_empty() {
            return Err(SmError::precondition("candidate_code must not be empty"));
        }
        match self.operation {
            FhirOperation::Expand => {
                let vs = self
                    .expand(value_set_id, at_date.as_deref())
                    .await?
                    .ok_or_else(|| self.not_found("value set", value_set_id))?;
                Ok(vs.contains_code(candidate_code))
            }
            FhirOperation::ValidateCode => {
                let mut query = vec![
                    ("url", value_set_id),
                    ("system", terminology_id),
                    ("code", candidate_code),
                ];
                if let Some(date) = at_date.as_deref() {
                    query.push(("date", date));
                }
                let params: FhirParameters = self
                    .get("/ValueSet/$validate-code", &query)
                    .await?
                    .ok_or_else(|| self.not_found("value set", value_set_id))?;
                params.result().ok_or_else(|| {
                    SmError::exception(format!(
                        "terminology provider '{}': $validate-code response had no 'result'",
                        self.name
                    ))
                })
            }
        }
    }

    /// `get_value_set` → FHIR `ValueSet/$expand`, mapped to a
    /// [`TerminologyExtract`] (flat `terms` for membership; the `contains`
    /// tree preserved as `relationships`). `terminology_id` is
    /// unused — the value set is identified by its URL (`value_set_code`).
    ///
    /// # Errors
    ///
    /// `VersionedObjectDoesNotExist` when `$expand` answers `404`; exception
    /// on a transport fault / non-2xx / malformed response.
    pub async fn get_value_set(
        &self,
        _terminology_id: &str,
        value_set_code: &str,
    ) -> Result<TerminologyExtract, SmError> {
        let vs = self
            .expand(value_set_code, None)
            .await?
            .ok_or_else(|| self.not_found("value set", value_set_code))?;
        Ok(vs.into_extract(value_set_code))
    }

    /// `subsumes` → FHIR `CodeSystem/$subsumes` (`codeA` = `ref_code`,
    /// `codeB` = `candidate_child_code`). True iff the outcome is `subsumes`
    /// — the SM's *strict* subsumption (`equivalent` is excluded).
    ///
    /// # Errors
    ///
    /// `VersionedObjectDoesNotExist` when the server answers `404` for the
    /// terminology; exception on a transport fault or a response with no
    /// `outcome` parameter.
    pub async fn subsumes(
        &self,
        terminology_id: &str,
        ref_code: &str,
        candidate_child_code: &str,
    ) -> Result<bool, SmError> {
        let params: FhirParameters = self
            .get(
                "/CodeSystem/$subsumes",
                &[
                    ("system", terminology_id),
                    ("codeA", ref_code),
                    ("codeB", candidate_child_code),
                ],
            )
            .await?
            .ok_or_else(|| self.not_found("terminology", terminology_id))?;
        let outcome = params.code("outcome").ok_or_else(|| {
            SmError::exception(format!(
                "terminology provider '{}': $subsumes response had no 'outcome'",
                self.name
            ))
        })?;
        Ok(outcome == "subsumes")
    }

    /// `has_term` → FHIR `CodeSystem/$lookup`: `true` when the lookup
    /// resolves (`200`), `false` when the code is unknown (`404`). `at_date`
    /// → the FHIR `date` parameter.
    ///
    /// # Errors
    ///
    /// Exception on a transport fault, any non-2xx status other than
    /// `404`/`410`, or a malformed response body.
    pub async fn has_term(
        &self,
        terminology_id: &str,
        code: &str,
        at_date: Option<String>,
    ) -> Result<bool, SmError> {
        let mut query = vec![("system", terminology_id), ("code", code)];
        if let Some(date) = at_date.as_deref() {
            query.push(("date", date));
        }
        let found: Option<FhirParameters> = self.get("/CodeSystem/$lookup", &query).await?;
        Ok(found.is_some())
    }

    /// `get_term` → FHIR `CodeSystem/$lookup`, mapped to a single-term
    /// [`TerminologyExtract`] (the `display` becomes the term text; a lookup
    /// with no `display` falls back to the code itself). `at_date` → the FHIR
    /// `date` parameter.
    ///
    /// NOTE (attributes): the SM `attributes` allow-list filters
    /// the meta-model attributes returned. `$lookup` returns only the concept
    /// `display` (mapped to the term text), so there is nothing further to
    /// filter; `attributes` is accepted and has no effect on the returned
    /// extract.
    ///
    /// # Errors
    ///
    /// `VersionedObjectDoesNotExist` when `$lookup` answers `404` (unknown
    /// term); exception on a transport fault / non-2xx / malformed response.
    pub async fn get_term(
        &self,
        terminology_id: &str,
        code: &str,
        _attributes: Option<BTreeMap<String, String>>,
        at_date: Option<String>,
    ) -> Result<TerminologyExtract, SmError> {
        let mut query = vec![("system", terminology_id), ("code", code)];
        if let Some(date) = at_date.as_deref() {
            query.push(("date", date));
        }
        let params: FhirParameters = self
            .get("/CodeSystem/$lookup", &query)
            .await?
            .ok_or_else(|| self.not_found("term", code))?;
        let text = params.string("display").unwrap_or_else(|| code.to_owned());
        let mut terms = BTreeMap::new();
        terms.insert(
            code.to_owned(),
            TermEntry::Defined(DefinedTerm {
                code: code.to_owned(),
                text,
                language: None,
                is_preferred_term: None,
            }),
        );
        Ok(TerminologyExtract {
            terminology_id: terminology_id.to_owned(),
            terminology_version: None,
            terms: Some(terms),
            relationships: None,
            relations: None,
        })
    }

    /// `has_value_set` → FHIR `ValueSet/$expand`: `true` when the value set
    /// expands (`200`), `false` when it is unknown (`404`). `terminology_id`
    /// is unused — the value set is identified by its URL.
    ///
    /// # Errors
    ///
    /// Exception on a transport fault, any non-2xx status other than
    /// `404`/`410`, or a malformed response body.
    pub async fn has_value_set(
        &self,
        _terminology_id: &str,
        value_set_code: &str,
    ) -> Result<bool, SmError> {
        Ok(self.expand(value_set_code, None).await?.is_some())
    }

    /// `get_terminology_description` → not modelled for a FHIR TS (NOTE
    /// at module head; the routing layer answers this from the bundle).
    ///
    /// # Errors
    ///
    /// Always [`CallStatusType::NotImplemented`].
    #[expect(
        clippy::unused_self,
        reason = "the SM interface declares this call on the service; the \
                  protocol adapter invokes every SM call uniformly, so the \
                  receiver stays even where this realization ignores it"
    )]
    pub fn get_terminology_description(
        &self,
        _terminology_id: &str,
    ) -> Result<TerminologyDescription, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "a FHIR terminology server does not expose terminology descriptions",
        ))
    }
}

// ─── the FHIR R4 wire subset (only what these operations read) ───────────────

/// A FHIR `Parameters` resource (the `$validate-code`/`$subsumes`/`$lookup`
/// response envelope) — only the fields these operations read.
#[derive(Debug, Clone, Deserialize)]
struct FhirParameters {
    #[serde(default)]
    parameter: Vec<FhirParameter>,
}

/// One `Parameters.parameter` entry (only the value kinds we consume).
#[derive(Debug, Clone, Deserialize)]
struct FhirParameter {
    name: String,
    #[serde(rename = "valueBoolean")]
    value_boolean: Option<bool>,
    #[serde(rename = "valueString")]
    value_string: Option<String>,
    #[serde(rename = "valueCode")]
    value_code: Option<String>,
}

impl FhirParameters {
    /// The `result` boolean of a `$validate-code` response.
    fn result(&self) -> Option<bool> {
        self.parameter
            .iter()
            .find(|p| p.name == "result")
            .and_then(|p| p.value_boolean)
    }

    /// A named `valueCode` parameter (e.g. `$subsumes` `outcome`).
    fn code(&self, name: &str) -> Option<&str> {
        self.parameter
            .iter()
            .find(|p| p.name == name)
            .and_then(|p| p.value_code.as_deref())
    }

    /// A named `valueString` parameter (e.g. `$lookup` `display`).
    fn string(&self, name: &str) -> Option<String> {
        self.parameter
            .iter()
            .find(|p| p.name == name)
            .and_then(|p| p.value_string.clone())
    }
}

/// A FHIR `ValueSet` resource with an expansion — only the expansion members.
#[derive(Debug, Clone, Deserialize)]
struct FhirValueSet {
    expansion: Option<FhirExpansion>,
}

/// `ValueSet.expansion`.
#[derive(Debug, Clone, Deserialize)]
struct FhirExpansion {
    #[serde(default)]
    contains: Vec<FhirContains>,
}

/// `ValueSet.expansion.contains` (hierarchical: may nest further members).
#[derive(Debug, Clone, Deserialize)]
struct FhirContains {
    code: Option<String>,
    display: Option<String>,
    #[serde(default)]
    contains: Vec<FhirContains>,
}

impl FhirValueSet {
    /// Whether `code` appears anywhere in the (possibly hierarchical)
    /// expansion.
    fn contains_code(&self, code: &str) -> bool {
        self.expansion
            .as_ref()
            .is_some_and(|e| e.contains.iter().any(|c| c.contains_code(code)))
    }

    /// Map the expansion into a [`TerminologyExtract`]: a flat `terms` map
    /// keyed by code (the membership view) plus, when the expansion nests
    /// members, the parent→child tree preserved as `relationships`.
    fn into_extract(self, terminology_id: &str) -> TerminologyExtract {
        let mut terms = BTreeMap::new();
        let mut relationships = Vec::new();
        if let Some(expansion) = self.expansion {
            for c in &expansion.contains {
                c.collect(&mut terms, &mut relationships);
            }
        }
        // The `child` relation is defined by the FHIR concept-property URI
        // (an `external_code` relation; `terminology_relation.adoc`
        // `Inv_valid_definition`), present only when a hierarchy was emitted.
        let relations = (!relationships.is_empty()).then(|| {
            let mut m = BTreeMap::new();
            m.insert(
                CHILD_RELATION.to_owned(),
                TerminologyRelation::external(CHILD_RELATION, FHIR_CHILD_PROPERTY),
            );
            m
        });
        TerminologyExtract {
            terminology_id: terminology_id.to_owned(),
            terminology_version: None,
            terms: if terms.is_empty() { None } else { Some(terms) },
            relationships: if relationships.is_empty() {
                None
            } else {
                Some(relationships)
            },
            relations,
        }
    }
}

impl FhirContains {
    /// Whether `code` is this member or any descendant.
    fn contains_code(&self, code: &str) -> bool {
        self.code.as_deref() == Some(code) || self.contains.iter().any(|c| c.contains_code(code))
    }

    /// Collect this member (and its descendants) into the flat `terms` map,
    /// and record a `Term_relationship` from this member to each direct child
    /// so the `$expand` hierarchy is not lost.
    fn collect(&self, out: &mut BTreeMap<String, TermEntry>, rels: &mut Vec<TermRelationship>) {
        if let Some(code) = &self.code {
            let entry = match &self.display {
                Some(text) => TermEntry::Defined(DefinedTerm {
                    code: code.clone(),
                    text: text.clone(),
                    language: None,
                    is_preferred_term: None,
                }),
                None => TermEntry::Bare(TermCode { code: code.clone() }),
            };
            out.insert(code.clone(), entry);

            let child_codes: Vec<String> = self
                .contains
                .iter()
                .filter_map(|c| c.code.clone())
                .collect();
            if !child_codes.is_empty() {
                rels.push(TermRelationship {
                    origin_code: code.clone(),
                    relation_name: CHILD_RELATION.to_owned(),
                    target_codes: Some(child_codes),
                });
            }
        }
        for child in &self.contains {
            child.collect(out, rels);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_params(json: &str) -> FhirParameters {
        serde_json::from_str(json).expect("parse Parameters")
    }

    #[test]
    fn validate_code_result_extracted() {
        let p = parse_params(
            r#"{"resourceType":"Parameters","parameter":[
                {"name":"result","valueBoolean":true},
                {"name":"display","valueString":"Buccal"}]}"#,
        );
        assert_eq!(p.result(), Some(true));
        assert_eq!(p.string("display").as_deref(), Some("Buccal"));
    }

    #[test]
    fn subsumes_outcome_extracted() {
        let p = parse_params(
            r#"{"resourceType":"Parameters","parameter":[
                {"name":"outcome","valueCode":"subsumes"}]}"#,
        );
        assert_eq!(p.code("outcome"), Some("subsumes"));
    }

    #[test]
    fn expansion_membership_and_flattening() {
        let vs: FhirValueSet = serde_json::from_str(
            r#"{"resourceType":"ValueSet","expansion":{"contains":[
                {"system":"s","code":"B","display":"Buccal"},
                {"system":"s","code":"L","contains":[
                    {"system":"s","code":"O","display":"Occlusal"}]}]}}"#,
        )
        .expect("parse ValueSet");
        assert!(vs.contains_code("B"));
        assert!(vs.contains_code("O")); // nested
        assert!(!vs.contains_code("Z"));
        let ext = vs.into_extract("http://example/vs/surface");
        let terms = ext.terms.expect("terms");
        assert_eq!(terms.len(), 3);
        assert!(matches!(terms.get("L"), Some(TermEntry::Bare(_))));
        assert!(matches!(terms.get("B"), Some(TermEntry::Defined(_))));
    }

    #[test]
    fn expansion_hierarchy_preserved_as_relationships() {
        // The parent→child `contains` tree survives as
        // relationships, with the `child` relation defined by its FHIR
        // property URI.
        let vs: FhirValueSet = serde_json::from_str(
            r#"{"resourceType":"ValueSet","expansion":{"contains":[
                {"system":"s","code":"L","display":"Lower","contains":[
                    {"system":"s","code":"O","display":"Occlusal"}]}]}}"#,
        )
        .expect("parse ValueSet");
        let ext = vs.into_extract("http://example/vs/surface");
        let rels = ext.relationships.expect("relationships");
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].origin_code, "L");
        assert_eq!(rels[0].relation_name, CHILD_RELATION);
        assert_eq!(rels[0].target_codes.as_deref(), Some(&["O".to_owned()][..]));
        let relations = ext.relations.expect("relations");
        let child = relations.get(CHILD_RELATION).expect("child relation");
        assert_eq!(child.external_code.as_deref(), Some(FHIR_CHILD_PROPERTY));
        assert_eq!(child.local_code, None);
    }

    #[test]
    fn flat_expansion_has_no_relationships() {
        let vs: FhirValueSet = serde_json::from_str(
            r#"{"resourceType":"ValueSet","expansion":{"contains":[
                {"system":"s","code":"A","display":"Alpha"},
                {"system":"s","code":"B","display":"Beta"}]}}"#,
        )
        .expect("parse ValueSet");
        let ext = vs.into_extract("http://example/vs/flat");
        assert_eq!(ext.terms.expect("terms").len(), 2);
        assert!(ext.relationships.is_none());
        assert!(ext.relations.is_none());
    }

    #[test]
    fn empty_url_rejected() {
        let cfg = FhirProviderConfig {
            kind: super::super::config::ProviderKind::Fhir,
            url: "  ".to_owned(),
            operation: FhirOperation::ValidateCode,
            connect_timeout_ms: 100,
            request_timeout_ms: 100,
            oauth2_client: None,
            client_cert_path: None,
            client_key_path: None,
            ca_bundle_path: None,
            cache_ttl_secs: 0,
            cache_capacity: 1024,
        };
        assert!(FhirTerminologyProvider::new("p", &cfg).is_err());
    }
}
