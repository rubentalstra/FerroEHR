// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! The live HTTP driver.
//!
//! A [`crate::exec::StepDriver`] realized PURELY
//! from the operation bindings: request construction (method, path/query
//! templates, format headers, `Prefer`/`If-Match` discipline), wire
//! observation classification, capture extraction per the closed
//! capture-source grammar, and the assertion evaluators. Nothing here
//! hard-codes an endpoint: a case executes because its bindings say how.

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

use std::collections::BTreeMap;

use base64::Engine as _;
use reqwest::StatusCode;
use serde_json::Value;

use crate::artifacts::ArtifactSet;
use crate::exec::assertions::{self, AssertionFailure};
use crate::exec::outcome::{self, Observation};
use crate::exec::resolve::Resolver;
use crate::exec::state::{Captured, VarStore};
use crate::exec::{Provisioned, StepDriver, StepObservation};
use crate::ids::{CaptureName, SmOperationRef};
use crate::ixit::{AuthMode, Instance, Ixit};
use crate::model::assertion::{Assertion, EquivalentTarget, RowsSpec};
use crate::model::binding::{OperationBinding, RequestBody, StripRule, WireCapture, WireFrom};
use crate::model::case::{CaseCore, EhrRequirement, FlowStep, ImportRequirement};
use crate::refgrammar::{CaptureField, Template, ValueRef};
use crate::vocab::{FormatName, HttpMethod, MemberChangeType, MemberVersionType, OutcomeKind};

/// One captured HTTP exchange (also the transcript-recording seam).
#[derive(Debug, Clone)]
pub struct Exchange {
    /// The HTTP method the driver sent.
    pub method: String,
    /// The absolute request URL.
    pub path: String,
    /// The status code the SUT answered with.
    pub status: StatusCode,
    /// The response headers, lower-cased names.
    pub headers: BTreeMap<String, String>,
    /// The response body, parsed when it was JSON, else absent.
    pub body: Option<Value>,
}

/// The live driver.
pub struct HttpDriver<'a> {
    set: &'a ArtifactSet,
    ixit: &'a Ixit,
    /// The party statement's declared spec versions — the right-hand side of
    /// every `applies` floor consulted while driving (the version-dated
    /// header expectations). `None` on a statement-blind sweep.
    spec_versions: Option<&'a crate::party::SpecVersions>,
    client: reqwest::blocking::Client,
    resolver: Resolver<'a>,
    /// The payloads committed this row (the `equivalent to: committed`
    /// comparison source), newest last.
    committed: Vec<Value>,
    /// The last response body per row (postcondition target).
    last_body: Option<Value>,
    /// The latest `version_uid` a SUCCESS outcome's binding capture yielded
    /// this row — the comparison source of the `latest-version-uid` header
    /// matcher (overview §"If-Match and accidental overwrites": the 412
    /// "SHOULD return also latest `version_uid` in the `ETag`").
    last_version_uid: Option<String>,
    /// The `restapi_specs_version` member the System OPTIONS manifest served,
    /// when the campaign drove that exchange (released OAS
    /// `system.openapi.yaml` `Options` schema — every member optional). An
    /// independent CONFIRMATION of the party's declared
    /// `spec_versions.its_rest`, never a source of truth: no `required` list
    /// binds it, and a server could dodge every dated MUST by
    /// under-advertising — divergence surfaces as a static-review finding.
    observed_restapi_specs_version: Option<String>,
    /// Recorded exchanges (the transcript seam).
    pub exchanges: Vec<Exchange>,
}

impl std::fmt::Debug for HttpDriver<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpDriver")
            .field("exchanges", &self.exchanges.len())
            .finish_non_exhaustive()
    }
}

impl<'a> HttpDriver<'a> {
    /// Build a driver over the loaded artifact set and an ixit topology.
    ///
    /// # Errors
    /// A message when the artifact set lacks a corpus or the client cannot
    /// be constructed.
    pub fn new(
        set: &'a ArtifactSet,
        ixit: &'a Ixit,
        spec_versions: Option<&'a crate::party::SpecVersions>,
    ) -> Result<Self, String> {
        let (_, manifest) = set
            .corpus
            .as_ref()
            .ok_or_else(|| "artifact set has no corpus manifest".to_owned())?;
        let corpus_dir = set
            .corpus_dir
            .as_deref()
            .ok_or_else(|| "artifact set has no corpus directory".to_owned())?;
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("http client: {e}"))?;
        Ok(Self {
            set,
            ixit,
            spec_versions,
            client,
            resolver: Resolver::new(manifest, corpus_dir, Some(ixit)),
            committed: Vec::new(),
            last_body: None,
            last_version_uid: None,
            observed_restapi_specs_version: None,
            exchanges: Vec::new(),
        })
    }

    /// Take the System-manifest `restapi_specs_version` this driver observed,
    /// if the campaign drove that exchange (see the field NOTE).
    pub fn take_observed_restapi_specs_version(&mut self) -> Option<String> {
        self.observed_restapi_specs_version.take()
    }

    fn binding_for(&self, case: &CaseCore, call: &str) -> Result<&'a OperationBinding, String> {
        self.binding_for_variant(case, call, None)
    }

    /// Select the operation binding, honoring the flow step's `variant`
    /// discriminator: a step's variant selects the binding declaring that
    /// variant; a variant-less step (or a variant with no dedicated binding —
    /// e.g. header-mutation variants) falls back to the variant-less binding
    /// for the operation.
    fn binding_for_variant(
        &self,
        case: &CaseCore,
        call: &str,
        variant: Option<&str>,
    ) -> Result<&'a OperationBinding, String> {
        let op = if call.contains('.') {
            SmOperationRef::parse(call).map_err(|e| e.to_string())?
        } else {
            case.sm_operation
                .as_ref()
                .ok_or_else(|| format!("case {} has no sm_operation anchor", case.id))?
                .sibling(call)
        };
        let mut bindings = self.set.bindings.iter().map(|(_, b)| b);
        if let Some(v) = variant
            && let Some(exact) = bindings
                .clone()
                .find(|b| b.sm_operation == op && b.variant.as_deref() == Some(v))
        {
            return Ok(exact);
        }
        bindings
            .find(|b| b.sm_operation == op && b.variant.is_none())
            .ok_or_else(|| format!("no binding declares operation {op}"))
    }

    fn instance_for(&self, step: &FlowStep) -> Result<&'a Instance, String> {
        match &step.on {
            Some(name) => self
                .ixit
                .instance(name)
                .ok_or_else(|| format!("ixit declares no instance {name}")),
            None => self.ixit.default_instance(),
        }
    }

    /// The instance the case's PRECONDITIONS are established on — templates,
    /// the minted `${ehr_id}`, the directory tree, the commit sets.
    ///
    /// The default is `sut`. A flow that addresses an instance on a different
    /// ORIGIN is driving a different DEPLOYMENT, and a precondition
    /// established on `sut` simply would not exist there — so provisioning
    /// follows that instance. Same-origin instances are the same server seen
    /// through a different principal (`readonly`, `unauthenticated`,
    /// `smart_app`) or at a different base path (`smart_platform`), and keep
    /// provisioning on `sut` — which is the point: the ground is laid by the
    /// party's ordinary principal and only the flow exercises the other one.
    ///
    /// NOTE: no openEHR spec governs this — our own design/extension (test
    /// topology, ISO/IEC 9646 IXIT territory).
    fn provisioning_instance(&self, case: &CaseCore) -> Result<&'a Instance, String> {
        let default = self.ixit.default_instance()?;
        for step in &case.flow {
            if let Some(name) = &step.on
                && let Some(instance) = self.ixit.instance(name)
                && !same_deployment(&instance.base_url, &default.base_url)
            {
                return Ok(instance);
            }
        }
        Ok(default)
    }

    /// The `Authorization` header for an instance. `scopes` is the SMART
    /// `scope` claim the step declared (`None` = the step declared none), and
    /// is consumed only by the `bearer_mint` principal.
    #[expect(
        clippy::disallowed_methods,
        reason = "credentials are read from the environment BY DESIGN: the ixit \
                  declares only the variable NAME so no secret ever enters the \
                  catalogue; cnf-runner is a standalone instrument with no access \
                  to the server's config tree, which is what that ban protects"
    )]
    fn auth_header(
        ixit: &Ixit,
        auth: &AuthMode,
        scopes: Option<&[String]>,
    ) -> Result<Option<String>, String> {
        match auth {
            AuthMode::None => Ok(None),
            AuthMode::Basic {
                user_env,
                password_env,
            } => {
                let user = std::env::var(user_env)
                    .map_err(|error| format!("credential env {user_env}: {error}"))?;
                let pass = std::env::var(password_env)
                    .map_err(|error| format!("credential env {password_env}: {error}"))?;
                let token = base64::engine::general_purpose::STANDARD
                    .encode(format!("{user}:{pass}").as_bytes());
                Ok(Some(format!("Basic {token}")))
            }
            AuthMode::Bearer { token_env } => {
                let token = std::env::var(token_env)
                    .map_err(|error| format!("credential env {token_env}: {error}"))?;
                Ok(Some(format!("Bearer {token}")))
            }
            AuthMode::BearerMint {
                subject,
                roles,
                default_scopes,
            } => {
                let lane = ixit.smart.as_ref().ok_or_else(|| {
                    "instance declares auth mode `bearer_mint` but the ixit declares no `smart` \
                     lane to mint against"
                        .to_owned()
                })?;
                // A step-level `scopes:` always wins (the SMART cases probe
                // exact grants); a plain catalogue step rides the instance's
                // standing grant.
                let token = mint_access_token(
                    &lane.mint,
                    subject.as_deref(),
                    roles.as_deref(),
                    scopes.unwrap_or(default_scopes),
                )?;
                Ok(Some(format!("Bearer {token}")))
            }
        }
    }

    fn media_type(format: FormatName) -> &'static str {
        match format {
            FormatName::CanonicalJson => "application/json",
            FormatName::CanonicalXml => "application/xml",
            FormatName::WtFlat => "application/openehr.wt.flat+json",
            FormatName::WtStructured => "application/openehr.wt.structured+json",
            FormatName::Wt => "application/openehr.wt+json",
        }
    }

    /// Resolve the request path + query from binding templates and the
    /// step's resolved `with` values / captures.
    fn build_url(
        binding: &OperationBinding,
        base: &str,
        with: &BTreeMap<String, Value>,
        vars: &VarStore,
    ) -> Result<String, String> {
        let request = binding
            .request
            .as_ref()
            .ok_or_else(|| "binding is unrealized".to_owned())?;
        let mut path = String::new();
        let raw = request.path.raw();
        let mut rest = raw;
        while let Some(start) = rest.find('{') {
            let (head, tail) = rest.split_at(start);
            path.push_str(head);
            let end = tail
                .find('}')
                .ok_or_else(|| format!("path {raw}: unterminated param"))?;
            let name = tail.get(1..end).unwrap_or_default();
            let value = with
                .get(name)
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .or_else(|| {
                    CaptureName::parse(name)
                        .ok()
                        .and_then(|c| vars.scalar(&c).map(ToOwned::to_owned))
                })
                .ok_or_else(|| format!("path param {{{name}}} unresolved"))?;
            path.push_str(&urlencoding::encode(&value));
            rest = tail.get(end + 1..).unwrap_or_default();
        }
        path.push_str(rest);

        let mut url = format!("{base}{path}");
        if let Some(query) = &request.query {
            let mut params: Vec<(String, String)> = Vec::new();
            for (name, value) in query {
                for template in value.templates() {
                    // A member bound to a LIST capture expands element-wise:
                    // the repeated form's whole point is one pair per value
                    // (RFC 6570 `{?p*}`). Only a repeated declaration may
                    // expand — a single-valued parameter stays single.
                    if value.is_repeated()
                        && let Some(items) = list_capture_items(template, vars)
                    {
                        for item in items {
                            params.push((name.clone(), item));
                        }
                        continue;
                    }
                    match assertions::render_template(template, vars) {
                        Ok(rendered) => params.push((name.clone(), rendered)),
                        // An optional ref that is genuinely UNBOUND omits the
                        // parameter — but a name that IS bound (in the step's
                        // `with:` or as an earlier capture in the var store)
                        // and does not render as a scalar is a case-authoring
                        // or capture-shape defect and must be loud, never a
                        // silent drop that masquerades as a SUT failure.
                        Err(e) if template_is_optional(template) => {
                            if let Some(referenced) = template_ref_name(template)
                                && (with.contains_key(referenced)
                                    || CaptureName::parse(referenced)
                                        .is_ok_and(|n| vars.get(&n).is_some()))
                            {
                                return Err(format!(
                                    "query {name}: the optional ref ${{{referenced}?}} is bound \
                                     but did not render as a scalar: {e}"
                                ));
                            }
                        }
                        Err(e) => return Err(format!("query {name}: {e}")),
                    }
                }
            }
            // `with` keys that match query names override/backfill
            for (name, declared) in query {
                if let Some(v) = with.get(name)
                    && !params.iter().any(|(n, _)| n == name)
                    && !v.is_null()
                {
                    match v {
                        Value::String(s) => params.push((name.clone(), s.clone())),
                        // A backfilled ARRAY is the repeated form and only
                        // that: JSON-encoding it into one pair would send
                        // `?p=%5B%22a%22%5D`, a value no released parameter
                        // grammar defines.
                        Value::Array(items) if declared.is_repeated() => {
                            for item in items {
                                params.push((name.clone(), scalar_text(item)?));
                            }
                        }
                        Value::Array(_) => {
                            return Err(format!(
                                "query {name}: the step's `with:` binds a list where the binding \
                                 declares a single-valued parameter — repeatability is the \
                                 binding's declaration, not the case's"
                            ));
                        }
                        Value::Object(_) => {
                            return Err(format!(
                                "query {name}: the step's `with:` binds an object; a query \
                                 parameter value is a scalar"
                            ));
                        }
                        other => params.push((name.clone(), other.to_string())),
                    }
                }
            }
            if !params.is_empty() {
                url.push('?');
                let encoded: Vec<String> = params
                    .iter()
                    .map(|(n, v)| format!("{n}={}", urlencoding::encode(v)))
                    .collect();
                url.push_str(&encoded.join("&"));
            }
        }
        Ok(url)
    }

    /// Perform the exchange and record it.
    fn send(
        &mut self,
        method: HttpMethod,
        url: &str,
        headers: &BTreeMap<String, String>,
        body: Option<&Value>,
        body_is_json: bool,
    ) -> Result<Exchange, String> {
        let m = match method {
            HttpMethod::Get => reqwest::Method::GET,
            HttpMethod::Post => reqwest::Method::POST,
            HttpMethod::Put => reqwest::Method::PUT,
            HttpMethod::Delete => reqwest::Method::DELETE,
            HttpMethod::Head => reqwest::Method::HEAD,
            HttpMethod::Options => reqwest::Method::OPTIONS,
        };
        let mut request = self.client.request(m, url);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        if let Some(payload) = body {
            request = if body_is_json {
                request.body(serde_json::to_vec(payload).map_err(|e| e.to_string())?)
            } else {
                match payload {
                    Value::String(text) => request.body(text.clone()),
                    other => request.body(serde_json::to_vec(other).map_err(|e| e.to_string())?),
                }
            };
        }
        let response = request.send().map_err(|e| format!("transport: {e}"))?;
        let status = response.status();
        let mut response_headers = BTreeMap::new();
        for (name, value) in response.headers() {
            if let Ok(v) = value.to_str() {
                response_headers.insert(name.as_str().to_owned(), v.to_owned());
            }
        }
        let text = response.text().map_err(|e| format!("transport: {e}"))?;
        let body = if text.is_empty() {
            None
        } else {
            serde_json::from_str(&text)
                .ok()
                .or(Some(Value::String(text)))
        };
        let exchange = Exchange {
            method: format!("{method:?}").to_uppercase(),
            path: url.to_owned(),
            status,
            headers: response_headers,
            body,
        };
        // CNF_DEBUG_EXCHANGES=1: dump every wire exchange to stderr (live
        // triage aid; the transcript seam is the durable record).
        #[expect(
            clippy::disallowed_methods,
            reason = "the instrument's own debug switch, not server configuration: \
                      cnf-runner is a standalone tool with no config tree"
        )]
        let debug_exchanges = std::env::var_os("CNF_DEBUG_EXCHANGES").is_some();
        if debug_exchanges {
            #[expect(
                clippy::print_stderr,
                reason = "env-gated triage output in the dev tool"
            )]
            {
                eprintln!(
                    "[exchange] {} {} -> {} | {}",
                    exchange.method,
                    exchange.path,
                    exchange.status.as_u16(),
                    exchange
                        .body
                        .as_ref()
                        .map(|b| b.to_string().chars().take(120).collect::<String>())
                        .unwrap_or_default()
                );
            }
        }
        self.exchanges.push(exchange.clone());
        Ok(exchange)
    }

    fn extract_capture(
        exchange: &Exchange,
        binding: &OperationBinding,
        spec: &WireCapture,
        vars: &VarStore,
    ) -> Option<String> {
        let from_source = |source: &WireFrom| -> Option<String> {
            match source {
                WireFrom::Header { name, last_segment } => {
                    let value = exchange
                        .headers
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case(name))
                        .map(|(_, v)| v.clone())?;
                    if *last_segment {
                        value.rsplit('/').next().map(ToOwned::to_owned)
                    } else {
                        Some(value)
                    }
                }
                WireFrom::Body { path } => {
                    let body = exchange.body.as_ref()?;
                    // dotted body paths with optional array indices
                    // (`ehr_id.value`, `versions[0].id.value`).
                    let mut current = body;
                    for seg in path.split('.') {
                        let (attr, index) = match seg.split_once('[') {
                            Some((a, rest)) => (
                                a,
                                rest.strip_suffix(']').and_then(|i| i.parse::<usize>().ok()),
                            ),
                            None => (seg, None),
                        };
                        if !attr.is_empty() {
                            current = current.get(attr)?;
                        }
                        if let Some(i) = index {
                            current = current.get(i)?;
                        }
                    }
                    match current {
                        Value::String(s) => Some(s.clone()),
                        other => Some(other.to_string()),
                    }
                }
                // A capture-typed source reads a case-bound variable when one
                // exists; otherwise it derives the referenced BINDING capture
                // from this same exchange (a case need not name every
                // intermediate — `versioned_object_uid: from capture
                // version_uid` works without the case capturing version_uid).
                WireFrom::Capture(name) => vars.scalar(name).map(ToOwned::to_owned).or_else(|| {
                    let referenced = binding
                        .captures
                        .as_deref()
                        .unwrap_or_default()
                        .iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, s)| s)?;
                    // one referencing level only — a chained reference would
                    // recurse here; the artifact gates keep chains shallow
                    if matches!(referenced.from, WireFrom::Capture(_)) {
                        return None;
                    }
                    Self::extract_capture(exchange, binding, referenced, vars)
                }),
            }
        };
        let mut value =
            from_source(&spec.from).or_else(|| spec.fallback.as_ref().and_then(from_source))?;
        if matches!(spec.strip, Some(StripRule::WeakQuotes)) {
            value = value.trim_start_matches("W/").trim_matches('"').to_owned();
        }
        // A transform that finds no such component yields NO capture — a
        // truncated identifier must leave the capture unbound (loud at its
        // use site), never bind the untransformed value.
        if let Some(transform) = spec.transform {
            value = transform.apply(&value)?;
        }
        Some(value)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the assertion's field set"
    )]
    fn eval_field_assertion(
        &mut self,
        body: &Value,
        path: &str,
        equals: Option<&crate::model::value::TemplatedValue>,
        not_equals: Option<&crate::model::value::TemplatedValue>,
        exists: Option<bool>,
        absent: Option<bool>,
        matches: Option<&str>,
        vars: &VarStore,
    ) -> Result<(), AssertionFailure> {
        let mut resolve =
            |tv: Option<&crate::model::value::TemplatedValue>| -> Result<Option<Value>, String> {
                tv.map(|v| {
                    self.resolver
                        .resolve_value(v, vars)
                        .map_err(|e| e.to_string())
                })
                .transpose()
            };
        let eq_r = resolve(equals);
        let neq_r = resolve(not_equals);
        match (eq_r, neq_r) {
            (Ok(eq), Ok(neq)) => assertions::eval_field(
                body,
                path,
                eq.as_ref(),
                neq.as_ref(),
                exists,
                absent,
                matches,
            ),
            (Err(e), _) | (_, Err(e)) => Err(AssertionFailure(e)),
        }
    }

    /// Evaluate a `signature` assertion against a read-back `ORIGINAL_VERSION`
    /// envelope (the version-envelope read step's body). `present`/`equals` are
    /// mode-agnostic; `verifiable` reconstructs the agreed canonical form and
    /// verifies per `signing` — the posture of the INSTANCE the step ran on
    /// (RM common master06 §Digital Signature; [`crate::exec::signature`]).
    #[expect(
        clippy::too_many_arguments,
        reason = "one parameter per declared signature fact — mirrors the assertion shape"
    )]
    fn eval_signature_assertion(
        &mut self,
        body: &Value,
        present: Option<bool>,
        verifiable: Option<bool>,
        equals: Option<&crate::model::value::TemplatedValue>,
        distinct_from: Option<&crate::model::value::TemplatedValue>,
        signing: Option<&crate::exec::signature::SigningMode>,
        vars: &VarStore,
    ) -> Result<(), AssertionFailure> {
        let signature = body.get("signature").and_then(Value::as_str);
        if present == Some(true) && signature.is_none_or(str::is_empty) {
            return Err(AssertionFailure(
                "signature: expected present, the ORIGINAL_VERSION envelope carries no signature"
                    .into(),
            ));
        }
        if let Some(other) = distinct_from {
            // Distinct-signature-per-version: the canonical form the signature
            // is computed over includes `uid`, so two distinct versions carry
            // distinct signatures (RM common master06 §Digital Signature +
            // version.adoc `canonical_form`: all attributes except signature).
            // Both sides must be non-empty — an absent signature satisfies
            // nothing, and an empty comparand means the earlier capture failed.
            let want = self
                .resolver
                .resolve_value(other, vars)
                .map_err(|e| AssertionFailure(e.to_string()))?;
            let want = want
                .as_str()
                .map_or_else(|| want.to_string(), ToOwned::to_owned);
            let Some(sig) = signature.filter(|s| !s.is_empty()) else {
                return Err(AssertionFailure(
                    "signature: distinct_from requested but the envelope carries no signature"
                        .into(),
                ));
            };
            if want.is_empty() {
                return Err(AssertionFailure(
                    "signature: distinct_from comparand is empty (the earlier signature capture failed)"
                        .into(),
                ));
            }
            if sig == want {
                return Err(AssertionFailure(
                    "signature: identical to the compared version's signature — the signature must be a function of the version's canonical content (RM common master06 §Digital Signature)"
                        .into(),
                ));
            }
        }
        if let Some(expected) = equals {
            let want = self
                .resolver
                .resolve_value(expected, vars)
                .map_err(|e| AssertionFailure(e.to_string()))?;
            let want = want
                .as_str()
                .map_or_else(|| want.to_string(), ToOwned::to_owned);
            if signature != Some(want.as_str()) {
                return Err(AssertionFailure(format!(
                    "signature: stored {signature:?} is not the client-supplied {want:?} (must be stored verbatim)"
                )));
            }
        }
        if verifiable == Some(true) {
            let Some(sig) = signature else {
                return Err(AssertionFailure(
                    "signature: verifiable requested but the envelope carries no signature".into(),
                ));
            };
            let Some(mode) = signing else {
                return Err(AssertionFailure(
                    "signature: verifiable requested but the ixit declares no `signing` posture \
                     for the addressed instance"
                        .into(),
                ));
            };
            match crate::exec::signature::verify(body, sig, mode) {
                Ok(true) => {}
                Ok(false) => {
                    return Err(AssertionFailure(
                        "signature: does not verify over the agreed canonical form (RFC 8785 JCS of the version minus signature)"
                            .into(),
                    ));
                }
                Err(e) => return Err(AssertionFailure(format!("signature verify: {e}"))),
            }
        }
        Ok(())
    }

    /// A Boolean SM return (`has_directory`, `has_path`, `has_query`) is
    /// realized on the wire as presence: 2xx = TRUE, the mapped not-found =
    /// FALSE — the response body is the resource (or empty per `Prefer`),
    /// never a boolean literal (SM `openehr_platform` `I_EHR_DIRECTORY` /
    /// `I_DEFINITION_QUERY` `has_*`: Boolean; ITS-REST realizes them as GET).
    fn eval_returns_assertion(
        exchange: &Exchange,
        body: &Value,
        equals: Option<&Value>,
        matches: Option<&str>,
        omits: Option<&str>,
    ) -> Result<(), AssertionFailure> {
        if let Some(Value::Bool(want)) = equals {
            let observed = exchange.status.is_success();
            if observed == *want {
                Ok(())
            } else {
                Err(AssertionFailure(format!(
                    "returns: wire presence {observed} != expected {want} (status {})",
                    exchange.status.as_u16()
                )))
            }
        } else {
            assertions::eval_returns(body, equals, matches, omits)
        }
    }

    /// Evaluate the pure-side assertions for a step against the exchange.
    #[expect(
        clippy::too_many_lines,
        reason = "one match arm per Assertion variant — a dispatch, each arm delegates"
    )]
    fn eval_assertions(
        &mut self,
        _case: &CaseCore,
        binding: &OperationBinding,
        assertions_list: &[Assertion],
        exchange: &Exchange,
        signing: Option<&crate::exec::signature::SigningMode>,
        vars: &VarStore,
    ) -> Vec<String> {
        let ctx_defaults: Vec<String> = self
            .set
            .selectors
            .as_ref()
            .and_then(|(_, s)| {
                s.ignore_sets
                    .iter()
                    .find(|(k, _)| matches!(k.0, crate::vocab::IgnoreSetName::CtxDefaults))
                    .map(|(_, d)| d.paths.clone())
            })
            .unwrap_or_default();
        let mut failures = Vec::new();
        for assertion in assertions_list {
            let body = exchange.body.as_ref().unwrap_or(&Value::Null);
            let result: Result<(), AssertionFailure> = match assertion {
                Assertion::Field {
                    path,
                    equals,
                    not_equals,
                    exists,
                    absent,
                    matches,
                } => self.eval_field_assertion(
                    body,
                    path,
                    equals.as_ref(),
                    not_equals.as_ref(),
                    *exists,
                    *absent,
                    matches.as_deref(),
                    vars,
                ),
                Assertion::Equivalent { to, ignoring } => {
                    // An unresolvable REFERENCE is a defect of the case, not a
                    // missing commit (#1853): reporting both as "no committed
                    // payload" sent triage after the wrong artifact.
                    let expected = match to {
                        EquivalentTarget::Committed => Ok(self.committed.last().cloned()),
                        EquivalentTarget::Ref(r) => {
                            self.resolver.resolve_ref(r, vars).map(Some).map_err(|e| {
                                AssertionFailure(format!(
                                    "equivalent: target reference unresolvable ({e})"
                                ))
                            })
                        }
                    };
                    match expected {
                        Err(failure) => Err(failure),
                        Ok(None) => {
                            Err(AssertionFailure("equivalent: no committed payload".into()))
                        }
                        Ok(Some(expected)) => {
                            let ignored = assertions::resolve_ignore_sets(
                                &ignoring.0,
                                &binding.server_assigned,
                                &ctx_defaults,
                            );
                            if assertions::equivalent(body, &expected, &ignored) {
                                Ok(())
                            } else {
                                Err(equivalence_mismatch(body, &expected))
                            }
                        }
                    }
                }
                Assertion::Returns {
                    equals,
                    matches,
                    omits,
                } => Self::eval_returns_assertion(
                    exchange,
                    body,
                    equals.as_ref(),
                    matches.as_deref(),
                    omits.as_deref(),
                ),
                Assertion::ResultSet {
                    match_mode,
                    rows,
                    count,
                    columns,
                } => self.eval_result_set(
                    body,
                    *match_mode,
                    rows.as_ref(),
                    *count,
                    columns.as_deref(),
                    vars,
                ),
                Assertion::XmlRoot {
                    name,
                    namespace,
                    xsi_type,
                } => assertions::eval_xml_root(body, name, *namespace, xsi_type.as_deref()),
                Assertion::InstanceOf { rm_type, .. } => {
                    // Structural check: the body self-identifies as the type.
                    match body.get("_type").and_then(Value::as_str) {
                        Some(t) if t == rm_type => Ok(()),
                        Some(t) => Err(AssertionFailure(format!(
                            "instance_of: body is {t}, expected {rm_type}"
                        ))),
                        None => Err(AssertionFailure(format!(
                            "instance_of: body carries no _type (expected {rm_type})"
                        ))),
                    }
                }
                // The signature family is wire-asserted against the
                // ORIGINAL_VERSION envelope the case's own flow reads (the
                // version-envelope read step; RM common master06 §Digital
                // Signature). present/equals are mode-agnostic; verifiable
                // dispatches on the signing posture of the instance the step
                // ran on (instance-first, party default second).
                Assertion::Signature {
                    present,
                    verifiable,
                    equals,
                    distinct_from,
                    ..
                } => self.eval_signature_assertion(
                    body,
                    *present,
                    *verifiable,
                    equals.as_ref(),
                    distinct_from.as_ref(),
                    signing,
                    vars,
                ),
                // The version family still needs a versioned-object read the
                // ITS does not surface uniformly for change_type/lifecycle
                // (in-case verification carries them); unique is aggregate
                // (law e); message_exemplar/state are informative.
                Assertion::Version { .. }
                | Assertion::Unique { .. }
                | Assertion::MessageExemplar { .. }
                | Assertion::State { .. } => Ok(()),
            };
            if let Err(AssertionFailure(message)) = result {
                failures.push(message);
            }
        }
        failures
    }

    fn eval_result_set(
        &mut self,
        body: &Value,
        match_mode: crate::vocab::ResultSetMatch,
        rows: Option<&RowsSpec>,
        count: Option<u64>,
        columns: Option<&[crate::model::assertion::ColumnSpec]>,
        vars: &VarStore,
    ) -> Result<(), AssertionFailure> {
        use crate::exec::resultset;
        use crate::vocab::ResultSetMatch;
        if let Some(cols) = columns {
            let names: Vec<String> = cols.iter().map(|c| c.name.clone()).collect();
            resultset::compare_columns(body, &names).map_err(|e| AssertionFailure(e.0))?;
        }
        let expected_rows: Option<Vec<Value>> = match rows {
            Some(RowsSpec::Inline(rows)) => {
                Some(rows.iter().map(|r| Value::Array(r.clone())).collect())
            }
            Some(RowsSpec::From(r)) => {
                // A committed-uid selection spec: evaluate over the captured
                // uid list bound by the requires.commit provisioning.
                let spec = self
                    .resolver
                    .resolve_ref(r, vars)
                    .map_err(|e| AssertionFailure(e.to_string()))?;
                let uids = vars
                    .get(&committed_uids_handle())
                    .and_then(|c| match c {
                        Captured::List(items) => Some(items.clone()),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        AssertionFailure(
                            "result_set rows.from: no committed-set uids bound (requires.commit)"
                                .into(),
                        )
                    })?;
                let min = spec
                    .get("systolic_min")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                // bp series semantics: uid k has systolic 100+10k
                let mut selected: Vec<(String, u64)> = uids
                    .iter()
                    .enumerate()
                    .filter_map(|(k, uid)| {
                        #[expect(
                            clippy::as_conversions,
                            reason = "committed-uid index widens exactly: usize is at most 64 bits on every supported target"
                        )]
                        let systolic = 100 + 10 * (k as u64);
                        (systolic >= min).then(|| (uid.clone(), systolic))
                    })
                    .collect();
                match spec.get("order").and_then(Value::as_str) {
                    Some("systolic_desc") => {
                        selected.sort_by(|(ua, sa), (ub, sb)| sb.cmp(sa).then(ua.cmp(ub)));
                    }
                    _ => selected.sort_by(|(a, _), (b, _)| a.cmp(b)),
                }
                if let Some(limit) = spec.get("limit").and_then(Value::as_u64) {
                    selected.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
                }
                Some(
                    selected
                        .into_iter()
                        .map(|(uid, _)| Value::Array(vec![Value::String(uid)]))
                        .collect(),
                )
            }
            None => None,
        };
        let outcome = match (match_mode, expected_rows, count) {
            (ResultSetMatch::Count, _, Some(n)) => resultset::compare_count(body, n),
            (ResultSetMatch::Ordered, Some(rows), _) => resultset::compare_ordered(body, &rows),
            (ResultSetMatch::Set, Some(rows), _) => resultset::compare_bag(body, &rows),
            (ResultSetMatch::Contains, Some(rows), _) => resultset::compare_contains(body, &rows),
            _ => {
                return Err(AssertionFailure(
                    "result_set: no comparable expectation resolved".into(),
                ));
            }
        };
        outcome.map_err(|e| AssertionFailure(e.0))
    }
}

