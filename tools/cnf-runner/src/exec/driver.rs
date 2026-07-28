//! The live HTTP driver — a [`crate::exec::StepDriver`] realized PURELY
//! from the operation bindings: request construction (method, path/query
//! templates, format headers, `Prefer`/`If-Match` discipline), wire
//! observation classification, capture extraction per the closed
//! capture-source grammar, and the assertion evaluators. Nothing here
//! hard-codes an endpoint: a case executes because its bindings say how.

use std::collections::BTreeMap;

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
use crate::model::case::{CaseCore, EhrRequirement, FlowStep};
use crate::refgrammar::{CaptureField, Template, ValueRef};
use crate::vocab::{FormatName, HttpMethod, OutcomeKind};

/// One captured HTTP exchange (also the transcript-recording seam).
#[derive(Debug, Clone)]
pub struct Exchange {
    pub method: String,
    pub path: String,
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Option<Value>,
}

/// The live driver.
pub struct HttpDriver<'a> {
    set: &'a ArtifactSet,
    ixit: &'a Ixit,
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
    pub fn new(set: &'a ArtifactSet, ixit: &'a Ixit) -> Result<Self, String> {
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
            client,
            resolver: Resolver::new(manifest, corpus_dir, Some(ixit)),
            committed: Vec::new(),
            last_body: None,
            last_version_uid: None,
            exchanges: Vec::new(),
        })
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
                    .map_err(|_| format!("credential env {user_env} unset"))?;
                let pass = std::env::var(password_env)
                    .map_err(|_| format!("credential env {password_env} unset"))?;
                let token = base64_encode(format!("{user}:{pass}").as_bytes());
                Ok(Some(format!("Basic {token}")))
            }
            AuthMode::Bearer { token_env } => {
                let token = std::env::var(token_env)
                    .map_err(|_| format!("credential env {token_env} unset"))?;
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
                        // silent drop (the group-9 triage, re-found by #594: a
                        // List/Body-bound capture on an optional slot rendered
                        // Err and the parameter vanished — the dropped bound
                        // parameter masquerading as a SUT failure).
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
        let status = response.status().as_u16();
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
        if std::env::var_os("CNF_DEBUG_EXCHANGES").is_some() {
            #[allow(clippy::print_stderr)] // env-gated triage output in the dev tool
            {
                eprintln!(
                    "[exchange] {} {} -> {} | {}",
                    exchange.method,
                    exchange.path,
                    exchange.status,
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

    #[allow(clippy::too_many_arguments)] // mirrors the assertion's field set
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
    #[allow(clippy::too_many_arguments)] // one parameter per declared signature fact — mirrors the assertion shape
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
    ) -> Result<(), AssertionFailure> {
        if let Some(Value::Bool(want)) = equals {
            let observed = (200..300).contains(&exchange.status);
            if observed == *want {
                Ok(())
            } else {
                Err(AssertionFailure(format!(
                    "returns: wire presence {observed} != expected {want} (status {})",
                    exchange.status
                )))
            }
        } else {
            assertions::eval_returns(body, equals, matches)
        }
    }

    /// Evaluate the pure-side assertions for a step against the exchange.
    #[allow(clippy::too_many_lines)] // one match arm per Assertion variant — a dispatch, each arm delegates
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
                    let expected = match to {
                        EquivalentTarget::Committed => self.committed.last().cloned(),
                        EquivalentTarget::Ref(r) => self.resolver.resolve_ref(r, vars).ok(),
                    };
                    match expected {
                        None => Err(AssertionFailure("equivalent: no committed payload".into())),
                        Some(expected) => {
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
                Assertion::Returns { equals, matches } => Self::eval_returns_assertion(
                    exchange,
                    body,
                    equals.as_ref(),
                    matches.as_deref(),
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
fn committed_uids_handle() -> CaptureName {
    // The name is part of the provisioning contract; parse cannot fail.
    CaptureName::parse("committed_uids").unwrap_or_else(|_| unreachable!())
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

/// Minimal base64 (standard alphabet, padding) — avoids a crypto dep for
/// one Basic-auth header.
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
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some((days * 86_400 + h * 3_600 + m * 60 + s) * 1_000)
}

/// Runner-clock milliseconds since the Unix epoch.
fn now_ms() -> i64 {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or_default(),
    )
    .unwrap_or(i64::MAX)
}

fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b = [
            chunk.first().copied().unwrap_or(0),
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        let idx = [(n >> 18) & 63, (n >> 12) & 63, (n >> 6) & 63, n & 63];
        for (i, v) in idx.iter().enumerate() {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[*v as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
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
fn mint_access_token(
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

    let issued_at = now_ms() / 1000;
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

impl HttpDriver<'_> {
    /// Build the canonical CONTRIBUTION envelope from the case model's
    /// bundled `versions:` construct (ITS contribution schema:
    /// `ORIGINAL_VERSION` members carrying `data` + `commit_audit` +
    /// `lifecycle_state`; `change_type` tokens map to the openEHR audit
    /// change-type codes — RM common `§change_control`).
    fn contribution_envelope(versions: &[Value]) -> Value {
        let members: Vec<Value> = versions
            .iter()
            .map(|member| {
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
                    return member.get("data").cloned().unwrap_or(Value::Null);
                }
                let change = member
                    .get("change_type")
                    .and_then(Value::as_str)
                    .unwrap_or("creation");
                let (code, label) = match change {
                    "modification" => ("251", "modification"),
                    "deletion" | "deleted" => ("523", "deleted"),
                    "amendment" => ("250", "amendment"),
                    _ => ("249", "creation"),
                };
                // a deleted member carries NO data and the `deleted`
                // lifecycle (RM common §change_control: version lifecycle
                // 523|deleted|); other members carry 532|complete|
                let (life_code, life_label) = if change == "deleted" || change == "deletion" {
                    ("523", "deleted")
                } else {
                    ("532", "complete")
                };
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
                if let Some(data) = member.get("data")
                    && !data.is_null()
                    && let Some(map) = version.as_object_mut()
                {
                    map.insert("data".to_owned(), data.clone());
                }
                // a list-valued capture (created.version_uids[]) addresses
                // its single member on a one-version commit set
                let preceding = match member.get("preceding_version_uid") {
                    Some(Value::Array(items)) => items.first().cloned(),
                    Some(Value::Null) | None => None,
                    Some(other) => Some(other.clone()),
                };
                if let Some(preceding) = preceding
                    && let Some(map) = version.as_object_mut()
                {
                    map.insert(
                        "preceding_version_uid".to_owned(),
                        serde_json::json!({
                            "_type": "OBJECT_VERSION_ID", "value": preceding
                        }),
                    );
                }
                version
            })
            .collect();
        serde_json::json!({
            "_type": "CONTRIBUTION",
            "versions": members,
            "audit": {
                "_type": "AUDIT_DETAILS",
                "system_id": "cnf-runner",
                "committer": { "_type": "PARTY_IDENTIFIED", "name": "cnf runner" },
                "change_type": { "_type": "DV_CODED_TEXT", "value": "creation",
                    "defining_code": { "_type": "CODE_PHRASE",
                        "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                        "code_string": "249" } }
            }
        })
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
                    return Ok(Some(Self::contribution_envelope(versions)));
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
        // Negatives against a non-existent resource have no captured base
        // body (nothing to GET) — the wire still needs a valid resource
        // payload, so fall back to a minimal RM-VALID canonical EHR_STATUS
        // (the SUT rejects on the unknown id, not the body). The fallback
        // MUST be RM-valid: EHR_STATUS is an unconditional archetype root
        // (RM ehr ehr_status.adoc `Is_archetype_root`) and a root without
        // ARCHETYPED violates `Archetyped_valid` (RM common locatable.adoc,
        // which also fixes archetype_node_id as "the stringified form of the
        // archetype_id found in the archetype_details object") — the old
        // details-less fallback masked a MISSING capture as a fake SUT 422
        // (the 2026-07-28 posture-run triage, finding 7). The masking half
        // of the fix: the fallback applies ONLY to a capture name the case
        // never declared, i.e. the deliberate no-resource negatives; a case
        // that DECLARED the capture and failed to bind it is a loud step
        // error, never a substituted body.
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
            let items: Vec<serde_json::Value> = match set {
                serde_json::Value::Array(a) => a,
                obj @ serde_json::Value::Object(_) => vec![obj],
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
            let mut uids = Vec::new();
            for item in items {
                let mut headers = BTreeMap::new();
                headers.insert("Content-Type".to_owned(), "application/json".to_owned());
                headers.insert("Accept".to_owned(), "application/json".to_owned());
                headers.insert("Prefer".to_owned(), "return=representation".to_owned());
                if let Some(auth) = Self::auth_header(self.ixit, &instance.auth, None)? {
                    headers.insert("Authorization".to_owned(), auth);
                }
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
    /// Headers: binding request headers + format headers + auth + instance
    /// extras. `scopes` is the step's resolved SMART `scope` claim (`None` =
    /// the step declared none), consumed only by a `bearer_mint` principal.
    #[allow(clippy::too_many_arguments)] // one parameter per header source; splitting hides the assembly order
    fn build_headers(
        set: &ArtifactSet,
        ixit: &Ixit,
        case: &CaseCore,
        step: &FlowStep,
        binding: &OperationBinding,
        instance: &Instance,
        vars: &VarStore,
        scopes: Option<&[String]>,
    ) -> Result<BTreeMap<String, String>, String> {
        let request_spec = binding
            .request
            .as_ref()
            .ok_or_else(|| "binding is unrealized".to_owned())?;
        let mut headers: BTreeMap<String, String> = BTreeMap::new();
        if let Some(request_headers) = &request_spec.headers {
            for (name, template) in request_headers {
                match assertions::render_template(template, vars) {
                    Ok(value) => {
                        headers.insert(name.clone(), value);
                    }
                    Err(e) => {
                        return Err(format!("step {}: header {name}: {e}", step.step));
                    }
                }
            }
        }
        let format = step.format.or_else(|| case.formats.first().copied());
        if let Some(format) = format {
            let media = Self::media_type(format);
            // The step's format has two distinct roles by request shape: on a
            // body-carrying request (POST/PUT commit) it names the REQUEST
            // body representation (a simplified INPUT format), so it sets
            // Content-Type only and the response is negotiated canonical —
            // the version-id headers the commit capture reads (ETag,
            // Location) are representation-independent and RFC 7231 §6.3.2
            // requires Location on a 201 regardless of the request body
            // format. On a bodyless request (GET read-back) it names the
            // desired RESPONSE representation and sets Accept.
            if request_spec.body.is_some() {
                headers
                    .entry("Content-Type".to_owned())
                    .or_insert_with(|| media.to_owned());
                headers
                    .entry("Accept".to_owned())
                    .or_insert_with(|| "application/json".to_owned());
            } else if step.format.is_some() {
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
                            // manifest-declared template identity wins — the
                            // step's `${ds:…}` body names the data set, and its
                            // corpus entry carries the authoritative
                            // `template_id` (this also serves cases that
                            // provision their template IN-FLOW, e.g. via
                            // I_DEFINITION_ADL2.upload_artefact, where
                            // `requires.templates` is rightly empty). Fallback:
                            // the case's provisioned template list (corpus key
                            // itself for entries that predate the metadata).
                            let body_ds_template_id =
                                step.with_entries().iter().find_map(|(_, v)| {
                                    v.refs().iter().find_map(|r| match r {
                                        crate::refgrammar::ValueRef::DataSet { key, .. } => set
                                            .corpus
                                            .as_ref()
                                            .and_then(|(_, m)| m.get(key))
                                            .and_then(|e| e.template_id.clone()),
                                        _ => None,
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
            // Format-less step: the canonical JSON default representation.
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
        Ok(headers)
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
    /// commits no version — RM common master06 §Change Control).
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
            let mut headers = BTreeMap::new();
            if let Some(hs) = &request_spec.headers {
                for (name, template) in hs {
                    if let Ok(v) = assertions::render_template(template, vars) {
                        headers.insert(name.clone(), v);
                    }
                }
            }
            headers.insert("Accept".to_owned(), "application/json".to_owned());
            headers.insert("Content-Type".to_owned(), "application/json".to_owned());
            if let Some(auth) = Self::auth_header(self.ixit, &instance.auth, None)? {
                headers.insert("Authorization".to_owned(), auth);
            }
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
                    // The live commit WINDOW: [request send, response
                    // receipt] in runner-clock milliseconds, WIDENED by the
                    // SUT's own second-resolution `Date` header — the SUT
                    // stamped the version inside it even when its clock is
                    // skewed from the runner's (containerized SUTs drift),
                    // so `before` resolves from the lower bound and `after`
                    // from the upper, both sound on the wire. Determinism
                    // law (d) governs the ${time:*} ARITHMETIC over this
                    // window, not the window itself; the transcript player
                    // binds its own recorded point ordinals.
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
        let mut headers = BTreeMap::new();
        if let Some(hs) = &request_spec.headers {
            for (name, template) in hs {
                if let Ok(v) = assertions::render_template(template, &VarStore::default()) {
                    headers.insert(name.clone(), v);
                }
            }
        }
        headers
            .entry("Content-Type".to_owned())
            .or_insert_with(|| "application/xml".to_owned());
        if let Some(auth) = Self::auth_header(self.ixit, &instance.auth, None)? {
            headers.insert("Authorization".to_owned(), auth);
        }
        let base = instance.base_url.trim_end_matches('/');
        let url = format!("{base}{}", request_spec.path.raw());
        // 409 tolerated: a re-run row re-uploads the same deterministic OPT.
        let _uploaded = self.send(request_spec.method, &url, &headers, Some(&payload), false)?;
        Ok(Provisioned::Ready)
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
        let headers = match Self::build_headers(
            self.set,
            self.ixit,
            case,
            step,
            binding,
            instance,
            &header_vars,
            scopes.as_deref(),
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

        // Track committed payloads for the equivalent comparison.
        if matches!(request_spec.method, HttpMethod::Post | HttpMethod::Put)
            && let Some(b) = &body
        {
            self.committed.push(b.clone());
        }
        self.last_body.clone_from(&exchange.body);

        // Classify (law c) and bind captures.
        let selectors = self.set.selectors.as_ref().map(|(_, s)| s);
        let observation = outcome::classify_status(binding, selectors, exchange.status, expected);
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
            assertion_failures.extend(self.eval_wire_expectation(
                expectation,
                &exchange,
                &headers,
                vars,
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
            let mut headers = BTreeMap::new();
            if let Some(hs) = &request_spec.headers {
                for (name, template) in hs {
                    if let Ok(v) = assertions::render_template(template, &VarStore::default()) {
                        headers.insert(name.clone(), v);
                    }
                }
            }
            headers.entry("Content-Type".to_owned()).or_insert_with(|| {
                if is_adl2 {
                    "text/plain".to_owned()
                } else {
                    "application/xml".to_owned()
                }
            });
            if let Some(auth) = Self::auth_header(self.ixit, &instance.auth, None)? {
                headers.insert("Authorization".to_owned(), auth);
            }
            let base = instance.base_url.trim_end_matches('/');
            let url = format!("{base}{}", request_spec.path.raw());
            // 409 tolerated: already provisioned (the send records it).
            let _uploaded =
                self.send(request_spec.method, &url, &headers, Some(&payload), false)?;
        }
        if let Provisioned::RowNotApplicable { citation } =
            self.provision_synthesized_opt(case, row)?
        {
            return Ok(Provisioned::RowNotApplicable { citation });
        }
        self.provision_ehr(case, vars)?;
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
            let mut headers = BTreeMap::new();
            headers.insert("Content-Type".to_owned(), "application/json".to_owned());
            headers.insert("Accept".to_owned(), "application/json".to_owned());
            headers.insert("Prefer".to_owned(), "return=representation".to_owned());
            if let Some(auth) = Self::auth_header(self.ixit, &instance.auth, None)? {
                headers.insert("Authorization".to_owned(), auth);
            }
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
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod tests {
    use super::*;

    /// Preconditions follow the DEPLOYMENT the flow addresses, and the
    /// discriminator is the origin — a party declares several instances per
    /// deployment (principals; the SMART Platform base path) and one instance
    /// per extra deployment (the second signing posture).
    #[test]
    fn same_deployment_compares_origins_not_paths() {
        // Same server, different API base path (sut vs smart_platform).
        assert!(same_deployment(
            "http://localhost:8080/ehrbase/rest/openehr/v1",
            "http://localhost:8080/ehrbase/rest"
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
            "http://localhost:8081/ehrbase/rest/openehr/v1",
            "http://localhost:8080/ehrbase/rest/openehr/v1"
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
    fn test_mint(roles: Vec<String>) -> crate::ixit::BearerMint {
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
        let mint = test_mint(vec!["USER".to_owned()]);
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
        let token =
            mint_access_token(&test_mint(vec!["USER".to_owned()]), None, None, &[]).unwrap();
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
            merged
                .scalar(&CaptureName::parse("preceding_version_uid").unwrap())
                .as_deref(),
            Some("vo::sys::2"),
            "the step's explicit with: value wins in its own scope"
        );
        // The underlying store is untouched — the shadow is step-scoped.
        assert_eq!(
            vars.scalar(&CaptureName::parse("preceding_version_uid").unwrap())
                .as_deref(),
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

    #[test]
    fn base64_and_list_extraction() {
        assert_eq!(base64_encode(b"user:pass"), "dXNlcjpwYXNz");
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
            status: 201,
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
}
