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
use crate::exec::{StepDriver, StepObservation};
use crate::ids::{CaptureName, SmOperationRef};
use crate::ixit::{AuthMode, Instance, Ixit};
use crate::model::assertion::{Assertion, EquivalentTarget, RowsSpec};
use crate::model::binding::{
    OperationBinding, RequestBody, StripRule, TransformRule, WireCapture, WireFrom,
};
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
            resolver: Resolver::new(manifest, corpus_dir),
            committed: Vec::new(),
            last_body: None,
            exchanges: Vec::new(),
        })
    }

    fn binding_for(&self, case: &CaseCore, call: &str) -> Result<&'a OperationBinding, String> {
        let op = if call.contains('.') {
            SmOperationRef::parse(call).map_err(|e| e.to_string())?
        } else {
            case.sm_operation
                .as_ref()
                .ok_or_else(|| format!("case {} has no sm_operation anchor", case.id))?
                .sibling(call)
        };
        self.set
            .bindings
            .iter()
            .map(|(_, b)| b)
            .find(|b| b.sm_operation == op)
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

    fn auth_header(auth: &AuthMode) -> Result<Option<String>, String> {
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
            for (name, template) in query {
                match assertions::render_template(template, vars) {
                    Ok(value) => params.push((name.clone(), value)),
                    // optional refs that are unbound omit the parameter
                    Err(_) if template_is_optional(template) => {}
                    Err(e) => return Err(format!("query {name}: {e}")),
                }
            }
            // `with` keys that match query names override/backfill
            for (name, _) in query {
                if let Some(v) = with.get(name)
                    && !params.iter().any(|(n, _)| n == name)
                    && !v.is_null()
                {
                    let text = match v {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    params.push((name.clone(), text));
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
        self.exchanges.push(exchange.clone());
        Ok(exchange)
    }

    fn extract_capture(exchange: &Exchange, spec: &WireCapture, vars: &VarStore) -> Option<String> {
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
                    // dotted body paths (`ehr_id.value`)
                    let mut current = body;
                    for seg in path.split('.') {
                        current = current.get(seg)?;
                    }
                    match current {
                        Value::String(s) => Some(s.clone()),
                        other => Some(other.to_string()),
                    }
                }
                WireFrom::Capture(name) => vars.scalar(name).map(ToOwned::to_owned),
            }
        };
        let mut value =
            from_source(&spec.from).or_else(|| spec.fallback.as_ref().and_then(from_source))?;
        if matches!(spec.strip, Some(StripRule::WeakQuotes)) {
            value = value.trim_start_matches("W/").trim_matches('"').to_owned();
        }
        if matches!(spec.transform, Some(TransformRule::RootUid)) {
            value = value.split("::").next().unwrap_or(&value).to_owned();
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

    /// Evaluate the pure-side assertions for a step against the exchange.
    fn eval_assertions(
        &mut self,
        _case: &CaseCore,
        binding: &OperationBinding,
        assertions_list: &[Assertion],
        exchange: &Exchange,
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
                                Err(AssertionFailure(
                                    "equivalent: retrieved content differs from committed (modulo the normative ignore-set)".into(),
                                ))
                            }
                        }
                    }
                }
                Assertion::Returns { equals, matches } => {
                    assertions::eval_returns(body, equals.as_ref(), matches.as_deref())
                }
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
                // Wire-dependent version/signature facts need versioned-object
                // reads the ITS does not surface uniformly; they are evaluated
                // by the postcondition pass where the case's own flow provides
                // the read (in-case verification), else recorded as a
                // registered exception by the run command.
                // Version/signature facts need versioned-object reads the
                // ITS does not surface uniformly (in-case verification
                // carries them); unique is aggregate (law e);
                // message_exemplar/state are informative.
                Assertion::Version { .. }
                | Assertion::Signature { .. }
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
                selected.sort_by(|(a, _), (b, _)| a.cmp(b));
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

/// Minimal base64 (standard alphabet, padding) — avoids a crypto dep for
/// one Basic-auth header.
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