/// Whether two ixit `base_url`s address the SAME deployment: same scheme,
/// host and effective port, whatever API path follows. A party declares
/// several instances per deployment (different principals, and the SMART
/// Platform base path), and exactly one instance per EXTRA deployment (the
/// second signing posture) — the origin is what tells those two cases apart.
/// A value neither side can parse falls back to exact comparison rather than
/// guessing (`Url::origin` is not usable here: for a non-special scheme it is
/// opaque and unequal even to itself).
fn same_deployment(left: &str, right: &str) -> bool {
    match (reqwest::Url::parse(left), reqwest::Url::parse(right)) {
        (Ok(left), Ok(right)) => {
            left.scheme() == right.scheme()
                && left.host_str() == right.host_str()
                && left.port_or_known_default() == right.port_or_known_default()
        }
        _ => left == right,
    }
}

/// The capture handle the provisioning binds committed-set uids under.
#[expect(
    clippy::expect_used,
    reason = "the sanctioned logically-impossible-Err escape: `committed_uids` is a \
              hardcoded literal satisfying CaptureName's grammar, so the parse \
              cannot fail — and a silent fallback name would mis-bind the \
              provisioning contract instead of failing loudly"
)]
fn committed_uids_handle() -> CaptureName {
    CaptureName::parse("committed_uids").expect("`committed_uids` should be a valid capture name")
}

fn template_is_optional(template: &Template) -> bool {
    template
        .as_single_ref()
        .is_some_and(|r| matches!(r, ValueRef::Capture { optional: true, .. }))
}

/// The items of a template that is exactly one capture reference bound to a
/// LIST capture — the expansion source of a repeated query parameter.
fn list_capture_items(template: &Template, vars: &VarStore) -> Option<Vec<String>> {
    let ValueRef::Capture { name, .. } = template.as_single_ref()? else {
        return None;
    };
    match vars.get(name) {
        Some(Captured::List(items)) => Some(items.clone()),
        _ => None,
    }
}

/// One query-parameter value's wire text. Only scalars have one.
fn scalar_text(value: &Value) -> Result<String, String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Number(_) | Value::Bool(_) => Ok(value.to_string()),
        other => Err(format!(
            "a query parameter value is a scalar, not a {}",
            json_shape(other)
        )),
    }
}

/// The capture name an optional single-ref template references (`${name?}`
/// → `name`), for the bound-but-unrendered diagnostic.
fn template_ref_name(template: &Template) -> Option<&str> {
    template.as_single_ref().and_then(|r| match r {
        ValueRef::Capture { name, .. } => Some(name.as_str()),
        _ => None,
    })
}

/// The equivalence-failure diagnostic: the first differing paths (path, got,
/// want), so a red row carries triage-usable evidence — two 80-char head
/// previews forced the 2026-07-28 composition-XML triage to reconstruct the
/// diff offline from the codec instead of reading it from results.json.
fn equivalence_mismatch(body: &Value, expected: &Value) -> AssertionFailure {
    let mut diffs: Vec<String> = Vec::new();
    diff_paths(body, expected, "$", &mut diffs);
    let shown = 6;
    let suffix = if diffs.len() > shown {
        format!(" … and {} more differing path(s)", diffs.len() - shown)
    } else {
        String::new()
    };
    AssertionFailure(format!(
        "equivalent: retrieved content differs from committed (modulo the normative ignore-set); {}{}",
        diffs
            .iter()
            .take(shown)
            .cloned()
            .collect::<Vec<_>>()
            .join("; "),
        suffix
    ))
}

/// Collect the paths where two JSON trees differ (leaf previews truncated,
/// structure-first): missing keys, length mismatches, and unequal leaves.
fn diff_paths(got: &Value, want: &Value, path: &str, out: &mut Vec<String>) {
    let brief = |v: &Value| {
        let s = v.to_string();
        if s.chars().count() > 60 {
            let head: String = s.chars().take(60).collect();
            format!("{head}…")
        } else {
            s
        }
    };
    match (got, want) {
        (Value::Object(x), Value::Object(y)) => {
            // `_type` presence on one side only is tolerated by the
            // comparator (assertions::rm_cells_equal) — keep the diagnostic
            // aligned so a red row never lists only tolerated diffs.
            for (k, vw) in y {
                match x.get(k) {
                    Some(vg) => diff_paths(vg, vw, &format!("{path}/{k}"), out),
                    None if k == "_type" => {}
                    None => out.push(format!(
                        "{path}/{k}: absent in retrieved (want {})",
                        brief(vw)
                    )),
                }
            }
            for (k, vg) in x {
                if !y.contains_key(k) && k != "_type" {
                    out.push(format!("{path}/{k}: surplus in retrieved ({})", brief(vg)));
                }
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                out.push(format!("{path}: array length {} vs {}", x.len(), y.len()));
            }
            for (i, (vg, vw)) in x.iter().zip(y).enumerate() {
                diff_paths(vg, vw, &format!("{path}[{i}]"), out);
            }
        }
        _ => {
            if !crate::exec::resultset::cells_equal(got, want) {
                out.push(format!("{path}: got {} want {}", brief(got), brief(want)));
            }
        }
    }
}

/// Parse an IMF-fixdate `Date` header ("Sun, 06 Nov 1994 08:49:37 GMT") to
/// epoch milliseconds (RFC 9110 §5.6.7); `None` on any other form.
///
/// NOTE: every `None` here is ABSENCE, not a swallowed defect (#1853) — the
/// caller only WIDENS its own commit window with this value, so an unparsable
/// `Date` leaves the runner-clock window standing rather than skipping a check.
fn parse_http_date_ms(value: &str) -> Option<i64> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    let [_, day, month, year, time, zone] = parts.as_slice() else {
        return None;
    };
    if !zone.eq_ignore_ascii_case("GMT") {
        return None;
    }
    let month = match *month {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let day: i64 = day.parse().ok()?;
    let year: i64 = year.parse().ok()?;
    let mut hms = time.split(':');
    let h: i64 = hms.next()?.parse().ok()?;
    let m: i64 = hms.next()?.parse().ok()?;
    let s: i64 = hms.next()?.parse().ok()?;
    // days since the epoch (civil-from-days inverse; Howard Hinnant's
    // published algorithm — pure integer arithmetic)
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    #[expect(
        clippy::integer_division,
        reason = "Hinnant's days-from-civil is DEFINED in exact integer (floor) \
                  division; a float step would break the calendar identity"
    )]
    let doy = (153 * mp + 2) / 5 + day - 1;
    #[expect(
        clippy::integer_division,
        reason = "Hinnant's days-from-civil is DEFINED in exact integer (floor) \
                  division; a float step would break the calendar identity"
    )]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some((days * 86_400 + h * 3_600 + m * 60 + s) * 1_000)
}

/// Runner-clock milliseconds since the Unix epoch.
///
/// Wall-clock time comes from `jiff`, the pinned time library
///; elapsed-time measurement uses
/// [`std::time::Instant`] instead.
pub(crate) fn now_ms() -> i64 {
    jiff::Timestamp::now().as_millisecond()
}

/// Mint one RS256 access token against the party's declared static test
/// issuer, carrying the step's SMART `scope` claim.
///
/// The CDR is a SMART **resource server** — it validates presented tokens and
/// never issues them (ITS-REST
/// `docs/smart_app_launch/master06-authentication.adoc` §Supported
/// Authentication Flows; token issuance is the Authorization Server's duty) —
/// so the conformance stack runs no Authorization Server and the driver takes
/// that role for the SMART lane only. The token is deliberately minimal: the
/// registered `iss`/`aud`/`sub`/`iat`/`exp` claims, the space-delimited
/// `scope` claim master08 §Resource Scopes defines, and the RBAC role claim
/// the SUT mines (the SMART gate AND-composes onto RBAC, so a role-less token
/// would be refused a layer earlier and prove nothing about SMART).
pub(crate) fn mint_access_token(
    mint: &crate::ixit::BearerMint,
    subject: Option<&str>,
    roles: Option<&[String]>,
    scopes: &[String],
) -> Result<String, String> {
    let pem = std::fs::read(&mint.key_file).map_err(|e| {
        format!(
            "smart mint: cannot read key file {}: {e}",
            mint.key_file.display()
        )
    })?;
    let key = jsonwebtoken::EncodingKey::from_rsa_pem(&pem)
        .map_err(|e| format!("smart mint: key file is not an RSA PEM: {e}"))?;
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(mint.kid.clone());

    let issued_at = jiff::Timestamp::now().as_second();
    let expires_at = issued_at.saturating_add(i64::try_from(mint.ttl_seconds).unwrap_or(i64::MAX));
    let mut claims = serde_json::Map::new();
    claims.insert("iss".to_owned(), Value::String(mint.issuer.clone()));
    if let Some(audience) = &mint.audience {
        claims.insert("aud".to_owned(), Value::String(audience.clone()));
    }
    claims.insert(
        "sub".to_owned(),
        Value::String(subject.unwrap_or(&mint.subject).to_owned()),
    );
    claims.insert("iat".to_owned(), Value::from(issued_at));
    claims.insert("exp".to_owned(), Value::from(expires_at));
    // master08 §Resource Scopes: scopes ride the OAuth 2.0 `scope` claim,
    // space-delimited (RFC 6749 §3.3). An empty declaration mints the claim
    // as an empty string — the scope-less token the fail-closed deny branch
    // needs, distinct from a token with no `scope` claim at all.
    claims.insert("scope".to_owned(), Value::String(scopes.join(" ")));
    claims.insert(
        "realm_access".to_owned(),
        serde_json::json!({ "roles": roles.unwrap_or(&mint.roles) }),
    );

    jsonwebtoken::encode(&header, &claims, &key)
        .map_err(|e| format!("smart mint: cannot sign the access token: {e}"))
}

/// The JSON shape name of a value, for diagnostics that must say WHAT was
/// captured instead of the expected object (a canonical-XML capture, for
/// instance, resolves as a string).
/// The shape name of a non-body capture, for the patched-body diagnostic.
fn json_shape_of_captured(captured: &Captured) -> &'static str {
    match captured {
        Captured::Scalar(_) => "a scalar",
        Captured::List(_) => "a list",
        Captured::Body(_) => "a body",
        Captured::InstantMs { .. } => "an instant",
    }
}