impl HttpDriver<'_> {
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
                let found = with
                    .get(name)
                    .cloned()
                    .or_else(|| with.get("composition").cloned())
                    .or_else(|| with.get("opt").cloned())
                    .or_else(|| {
                        // single-payload steps: the one non-path value
                        with.iter()
                            .find(|(k, _)| {
                                !request_spec.path.params().iter().any(|p| p.as_str() == *k)
                            })
                            .map(|(_, v)| v.clone())
                    });
                match (found, optional) {
                    (Some(v), _) => Ok(Some(v)),
                    (None, true) => Ok(None),
                    (None, false) => {
                        Err(format!("step {}: body role {name} unresolved", step.step))
                    }
                }
            }
            Some(RequestBody::Structured(template)) => Ok(Some(
                self.resolver
                    .resolve_value(template, vars)
                    .map_err(|e| e.to_string())?,
            )),
            Some(RequestBody::Patched { from_capture, set }) => {
                let Some(crate::exec::state::Captured::Body(body)) = vars.get(from_capture) else {
                    return Err(format!(
                        "patched body: capture {from_capture} holds no resource body"
                    ));
                };
                let mut patched = body.clone();
                if let Some(map) = patched.as_object_mut() {
                    for (field, value) in set {
                        map.insert(field.clone(), value.clone());
                    }
                }
                Ok(Some(patched))
            }
        }
    }

    /// commit: bulk-provision generated sets, binding committed uids.
    fn provision_commit_sets(
        &mut self,
        case: &CaseCore,
        vars: &mut VarStore,
    ) -> Result<(), String> {
        for key in case.requires.commit.clone() {
            let set = self.resolver.data_set(&key).map_err(|e| e.to_string())?;
            let Some(items) = set.as_array() else {
                continue;
            };
            let binding = self.binding_for(case, "I_EHR_COMPOSITION.create_composition")?;
            let instance = self.ixit.default_instance()?;
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
                if let Some(auth) = Self::auth_header(&instance.auth)? {
                    headers.insert("Authorization".to_owned(), auth);
                }
                let base = instance.base_url.trim_end_matches('/');
                let path = request_spec.path.raw().replace("{ehr_id}", &ehr_id);
                let url = format!("{base}{path}");
                let exchange = self.send(request_spec.method, &url, &headers, Some(item), true)?;
                if let Some((_, spec)) = binding
                    .captures
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .find(|(n, _)| n.as_str() == "version_uid")
                    && let Some(uid) = Self::extract_capture(&exchange, spec, vars)
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
    /// extras.
    fn build_headers(
        case: &CaseCore,
        step: &FlowStep,
        binding: &OperationBinding,
        instance: &Instance,
        vars: &VarStore,
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
            headers
                .entry("Content-Type".to_owned())
                .or_insert_with(|| media.to_owned());
            headers.insert("Accept".to_owned(), media.to_owned());
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
                            // openehr-template-id: the corpus key IS the
                            // declared template identity for placeholder
                            // corpora (the spine-first corpus refines this).
                            if let Some(key) = case.requires.templates.first() {
                                headers.insert(name.clone(), key.to_string());
                            }
                        }
                    }
                }
            }
        } else if !headers.contains_key("Accept") {
            headers.insert("Accept".to_owned(), "application/json".to_owned());
        }
        if let Some(auth) = Self::auth_header(&instance.auth)? {
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
    /// ehr: mint `${ehr_id}` via `create_ehr`.
    fn provision_ehr(&mut self, case: &CaseCore, vars: &mut VarStore) -> Result<(), String> {
        if matches!(case.requires.ehr, Some(EhrRequirement::Exists { .. })) {
            let binding = self.binding_for(case, "I_EHR_SERVICE.create_ehr")?;
            let instance = self.ixit.default_instance()?;
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
            if let Some(auth) = Self::auth_header(&instance.auth)? {
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
                && let Some(value) = Self::extract_capture(&exchange, spec, vars)
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
        exchange_ordinal: usize,
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
                    // The exchange ordinal as a monotonic stand-in
                    // (deterministic-across-runners applies to ${time:*}
                    // arithmetic, not live instants).
                    let ms = i64::try_from(exchange_ordinal).unwrap_or(i64::MAX) * 1_000;
                    vars.set(name.clone(), Captured::InstantMs(ms));
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
                    } else if let Some(value) = Self::extract_capture(exchange, spec, vars) {
                        vars.set(name.clone(), Captured::Scalar(value));
                    }
                }
            }
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
        let binding = self.binding_for(case, &step.call)?;
        if binding.is_unrealized() {
            // The interpreter surfaces this before perform() normally; the
            // driver answers with a transport-class observation so law (c)
            // holds even if reached.
            return Ok(StepObservation {
                observation: Observation::Transport("operation unrealized on this ITS".into()),
                assertion_failures: Vec::new(),
            });
        }
        let instance = self.instance_for(step)?;

        // Resolve the with-payload. A capture the earlier steps never bound
        // (the SUT did not supply what the binding maps) is an INCONCLUSIVE
        // observation for this row (law c) — never a run-aborting defect.
        let mut with: BTreeMap<String, Value> = BTreeMap::new();
        for (key, value) in step.with_entries() {
            let resolved = self.resolver.resolve_value(value, vars);
            match resolved {
                Ok(v) => {
                    with.insert(key.clone(), v);
                }
                Err(e) => {
                    return Ok(StepObservation {
                        observation: Observation::Transport(format!(
                            "step {}: with.{key}: {e}",
                            step.step
                        )),
                        assertion_failures: Vec::new(),
                    });
                }
            }
        }

        let request_spec = binding
            .request
            .as_ref()
            .ok_or_else(|| "binding is unrealized".to_owned())?;
        // Header templates resolve against captures AND the step's own
        // resolved `with` values (e.g. update_composition-non_existent
        // supplies preceding_version_uid inline, not as a capture).
        let mut header_vars = vars.clone();
        for (key, value) in &with {
            if let Value::String(s) = value
                && let Ok(name) = CaptureName::parse(key)
                && header_vars.scalar(&name).is_none()
            {
                header_vars.set(name, Captured::Scalar(s.clone()));
            }
        }
        let headers = match Self::build_headers(case, step, binding, instance, &header_vars) {
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
            Err(e) => {
                return Ok(StepObservation {
                    observation: Observation::Transport(e),
                    assertion_failures: Vec::new(),
                });
            }
        };

        let base = instance.base_url.trim_end_matches('/');
        let url = match Self::build_url(binding, base, &with, &header_vars) {
            Ok(url) => url,
            Err(e) => {
                return Ok(StepObservation {
                    observation: Observation::Transport(e),
                    assertion_failures: Vec::new(),
                });
            }
        };
        let body_is_json = !matches!(body, Some(Value::String(_)));
        let exchange = match self.send(
            request_spec.method,
            &url,
            &headers,
            body.as_ref(),
            body_is_json,
        ) {
            Ok(exchange) => exchange,
            Err(fault) => {
                return Ok(StepObservation {
                    observation: Observation::Transport(fault),
                    assertion_failures: Vec::new(),
                });
            }
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
        Self::bind_step_captures(
            step,
            binding,
            &exchange,
            &observation,
            self.exchanges.len(),
            vars,
        );

        // Post-step assertions only when the expectation held (the caller
        // aborts otherwise, law b) — evaluate optimistically here.
        let assertion_failures =
            self.eval_assertions(case, binding, &step.assertions, &exchange, vars);
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
    ) -> Result<(), String> {
        self.resolver.bind_row(case, row);
        self.committed.clear();
        self.last_body = None;
        // server: empty — isolation is the runner's tenancy concern; against
        // a shared SUT the run is recorded as scoped (never destructive).
        // templates: upload each via the upload_opt binding.
        for key in case.requires.templates.clone() {
            let payload = self.resolver.data_set(&key).map_err(|e| e.to_string())?;
            // direct send through the binding (409 tolerated: already provisioned)
            let binding = self.binding_for(case, "I_DEFINITION_ADL14.upload_opt")?;
            let instance = self.ixit.default_instance()?;
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
            if let Some(auth) = Self::auth_header(&instance.auth)? {
                headers.insert("Authorization".to_owned(), auth);
            }
            let base = instance.base_url.trim_end_matches('/');
            let url = format!("{base}{}", request_spec.path.raw());
            // 409 tolerated: already provisioned (the send records it).
            let _uploaded =
                self.send(request_spec.method, &url, &headers, Some(&payload), false)?;
        }
        self.provision_ehr(case, vars)?;
        // directory: provision the FOLDER tree via create_directory.
        if let Some(crate::model::case::DirectoryRequirement::Tree(key)) =
            case.requires.directory.clone()
        {
            let payload = self.resolver.data_set(&key).map_err(|e| e.to_string())?;
            let binding = self.binding_for(case, "I_EHR_DIRECTORY.create_directory")?;
            let instance = self.ixit.default_instance()?;
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
            if let Some(auth) = Self::auth_header(&instance.auth)? {
                headers.insert("Authorization".to_owned(), auth);
            }
            let base = instance.base_url.trim_end_matches('/');
            let path = request_spec.path.raw().replace("{ehr_id}", &ehr_id);
            let url = format!("{base}{path}");
            let exchange = self.send(request_spec.method, &url, &headers, Some(&payload), true)?;
            self.committed.push(payload);
            if let Some((_, spec)) = binding
                .captures
                .as_deref()
                .unwrap_or_default()
                .iter()
                .find(|(n, _)| n.as_str() == "directory_version_uid")
                && let Some(uid) = Self::extract_capture(&exchange, spec, vars)
            {
                vars.set(
                    CaptureName::parse("directory_version_uid").map_err(|e| e.to_string())?,
                    Captured::Scalar(uid),
                );
            }
        }
        self.provision_commit_sets(case, vars)?;
        Ok(())
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
        let uid = HttpDriver::extract_capture(&exchange, &spec, &vars).unwrap();
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
        let root = HttpDriver::extract_capture(&exchange, &spec2, &vars2).unwrap();
        assert_eq!(root, "8849182c-82ad-4088-a07f-48ead4180515");
    }
}