fn json_shape(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// The attestation change kind, spelled the one way it is spelled everywhere:
/// the case-level `change_type` token, the member key carrying the
/// `UPDATE_ATTESTATION` payload, and — because they coincide — the openEHR
/// rubric of code 666 (TERM `SupportTerminology`, audit change type:
/// `<concept id="666" rubric="attestation"/>`).
///
/// RM common `master06-change_control_package.adoc` §Contributions:
/// "attestation of item: a new `ATTESTATION` is added to the attestations list
/// of an existing `ORIGINAL_VERSION`; the `ATTESTATION.commit_audit.change_type`
/// is set to the code `666|attestation|`".
const ATTESTATION_TOKEN: &str = "attestation";
/// The `audit_change_type` code the token above names (`666|attestation|`).
const ATTESTATION_CODE: &str = "666";

/// The reserved sentinel a case writes in place of a value to say "omit this
/// member entirely" — the same word the parameter matrix reserves
/// ([`crate::model::case::MatrixCell::Absent`]).
const ABSENT_SENTINEL: &str = "absent";

impl HttpDriver<'_> {
    /// An openEHR `DV_CODED_TEXT` over the `openehr` terminology.
    fn openehr_coded_text(code: &str, rubric: &str) -> Value {
        serde_json::json!({
            "_type": "DV_CODED_TEXT", "value": rubric,
            "defining_code": { "_type": "CODE_PHRASE",
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                "code_string": code }
        })
    }

    /// The `preceding_version_uid` a CONTRIBUTION member names, if any. A
    /// list-valued capture (`created.version_uids[]`) addresses its single
    /// member on a one-version commit set.
    fn member_preceding(member: &Value) -> Option<Value> {
        match member.get("preceding_version_uid") {
            Some(Value::Array(items)) => items.first().cloned(),
            Some(Value::Null) | None => None,
            Some(other) => Some(other.clone()),
        }
    }

    /// A `666|attestation|` CONTRIBUTION member: an `ATTESTATION` attached to
    /// an EXISTING `ORIGINAL_VERSION`.
    ///
    /// RM common `master06-change_control_package.adoc` §Contributions makes
    /// this member a change that commits NO new version — "all logical changes
    /// … are achieved by physically committing new Versions, **or for
    /// attestations, new Attestation objects to existing Versions**" — so it
    /// carries neither `data` nor a version `lifecycle_state`; it names its
    /// target with `preceding_version_uid` and its `commit_audit` IS the
    /// `UPDATE_ATTESTATION` (ITS-REST `specifications/schemas/common/
    /// UpdateAttestation.yaml`: `UPDATE_AUDIT` + `reason` + `is_pending`,
    /// both required), supplied verbatim by the case's corpus fixture so an
    /// invalid attestation shape reaches the wire unrepaired. The runner fills
    /// only the `UPDATE_AUDIT` parts a client always sends and the fixture
    /// does not state.
    fn attestation_member(member: &Value) -> Value {
        let mut audit = member
            .get(ATTESTATION_TOKEN)
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        audit
            .entry("_type")
            .or_insert_with(|| Value::String("UPDATE_ATTESTATION".to_owned()));
        audit
            .entry("system_id")
            .or_insert_with(|| Value::String("cnf-runner".to_owned()));
        audit.entry("committer").or_insert_with(
            || serde_json::json!({ "_type": "PARTY_IDENTIFIED", "name": "cnf runner" }),
        );
        audit
            .entry("change_type")
            .or_insert_with(|| Self::openehr_coded_text(ATTESTATION_CODE, ATTESTATION_TOKEN));
        let mut version = serde_json::json!({
            "_type": "ORIGINAL_VERSION",
            "commit_audit": Value::Object(audit)
        });
        if let Some(preceding) = Self::member_preceding(member)
            && let Some(map) = version.as_object_mut()
        {
            map.insert(
                "preceding_version_uid".to_owned(),
                serde_json::json!({ "_type": "OBJECT_VERSION_ID", "value": preceding }),
            );
        }
        version
    }

    /// Build the canonical CONTRIBUTION envelope from the case model's
    /// bundled `versions:` construct (ITS contribution schema:
    /// `ORIGINAL_VERSION` members carrying `data` + `commit_audit` +
    /// `lifecycle_state`; `change_type` tokens map to the openEHR audit
    /// change-type codes — RM common `§change_control`).
    ///
    /// Two member keys OVERRIDE what the envelope would otherwise derive, each
    /// a closed vocabulary so an unauthorable value cannot be spelled:
    /// `_type` ([`crate::vocab::MemberVersionType`]) fixes the member's own
    /// class self-tag, which ITS-REST `docs/overview/Resources.md` §Resource
    /// representation permits and the AMB-89 refusal branch needs; and
    /// `lifecycle_state` ([`crate::vocab::VersionLifecycleState`]) fixes the
    /// committed version lifecycle independently of the change kind, which the
    /// master06 §Version Lifecycle transitions need (`incomplete` →
    /// `abandoned` is a `modification` whose STATE is the point). Both are
    /// generated into the same member the envelope builds, so a lifecycle case
    /// no longer has to author the whole `ORIGINAL_VERSION` verbatim.
    ///
    /// # Errors
    /// A message naming the offending member when an override token is outside
    /// its closed vocabulary — never a silent fallback to the derived value,
    /// which would commit a version the case did not ask for.
    fn contribution_envelope(
        versions: &[Value],
        commit_audit: Option<&Value>,
    ) -> Result<Value, String> {
        // The envelope audit's aggregate change type. RM common
        // `master06-change_control_package.adoc` §Contributions fixes the
        // attestation-only value verbatim — "`666|attestation|`: used when the
        // only changes are attestation of one or more of the member versions"
        // — so an all-attestation change set reports 666 rather than claiming a
        // creation nothing performed. Every other combination keeps the
        // creation default.
        let all_attestations = !versions.is_empty()
            && versions.iter().all(|member| {
                Self::member_change_type(member) == Ok(MemberChangeType::Attestation)
            });
        let members: Vec<Value> = versions
            .iter()
            .map(Self::contribution_member)
            .collect::<Result<Vec<Value>, String>>()?;
        let aggregate = if all_attestations {
            Self::openehr_coded_text(ATTESTATION_CODE, ATTESTATION_TOKEN)
        } else {
            Self::openehr_coded_text(
                MemberChangeType::Creation.code(),
                MemberChangeType::Creation.token(),
            )
        };
        let mut audit = serde_json::json!({
            "_type": "AUDIT_DETAILS",
            "system_id": "cnf-runner",
            "committer": { "_type": "PARTY_IDENTIFIED", "name": "cnf runner" },
            "change_type": aggregate
        });
        Self::apply_commit_audit_override(&mut audit, commit_audit)?;
        Ok(serde_json::json!({
            "_type": "CONTRIBUTION",
            "versions": members,
            "audit": audit
        }))
    }

    /// Apply a case's `audit:` override onto the derived commit audit.
    ///
    /// The derived envelope audit is what a conformant commit carries, so a
    /// case that is ABOUT the audit states only its delta: each key the
    /// override names replaces the derived value verbatim, and the reserved
    /// `absent` sentinel OMITS the key entirely. Omission is what the
    /// mandatory-member refusals need — RM common
    /// `UML/classes/org.openehr.rm.common.audit_details.adoc` §Attributes
    /// makes `change_type` and `committer` 1..1, and the released OAS
    /// `specifications/schemas/common/UpdateAudit.yaml` §required lists both
    /// on the commit DTO — and a verbatim value is what an out-of-group
    /// `change_type` code needs, since the closed vocabulary cannot spell one
    /// (§Invariants `Change_type_valid`).
    ///
    /// # Errors
    /// A message when the override is not an object — never a silent ignore,
    /// which would send the derived audit a case deliberately altered.
    fn apply_commit_audit_override(
        audit: &mut Value,
        commit_audit: Option<&Value>,
    ) -> Result<(), String> {
        let Some(commit_audit) = commit_audit else {
            return Ok(());
        };
        let Some(overrides) = commit_audit.as_object() else {
            return Err(format!(
                "`audit:` must be an AUDIT_DETAILS object stating the members to override \
                 (or `absent` to omit one), got {commit_audit}"
            ));
        };
        let Some(target) = audit.as_object_mut() else {
            return Err("the derived commit audit is not an object".to_owned());
        };
        for (key, value) in overrides {
            if value.as_str() == Some(ABSENT_SENTINEL) {
                target.remove(key);
            } else {
                target.insert(key.clone(), value.clone());
            }
        }
        Ok(())
    }

    /// One CONTRIBUTION member of the bundled `versions:` construct, in its
    /// wire shape.
    ///
    /// # Errors
    /// A message when an override token is outside its closed vocabulary or
    /// contradicts the member form (see [`Self::contribution_envelope`]).
    fn contribution_member(member: &Value) -> Result<Value, String> {
        let member_type = Self::member_type_override(member)?;
        let member_lifecycle = Self::member_lifecycle_override(member)?;
        // A pre-built ORIGINAL_VERSION member (e.g. the signed-version
        // fixture carrying a client-supplied VERSION.signature — RM
        // common §change_control) passes through verbatim: it already
        // IS the wire shape.
        if member
            .get("data")
            .and_then(|d| d.get("_type"))
            .and_then(Value::as_str)
            == Some("ORIGINAL_VERSION")
        {
            if member_type.is_some() || member_lifecycle.is_some() {
                return Err(
                    "a verbatim ORIGINAL_VERSION member already spells its own `_type` \
                         and `lifecycle_state`; a member-level override beside it would \
                         state the shape twice"
                        .to_owned(),
                );
            }
            return Ok(member.get("data").cloned().unwrap_or(Value::Null));
        }
        let change = Self::member_change_type(member)?;
        if change == MemberChangeType::Attestation {
            if member_lifecycle.is_some() {
                // master06 §Contributions: an attestation member
                // commits NO new version, so it carries no version
                // lifecycle to state.
                return Err(
                    "an attestation member commits no new version (RM common master06 \
                         §Contributions), so it carries no `lifecycle_state`"
                        .to_owned(),
                );
            }
            let mut version = Self::attestation_member(member);
            Self::apply_member_type(&mut version, member_type);
            return Ok(version);
        }
        let (code, label) = (change.code(), change.token());
        // a deleted member carries NO data and the `deleted`
        // lifecycle (RM common §change_control: version lifecycle
        // 523|deleted|); other members carry 532|complete| — unless
        // the case states the lifecycle itself, which is what the
        // master06 §Version Lifecycle transition cases do.
        let derived = if change == MemberChangeType::Deleted {
            crate::vocab::VersionLifecycleState::Deleted
        } else {
            crate::vocab::VersionLifecycleState::Complete
        };
        let lifecycle = member_lifecycle.unwrap_or(derived);
        let (life_code, life_label) = (lifecycle.code(), lifecycle.token());
        let mut version = serde_json::json!({
            "_type": "ORIGINAL_VERSION",
            "lifecycle_state": {
                "_type": "DV_CODED_TEXT",
                "value": life_label,
                "defining_code": { "_type": "CODE_PHRASE",
                    "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                    "code_string": life_code }
            },
            "commit_audit": {
                "_type": "AUDIT_DETAILS",
                "system_id": "cnf-runner",
                "committer": { "_type": "PARTY_IDENTIFIED", "name": "cnf runner" },
                "change_type": { "_type": "DV_CODED_TEXT", "value": label,
                    "defining_code": { "_type": "CODE_PHRASE",
                        "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                        "code_string": code } }
            }
        });
        // A case-supplied `commit_audit:` merges OVER the generated
        // one, attribute by attribute, so a case can state the commit
        // audit's concrete class and any attribute of it while the
        // runner still fills the parts every client always sends
        // (`system_id`, `committer`, and the `change_type` the
        // member's own token already fixed). The member's
        // `change_type` token stays authoritative: it is what the case
        // declares the change to BE.
        if let Some(supplied) = member.get("commit_audit").and_then(Value::as_object)
            && let Some(audit) = version
                .get_mut("commit_audit")
                .and_then(Value::as_object_mut)
        {
            for (key, value) in supplied {
                if key != "change_type" {
                    audit.insert(key.clone(), value.clone());
                }
            }
        }
        if let Some(data) = member.get("data")
            && !data.is_null()
            && let Some(map) = version.as_object_mut()
        {
            map.insert("data".to_owned(), data.clone());
        }
        if let Some(preceding) = Self::member_preceding(member)
            && let Some(map) = version.as_object_mut()
        {
            map.insert(
                "preceding_version_uid".to_owned(),
                serde_json::json!({
                    "_type": "OBJECT_VERSION_ID", "value": preceding
                }),
            );
        }
        Self::apply_member_type(&mut version, member_type);
        Ok(version)
    }

    /// The member's `change_type`, parsed against the closed
    /// `audit_change_type` group; absent means `249|creation|`, the change a
    /// first version records (RM common master06 §Contributions).
    ///
    /// # Errors
    /// A message when the token is not a member of the group — never the
    /// creation default, which would commit an audit the case never asked for.
    fn member_change_type(member: &Value) -> Result<MemberChangeType, String> {
        match member.get("change_type") {
            None => Ok(MemberChangeType::Creation),
            Some(Value::String(token)) => MemberChangeType::from_token(token).ok_or_else(|| {
                format!(
                    "CONTRIBUTION member `change_type: {token}` is not a member of the \
                         openEHR audit_change_type group ({})",
                    MemberChangeType::ALL
                        .iter()
                        .map(|c| c.token())
                        .collect::<Vec<_>>()
                        .join(" | ")
                )
            }),
            Some(other) => Err(format!(
                "CONTRIBUTION member `change_type` must be a change-type token, got {other}"
            )),
        }
    }

    /// The member's `_type` self-tag override, parsed against the closed
    /// [`crate::vocab::MemberVersionType`] vocabulary.
    ///
    /// # Errors
    /// A message when the value is not one of the RM `VERSION`-family class
    /// names a member may carry.
    fn member_type_override(member: &Value) -> Result<Option<MemberVersionType>, String> {
        match member.get("_type") {
            None => Ok(None),
            Some(Value::String(token)) => MemberVersionType::from_token(token)
                .map(Some)
                .ok_or_else(|| {
                    format!(
                        "CONTRIBUTION member `_type: {token}` is outside the closed member class \
                         vocabulary ({})",
                        MemberVersionType::ALL
                            .iter()
                            .map(|t| t.token())
                            .collect::<Vec<_>>()
                            .join(" | ")
                    )
                }),
            Some(other) => Err(format!(
                "CONTRIBUTION member `_type` must be a class name string, got {other}"
            )),
        }
    }

    /// The member's `lifecycle_state` override, parsed against the closed
    /// `version_lifecycle_state` group.
    ///
    /// # Errors
    /// A message when the value is not one of the five states RM common
    /// master06 §Version Lifecycle names.
    fn member_lifecycle_override(
        member: &Value,
    ) -> Result<Option<crate::vocab::VersionLifecycleState>, String> {
        match member.get("lifecycle_state") {
            None => Ok(None),
            Some(Value::String(token)) => crate::vocab::VersionLifecycleState::from_token(token)
                .map(Some)
                .ok_or_else(|| {
                    format!(
                        "CONTRIBUTION member `lifecycle_state: {token}` is not a state of the \
                         openEHR version_lifecycle_state group ({})",
                        crate::vocab::VersionLifecycleState::ALL
                            .iter()
                            .map(|s| s.token())
                            .collect::<Vec<_>>()
                            .join(" | ")
                    )
                }),
            Some(other) => Err(format!(
                "CONTRIBUTION member `lifecycle_state` must be a state token, got {other}"
            )),
        }
    }

    /// Stamp the member's class self-tag when the case declared one.
    fn apply_member_type(version: &mut Value, member_type: Option<MemberVersionType>) {
        if let Some(member_type) = member_type
            && let Some(map) = version.as_object_mut()
        {
            map.insert(
                "_type".to_owned(),
                Value::String(member_type.token().to_owned()),
            );
        }
    }

    /// Body per the binding request contract.
    fn select_body(
        &mut self,
        request_spec: &crate::model::binding::RequestSpec,
        with: &BTreeMap<String, Value>,
        step: &FlowStep,
        vars: &VarStore,
    ) -> Result<Option<Value>, String> {
        match &request_spec.body {
            None => Ok(None),
            Some(RequestBody::Named { name, optional }) => {
                // The bundled version-set construct: `versions:` becomes a
                // canonical CONTRIBUTION envelope (ORIGINAL_VERSION members
                // with commit_audit, per the ITS contribution schema).
                if name == "contribution"
                    && let Some(versions) = with.get("versions").and_then(Value::as_array)
                {
                    return Self::contribution_envelope(versions, with.get("audit"))
                        .map(Some)
                        .map_err(|e| format!("step {}: {e}", step.step));
                }
                let found = with
                    .get(name)
                    .cloned()
                    .or_else(|| with.get("composition").cloned())
                    .or_else(|| with.get("opt").cloned())
                    .or_else(|| {
                        // single-payload steps: the one non-path value.
                        // Objects/arrays always qualify; a STRING qualifies
                        // only for a text-format role (`aql_text`, `*_text`)
                        // — any other scalar is a header/path realization,
                        // never a resource body.
                        let text_role = name.ends_with("_text");
                        with.iter()
                            .find(|(k, v)| {
                                (v.is_object() || v.is_array() || (text_role && v.is_string()))
                                    && !request_spec.path.params().iter().any(|p| p.as_str() == *k)
                            })
                            .map(|(_, v)| v.clone())
                    });
                match (found, optional) {
                    // a resolved `null` means ABSENT (the recipe's omitted
                    // payload — e.g. `ehr_status: absent` rows): send no body
                    (Some(Value::Null) | None, true) => Ok(None),
                    (Some(v), _) => Ok(Some(v)),
                    (None, false) => {
                        Err(format!("step {}: body role {name} unresolved", step.step))
                    }
                }
            }
            Some(RequestBody::Structured(template)) => {
                // Structured templates reference the step's own with-values
                // (`${q}`, `${fetch?}`) as well as captures.
                let mut merged = vars.clone();
                for (key, value) in with {
                    if let Ok(name) = CaptureName::parse(key)
                        && merged.get(&name).is_none()
                    {
                        match value {
                            Value::String(s) => {
                                merged.set(name, Captured::Scalar(s.clone()));
                            }
                            other => {
                                merged.set(name, Captured::Body(other.clone()));
                            }
                        }
                    }
                }
                Ok(Some(
                    self.resolver
                        .resolve_value(template, &merged)
                        .map_err(|e| e.to_string())?,
                ))
            }
            Some(RequestBody::Patched { from_capture, set }) => {
                Self::patched_body(from_capture, set, vars).map(Some)
            }
        }
    }

    /// The read-modify-write body of an SM field-setter binding (AMB-15): the
    /// captured resource with the declared `set:` fields overwritten.
    ///
    /// A captured body that is not a JSON object CANNOT carry the mutation —
    /// a canonical-XML capture, for instance, resolves as a `Value::String`.
    /// Applying nothing and sending the resource back unchanged would be a
    /// FALSE GREEN: the PUT succeeds while exercising no setter at all. So a
    /// non-object base is a loud step error, which the caller turns into a
    /// transport-class (inconclusive, runner-side) observation — never a
    /// silent no-op.
    fn patched_body(
        from_capture: &CaptureName,
        set: &[(String, Value)],
        vars: &VarStore,
    ) -> Result<Value, String> {
        // Negatives against a non-existent resource have no captured base body, so
        // the wire gets a minimal RM-VALID canonical EHR_STATUS (the SUT must reject
        // on the unknown id, not the body). RM-validity is load-bearing: EHR_STATUS
        // is an unconditional archetype root (RM ehr `ehr_status.adoc`
        // `Is_archetype_root`) and a root without ARCHETYPED violates
        // `Archetyped_valid` (RM common `locatable.adoc`). The fallback applies ONLY
        // to a capture name the case never declared; a DECLARED capture that failed
        // to bind is a loud step error, never a substituted body.
        let mut patched = match vars.get(from_capture) {
            Some(Captured::Body(body)) => body.clone(),
            Some(other) => {
                return Err(format!(
                    "patched body: capture {from_capture} is bound but holds {} — a declared \
                     capture that did not bind a body is a case defect, not a substitutable one",
                    json_shape_of_captured(other)
                ));
            }
            None if matches!(from_capture.as_str(), "status_body" | "ehr_status") => {
                serde_json::json!({
                    "_type": "EHR_STATUS",
                    "name": { "_type": "DV_TEXT", "value": "ehr status" },
                    "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
                    "archetype_details": {
                        "_type": "ARCHETYPED",
                        "archetype_id": {
                            "_type": "ARCHETYPE_ID",
                            "value": "openEHR-EHR-EHR_STATUS.generic.v1"
                        },
                        "rm_version": "1.1.0"
                    },
                    "subject": { "_type": "PARTY_SELF" },
                    "is_queryable": true,
                    "is_modifiable": true
                })
            }
            None => {
                return Err(format!(
                    "patched body: capture {from_capture} holds no resource body"
                ));
            }
        };
        let Some(map) = patched.as_object_mut() else {
            let fields: Vec<&str> = set.iter().map(|(field, _)| field.as_str()).collect();
            return Err(format!(
                "patched body: capture {from_capture} holds a {} base, not a JSON object, so the \
                 declared set: [{}] cannot be applied — writing the captured resource back \
                 unmutated would exercise nothing",
                json_shape(&patched),
                fields.join(", ")
            ));
        };
        for (field, value) in set {
            map.insert(field.clone(), value.clone());
        }
        Ok(patched)
    }

    /// commit: bulk-provision generated sets, binding committed uids.
    fn provision_commit_sets(
        &mut self,
        case: &CaseCore,
        vars: &mut VarStore,
    ) -> Result<(), String> {
        for key in case.requires.commit.clone() {
            let set = self.resolver.data_set(&key).map_err(|e| e.to_string())?;
            // A generated set is an array; a plain composition fixture is a
            // single object and commits as a one-item set. Anything else is a
            // catalogue defect — never skip silently (the precondition "the
            // EHR has commits" must hold or the run must fail).
            let items: Vec<Value> = match set {
                Value::Array(a) => a,
                obj @ Value::Object(_) => vec![obj],
                other => {
                    return Err(format!(
                        "requires.commit key {key}: expected a set array or a composition object, got {other}"
                    ));
                }
            };
            let binding = self.binding_for(case, "I_EHR_COMPOSITION.create_composition")?;
            let instance = self.provisioning_instance(case)?;
            let request_spec = binding
                .request
                .as_ref()
                .ok_or_else(|| "create_composition unrealized".to_owned())?;
            let ehr_id = vars
                .scalar(&CaptureName::parse("ehr_id").map_err(|e| e.to_string())?)
                .ok_or_else(|| "requires.commit without a provisioned ehr".to_owned())?
                .to_owned();
            let headers = Self::compose_headers(
                self.set,
                self.ixit,
                case,
                None,
                binding,
                instance,
                vars,
                None,
                self.spec_versions,
            )?;
            let mut uids = Vec::new();
            for item in items {
                let base = instance.base_url.trim_end_matches('/');
                let path = request_spec.path.raw().replace("{ehr_id}", &ehr_id);
                let url = format!("{base}{path}");
                let exchange = self.send(request_spec.method, &url, &headers, Some(&item), true)?;
                if let Some((_, spec)) = binding
                    .captures
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .find(|(n, _)| n.as_str() == "version_uid")
                    && let Some(uid) = Self::extract_capture(&exchange, binding, spec, vars)
                {
                    uids.push(uid);
                }
            }
            vars.set(committed_uids_handle(), Captured::List(uids));
        }
        Ok(())
    }
}

impl HttpDriver<'_> {
    /// **The ONE request-header construction path** (issue #629): binding
    /// request headers + format headers + auth + instance extras, for every
    /// request the runner sends — a driven flow step AND precondition
    /// provisioning alike.
    ///
    /// It is one function because it was two: the step path injected
    /// `Accept: application/json` while the provisioning path for the SAME
    /// operation sent none, so one operation went on the wire two different
    /// ways depending on which code path reached it (the ADL 1.4 template
    /// upload 406'd as a case and succeeded as a precondition). A binding
    /// declares the request it intends; nothing else may quietly send a
    /// different one.
    ///
    /// `step` is `None` for provisioning. That is the only difference the
    /// caller may express, and it means exactly one thing: no step/case
    /// FORMAT applies, because a precondition lays its ground through the
    /// canonical wire the binding pins, never through the simplified format
    /// the case happens to exercise.
    ///
    /// `scopes` is the step's resolved SMART `scope` claim (`None` = the step
    /// declared none), consumed only by a `bearer_mint` principal.
    #[expect(
        clippy::too_many_arguments,
        reason = "one parameter per header source; splitting hides the assembly order"
    )]
    fn compose_headers(
        set: &ArtifactSet,
        ixit: &Ixit,
        case: &CaseCore,
        step: Option<&FlowStep>,
        binding: &OperationBinding,
        instance: &Instance,
        vars: &VarStore,
        scopes: Option<&[String]>,
        spec_versions: Option<&crate::party::SpecVersions>,
    ) -> Result<BTreeMap<String, String>, String> {
        let request_spec = binding
            .request
            .as_ref()
            .ok_or_else(|| "binding is unrealized".to_owned())?;
        let site = step.map_or_else(
            || "provisioning".to_owned(),
            |step| format!("step {}", step.step),
        );
        let mut headers: BTreeMap<String, String> = BTreeMap::new();
        if let Some(request_headers) = &request_spec.headers {
            for (name, template) in request_headers {
                match assertions::render_template(template, vars) {
                    Ok(value) => {
                        headers.insert(name.clone(), value);
                    }
                    Err(e) => {
                        return Err(format!("{site}: header {name}: {e}"));
                    }
                }
            }
        }
        let format = step.and_then(|step| step.format.or_else(|| case.formats.first().copied()));
        if let Some(format) = format {
            let media = Self::media_type(format);
            // The step's format has two distinct roles by request shape: on a
            // body-carrying request (POST/PUT commit) it names the REQUEST body
            // representation, so it sets Content-Type only and the response stays
            // negotiated canonical — ETag/Location are representation-independent,
            // and RFC 7231 §6.3.2 requires Location on a 201 regardless of the
            // request body format. On a bodyless request (GET read-back) it names
            // the desired RESPONSE representation and sets Accept.
            if request_spec.body.is_some() {
                headers
                    .entry("Content-Type".to_owned())
                    .or_insert_with(|| media.to_owned());
                headers
                    .entry("Accept".to_owned())
                    .or_insert_with(|| "application/json".to_owned());
            } else if step.is_some_and(|step| step.format.is_some()) {
                // An EXPLICIT step-level format names the desired RESPONSE
                // representation and overrides any binding-pinned Accept
                // (e.g. get_opt pins application/xml, but `format: wt`
                // requests the Web Template JSON representation).
                headers.insert("Accept".to_owned(), media.to_owned());
            } else {
                // The case-level default format is only a fallback — a
                // binding-pinned Accept (the canonical OPT XML / ADL2 text
                // representations) wins over it.
                headers
                    .entry("Accept".to_owned())
                    .or_insert_with(|| media.to_owned());
            }
            if let Some(format_headers) = &binding.format_headers
                && let Some((_, map)) = format_headers.iter().find(|(k, _)| k.0 == format)
            {
                for (name, req) in &map.0 {
                    match req {
                        crate::model::binding::FormatHeaderReq::Literal(t) => {
                            if let Ok(v) = assertions::render_template(t, vars) {
                                headers.insert(name.clone(), v);
                            }
                        }
                        crate::model::binding::FormatHeaderReq::Required => {
                            // openehr-template-id: the committed payload's own
                            // manifest-declared template identity wins — the step's
                            // `${ds:…}` body names the data set and its corpus entry
                            // carries the authoritative `template_id` (which also
                            // serves cases provisioning their template IN-FLOW, where
                            // `requires.templates` is rightly empty). Fallback: the
                            // case's provisioned template list.
                            let body_ds_template_id = step.and_then(|step| {
                                step.with_entries().iter().find_map(|(_, v)| {
                                    v.refs().iter().find_map(|r| match r {
                                        ValueRef::DataSet { key, .. } => set
                                            .corpus
                                            .as_ref()
                                            .and_then(|(_, m)| m.get(key))
                                            .and_then(|e| e.template_id.clone()),
                                        _ => None,
                                    })
                                })
                            });
                            let template_id = body_ds_template_id.or_else(|| {
                                case.requires.templates.first().map(|key| {
                                    set.corpus
                                        .as_ref()
                                        .and_then(|(_, m)| m.get(key))
                                        .and_then(|e| e.template_id.clone())
                                        .unwrap_or_else(|| key.to_string())
                                })
                            });
                            if let Some(template_id) = template_id {
                                headers.insert(name.clone(), template_id);
                            }
                        }
                    }
                }
            }
        } else {
            // Format-less request (a provisioning call, or a step whose case
            // names no format): the canonical JSON default representation.
            // A body-carrying request MUST label its payload — a strict SUT
            // rightly 415s an unlabeled body (ITS-REST overview Resources
            // §Data representation: JSON is the default representation;
            // HTTP semantics: a sender SHOULD send Content-Type on a body).
            if request_spec.body.is_some() {
                headers
                    .entry("Content-Type".to_owned())
                    .or_insert_with(|| "application/json".to_owned());
            }
            if !headers.contains_key("Accept") {
                headers.insert("Accept".to_owned(), "application/json".to_owned());
            }
        }
        if let Some(auth) = Self::auth_header(ixit, &instance.auth, scopes)? {
            headers.insert("Authorization".to_owned(), auth);
        }
        if let Some(extra) = &instance.headers {
            for (name, value) in extra {
                headers.insert(name.clone(), value.clone());
            }
        }
        Ok(Self::spell_committal_headers(headers, spec_versions))
    }

    /// Select the committal-metadata request-header SPELLING from the
    /// party's declared ITS-REST release.
    ///
    /// The overview `Requests_and_responses.md` §Deprecated headers dates
    /// the current spellings to Release 1.1.0 and maps each to its
    /// pre-1.1.0 counterpart. Field names are case-insensitive (RFC 9110
    /// §5.1), so three of the five rows — `openEHR-VERSION`, `openEHR-uri`,
    /// `openEHR-EHR-id` — are literally the SAME field as their
    /// replacements and need no selection; do not re-split them. Two rows
    /// change an underscore to a hyphen and are therefore genuinely
    /// distinct fields: `openEHR-AUDIT_DETAILS` → `openehr-audit-details`
    /// and `openEHR-TEMPLATE_ID` → `openehr-template-id`. A party declaring
    /// an ITS-REST release BEFORE 1.1.0 never defined the hyphenated names,
    /// so a driven request rewrites those two to the pre-1.1.0 spelling —
    /// otherwise an undated behaviour goes red for a field name the party's
    /// release does not know, which is not the behaviour under test.
    ///
    /// A party declaring 1.1.0+ — or declaring nothing — keeps the
    /// canonical spellings (scope for undeclared parties is the version
    /// floors' job, never this function's). Bindings that deliberately
    /// author the DEPRECATED spellings (the backward-compatibility cases)
    /// are untouched: their declared names case-fold to the underscore
    /// forms, which this map does not contain.
    fn spell_committal_headers(
        headers: BTreeMap<String, String>,
        spec_versions: Option<&crate::party::SpecVersions>,
    ) -> BTreeMap<String, String> {
        let pre_1_1_0 = spec_versions
            .and_then(|versions| versions.its_rest.as_deref())
            .and_then(|raw| semver::Version::parse(raw).ok())
            .is_some_and(|version| version < semver::Version::new(1, 1, 0));
        if !pre_1_1_0 {
            return headers;
        }
        headers
            .into_iter()
            .map(|(name, value)| {
                let spelled = if name.eq_ignore_ascii_case("openehr-audit-details") {
                    "openEHR-AUDIT_DETAILS".to_owned()
                } else if name.eq_ignore_ascii_case("openehr-template-id") {
                    "openEHR-TEMPLATE_ID".to_owned()
                } else {
                    name
                };
                (spelled, value)
            })
            .collect()
    }
}

impl HttpDriver<'_> {
    /// Execute the wire expectation's declared header matchers (#403,
    /// `exec/headers.rs`) and body selector (#415, `exec/bodies.rs`) against
    /// the exchange. `prefer_conditional`/`error_loose` branch on the
    /// `Prefer` this request actually sent, so the sent value travels with
    /// the negotiated `Accept`.
    fn eval_wire_expectation(
        &self,
        expectation: &crate::model::binding::WireExpectation,
        exchange: &Exchange,
        sent_headers: &BTreeMap<String, String>,
        vars: &VarStore,
    ) -> Vec<String> {
        let header_ctx = crate::exec::headers::RequestContext {
            accept: sent_headers.get("Accept").map(String::as_str),
            last_version_uid: self.last_version_uid.as_deref(),
            spec_versions: self.spec_versions,
        };
        let mut failures =
            crate::exec::headers::evaluate(expectation, &exchange.headers, &header_ctx, vars);
        let body_ctx = crate::exec::bodies::RequestContext {
            accept: sent_headers.get("Accept").map(String::as_str),
            prefer: sent_headers.get("Prefer").map(String::as_str),
        };
        failures.extend(crate::exec::bodies::evaluate(
            expectation,
            exchange.body.as_ref(),
            &exchange.headers,
            &body_ctx,
        ));
        failures
    }

    /// Remember the newest `version_uid` a SUCCESS outcome's binding capture
    /// yields on this row — the `latest-version-uid` header matcher's
    /// comparison source. Only success-class outcomes advance it (an error
    /// commits no version — RM common master06 §Committal and Audits).
    fn track_latest_version_uid(
        &mut self,
        binding: &OperationBinding,
        exchange: &Exchange,
        observation: &Observation,
        vars: &VarStore,
    ) {
        let Observation::Kind(kind) = observation else {
            return;
        };
        if kind.class() != crate::vocab::OutcomeClass::Success {
            return;
        }
        let Some(spec) = binding
            .captures
            .as_deref()
            .unwrap_or_default()
            .iter()
            .find(|(n, _)| n.as_str() == "version_uid")
            .map(|(_, s)| s)
        else {
            return;
        };
        if let Some(uid) = Self::extract_capture(exchange, binding, spec, vars) {
            self.last_version_uid = Some(uid);
        }
    }

    /// ehr: mint `${ehr_id}` via `create_ehr`.
    fn provision_ehr(&mut self, case: &CaseCore, vars: &mut VarStore) -> Result<(), String> {
        if matches!(case.requires.ehr, Some(EhrRequirement::Exists { .. })) {
            let binding = self.binding_for(case, "I_EHR_SERVICE.create_ehr")?;
            let instance = self.provisioning_instance(case)?;
            let request_spec = binding
                .request
                .as_ref()
                .ok_or_else(|| "create_ehr unrealized".to_owned())?;
            let headers = Self::compose_headers(
                self.set,
                self.ixit,
                case,
                None,
                binding,
                instance,
                vars,
                None,
                self.spec_versions,
            )?;
            let base = instance.base_url.trim_end_matches('/');
            let url = format!("{base}{}", request_spec.path.raw());
            let exchange = self.send(request_spec.method, &url, &headers, None, true)?;
            if let Some((_, spec)) = binding
                .captures
                .as_deref()
                .unwrap_or_default()
                .iter()
                .find(|(n, _)| n.as_str() == "ehr_id")
                && let Some(value) = Self::extract_capture(&exchange, binding, spec, vars)
            {
                vars.set(
                    CaptureName::parse("ehr_id").map_err(|e| e.to_string())?,
                    Captured::Scalar(value),
                );
            }
        }
        Ok(())
    }

    /// party: mint `${party_id}` via `create_party` from the named corpus set.
    ///
    /// The handle is the party's `VERSIONED_OBJECT` uid — the identifier the SM
    /// admin operations take (`i_admin_archive.adoc` `archive_parties`,
    /// `i_admin_service.adoc` `physical_party_delete`), not the version uid the
    /// create's `ETag` carries; the `create_party` binding maps both captures,
    /// so the container uid is read straight off it.
    fn provision_party(&mut self, case: &CaseCore, vars: &mut VarStore) -> Result<(), String> {
        let Some(crate::model::case::PartyRequirement::Exists(key)) = case.requires.party.clone()
        else {
            return Ok(());
        };
        let payload = self.resolver.data_set(&key).map_err(|e| e.to_string())?;
        if let Some(uid) = self.create_party(case, &payload, vars)? {
            vars.set(
                CaptureName::parse("party_id").map_err(|e| e.to_string())?,
                Captured::Scalar(uid),
            );
        }
        Ok(())
    }

    /// Create one demographic PARTY from a corpus payload and return its
    /// `VERSIONED_OBJECT` uid.
    ///
    /// The route is the one the payload's own concrete RM type names: ITS-REST
    /// 1.1.0 surfaces one create endpoint per concrete PARTY subtype (the
    /// `create_party` binding realizes PERSON, its `agent`/`group`/
    /// `organisation`/`role` variants the other four), so a payload's `_type`
    /// selects the variant rather than the caller guessing one.
    fn create_party(
        &mut self,
        case: &CaseCore,
        payload: &Value,
        vars: &VarStore,
    ) -> Result<Option<String>, String> {
        let variant = Self::party_create_variant(payload)?;
        let binding =
            self.binding_for_variant(case, "I_DEMOGRAPHIC_SERVICE.create_party", variant)?;
        if binding.variant.as_deref() != variant {
            return Err(format!(
                "create_party declares no {} realization for the provisioned party payload",
                variant.unwrap_or("person"),
            ));
        }
        let instance = self.provisioning_instance(case)?;
        let request_spec = binding
            .request
            .as_ref()
            .ok_or_else(|| "create_party unrealized".to_owned())?;
        let headers = Self::compose_headers(
            self.set,
            self.ixit,
            case,
            None,
            binding,
            instance,
            vars,
            None,
            self.spec_versions,
        )?;
        let base = instance.base_url.trim_end_matches('/');
        let url = format!("{base}{}", request_spec.path.raw());
        let exchange = self.send(request_spec.method, &url, &headers, Some(payload), true)?;
        Ok(binding
            .captures
            .as_deref()
            .unwrap_or_default()
            .iter()
            .find(|(n, _)| n.as_str() == "versioned_object_uid")
            .and_then(|(_, spec)| Self::extract_capture(&exchange, binding, spec, vars)))
    }

    /// The `create_party` binding variant a party payload's concrete RM type
    /// selects — `None` for PERSON, which the variant-less binding realizes.
    fn party_create_variant(payload: &Value) -> Result<Option<&'static str>, String> {
        match payload.get("_type").and_then(Value::as_str) {
            Some("PERSON") => Ok(None),
            Some("AGENT") => Ok(Some("agent")),
            Some("GROUP") => Ok(Some("group")),
            Some("ORGANISATION") => Ok(Some("organisation")),
            Some("ROLE") => Ok(Some("role")),
            other => Err(format!(
                "a provisioned party payload must be a concrete PARTY subtype \
                 (PERSON | AGENT | GROUP | ORGANISATION | ROLE), got {}",
                other.unwrap_or("<no _type>")
            )),
        }
    }

    /// `party_relationship`: mint `${party_relationship_id}` — the relationship's
    /// `VERSIONED_OBJECT` uid — by creating both endpoint parties over the
    /// released demographic wire and then the relationship between them.
    ///
    /// The endpoint substitution is what RM demographic
    /// `master02-demographic_package.adoc` §Party Relationships requires:
    /// `source`/`target` are "`OBJECT_REFs` containing `HIER_OBJECT_IDs` to
    /// denote the Version container of a Party, rather than
    /// `OBJECT_VERSION_IDs`" — so each `PARTY_REF.id.value` becomes the
    /// container uid the create just minted, and the corpus payload's declared
    /// `PARTY_REF.type` must be the type of the party provisioned for that end
    /// (a mismatch is a catalogue defect, refused here rather than sent).
    ///
    /// NOTE: the relationship create itself is driven over the SUT's own
    /// `party-relationship` extension route — no openEHR spec governs it
    /// (register AMB-32), the released ITS-REST surfaces no
    /// `PARTY_RELATIONSHIP` resource — so this precondition is only available
    /// on a party that serves that family.
    fn provision_party_relationship(
        &mut self,
        case: &CaseCore,
        vars: &mut VarStore,
    ) -> Result<(), String> {
        let Some(crate::model::case::PartyRelationshipRequirement::Exists {
            source,
            target,
            relationship,
        }) = case.requires.party_relationship.clone()
        else {
            return Ok(());
        };
        let mut body = self
            .resolver
            .data_set(&relationship)
            .map_err(|e| e.to_string())?;
        for (end, key) in [("source", &source), ("target", &target)] {
            let payload = self.resolver.data_set(key).map_err(|e| e.to_string())?;
            let party_type = payload
                .get("_type")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let declared = body
                .get(end)
                .and_then(|r| r.get("type"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            if declared != party_type {
                return Err(format!(
                    "requires.party_relationship: {relationship} declares {end} PARTY_REF.type \
                     {declared:?}, but {key} provisions a {party_type:?}"
                ));
            }
            let uid = self
                .create_party(case, &payload, vars)?
                .ok_or_else(|| format!("provisioning the {end} party minted no uid"))?;
            *body
                .get_mut(end)
                .and_then(|r| r.get_mut("id"))
                .and_then(|id| id.get_mut("value"))
                .ok_or_else(|| {
                    format!("requires.party_relationship: {relationship} has no {end}.id.value")
                })? = Value::String(uid);
        }
        let binding = self.binding_for(case, "I_DEMOGRAPHIC_SERVICE.create_party_relationship")?;
        let instance = self.provisioning_instance(case)?;
        let request_spec = binding
            .request
            .as_ref()
            .ok_or_else(|| "create_party_relationship unrealized".to_owned())?;
        let headers = Self::compose_headers(
            self.set,
            self.ixit,
            case,
            None,
            binding,
            instance,
            vars,
            None,
            self.spec_versions,
        )?;
        let base = instance.base_url.trim_end_matches('/');
        let url = format!("{base}{}", request_spec.path.raw());
        let exchange = self.send(request_spec.method, &url, &headers, Some(&body), true)?;
        if let Some((_, spec)) = binding
            .captures
            .as_deref()
            .unwrap_or_default()
            .iter()
            .find(|(n, _)| n.as_str() == "versioned_object_uid")
            && let Some(value) = Self::extract_capture(&exchange, binding, spec, vars)
        {
            vars.set(
                CaptureName::parse("party_relationship_id").map_err(|e| e.to_string())?,
                Captured::Scalar(value),
            );
        }
        Ok(())
    }

    /// `import`: replay an EHR-Extract received from another system, then mint
    /// the identities it carried — `${imported_versioned_object_uid}`,
    /// `${imported_version_uid}` and, when the named container carries one,
    /// `${imported_branch_version_uid}`.
    ///
    /// The identities come from the EXTRACT itself rather than from a read of
    /// the SUT, because RM common `master06-change_control_package.adoc`
    /// §Copying keeps them: "the `ORIGINAL_VERSION` instance is never
    /// modified", and its `uid` is what the receiving system's
    /// `IMPORTED_VERSION` is identified by. Reading the SUT to learn what to
    /// address would make the server's own answer the reference for the
    /// case's expectation, which is exactly what the attribution law forbids.
    ///
    /// Which operation lands it follows master06 §Copying's receiving
    /// situations: an already-provisioned `${ehr_id}` takes `import_ehr_extract`
    /// (Cases 2/3), and no EHR at all takes `import_ehr` (Case 1), whose
    /// answer names the clone it created and mints `${ehr_id}`.
    ///
    /// NOTE: the import is driven over the SUT's own `message-extract`
    /// extension routes — ITS-REST 1.1.0 publishes no MESSAGE / EHR-Extract
    /// API at all (register AMB-34) — so this precondition is only available
    /// on a party that serves that family, which `crate::run` enforces at
    /// SELECTION time.
    fn provision_import(
        &mut self,
        case: &CaseCore,
        vars: &mut VarStore,
    ) -> Result<Provisioned, String> {
        let Some(ImportRequirement::Received { extract, container }) = case.requires.import.clone()
        else {
            return Ok(Provisioned::Ready);
        };
        let payload = self
            .resolver
            .data_set(&extract)
            .map_err(|e| e.to_string())?;
        let ehr_handle = CaptureName::parse("ehr_id").map_err(|e| e.to_string())?;
        let into_existing = vars.scalar(&ehr_handle).map(str::to_owned);
        let call = if into_existing.is_some() {
            "I_EHR_EXTRACT_SERVICE.import_ehr_extract"
        } else {
            "I_EHR_EXTRACT_SERVICE.import_ehr"
        };
        let binding = self.binding_for(case, call)?;
        let instance = self.provisioning_instance(case)?;
        let request_spec = binding
            .request
            .as_ref()
            .ok_or_else(|| format!("{call} unrealized"))?;
        let headers = Self::compose_headers(
            self.set,
            self.ixit,
            case,
            None,
            binding,
            instance,
            vars,
            None,
            self.spec_versions,
        )?;
        let base = instance.base_url.trim_end_matches('/');
        // `import_ehr` takes its optional target id as a QUERY parameter and
        // provisioning never supplies one (an absent id is the Case-1 clone
        // that re-uses the source EHR id); `import_ehr_extract` takes it in
        // the path.
        let path = match &into_existing {
            Some(ehr_id) => request_spec.path.raw().replace("{an_ehr_id}", ehr_id),
            None => request_spec.path.raw().to_owned(),
        };
        let url = format!("{base}{path}");
        let exchange = self.send(request_spec.method, &url, &headers, Some(&payload), true)?;
        if let Some(reason) = Self::provisioning_refusal(call, &exchange, false) {
            return Ok(Provisioned::RowErrored { reason });
        }
        if into_existing.is_none() {
            let Some(minted) = binding
                .captures
                .as_deref()
                .unwrap_or_default()
                .iter()
                .find(|(n, _)| n.as_str() == "ehr_id")
                .and_then(|(_, spec)| Self::extract_capture(&exchange, binding, spec, vars))
            else {
                return Err(format!(
                    "requires.import: {call} answered without naming the EHR it cloned, so the \
                     case has no ${{ehr_id}} to read through"
                ));
            };
            vars.set(ehr_handle, Captured::Scalar(minted));
        }
        for (name, value) in Self::imported_identities(&payload, container, &extract)? {
            vars.set(
                CaptureName::parse(name).map_err(|e| e.to_string())?,
                Captured::Scalar(value),
            );
        }
        Ok(Provisioned::Ready)
    }

    /// The identities the named `X_VERSIONED_*` content item of an EXTRACT
    /// carries: the container uid, its latest TRUNK version uid, and — when
    /// the extract carries one — its latest BRANCH version uid.
    ///
    /// The extract must carry exactly one content item of the named class: the
    /// handles name ONE versioned object, and a fixture carrying two of a
    /// class would leave which one silently positional.
    ///
    /// "Latest" is by `OBJECT_VERSION_ID.version_tree_id` order (RM common
    /// master06 §Version Identification: the trunk number, then the branch
    /// number and branch version), so the choice is a property of the fixture
    /// rather than of document order.
    fn imported_identities(
        extract: &Value,
        container: crate::vocab::XVersionedClass,
        key: &crate::ids::CorpusKey,
    ) -> Result<Vec<(&'static str, String)>, String> {
        let empty: Vec<Value> = Vec::new();
        let mut wrappers = Vec::new();
        for chapter in extract
            .get("chapters")
            .and_then(Value::as_array)
            .unwrap_or(&empty)
        {
            for item in chapter
                .get("items")
                .and_then(Value::as_array)
                .unwrap_or(&empty)
            {
                if let Some(wrapper) = item.get("item")
                    && wrapper.get("_type").and_then(Value::as_str) == Some(container.token())
                {
                    wrappers.push(wrapper);
                }
            }
        }
        let [wrapper] = wrappers.as_slice() else {
            return Err(format!(
                "requires.import: {key} carries {} content items of class {} — the precondition \
                 names exactly one versioned object",
                wrappers.len(),
                container.token()
            ));
        };
        let container_uid = wrapper
            .pointer("/uid/value")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                format!(
                    "requires.import: the {} of {key} carries no uid.value",
                    container.token()
                )
            })?
            .to_owned();
        let mut trunk: Option<(Vec<u64>, String)> = None;
        let mut branch: Option<(Vec<u64>, String)> = None;
        for version in wrapper
            .get("versions")
            .and_then(Value::as_array)
            .unwrap_or(&empty)
        {
            let uid = version
                .pointer("/uid/value")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    format!(
                        "requires.import: a version of the {} of {key} carries no uid.value",
                        container.token()
                    )
                })?;
            let tree = Self::version_tree_id(uid).ok_or_else(|| {
                format!("requires.import: {uid} is not an OBJECT_VERSION_ID of {key}")
            })?;
            let slot = if tree.len() > 1 {
                &mut branch
            } else {
                &mut trunk
            };
            if slot.as_ref().is_none_or(|(seen, _)| *seen < tree) {
                *slot = Some((tree, uid.to_owned()));
            }
        }
        let mut minted = vec![("imported_versioned_object_uid", container_uid)];
        let (_, trunk_uid) = trunk.ok_or_else(|| {
            format!(
                "requires.import: the {} of {key} carries no trunk version",
                container.token()
            )
        })?;
        minted.push(("imported_version_uid", trunk_uid));
        if let Some((_, branch_uid)) = branch {
            minted.push(("imported_branch_version_uid", branch_uid));
        }
        Ok(minted)
    }

    /// The numeric segments of an `OBJECT_VERSION_ID`'s `version_tree_id`
    /// (`object_id::creating_system_id::version_tree_id`, RM common master06
    /// §Version Identification): `[n]` on the trunk, `[n, branch, version]` on
    /// a branch.
    ///
    /// NOTE: `None` is ABSENCE — "this string is not an `OBJECT_VERSION_ID`"
    /// (#1853); the `Option<Vec<_>>` collect refuses a partly-numeric tree
    /// outright, and the one caller turns it into a typed, uid-naming error.
    fn version_tree_id(uid: &str) -> Option<Vec<u64>> {
        let (_, tree) = uid.rsplit_once("::")?;
        tree.split('.').map(|s| s.parse::<u64>().ok()).collect()
    }

    /// Bind the step's captures from the exchange when the observation
    /// matched a mapped kind (the closed capture-source grammar).
    fn bind_step_captures(
        step: &FlowStep,
        binding: &OperationBinding,
        exchange: &Exchange,
        observation: &Observation,
        sent_ms: i64,
        vars: &mut VarStore,
    ) {
        let Observation::Kind(kind) = observation else {
            return;
        };
        for (name, source) in step.captures() {
            if source.outcome != *kind {
                continue;
            }
            match &source.field {
                CaptureField::Body => {
                    if let Some(b) = &exchange.body {
                        vars.set(name.clone(), Captured::Body(b.clone()));
                    }
                }
                CaptureField::CommitTime => {
                    // The live commit WINDOW: [request send, response receipt] in
                    // runner-clock milliseconds, WIDENED by the SUT's own
                    // second-resolution `Date` header — the SUT stamped the version
                    // inside it even when its clock is skewed from the runner's, so
                    // `before` resolves from the lower bound and `after` from the
                    // upper. Determinism law (d) governs the `${time:*}` ARITHMETIC
                    // over this window, not the window itself.
                    let mut lo = sent_ms;
                    let mut hi = now_ms();
                    if let Some(date_ms) = exchange
                        .headers
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case("date"))
                        .and_then(|(_, v)| parse_http_date_ms(v))
                    {
                        lo = lo.min(date_ms);
                        hi = hi.max(date_ms.saturating_add(999));
                    }
                    vars.set(name.clone(), Captured::InstantMs { lo, hi });
                }
                CaptureField::Field { name: field, list } => {
                    let Some(spec) = binding
                        .captures
                        .as_deref()
                        .unwrap_or_default()
                        .iter()
                        .find(|(n, _)| n == field)
                        .map(|(_, s)| s)
                    else {
                        continue;
                    };
                    if *list {
                        if let (Some(body), WireFrom::Body { path }) = (&exchange.body, &spec.from)
                        {
                            let items = extract_list(body, path);
                            vars.set(name.clone(), Captured::List(items));
                        }
                    } else if let Some(value) = Self::extract_capture(exchange, binding, spec, vars)
                    {
                        vars.set(name.clone(), Captured::Scalar(value));
                    }
                }
            }
        }
    }
}

impl HttpDriver<'_> {
    /// Resolve the step's `with` map; an unresolvable reference is reported
    /// as the error string the caller turns into a transport-class
    /// (inconclusive) observation.
    fn resolve_with(
        &mut self,
        step: &FlowStep,
        vars: &VarStore,
    ) -> Result<BTreeMap<String, Value>, String> {
        let mut with: BTreeMap<String, Value> = BTreeMap::new();
        for (key, value) in step.with_entries() {
            let v = self
                .resolver
                .resolve_value(value, vars)
                .map_err(|e| format!("step {}: with.{key}: {e}", step.step))?;
            with.insert(key.clone(), v);
        }
        Ok(with)
    }

    /// Resolve the step's declared SMART `scope` claim. `None` when the step
    /// declares no `scopes:` key at all; `Some(vec![])` for an explicitly
    /// empty declaration — the scope-less token the fail-closed deny branch
    /// needs (master08 §Scopes ¶2), which is a different request from "this
    /// step is not a SMART step".
    fn resolve_scopes(
        &mut self,
        step: &FlowStep,
        vars: &VarStore,
    ) -> Result<Option<Vec<String>>, String> {
        if !step.declares_scopes() {
            return Ok(None);
        }
        let mut scopes = Vec::new();
        for value in step.scope_templates() {
            let resolved = self
                .resolver
                .resolve_value(value, vars)
                .map_err(|e| format!("step {}: scopes: {e}", step.step))?;
            match resolved {
                Value::String(s) => scopes.push(s),
                other => {
                    return Err(format!(
                        "step {}: scopes entry resolved to a {}, expected a scope string",
                        step.step,
                        json_shape(&other)
                    ));
                }
            }
        }
        Ok(Some(scopes))
    }

    /// Captures plus the step's own scalar `with` values (the header/query
    /// template resolution scope). A step's `with:` value SHADOWS a
    /// same-named earlier capture in this step's scope — the step's explicit
    /// input is the most specific binding, and the old keep-the-capture guard
    /// silently rendered a STALE value into header templates (the run-2
    /// triage, 2026-07-28: a case that captured `preceding_version_uid` at
    /// step 1 and passed a newer uid under the same name at step 4 had its
    /// If-Match rendered from step 1 — the SUT's 412 was correct and the red
    /// row was this drop). The var store itself is not mutated; the shadow
    /// lives only in the step-scoped merge.
    fn merge_with_vars(vars: &VarStore, with: &BTreeMap<String, Value>) -> VarStore {
        let mut merged = vars.clone();
        for (key, value) in with {
            // Every SCALAR `with:` value is promoted into the template vars —
            // numbers and booleans render as their wire text exactly like the
            // structured-body path does (a number-typed `url_fetch: 4` must
            // reach a `${url_fetch?}` URL slot; silently skipping non-strings
            // turned a runner gap into a fake SUT failure — the group-9
            // triage). Objects/arrays stay out: they have no scalar wire
            // text.
            let text = match value {
                Value::String(s) => Some(s.clone()),
                Value::Number(n) => Some(n.to_string()),
                Value::Bool(b) => Some(b.to_string()),
                _ => None,
            };
            if let Some(text) = text
                && let Ok(name) = CaptureName::parse(key)
            {
                merged.set(name, Captured::Scalar(text));
            }
        }
        merged
    }
}

impl<'a> HttpDriver<'a> {
    /// A binding variant named `with_<p>` realizes the SM operation's
    /// optional-argument form (e.g. `create_ehr(ehr_id?)` -> PUT
    /// `/ehr/{ehr_id}`): auto-selected when the step binds `<p>` non-null
    /// and names no explicit variant.
    fn auto_variant(
        &self,
        binding: &'a OperationBinding,
        step: &FlowStep,
        with: &BTreeMap<String, Value>,
    ) -> &'a OperationBinding {
        if step.variant.is_some() {
            return binding;
        }
        self.set
            .bindings
            .iter()
            .map(|(_, b)| b)
            .find(|b| {
                b.sm_operation == binding.sm_operation
                    && b.variant.as_deref().is_some_and(|v| {
                        v.strip_prefix("with_").is_some_and(|param| {
                            with.get(param).is_some_and(|value| !value.is_null())
                        })
                    })
            })
            .unwrap_or(binding)
    }
}

/// Temporal separability: when a step CAPTURES a commit instant and an
/// earlier one is already bound, space the commits so their
/// second-resolution windows cannot overlap — `version_at_time` grounds
/// (before/between/after) need distinguishable instants even against a
/// clock-skewed containerized SUT.
fn pace_commit_capture(step: &FlowStep, vars: &VarStore) {
    if step
        .captures()
        .iter()
        .any(|(_, s)| matches!(s.field, CaptureField::CommitTime))
        && let Some(prev_hi) = vars.latest_instant_hi()
    {
        let wait = prev_hi.saturating_add(1_100).saturating_sub(now_ms());
        if (1..=5_000).contains(&wait) {
            std::thread::sleep(std::time::Duration::from_millis(
                u64::try_from(wait).unwrap_or(0),
            ));
        }
    }
}

impl HttpDriver<'_> {
    /// Per-row synthesized OPT (issue #228): a content case whose
    /// `constraint_context` declares constraint-axis columns commits each row
    /// against a freshly synthesized OPT baking THAT row's constraint. Build it
    /// from the row's cells and upload it (409 tolerated) under the deterministic
    /// per-row template id the carrier stamps (`recipes::synth_template_id`).
    fn provision_synthesized_opt(
        &mut self,
        case: &CaseCore,
        row: usize,
    ) -> Result<Provisioned, String> {
        let Some(ctx) = &case.constraint_context else {
            return Ok(Provisioned::Ready);
        };
        if ctx.constraint_columns.is_empty() {
            return Ok(Provisioned::Ready);
        }
        let Some(matrix) = case.parameters.as_ref().and_then(|p| p.matrix.as_ref()) else {
            return Ok(Provisioned::Ready);
        };
        let Some(cells) = matrix.rows.get(row) else {
            return Ok(Provisioned::Ready);
        };
        let rm_class = case
            .rm_class
            .as_deref()
            .ok_or_else(|| "content constraint case without rm_class".to_owned())?;
        let case_id = case.id.to_string();
        // A row whose expected REJECTION rests solely on constraint axes the
        // OPT 1.4 wire cannot serialize (ITS-XML 1.0.2 Archetype.xsd) is
        // unrealizable on this technology profile — N/A, register-cited.
        if let Some(citation) =
            crate::exec::content_synth::unrealizable_row(rm_class, &matrix.columns, cells)
        {
            return Ok(Provisioned::RowNotApplicable { citation });
        }
        let template_id = crate::exec::recipes::synth_template_id(&case_id, row, cells);
        let xml = crate::exec::content_synth::synthesize_opt(
            &case_id,
            rm_class,
            &template_id,
            &matrix.columns,
            cells,
        )
        .map_err(|e| e.to_string())?;
        let payload = Value::String(xml);
        let binding = self.binding_for(case, "I_DEFINITION_ADL14.upload_opt")?;
        let instance = self.provisioning_instance(case)?;
        let request_spec = binding
            .request
            .as_ref()
            .ok_or_else(|| "upload_opt unrealized".to_owned())?;
        let headers = Self::compose_headers(
            self.set,
            self.ixit,
            case,
            None,
            binding,
            instance,
            &VarStore::default(),
            None,
            self.spec_versions,
        )?;
        let base = instance.base_url.trim_end_matches('/');
        let url = format!("{base}{}", request_spec.path.raw());
        // 409 tolerated: a re-run row re-uploads the same deterministic OPT.
        let uploaded = self.send(request_spec.method, &url, &headers, Some(&payload), false)?;
        if let Some(reason) = Self::provisioning_refusal("upload_opt", &uploaded, true) {
            return Ok(Provisioned::RowErrored { reason });
        }
        Ok(Provisioned::Ready)
    }

    /// Judge a PROVISIONING exchange: 2xx establishes the ground and,
    /// where the caller says so, 409 means it already exists (idempotent
    /// re-provisioning on a shared world — the deterministic OPT re-upload)
    /// — anything else is a REFUSAL, and the case's required ground does
    /// not exist. The refusal is surfaced as an inconclusive row naming
    /// this exchange (the triage law: an unestablished `requires`
    /// precondition is a step-resolution failure, never a SUT failure of
    /// the behaviour under test — the 2026-07-28 java run reported 197
    /// swallowed template-upload 406s as content-validation failures).
    ///
    /// `conflict_is_ground` is PER-CALL because 409's meaning inverts by
    /// operation: on the OPT upload it says the ground already holds; on an
    /// extract import it says the container exists IN ANOTHER EHR (RM
    /// common master06 §Copying — one received `object_id`, one local
    /// container), so the ground can never hold and the row is
    /// inconclusive, not driven.
    fn provisioning_refusal(
        what: &str,
        exchange: &Exchange,
        conflict_is_ground: bool,
    ) -> Option<String> {
        if exchange.status.is_success()
            || (conflict_is_ground && exchange.status == StatusCode::CONFLICT)
        {
            return None;
        }
        let body_head: String = exchange
            .body
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default()
            .chars()
            .take(200)
            .collect();
        Some(format!(
            "provisioning {what} refused: status {} — the case's required ground was never \
             established; the behaviour under test was not driven (body: {body_head})",
            exchange.status.as_u16()
        ))
    }

    /// Post-send bookkeeping shared state: the committed-payload trail for
    /// the equivalent comparison, the last response body, and the System
    /// OPTIONS manifest's `restapi_specs_version`, when served (released OAS
    /// `system.openapi.yaml` `Options` — every member optional): observed as
    /// an independent confirmation of the party's declared `its_rest`
    /// version, never as the truth (see the field NOTE).
    fn record_exchange_bookkeeping(
        &mut self,
        binding: &OperationBinding,
        request_spec: &crate::model::binding::RequestSpec,
        body: Option<&Value>,
        exchange: &Exchange,
    ) {
        if matches!(request_spec.method, HttpMethod::Post | HttpMethod::Put)
            && let Some(b) = body
        {
            self.committed.push(b.clone());
        }
        self.last_body.clone_from(&exchange.body);
        if binding.sm_operation.interface() == "I_ITS_REST_SYSTEM"
            && binding.sm_operation.operation() == "options"
            && let Some(version) = exchange
                .body
                .as_ref()
                .and_then(|b| b.get("restapi_specs_version"))
                .and_then(Value::as_str)
        {
            self.observed_restapi_specs_version = Some(version.to_owned());
        }
    }
}

impl StepDriver for HttpDriver<'_> {
    fn perform(
        &mut self,
        case: &CaseCore,
        step: &FlowStep,
        expected: OutcomeKind,
        row: usize,
        vars: &mut VarStore,
    ) -> Result<StepObservation, String> {
        self.resolver.bind_row(case, row);
        let binding = self.binding_for_variant(case, &step.call, step.variant.as_deref())?;
        if binding.is_unrealized() {
            // The interpreter surfaces this before perform() normally; the
            // driver answers with a transport-class observation so law (c)
            // holds even if reached.
            return Ok(StepObservation::transport(
                "operation unrealized on this ITS".into(),
            ));
        }
        // A named instance the ixit does not declare (e.g. no `readonly`
        // principal on a SUT without role separation) is a topology gap of
        // THIS party, not an interpreter defect: the row is inconclusive
        // (law c), and the roll-up records the case against the missing
        // ground instead of aborting the campaign.
        let instance = match self.instance_for(step) {
            Ok(instance) => instance,
            Err(e) => {
                return Ok(StepObservation::transport(format!(
                    "instance unavailable on this ixit topology: {e}"
                )));
            }
        };

        // Resolve the with-payload. A capture the earlier steps never bound
        // (the SUT did not supply what the binding maps) is an INCONCLUSIVE
        // observation for this row (law c) — never a run-aborting defect.
        let with = match self.resolve_with(step, vars) {
            Ok(with) => with,
            Err(e) => {
                return Ok(StepObservation {
                    observation: Observation::Transport(e),
                    assertion_failures: Vec::new(),
                });
            }
        };

        // The step's SMART `scope` claim resolves on the same footing as
        // `with` (row-parameterized scope strings are how the master08
        // grammar is exercised across its rows).
        let scopes = match self.resolve_scopes(step, vars) {
            Ok(scopes) => scopes,
            Err(e) => return Ok(StepObservation::transport(e)),
        };

        let binding = self.auto_variant(binding, step, &with);
        let request_spec = binding
            .request
            .as_ref()
            .ok_or_else(|| "binding is unrealized".to_owned())?;
        // Header templates resolve against captures AND the step's own
        // resolved `with` values (e.g. update_composition-non_existent
        // supplies preceding_version_uid inline, not as a capture).
        let header_vars = Self::merge_with_vars(vars, &with);
        let headers = match Self::compose_headers(
            self.set,
            self.ixit,
            case,
            Some(step),
            binding,
            instance,
            &header_vars,
            scopes.as_deref(),
            self.spec_versions,
        ) {
            Ok(headers) => headers,
            Err(e) => {
                return Ok(StepObservation {
                    observation: Observation::Transport(e),
                    assertion_failures: Vec::new(),
                });
            }
        };

        let body = match self.select_body(request_spec, &with, step, vars) {
            Ok(body) => body,
            Err(e) => return Ok(StepObservation::transport(e)),
        };

        let base = instance.base_url.trim_end_matches('/');
        let url = match Self::build_url(binding, base, &with, &header_vars) {
            Ok(url) => url,
            Err(e) => return Ok(StepObservation::transport(e)),
        };
        let body_is_json = !matches!(body, Some(Value::String(_)));
        pace_commit_capture(step, vars);
        let sent_ms = now_ms();
        let exchange = match self.send(
            request_spec.method,
            &url,
            &headers,
            body.as_ref(),
            body_is_json,
        ) {
            Ok(exchange) => exchange,
            Err(fault) => return Ok(StepObservation::transport(fault)),
        };

        self.record_exchange_bookkeeping(binding, request_spec, body.as_ref(), &exchange);

        // Classify (law c) and bind captures.
        let selectors = self.set.selectors.as_ref().map(|(_, s)| s);
        let observation =
            outcome::classify_status(binding, selectors, exchange.status.as_u16(), expected);
        Self::bind_step_captures(step, binding, &exchange, &observation, sent_ms, vars);
        self.track_latest_version_uid(binding, &exchange, &observation, vars);

        // Post-step assertions only when the expectation held (the caller
        // aborts otherwise, law b) — evaluate optimistically here.
        // The signature assertions verify against the posture of the instance
        // THIS step ran on (RM common master06 §Digital Signature: the mode is
        // a deployment fact), so a party running two postures as two instances
        // is judged per instance, never against one party-wide default.
        let signing = self.ixit.signing_of(instance);
        let mut assertion_failures =
            self.eval_assertions(case, binding, &step.assertions, &exchange, signing, vars);
        // The expected outcome's declared header matchers and body selector
        // are executed assertions too (issues #403 + #415 — both were parsed
        // but never evaluated). Evaluated only when the observation IS the
        // expected kind: the declarations belong to that outcome's wire
        // expectation.
        if observation == Observation::Kind(expected)
            && let Some(expectation) = binding.outcome(expected)
        {
            // The SAME merged scope request building used (#1852): a step
            // that supplies an identity inline (`with:`) instead of capturing
            // it must be able to pin it in its outcome matchers too.
            // NOTE: a name in `crate::exec::headers::structural_token` outranks
            // that scope — ITS-REST `operations/composition_get.yaml` lets a
            // `uid_based_id` argument be spelled two ways, so it is not an identity.
            assertion_failures.extend(self.eval_wire_expectation(
                expectation,
                &exchange,
                &headers,
                &header_vars,
            ));
        }
        Ok(StepObservation {
            observation,
            assertion_failures,
        })
    }

    fn provision(
        &mut self,
        case: &CaseCore,
        row: usize,
        vars: &mut VarStore,
    ) -> Result<Provisioned, String> {
        self.resolver.bind_row(case, row);
        self.committed.clear();
        self.last_body = None;
        self.last_version_uid = None;
        // server: empty — isolation is the runner's tenancy concern; against
        // a shared SUT the run is recorded as scoped (never destructive).
        // templates: upload each via the upload_opt binding.
        for key in case.requires.templates.clone() {
            let payload = self.resolver.data_set(&key).map_err(|e| e.to_string())?;
            // direct send through the upload binding matching the corpus
            // format (409 tolerated: already provisioned): OPT 1.4 XML goes
            // to the ADL 1.4 endpoint, ADL2 text to the ADL2 one
            let is_adl2 = self
                .resolver
                .corpus_format(&key)
                .is_some_and(|f| matches!(f, crate::vocab::CorpusFormat::Adl2Text));
            let upload_op = if is_adl2 {
                "I_DEFINITION_ADL2.upload_artefact"
            } else {
                "I_DEFINITION_ADL14.upload_opt"
            };
            let binding = self.binding_for(case, upload_op)?;
            let instance = self.provisioning_instance(case)?;
            let request_spec = binding
                .request
                .as_ref()
                .ok_or_else(|| "upload_opt unrealized".to_owned())?;
            let headers = Self::compose_headers(
                self.set,
                self.ixit,
                case,
                None,
                binding,
                instance,
                &VarStore::default(),
                None,
                self.spec_versions,
            )?;
            let base = instance.base_url.trim_end_matches('/');
            let url = format!("{base}{}", request_spec.path.raw());
            // 409 tolerated: already provisioned (the send records it).
            let uploaded = self.send(request_spec.method, &url, &headers, Some(&payload), false)?;
            if let Some(reason) = Self::provisioning_refusal("upload_opt", &uploaded, true) {
                return Ok(Provisioned::RowErrored { reason });
            }
        }
        if let Provisioned::RowNotApplicable { citation } =
            self.provision_synthesized_opt(case, row)?
        {
            return Ok(Provisioned::RowNotApplicable { citation });
        }
        self.provision_ehr(case, vars)?;
        // import: replay a received EHR-Extract (master06 §Copying Case 1 when
        // no EHR was provisioned, Cases 2/3 into the one that was), so a
        // released read of an IMPORTED_VERSION has its foreign version to read.
        if let Provisioned::RowErrored { reason } = self.provision_import(case, vars)? {
            return Ok(Provisioned::RowErrored { reason });
        }
        // directory: provision the FOLDER tree via create_directory.
        if let Some(crate::model::case::DirectoryRequirement::Tree(key)) =
            case.requires.directory.clone()
        {
            let payload = self.resolver.data_set(&key).map_err(|e| e.to_string())?;
            let binding = self.binding_for(case, "I_EHR_DIRECTORY.create_directory")?;
            let instance = self.provisioning_instance(case)?;
            let request_spec = binding
                .request
                .as_ref()
                .ok_or_else(|| "create_directory unrealized".to_owned())?;
            let ehr_id = vars
                .scalar(&CaptureName::parse("ehr_id").map_err(|e| e.to_string())?)
                .ok_or_else(|| "requires.directory without a provisioned ehr".to_owned())?
                .to_owned();
            let headers = Self::compose_headers(
                self.set,
                self.ixit,
                case,
                None,
                binding,
                instance,
                vars,
                None,
                self.spec_versions,
            )?;
            let base = instance.base_url.trim_end_matches('/');
            let path = request_spec.path.raw().replace("{ehr_id}", &ehr_id);
            let url = format!("{base}{path}");
            let exchange = self.send(request_spec.method, &url, &headers, Some(&payload), true)?;
            self.committed.push(payload);
            // the binding names its ETag capture `version_uid`; provisioning
            // publishes it as `directory_version_uid` for the case flows
            if let Some((_, spec)) = binding
                .captures
                .as_deref()
                .unwrap_or_default()
                .iter()
                .find(|(n, _)| matches!(n.as_str(), "directory_version_uid" | "version_uid"))
                && let Some(uid) = Self::extract_capture(&exchange, binding, spec, vars)
            {
                vars.set(
                    CaptureName::parse("directory_version_uid").map_err(|e| e.to_string())?,
                    Captured::Scalar(uid),
                );
            }
        }
        self.provision_party(case, vars)?;
        self.provision_party_relationship(case, vars)?;
        self.provision_commit_sets(case, vars)?;
        Ok(Provisioned::Ready)
    }

    fn postconditions(
        &mut self,
        case: &CaseCore,
        row: usize,
        vars: &mut VarStore,
    ) -> Result<Vec<String>, String> {
        self.resolver.bind_row(case, row);
        let body = self.last_body.clone().unwrap_or(Value::Null);
        let mut failures = Vec::new();
        for assertion in &case.postconditions {
            if assertion.is_aggregate() {
                continue; // law e
            }
            match assertion {
                Assertion::Field {
                    path,
                    equals,
                    not_equals,
                    exists,
                    absent,
                    matches,
                } => {
                    if let Err(AssertionFailure(m)) = self.eval_field_assertion(
                        &body,
                        path,
                        equals.as_ref(),
                        not_equals.as_ref(),
                        *exists,
                        *absent,
                        matches.as_deref(),
                        vars,
                    ) {
                        failures.push(m);
                    }
                }
                // Equivalent/returns/result_set/instance_of postconditions
                // ride the flow's read step (the verification carrier);
                // version/signature need versioned-object reads (registered
                // exceptions in the run report); unique is aggregate (law e);
                // message_exemplar/state are informative.
                Assertion::Equivalent { .. }
                | Assertion::Returns { .. }
                | Assertion::ResultSet { .. }
                | Assertion::InstanceOf { .. }
                | Assertion::XmlRoot { .. }
                | Assertion::Version { .. }
                | Assertion::Signature { .. }
                | Assertion::Unique { .. }
                | Assertion::MessageExemplar { .. }
                | Assertion::State { .. } => {}
            }
        }
        Ok(failures)
    }

    fn aggregates(
        &mut self,
        case: &CaseCore,
        all_rows: &[VarStore],
    ) -> Result<Vec<String>, String> {
        let mut failures = Vec::new();
        for assertion in &case.postconditions {
            if let Assertion::Unique { over, .. } = assertion
                && let ValueRef::Capture { name, .. } = &over.0
                && let Err(AssertionFailure(m)) = assertions::eval_unique(name, all_rows)
            {
                failures.push(m);
            }
        }
        Ok(failures)
    }
}

/// Extract a list capture from a body path of the form
/// `versions[*].id.value`.
fn extract_list(body: &Value, path: &str) -> Vec<String> {
    let mut current = vec![body];
    for seg in path.split('.') {
        let (attr, star) = match seg.strip_suffix("[*]") {
            Some(attr) => (attr, true),
            None => (seg, false),
        };
        let mut next = Vec::new();
        for v in current {
            let v = if attr.is_empty() {
                Some(v)
            } else {
                v.get(attr)
            };
            if let Some(v) = v {
                if star {
                    if let Some(items) = v.as_array() {
                        next.extend(items.iter());
                    }
                } else {
                    next.push(v);
                }
            }
        }
        current = next;
    }
    current
        .into_iter()
        .filter_map(|v| match v {
            Value::String(s) => Some(s.clone()),
            other => other.as_str().map(ToOwned::to_owned),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A status a message RENDERS reaches the recorded artifacts as a bare
    /// wire number (`docs/conformance/<sut>/results.json` carries
    /// `status 406 maps to no outcome …` rows), so the rendering is pinned
    /// independently of how the status is held in memory.
    #[test]
    fn rendered_statuses_stay_bare_wire_numbers() {
        let refused = Exchange {
            method: "POST".into(),
            path: "/ehr".into(),
            status: StatusCode::CONFLICT,
            headers: BTreeMap::new(),
            body: None,
        };
        let message = HttpDriver::provisioning_refusal("an EHR", &refused, false)
            .expect("a 409 with conflict_is_ground=false is a refusal");
        assert!(
            message.starts_with("provisioning an EHR refused: status 409 —"),
            "{message}"
        );
        assert!(HttpDriver::provisioning_refusal("an EHR", &refused, true).is_none());

        let ok = Exchange {
            status: StatusCode::OK,
            ..refused
        };
        let failure = HttpDriver::eval_returns_assertion(
            &ok,
            &Value::Null,
            Some(&Value::Bool(false)),
            None,
            None,
        )
        .expect_err("a 2xx exchange contradicts `returns: false`");
        assert_eq!(
            failure.0,
            "returns: wire presence true != expected false (status 200)"
        );
    }

    /// Preconditions follow the DEPLOYMENT the flow addresses, and the
    /// discriminator is the origin — a party declares several instances per
    /// deployment (principals; the SMART Platform base path) and one instance
    /// per extra deployment (the second signing posture).
    #[test]
    fn same_deployment_compares_origins_not_paths() {
        // Same server, different API base path (sut vs smart_platform).
        assert!(same_deployment(
            "http://localhost:8080/ferroehr/rest/openehr/v1",
            "http://localhost:8080/ferroehr/rest"
        ));
        // Same server, trailing-slash noise.
        assert!(same_deployment(
            "http://localhost:8080/",
            "http://localhost:8080"
        ));
        // The default port is the same port.
        assert!(same_deployment("http://host/api", "http://host:80/api"));
        // A second deployment on another port is NOT the same deployment.
        assert!(!same_deployment(
            "http://localhost:8081/ferroehr/rest/openehr/v1",
            "http://localhost:8080/ferroehr/rest/openehr/v1"
        ));
        // Different host, and different scheme, likewise.
        assert!(!same_deployment(
            "http://other:8080/x",
            "http://localhost:8080/x"
        ));
        assert!(!same_deployment(
            "https://localhost/x",
            "http://localhost/x"
        ));
        // Unparseable values fall back to exact comparison, never a guess.
        assert!(same_deployment("not a url", "not a url"));
        assert!(!same_deployment("not a url", "also not a url"));
    }

    /// The committed CNF SMART test issuer (`tools/cnf-runner/party/smart/`) —
    /// public test material by design, never production key material.
    fn test_mint(roles: &[String]) -> crate::ixit::BearerMint {
        let key_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("party/smart/cnf-smart-test.key.pem");
        serde_json::from_value(serde_json::json!({
            "issuer": "https://as.cnf.test",
            "audience": "cnf-smart-sut",
            "subject": "cnf-smart-app",
            "roles": roles,
            "key_file": key_file,
            "kid": "cnf-smart-test",
            "ttl_seconds": 300
        }))
        .unwrap()
    }

    /// Verify a minted token against the COMMITTED PUBLIC JWKS — the same
    /// document the compose overlay mounts as the SUT's
    /// `auth.oidc.jwks_json_file` — so the key pair itself is under test, not
    /// just the encoder. Mirrors the SUT's own resolution
    /// (`kid` → JWKS entry → `DecodingKey::from_jwk`).
    fn decode_against_committed_jwks(token: &str) -> serde_json::Map<String, Value> {
        let header = jsonwebtoken::decode_header(token).unwrap();
        assert_eq!(header.alg, jsonwebtoken::Algorithm::RS256);
        assert_eq!(header.kid.as_deref(), Some("cnf-smart-test"));

        let jwks_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("party/smart/jwks.json");
        let jwks: jsonwebtoken::jwk::JwkSet =
            serde_json::from_str(&std::fs::read_to_string(jwks_path).unwrap()).unwrap();
        let jwk = jwks.find("cnf-smart-test").unwrap();
        let key = jsonwebtoken::DecodingKey::from_jwk(jwk).unwrap();
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_issuer(&["https://as.cnf.test"]);
        validation.set_audience(&["cnf-smart-sut"]);
        jsonwebtoken::decode::<serde_json::Map<String, Value>>(token, &key, &validation)
            .unwrap()
            .claims
    }

    /// The minted token is a real RS256 JWT the SUT's own validator reads:
    /// `kid`-tagged header, the registered claims, the master08 `scope` claim
    /// space-delimited (RFC 6749 §3.3), and the RBAC role claim.
    #[test]
    fn mint_signs_a_scope_carrying_rs256_token() {
        let mint = test_mint(&["USER".to_owned()]);
        let token = mint_access_token(
            &mint,
            None,
            None,
            &["user/template-*.r".to_owned(), "openid".to_owned()],
        )
        .unwrap();
        let claims = decode_against_committed_jwks(&token);

        assert_eq!(claims["sub"], Value::from("cnf-smart-app"));
        assert_eq!(claims["scope"], Value::from("user/template-*.r openid"));
        assert_eq!(claims["realm_access"]["roles"][0], Value::from("USER"));
        assert!(claims["exp"].as_i64().unwrap() > claims["iat"].as_i64().unwrap());
    }

    /// An empty scope declaration is a DIFFERENT request from "no SMART
    /// step": it mints the scope-less token the fail-closed deny branch needs
    /// (master08 §Scopes ¶2), so the claim is present and empty.
    #[test]
    fn mint_emits_an_empty_scope_claim_for_a_scopeless_token() {
        let token = mint_access_token(&test_mint(&["USER".to_owned()]), None, None, &[]).unwrap();
        let claims = decode_against_committed_jwks(&token);
        assert_eq!(claims["scope"], Value::from(""));
    }

    /// `bearer_mint` without a declared lane is a runner-visible authoring
    /// defect, never a silently un-authenticated request.
    #[test]
    fn bearer_mint_without_a_lane_is_an_error() {
        let ixit: Ixit = serde_json::from_value(serde_json::json!({
            "instances": { "sut": { "base_url": "http://x", "auth": { "mode": "bearer_mint" } } }
        }))
        .unwrap();
        let no_scopes: &[String] = &[];
        let error = HttpDriver::auth_header(
            &ixit,
            &AuthMode::BearerMint {
                subject: None,
                roles: None,
                default_scopes: Vec::new(),
            },
            Some(no_scopes),
        )
        .unwrap_err();
        assert!(error.contains("no `smart` lane"), "{error}");
    }

    fn test_binding() -> OperationBinding {
        serde_json::from_value(serde_json::json!({
            "sm_operation": "I_EHR_SERVICE.create_ehr",
            "its": "its-rest",
            "request": { "method": "POST", "path": "/ehr" },
            "outcomes": { "created": { "status": 201 } },
            "captures": {
                "version_uid": { "from": "header ETag", "strip": "weak-quotes" }
            }
        }))
        .unwrap()
    }

    /// The run-2 triage regression (2026-07-28): a step's explicit `with:`
    /// value must SHADOW a same-named earlier capture in the step's template
    /// scope. The old keep-the-capture guard rendered a STALE
    /// `preceding_version_uid` into an If-Match header (step 1's capture
    /// instead of the newer uid step 4 passed), and the SUT's correct 412
    /// showed up as a red row.
    #[test]
    fn with_value_shadows_same_named_capture_in_step_scope() {
        let mut vars = VarStore::default();
        vars.set(
            CaptureName::parse("preceding_version_uid").unwrap(),
            Captured::Scalar("vo::sys::1".to_owned()),
        );
        let mut with = BTreeMap::new();
        with.insert(
            "preceding_version_uid".to_owned(),
            serde_json::json!("vo::sys::2"),
        );
        let merged = HttpDriver::merge_with_vars(&vars, &with);
        assert_eq!(
            merged.scalar(&CaptureName::parse("preceding_version_uid").unwrap()),
            Some("vo::sys::2"),
            "the step's explicit with: value wins in its own scope"
        );
        // The underlying store is untouched — the shadow is step-scoped.
        assert_eq!(
            vars.scalar(&CaptureName::parse("preceding_version_uid").unwrap()),
            Some("vo::sys::1")
        );
    }

    /// The group-9 triage regression: a NUMBER-typed `with:` value must
    /// promote into the template vars and render on an optional URL slot —
    /// `url_fetch: 4` reaching `${url_fetch?}` emits `?fetch=4`; silently
    /// skipping non-string scalars dropped the parameter and masqueraded as
    /// a SUT failure.
    /// The #594 regression: an optional query slot whose referenced name IS
    /// bound — in the var store as a non-scalar capture (List/Body) — must be
    /// a loud error, never a silent omission (the dropped-bound-parameter
    /// false-green shape). A genuinely unbound optional slot still omits.
    #[test]
    fn a_bound_nonscalar_capture_on_an_optional_slot_is_loud() {
        let binding: OperationBinding = serde_json::from_value(serde_json::json!({
            "sm_operation": "I_QUERY_SERVICE.execute_ad_hoc_query",
            "its": "its-rest",
            "request": {
                "method": "POST",
                "path": "/query/aql",
                "query": { "fetch": "${url_fetch?}" }
            },
            "outcomes": { "ok": { "status": 200 } }
        }))
        .unwrap();
        // Bound in the VAR STORE (not the step's with:) as a List — the shape
        // the pre-#594 guard missed.
        let mut vars = VarStore::default();
        vars.set(
            CaptureName::parse("url_fetch").unwrap(),
            Captured::List(vec!["a".to_owned(), "b".to_owned()]),
        );
        let with = BTreeMap::new();
        let err = HttpDriver::build_url(&binding, "http://sut", &with, &vars).unwrap_err();
        assert!(err.contains("did not render as a scalar"), "{err}");
        // Unbound: the parameter is omitted, no error.
        let vars = VarStore::default();
        let url = HttpDriver::build_url(&binding, "http://sut", &with, &vars).unwrap();
        assert_eq!(url, "http://sut/query/aql");
    }

    #[test]
    fn numeric_with_values_render_on_optional_url_slots() {
        let binding: OperationBinding = serde_json::from_value(serde_json::json!({
            "sm_operation": "I_QUERY_SERVICE.execute_ad_hoc_query",
            "its": "its-rest",
            "request": {
                "method": "POST",
                "path": "/query/aql",
                "query": { "fetch": "${url_fetch?}" }
            },
            "outcomes": { "ok": { "status": 200 } }
        }))
        .unwrap();
        let mut with = BTreeMap::new();
        with.insert("url_fetch".to_owned(), serde_json::json!(4));
        let vars = HttpDriver::merge_with_vars(&VarStore::default(), &with);
        let url = HttpDriver::build_url(&binding, "http://sut", &with, &vars).unwrap();
        assert_eq!(url, "http://sut/query/aql?fetch=4");

        // An unbound optional slot still omits the parameter.
        let empty = BTreeMap::new();
        let vars = HttpDriver::merge_with_vars(&VarStore::default(), &empty);
        let url = HttpDriver::build_url(&binding, "http://sut", &empty, &vars).unwrap();
        assert_eq!(url, "http://sut/query/aql");

        // A bound-but-unrenderable (object) value is LOUD, never a drop.
        let mut with = BTreeMap::new();
        with.insert("url_fetch".to_owned(), serde_json::json!({"n": 4}));
        let vars = HttpDriver::merge_with_vars(&VarStore::default(), &with);
        let err = HttpDriver::build_url(&binding, "http://sut", &with, &vars).unwrap_err();
        assert!(err.contains("did not render as a scalar"), "{err}");
    }

    /// The RFC 6570 exploded form (`?p=a&p=b`): a binding declares the
    /// parameter as a SEQUENCE of templates, each member contributes its own
    /// pair, and an unbound optional member is simply absent — so one
    /// declaration serves the whole-set, one-id and two-id calls of the admin
    /// bulk delete.
    #[test]
    fn repeated_query_parameters_render_one_pair_per_member() {
        let binding: OperationBinding = serde_json::from_value(serde_json::json!({
            "sm_operation": "I_ADMIN_SERVICE.physical_ehr_delete",
            "its": "its-rest",
            "variant": "delete_all",
            "request": {
                "method": "DELETE",
                "path": "/admin/ehr/all",
                "query": { "ehr_id": ["${ehr_id_subset?}", "${ehr_id_subset_2?}"] }
            },
            "outcomes": { "ok_empty": { "status": 204 } }
        }))
        .unwrap();

        // Neither member bound: no query at all (the whole-set call).
        let empty = BTreeMap::new();
        let vars = HttpDriver::merge_with_vars(&VarStore::default(), &empty);
        assert_eq!(
            HttpDriver::build_url(&binding, "http://sut", &empty, &vars).unwrap(),
            "http://sut/admin/ehr/all"
        );

        // One member bound: the one-id subset.
        let mut with = BTreeMap::new();
        with.insert("ehr_id_subset".to_owned(), serde_json::json!("a"));
        let vars = HttpDriver::merge_with_vars(&VarStore::default(), &with);
        assert_eq!(
            HttpDriver::build_url(&binding, "http://sut", &with, &vars).unwrap(),
            "http://sut/admin/ehr/all?ehr_id=a"
        );

        // Both bound: the repeated form, in authored member order.
        with.insert("ehr_id_subset_2".to_owned(), serde_json::json!("b"));
        let vars = HttpDriver::merge_with_vars(&VarStore::default(), &with);
        assert_eq!(
            HttpDriver::build_url(&binding, "http://sut", &with, &vars).unwrap(),
            "http://sut/admin/ehr/all?ehr_id=a&ehr_id=b"
        );

        // A member bound to a LIST capture expands element-wise, so the
        // declaration is not capped at its authored arity.
        let mut vars = VarStore::default();
        vars.set(
            CaptureName::parse("ehr_id_subset").unwrap(),
            Captured::List(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]),
        );
        assert_eq!(
            HttpDriver::build_url(&binding, "http://sut", &empty, &vars).unwrap(),
            "http://sut/admin/ehr/all?ehr_id=a&ehr_id=b&ehr_id=c"
        );
    }

    /// A single-valued declaration never becomes repeated by accident: a list
    /// bound under its name is a loud authoring error, never a silently
    /// JSON-encoded `?p=%5B%22a%22%5D`.
    #[test]
    fn a_single_valued_query_parameter_refuses_a_list() {
        let binding: OperationBinding = serde_json::from_value(serde_json::json!({
            "sm_operation": "I_ADMIN_SERVICE.physical_ehr_delete",
            "its": "its-rest",
            "request": {
                "method": "DELETE",
                "path": "/admin/ehr/all",
                "query": { "ehr_id": "${ehr_id_subset?}" }
            },
            "outcomes": { "ok_empty": { "status": 204 } }
        }))
        .unwrap();
        let mut with = BTreeMap::new();
        with.insert("ehr_id".to_owned(), serde_json::json!(["a", "b"]));
        let vars = HttpDriver::merge_with_vars(&VarStore::default(), &with);
        let err = HttpDriver::build_url(&binding, "http://sut", &with, &vars).unwrap_err();
        assert!(err.contains("single-valued parameter"), "{err}");

        // …and a list capture never EXPANDS a single-valued declaration:
        // repeatability is the binding's decision, not the case's. Since
        // #594 the refusal is LOUD — the pre-#594 behaviour (silently
        // omitting the bound parameter) was the dropped-bound-parameter
        // false-green shape; an error preserves this arm's intent (no
        // expansion) without the silent drop.
        let mut vars = VarStore::default();
        vars.set(
            CaptureName::parse("ehr_id_subset").unwrap(),
            Captured::List(vec!["a".to_owned(), "b".to_owned()]),
        );
        let empty = BTreeMap::new();
        let err = HttpDriver::build_url(&binding, "http://sut", &empty, &vars).unwrap_err();
        assert!(err.contains("did not render as a scalar"), "{err}");
    }

    /// A `Patched` binding whose captured base body is NOT a JSON object
    /// (the canonical-XML capture shape: a `Value::String`) must fail loudly.
    /// Silently skipping the declared `set:` writes the resource back
    /// unmutated and the step "passes" while exercising nothing.
    #[test]
    fn patched_body_on_a_non_object_capture_is_loud() {
        let from = CaptureName::parse("status_body").unwrap();
        let set = vec![("is_queryable".to_owned(), serde_json::json!(false))];

        let mut xml_vars = VarStore::default();
        xml_vars.set(
            from.clone(),
            Captured::Body(Value::String(
                "<ehr_status><is_queryable>true</is_queryable></ehr_status>".to_owned(),
            )),
        );
        let err = HttpDriver::patched_body(&from, &set, &xml_vars).unwrap_err();
        assert!(err.contains("not a JSON object"), "{err}");
        assert!(err.contains("is_queryable"), "{err}");

        // The object path still patches (and only the declared fields).
        let mut json_vars = VarStore::default();
        json_vars.set(
            from.clone(),
            Captured::Body(serde_json::json!({
                "_type": "EHR_STATUS", "is_queryable": true, "is_modifiable": true
            })),
        );
        let patched = HttpDriver::patched_body(&from, &set, &json_vars).unwrap();
        assert_eq!(patched.get("is_queryable"), Some(&serde_json::json!(false)));
        assert_eq!(patched.get("is_modifiable"), Some(&serde_json::json!(true)));

        // An unbound EHR_STATUS capture keeps its minimal-resource fallback.
        let fallback = HttpDriver::patched_body(&from, &set, &VarStore::default()).unwrap();
        assert_eq!(
            fallback.get("_type"),
            Some(&serde_json::json!("EHR_STATUS"))
        );
        assert_eq!(
            fallback.get("is_queryable"),
            Some(&serde_json::json!(false))
        );

        // An unbound capture with no fallback role is still loud.
        let other = CaptureName::parse("composition_body").unwrap();
        let err = HttpDriver::patched_body(&other, &set, &VarStore::default()).unwrap_err();
        assert!(err.contains("holds no resource body"), "{err}");
    }

    /// A `666|attestation|` member is the attestation wire shape RM common
    /// `master06-change_control_package.adoc` §Contributions describes: it
    /// commits no new version, so it carries NEITHER `data` NOR a version
    /// `lifecycle_state`; it names its target with `preceding_version_uid`
    /// and its `commit_audit` is the fixture's `UPDATE_ATTESTATION`, carried
    /// verbatim. The envelope audit reports the attestation-only aggregate
    /// (§Contributions: "`666|attestation|`: used when the only changes are
    /// attestation of one or more of the member versions").
    #[test]
    fn an_attestation_member_commits_no_version_and_carries_the_fixture_audit() {
        let envelope = HttpDriver::contribution_envelope(
            &[serde_json::json!({
                "change_type": "attestation",
                "preceding_version_uid": "8849182c-82ad-4088-a07f-48ead4180515::cnf::1",
                "attestation": {
                    "_type": "UPDATE_ATTESTATION",
                    "reason": { "_type": "DV_TEXT", "value": "witnessed" },
                    "is_pending": false
                }
            })],
            None,
        )
        .expect("the attestation member builds");
        let member = &envelope["versions"][0];
        assert_eq!(member["_type"], serde_json::json!("ORIGINAL_VERSION"));
        assert!(member.get("data").is_none(), "no content: {member}");
        assert!(
            member.get("lifecycle_state").is_none(),
            "no new version, so no version lifecycle: {member}"
        );
        assert_eq!(
            member["preceding_version_uid"]["value"],
            serde_json::json!("8849182c-82ad-4088-a07f-48ead4180515::cnf::1")
        );
        let audit = &member["commit_audit"];
        assert_eq!(audit["_type"], serde_json::json!("UPDATE_ATTESTATION"));
        assert_eq!(audit["reason"]["value"], serde_json::json!("witnessed"));
        assert_eq!(audit["is_pending"], serde_json::json!(false));
        // the UPDATE_AUDIT parts the fixture left unstated
        assert_eq!(
            audit["change_type"]["defining_code"]["code_string"],
            serde_json::json!("666")
        );
        assert!(audit["committer"]["name"].is_string(), "{audit}");
        // the attestation-only aggregate
        assert_eq!(
            envelope["audit"]["change_type"]["defining_code"]["code_string"],
            serde_json::json!("666")
        );
    }

    /// An INVALID attestation fixture reaches the wire unrepaired — the runner
    /// fills only the `UPDATE_AUDIT` parts, never the `UPDATE_ATTESTATION`
    /// attributes under test, so a refusal case exercises the server's
    /// `ATTESTATION` invariants rather than the runner's defaults.
    #[test]
    fn an_invalid_attestation_fixture_is_not_repaired() {
        let envelope = HttpDriver::contribution_envelope(
            &[serde_json::json!({
                "change_type": "attestation",
                "preceding_version_uid": "8849182c-82ad-4088-a07f-48ead4180515::cnf::1",
                "attestation": { "_type": "UPDATE_ATTESTATION", "is_pending": false, "items": [] }
            })],
            None,
        )
        .expect("the attestation member builds");
        let audit = &envelope["versions"][0]["commit_audit"];
        assert!(
            audit.get("reason").is_none(),
            "reason stays absent: {audit}"
        );
        assert_eq!(audit["items"], serde_json::json!([]));
    }

    /// A change set that is NOT attestation-only keeps the creation aggregate.
    #[test]
    fn a_mixed_change_set_keeps_the_creation_aggregate() {
        let envelope = HttpDriver::contribution_envelope(&[
            serde_json::json!({ "change_type": "creation", "data": { "_type": "COMPOSITION" } }),
            serde_json::json!({
                "change_type": "attestation",
                "preceding_version_uid": "8849182c-82ad-4088-a07f-48ead4180515::cnf::1",
                "attestation": { "is_pending": true, "reason": { "value": "witnessed" } }
            }),
        ], None)
        .expect("the mixed change set builds");
        assert_eq!(
            envelope["audit"]["change_type"]["defining_code"]["code_string"],
            serde_json::json!("249")
        );
    }

    /// The two member-level overrides, each closed: `_type` puts the class
    /// self-tag ITS-REST `docs/overview/Resources.md` §Resource representation
    /// permits on the member (the AMB-89 `IMPORTED_VERSION` refusal branch is
    /// authorable only through it), and `lifecycle_state` fixes the committed
    /// state independently of the change kind (RM common master06 §Version
    /// Lifecycle transitions).
    #[test]
    fn member_overrides_are_closed_vocabularies() {
        let envelope = HttpDriver::contribution_envelope(
            &[serde_json::json!({
                "change_type": "modification",
                "_type": "IMPORTED_VERSION",
                "lifecycle_state": "abandoned",
                "data": { "_type": "COMPOSITION" }
            })],
            None,
        )
        .expect("the overridden member builds");
        let member = &envelope["versions"][0];
        assert_eq!(member["_type"], serde_json::json!("IMPORTED_VERSION"));
        assert_eq!(
            member["lifecycle_state"]["defining_code"]["code_string"],
            serde_json::json!("801")
        );
        assert_eq!(
            member["lifecycle_state"]["value"],
            serde_json::json!("abandoned")
        );
        // the change kind still fixes the audit change type
        assert_eq!(
            member["commit_audit"]["change_type"]["defining_code"]["code_string"],
            serde_json::json!("251")
        );

        // Absent overrides keep the derived shape.
        let derived = HttpDriver::contribution_envelope(
            &[serde_json::json!({ "change_type": "creation", "data": { "_type": "COMPOSITION" } })],
            None,
        )
        .expect("the derived member builds");
        assert_eq!(
            derived["versions"][0]["_type"],
            serde_json::json!("ORIGINAL_VERSION")
        );
        assert_eq!(
            derived["versions"][0]["lifecycle_state"]["defining_code"]["code_string"],
            serde_json::json!("532")
        );

        // Out-of-vocabulary tokens are refused, never silently defaulted.
        for bad in [
            serde_json::json!({ "change_type": "creation", "_type": "CONTRIBUTION" }),
            serde_json::json!({ "change_type": "creation", "lifecycle_state": "finished" }),
        ] {
            assert!(
                HttpDriver::contribution_envelope(std::slice::from_ref(&bad), None).is_err(),
                "{bad} must be refused"
            );
        }

        // A verbatim member already spells its own shape — an override beside
        // it would state it twice.
        assert!(
            HttpDriver::contribution_envelope(
                &[serde_json::json!({
                    "_type": "ORIGINAL_VERSION",
                    "data": { "_type": "ORIGINAL_VERSION" }
                })],
                None
            )
            .is_err()
        );
        // An attestation member commits no version, so it has no lifecycle.
        assert!(
            HttpDriver::contribution_envelope(
                &[serde_json::json!({
                    "change_type": "attestation",
                    "lifecycle_state": "complete",
                    "attestation": { "is_pending": false }
                })],
                None
            )
            .is_err()
        );
    }

    /// A case's `audit:` override states only its delta against the derived
    /// commit audit, and the `absent` sentinel omits a member outright — the
    /// seam the mandatory-member refusals need (RM common
    /// `UML/classes/org.openehr.rm.common.audit_details.adoc` §Attributes:
    /// `change_type` and `committer` are 1..1; released OAS
    /// `specifications/schemas/common/UpdateAudit.yaml` §required lists both).
    #[test]
    fn a_case_overrides_or_omits_the_commit_audit() {
        let member = serde_json::json!({ "data": { "_type": "COMPOSITION" } });
        let build = |audit: Option<Value>| {
            HttpDriver::contribution_envelope(std::slice::from_ref(&member), audit.as_ref())
        };

        // No override: the derived audit, unchanged.
        let derived = build(None).expect("the derived envelope builds");
        assert_eq!(derived["audit"]["change_type"]["value"], "creation");
        assert_eq!(derived["audit"]["committer"]["name"], "cnf runner");

        // `absent` omits the member entirely — the omitted-change_type and
        // omitted-committer refusal shapes.
        let omitted = build(Some(serde_json::json!({ "change_type": "absent" })))
            .expect("the omission builds");
        assert!(omitted["audit"].get("change_type").is_none(), "{omitted}");
        assert!(omitted["audit"].get("committer").is_some(), "{omitted}");
        let omitted =
            build(Some(serde_json::json!({ "committer": "absent" }))).expect("the omission builds");
        assert!(omitted["audit"].get("committer").is_none(), "{omitted}");

        // A verbatim value replaces the derived one — an out-of-group
        // `change_type` code the closed vocabulary cannot spell.
        let overridden = build(Some(serde_json::json!({
            "change_type": {
                "_type": "DV_CODED_TEXT",
                "value": "not a change type",
                "defining_code": {
                    "_type": "CODE_PHRASE",
                    "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                    "code_string": "999"
                }
            }
        })))
        .expect("the override builds");
        assert_eq!(
            overridden["audit"]["change_type"]["defining_code"]["code_string"],
            "999"
        );

        // A non-object override is refused, never silently ignored.
        assert!(build(Some(serde_json::json!("nonsense"))).is_err());
    }

    /// The identities a `requires.import` mints come from the EXTRACT itself
    /// — master06 §Copying keeps the received version container's identity —
    /// and name ONE versioned object: the content item of the class the
    /// precondition declares, with its latest trunk and latest branch
    /// position.
    #[test]
    fn imported_identities_are_read_from_the_extract() {
        let key = crate::ids::CorpusKey::parse("cnf.messaging.ehr_extract.v1").unwrap();
        let version =
            |uid: &str| serde_json::json!({ "_type": "ORIGINAL_VERSION", "uid": { "value": uid } });
        let extract = serde_json::json!({
            "chapters": [ { "items": [
                { "item": { "_type": "X_VERSIONED_EHR_STATUS",
                            "uid": { "value": "status-vo" },
                            "versions": [version("status-vo::src::1")] } },
                { "item": { "_type": "X_VERSIONED_COMPOSITION",
                            "uid": { "value": "comp-vo" },
                            "versions": [
                                version("comp-vo::src::1"),
                                version("comp-vo::src::2"),
                                version("comp-vo::other::1.1.1")
                            ] } }
            ] } ]
        });
        let minted = HttpDriver::imported_identities(
            &extract,
            crate::vocab::XVersionedClass::Composition,
            &key,
        )
        .expect("the composition container mints");
        assert_eq!(
            minted,
            vec![
                ("imported_versioned_object_uid", "comp-vo".to_owned()),
                ("imported_version_uid", "comp-vo::src::2".to_owned()),
                (
                    "imported_branch_version_uid",
                    "comp-vo::other::1.1.1".to_owned()
                ),
            ]
        );

        // The sibling container of the SAME extract is addressable by naming
        // its class — position never decides.
        let status = HttpDriver::imported_identities(
            &extract,
            crate::vocab::XVersionedClass::EhrStatus,
            &key,
        )
        .expect("the status container mints");
        assert_eq!(status[0].1, "status-vo");
        assert_eq!(status.len(), 2, "trunk only, no branch: {status:?}");

        // A class the extract does not carry — or carries twice — names no
        // single versioned object, and that is a loud provisioning error.
        assert!(
            HttpDriver::imported_identities(&extract, crate::vocab::XVersionedClass::Folder, &key)
                .is_err()
        );
        let doubled = serde_json::json!({
            "chapters": [ { "items": [
                { "item": { "_type": "X_VERSIONED_FOLDER", "uid": { "value": "a" },
                            "versions": [version("a::src::1")] } },
                { "item": { "_type": "X_VERSIONED_FOLDER", "uid": { "value": "b" },
                            "versions": [version("b::src::1")] } }
            ] } ]
        });
        assert!(
            HttpDriver::imported_identities(&doubled, crate::vocab::XVersionedClass::Folder, &key)
                .is_err()
        );
    }

    #[test]
    fn base64_and_list_extraction() {
        assert_eq!(
            base64::engine::general_purpose::STANDARD.encode(b"user:pass"),
            "dXNlcjpwYXNz"
        );
        let body = serde_json::json!({
            "versions": [ { "id": { "value": "v1" } }, { "id": { "value": "v2" } } ]
        });
        assert_eq!(
            extract_list(&body, "versions[*].id.value"),
            vec!["v1", "v2"]
        );
    }

    #[test]
    fn capture_extraction_strips_and_transforms() {
        let exchange = Exchange {
            method: "POST".into(),
            path: "/ehr".into(),
            status: StatusCode::CREATED,
            headers: BTreeMap::from([(
                "etag".to_owned(),
                "W/\"8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::1\"".to_owned(),
            )]),
            body: None,
        };
        let spec: WireCapture = serde_json::from_value(serde_json::json!({
            "from": "header ETag", "strip": "weak-quotes"
        }))
        .unwrap();
        let vars = VarStore::default();
        let uid = HttpDriver::extract_capture(&exchange, &test_binding(), &spec, &vars).unwrap();
        assert_eq!(
            uid,
            "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::1"
        );

        let spec2: WireCapture = serde_json::from_value(serde_json::json!({
            "from": "capture version_uid", "transform": "root-uid"
        }))
        .unwrap();
        let mut vars2 = VarStore::default();
        vars2.set(
            CaptureName::parse("version_uid").unwrap(),
            Captured::Scalar(uid),
        );
        let root = HttpDriver::extract_capture(&exchange, &test_binding(), &spec2, &vars2).unwrap();
        assert_eq!(root, "8849182c-82ad-4088-a07f-48ead4180515");

        // The middle segment — the creating system id (#570).
        let spec3: WireCapture = serde_json::from_value(serde_json::json!({
            "from": "header ETag", "strip": "weak-quotes",
            "transform": "creating-system-id"
        }))
        .unwrap();
        let system =
            HttpDriver::extract_capture(&exchange, &test_binding(), &spec3, &vars).unwrap();
        assert_eq!(system, "openEHRSys.example.com");

        // A value with no middle segment binds NOTHING rather than binding
        // the whole value as if it were a system id.
        let truncated = Exchange {
            headers: BTreeMap::from([("etag".to_owned(), "\"8849182c\"".to_owned())]),
            ..exchange
        };
        assert!(HttpDriver::extract_capture(&truncated, &test_binding(), &spec3, &vars).is_none());
    }

    /// The committal-metadata spelling selection (overview
    /// `Requests_and_responses.md` §Deprecated headers): a party declaring an
    /// ITS-REST release before 1.1.0 gets the pre-1.1.0 spellings for the two
    /// genuinely distinct fields (underscore vs hyphen under RFC 9110 §5.1);
    /// everyone else — including an undeclared party — keeps the canonical
    /// names, and a deliberately deprecated-authored name is never touched.
    #[test]
    fn committal_header_spelling_follows_the_declared_its_rest_release() {
        let headers = BTreeMap::from([
            ("openehr-audit-details".to_owned(), "a".to_owned()),
            ("openehr-template-id".to_owned(), "t".to_owned()),
            // Case-insensitively identical to its replacement (same field) —
            // never rewritten.
            ("openEHR-VERSION".to_owned(), "v".to_owned()),
        ]);
        let v103 = crate::party::SpecVersions {
            its_rest: Some("1.0.3".to_owned()),
            ..crate::party::SpecVersions::default()
        };
        let spelled = HttpDriver::spell_committal_headers(headers.clone(), Some(&v103));
        assert_eq!(
            spelled.get("openEHR-AUDIT_DETAILS").map(String::as_str),
            Some("a"),
            "the hyphenated 1.1.0 audit field is rewritten to the pre-1.1.0 spelling"
        );
        assert_eq!(
            spelled.get("openEHR-TEMPLATE_ID").map(String::as_str),
            Some("t"),
            "the hyphenated 1.1.0 template field is rewritten to the pre-1.1.0 spelling"
        );
        assert!(!spelled.contains_key("openehr-audit-details"));
        assert!(!spelled.contains_key("openehr-template-id"));
        assert_eq!(
            spelled.get("openEHR-VERSION").map(String::as_str),
            Some("v")
        );

        // A deliberately DEPRECATED-authored name (the backward-compatibility
        // cases) case-folds to the underscore form, which the map does not
        // contain — never touched, for either party.
        let deprecated = BTreeMap::from([("openEHR-AUDIT_DETAILS".to_owned(), "old".to_owned())]);
        let kept = HttpDriver::spell_committal_headers(deprecated.clone(), Some(&v103));
        assert_eq!(kept, deprecated);

        // 1.1.0 party: untouched.
        let v110 = crate::party::SpecVersions {
            its_rest: Some("1.1.0".to_owned()),
            ..crate::party::SpecVersions::default()
        };
        let same = HttpDriver::spell_committal_headers(headers.clone(), Some(&v110));
        assert!(same.contains_key("openehr-audit-details"));
        assert!(same.contains_key("openehr-template-id"));

        // Undeclared: untouched (scope is the version floors' job).
        let none = HttpDriver::spell_committal_headers(headers, None);
        assert!(none.contains_key("openehr-audit-details"));
    }
}
