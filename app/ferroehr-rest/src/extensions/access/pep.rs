// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The policy-enforcement point (PEP) for the ITS-REST surface.
//!
//! Composes the enterprise **ABAC** gate with the **SMART** resource-scope +
//! launch-context gate, driven from the generic dispatch mount (`crate::api`)
//! so no per-operation code is needed.
//!
//! # What the openEHR specs say (and don't)
//!
//! - **ABAC / RBAC are NOT governed by any openEHR spec.** The SM places
//!   authorisation out of band (SM `openehr_platform/master02-overview.adoc`
//!   §General Assumptions), and no CNF profile carries an authorisation
//!   requirement. The attribute model, the patient gate, and the PDP fan-out
//!   here are **our own enterprise design**, not a spec surface — flagged explicitly per the repo rule that
//!   spec-silent behaviour is called out rather than dressed as conformance.
//! - **SMART App Launch IS spec-grounded.** The scope grammar and the
//!   launch-context binding come from the vendored openEHR SMART edition
//!   (`docs/specs/openehr/ITS-REST/docs/smart_app_launch/master08-scopes.adoc`
//!   §Resource Scopes; `master07-*.adoc` §Context Selection). The 401/403
//!   discipline the deny paths honour is ITS-REST
//!   `.../overview/Requests_and_responses.md` §Authentication and authorization.
//!
//! # Composition order (the layering)
//!
//! `EHR_ACCESS` (spec base) → RBAC → **ABAC** → **SMART scope gate**. Each is an
//! additive restriction (AND-composition: a request must clear every enabled
//! gate). The SMART gate therefore runs **after** the RBAC/Cedar decision has
//! already permitted (`decide(...)`), never in place of it.
//!
//! # ABAC timing
//!
//! - **Pre-checks** run before the backend call — they have the op id, the
//!   resolved path params, the query string, and the request body (composition
//!   template, contribution template set).
//! - **Post-checks** run after the backend call on a successful response, using
//!   the [`AuditObject`](crate::system_log::middleware::AuditObject) the dispatch already set
//!   (owning EHR id + resource version uid) — no body re-parsing.
//!
//! Every deny is a `403` and every engine failure a `500` (fail-closed, v1
//! parity), each carrying the [`Principal`] on the response extensions so the
//! ATNA audit layer records it for free. Query execution's ABAC scope + post-check
//! live in the query path. ABAC is inert unless an
//! `crate::extensions::access::authz::AbacGate` is wired
//! (`abac.enabled`); the SMART gate is inert until SMART is enabled (see
//! `smart_config`). End-to-end coverage through the assembled router (pre-check
//! deny/allow, post-check deny, and the ATNA deny record) lives in
//! `tests/abac_e2e.rs`.
//!
//! The PEP helpers return `Result<(), Response>` (the deny/error path is a ready
//! axum `Response`, which is a large type) — `result_large_err` is allowed
//! module-wide accordingly.
#![expect(
    clippy::result_large_err,
    reason = "the Err variant of every decision point in this module is a \
              ready-to-return axum `Response`, which is large by nature"
)]

use axum::response::{IntoResponse, Response};

use crate::extensions::access::authz::classify::{access_of, kind_of};
use crate::extensions::access::authz::request::{
    AccessMode, Attr, AuthzRequest, Decision, ResourceKind,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::Composition;

use crate::api::RequestParts;
use crate::extensions::access::authn::{Principal, current_principal};
use crate::extensions::access::authz::AbacGate;
use crate::overview::error::RestError;

use crate::smart::enforce::{self, GateConfig, ScopeDecision};
use crate::state::AppState;
use crate::{negotiate, params};
use ferroehr::config::smart::SmartConfig;
use ferroehr::versioning::object_version_id::parse_uid_based_id;
use openehr_its::rest::smart_scopes::SmartScope;

/// Whether an operation is ABAC-checked, and when.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Not ABAC-checked (item tags, definition/demographic/admin — RBAC only —,
    /// and query, whose scope + check live in the query path).
    Skip,
    /// Checked before the backend call.
    Pre,
    /// Checked after the backend call, on a successful response.
    Post,
}

/// The enforcement matrix, keyed by operation id.
#[expect(
    clippy::match_same_arms,
    reason = "the arms are grouped by resource family, so naming every operation \
              explicitly is the point — merging equal arms would hide which \
              family an operation is gated as"
)]
fn mode_of(op: &str) -> Mode {
    match op {
        "ehr_create" | "ehr_create_with_id" | "ehr_get_by_id" | "ehr_get_by_subject" => Mode::Pre,
        "composition_create" | "composition_update" | "composition_delete" => Mode::Pre,
        "contribution_create" => Mode::Pre,
        // The contribution-list extension is an EHR-scoped read: pre-check it on
        // the target EHR (subject gate) exactly like the other EHR reads, before
        // any backend work. `kind_of` already maps `contribution_*` to the
        // Contribution resource; the ehr_id is a path param, so Pre suffices.
        "contribution_list" => Mode::Pre,
        "composition_get" => Mode::Post,
        "contribution_get" => Mode::Post,
        _ if op.starts_with("versioned_composition_") => Mode::Post,
        _ if op.starts_with("ehr_status_") || op.starts_with("versioned_ehr_status_") => Mode::Pre,
        _ if op.starts_with("directory_") => Mode::Pre,
        // Item tags, query, definition/demographic/admin: not ABAC-checked.
        _ => Mode::Skip,
    }
}

/// The ABAC preparation for a query execution: the patient subject scope
/// to thread into the SQL, and whether the executor should collect the touched
/// EHR/template sets for the post-check. `Ok((None, false))` when ABAC is off.
/// `Err(response)` is a ready 403 (a missing configured patient claim).
pub(crate) fn query_pre(
    state: &AppState,
    op: &'static str,
) -> Result<(Option<String>, bool), Response> {
    let Some(handle) = state.authz() else {
        return Ok((None, false));
    };
    let Some(abac) = handle.abac() else {
        return Ok((None, false));
    };
    if kind_of(op) != Some(ResourceKind::Query) {
        return Ok((None, false));
    }
    let principal = current_principal().ok_or_else(|| {
        forbidden_unauthenticated("authentication required for attribute-based access control")
    })?;
    let patient = resolve_patient_claim(abac, &principal)?;
    // Collect attributes whenever ABAC is on so the post-check has the touched
    // template set; the subject scope pre-filters rows to the caller's patient.
    Ok((patient, true))
}

/// The ABAC post-check for a query execution: the PDP fan-out over the
/// touched template set: a query touching no template asks nothing of the PDP
/// and is therefore not denied by it. The patient gate
/// is already enforced by the subject-scope pre-filter (rows outside the
/// caller's patient are never fetched), so it is not re-run per EHR here.
pub(crate) async fn query_post(
    state: &AppState,
    op: &'static str,
    outcome: &ferroehr::service::query::request::QueryOutcome,
) -> Result<(), Response> {
    let Some(handle) = state.authz() else {
        return Ok(());
    };
    let Some(abac) = handle.abac() else {
        return Ok(());
    };
    if kind_of(op) != Some(ResourceKind::Query) {
        return Ok(());
    }
    // Empty result set → nothing touched → permit.
    if outcome.ehr_ids.is_empty() && outcome.template_ids.is_empty() {
        return Ok(());
    }
    let principal = current_principal().ok_or_else(|| {
        forbidden_unauthenticated("authentication required for attribute-based access control")
    })?;
    let patient = resolve_patient_claim(abac, &principal)?;
    let template =
        (!outcome.template_ids.is_empty()).then(|| Attr::Set(outcome.template_ids.clone()));
    let req = AuthzRequest {
        operation_id: op,
        subject: &principal.subject,
        roles: &principal.roles,
        scopes: &principal.scopes,
        kind: ResourceKind::Query,
        access: AccessMode::Execute,
        organization: organization(abac, &principal),
        patient: patient.map(Attr::One),
        template,
    };
    decide(abac, &principal, &req).await
}

/// Pre-check the request. `Err(response)` short-circuits the dispatch with a
/// ready 403/500; `Ok(())` lets it proceed.
pub(crate) async fn pre_check(
    state: &AppState,
    op: &'static str,
    parts: &RequestParts,
) -> Result<(), Response> {
    // The SMART gate for the ABAC-`Skip` resource families (template + AQL,
    // master08 §Resource Scopes) runs FIRST and independently of the ABAC
    // wiring: those operations never reach the ABAC-integrated `smart_gate`
    // below (query authz lives in the query path, template ops are RBAC-only),
    // so their fine-grained scopes are enforced here, with the resource id
    // resolved from the route's own path parameters.
    smart_skip_family_gate(state, op, parts)?;
    let Some(handle) = state.authz() else {
        return Ok(());
    };
    let Some(abac) = handle.abac() else {
        return Ok(());
    };
    if mode_of(op) != Mode::Pre {
        return Ok(());
    }
    let Some(kind) = kind_of(op) else {
        return Ok(());
    };
    // DIRECTORY is gated only when `abac.check_directory` says so — engine-
    // independent, so a Cedar deployment can enable it too.
    if kind == ResourceKind::Directory && !abac.directory_checked {
        return Ok(());
    }

    let principal = current_principal().ok_or_else(|| {
        forbidden_unauthenticated("authentication required for attribute-based access control")
    })?;

    let patient = resolve_patient_claim(abac, &principal)?;
    let ehr_id = parts.path.get("ehr_id").cloned();

    // The local patient gate, before any engine call.
    patient_gate(
        abac,
        &principal,
        patient.as_deref(),
        op,
        ehr_id.as_deref(),
        parts,
    )
    .await?;

    // Resolve the template attribute per resource kind.
    let template = pre_template(abac, &principal, op, kind, parts).await?;

    let req = AuthzRequest {
        operation_id: op,
        subject: &principal.subject,
        roles: &principal.roles,
        scopes: &principal.scopes,
        kind,
        access: access_of(op).unwrap_or(AccessMode::Read),
        organization: organization(abac, &principal),
        patient: patient.map(Attr::One),
        template,
    };
    decide(abac, &principal, &req).await?;

    // The SMART resource-scope + launch-context gate, AND-composed after the
    // RBAC/Cedar decision (master08 §Resource Scopes; master07 §Context
    // Selection). The resource id is the template the ABAC pre-check already
    // resolved (composition create/update); an unresolved id matches only a
    // broad `*`/`**` scope.
    let resource_id = req.template.as_ref().and_then(attr_single);
    smart_gate(state, abac, &principal, op, resource_id, ehr_id.as_deref()).await
}

/// Post-check a successful response using the resource ids the dispatch recorded.
pub(crate) async fn post_check(state: &AppState, op: &'static str, resp: Response) -> Response {
    if !resp.status().is_success() {
        return resp;
    }
    let Some(handle) = state.authz() else {
        return resp;
    };
    let Some(abac) = handle.abac() else {
        return resp;
    };
    if mode_of(op) != Mode::Post {
        return resp;
    }
    let Some(kind) = kind_of(op) else {
        return resp;
    };
    let Some(principal) = current_principal() else {
        return resp;
    };

    let object = resp
        .extensions()
        .get::<crate::system_log::middleware::AuditObject>()
        .cloned();
    let ehr_id = object.as_ref().and_then(|o| o.ehr_id.clone());
    let uid = object.as_ref().and_then(|o| o.uid.clone());

    let patient = match resolve_patient_claim(abac, &principal) {
        Ok(p) => p,
        Err(deny) => return deny,
    };
    if let Err(deny) = subject_gate(abac, &principal, patient.as_deref(), ehr_id.as_deref()).await {
        return deny;
    }

    // Template attribute: for a composition read, from the returned version uid.
    let template = match kind {
        ResourceKind::Composition => match post_template(abac, &principal, uid.as_deref()).await {
            Ok(t) => t.map(Attr::One),
            Err(deny) => return deny,
        },
        _ => None,
    };

    let req = AuthzRequest {
        operation_id: op,
        subject: &principal.subject,
        roles: &principal.roles,
        scopes: &principal.scopes,
        kind,
        access: access_of(op).unwrap_or(AccessMode::Read),
        organization: organization(abac, &principal),
        patient: patient.map(Attr::One),
        template,
    };
    if let Err(deny) = decide(abac, &principal, &req).await {
        return deny;
    }
    // The SMART gate on the post-resolved template (composition reads), AND-
    // composed after the Cedar decision (master08 §Resource Scopes; master07
    // §Context Selection).
    let resource_id = req.template.as_ref().and_then(attr_single);
    match smart_gate(state, abac, &principal, op, resource_id, ehr_id.as_deref()).await {
        Ok(()) => resp,
        Err(deny) => deny,
    }
}

/// Resolve the patient claim, if the gate is configured.
///
/// A configured-but-missing claim is a 403: the deployment asked for patient
/// scoping and the token does not carry it, so no decision can permit the
/// request. It is not a 500 — the server is working correctly and the caller's
/// token is the thing that is insufficient.
fn resolve_patient_claim(
    abac: &AbacGate,
    principal: &Principal,
) -> Result<Option<String>, Response> {
    let Some(claim) = &abac.patient_claim else {
        return Ok(None);
    };
    match crate::extensions::access::authz::roles::claim_string(&principal.claims, claim) {
        Some(v) => Ok(Some(v)),
        None => Err(forbidden(
            principal,
            &format!("ABAC requires a '{claim}' claim on the access token"),
        )),
    }
}

/// The organization attribute, if the claim is configured and present.
fn organization(abac: &AbacGate, principal: &Principal) -> Option<String> {
    abac.organization_claim
        .as_ref()
        .and_then(|c| crate::extensions::access::authz::roles::claim_string(&principal.claims, c))
}

/// The full pre-check patient gate: the subject match for target-EHR ops, plus
/// the by-subject special case, where the request parameter IS the subject.
///
/// For `ehr_get_by_subject` an ABSENT `subject_id` deliberately passes this gate
/// so the route answers `400`, not `403`. The released specification declares the
/// parameter required (`specifications/parameters/query/subject_id.yaml`:
/// `required: true`), so such a request is invalid for EVERY caller and no
/// authorization decision could make it succeed — RFC 9110 §15.5.1's
/// client-error case. Answering `403` would assert something false, that
/// permission is the obstacle, and send an integrator hunting entitlements for a
/// malformed request. The typed parameter
/// (`EhrGetBySubjectParams::subject_id`, a non-optional `String`) is what
/// produces the `400`.
async fn patient_gate(
    abac: &AbacGate,
    principal: &Principal,
    patient: Option<&str>,
    op: &str,
    ehr_id: Option<&str>,
    parts: &RequestParts,
) -> Result<(), Response> {
    if op == "ehr_get_by_subject" {
        // Here the request parameter IS the subject, so the gate compares the
        // caller's patient claim to it directly rather than resolving an EHR.
        if let Some(patient) = patient
            && let Some(subject) = params::query_param(parts.query.as_deref(), "subject_id")
            && subject != patient
        {
            return Err(forbidden(principal, "patient scope (subject mismatch)"));
        }
        return Ok(());
    }
    subject_gate(abac, principal, patient, ehr_id).await
}

/// The subject-match gate over a target EHR: the claim must equal the EHR's
/// subject external ref (a subject-less EHR passes).
async fn subject_gate(
    abac: &AbacGate,
    principal: &Principal,
    patient: Option<&str>,
    ehr_id: Option<&str>,
) -> Result<(), Response> {
    let (Some(patient), Some(ehr_id)) = (patient, ehr_id) else {
        return Ok(());
    };
    let subject = (abac.resolvers.subject)(ehr_id.to_owned())
        .await
        .map_err(|e| engine_error(principal, "resolve the EHR subject attribute", &e))?;
    match subject {
        Some(subject) if subject != patient => {
            Err(forbidden(principal, "patient scope (subject mismatch)"))
        }
        _ => Ok(()),
    }
}

/// The template attribute for a pre-checked op.
async fn pre_template(
    abac: &AbacGate,
    principal: &Principal,
    op: &str,
    kind: ResourceKind,
    parts: &RequestParts,
) -> Result<Option<Attr>, Response> {
    match op {
        "composition_create" | "composition_update" => {
            Ok(composition_template(parts).map(Attr::One))
        }
        "composition_delete" => {
            // The template of the preceding (current) version being deleted.
            let Some(uid) = parts.path.get("uid_based_id") else {
                return Ok(None);
            };
            let (vo_id, _) = resolve_target(principal, uid)?;
            let template = (abac.resolvers.template_of_version)(vo_id, None)
                .await
                .map_err(|e| engine_error(principal, "resolve the template attribute", &e))?;
            Ok(template.map(Attr::One))
        }
        "contribution_create" => {
            let templates = contribution_templates(parts, principal)?;
            Ok((!templates.is_empty()).then_some(Attr::Set(templates)))
        }
        _ => {
            // EHR / EHR_STATUS / DIRECTORY carry no template.
            let _ = kind;
            Ok(None)
        }
    }
}

/// The template of a returned composition version (post-check), via the version
/// uid the dispatch recorded.
async fn post_template(
    abac: &AbacGate,
    principal: &Principal,
    uid: Option<&str>,
) -> Result<Option<String>, Response> {
    let Some(uid) = uid else {
        return Ok(None);
    };
    let (vo_id, version) = resolve_target(principal, uid)?;
    (abac.resolvers.template_of_version)(vo_id, version)
        .await
        .map_err(|e| engine_error(principal, "resolve the template attribute", &e))
}

/// Extract the composition template id from the request body: the canonical
/// pointer `/archetype_details/template_id/value` (JSON or XML), or the
/// `openehr-template-id` request header for a FLAT/STRUCTURED body
/// (`Requests_and_responses` §openehr-template-id).
fn composition_template(parts: &RequestParts) -> Option<String> {
    let h = &parts.headers;
    if crate::formats::dispatch::is_simplified_body(h) {
        return crate::formats::dispatch::header_template_id(h);
    }
    // The same typed decode the commit seam performs — the template id is
    // read off the RM value's own `ARCHETYPED.template_id`, never a JSON
    // pointer that a shape change could silently stop matching.
    let composition = negotiate::rm_value::<Composition>(h, &parts.body).ok()?;
    composition
        .archetype_details
        .and_then(|a| a.template_id)
        .map(|id| id.value)
}

/// The set of COMPOSITION template ids in a contribution payload. A COMPOSITION
/// version without a template is unresolvable → fail-closed (403).
fn contribution_templates(
    parts: &RequestParts,
    principal: &Principal,
) -> Result<Vec<String>, Response> {
    let Ok(value) = negotiate::json_value(&parts.headers, &parts.body) else {
        // A malformed body: let the dispatch return the 400; nothing to check.
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    if let Some(versions) = value.get("versions").and_then(|v| v.as_array()) {
        for version in versions {
            let Some(data) = version.get("data") else {
                continue;
            };
            if data.get("_type").and_then(|t| t.as_str()) != Some("COMPOSITION") {
                continue;
            }
            // A CONTRIBUTION body is a wrapper (a version set + audit) with no
            // canonical RM type of its own, so the enclosing value stays JSON —
            // but each `versions[i].data` IS a canonical COMPOSITION, and the
            // template id is read off the typed value's own
            // `ARCHETYPED.template_id`, the same way `composition_template`
            // does. A JSON pointer would keep compiling while silently matching
            // nothing if the canonical shape moved.
            let Ok(composition) = openehr_its::json::from_canonical_value::<Composition>(data)
            else {
                // Undecodable: the dispatch answers 400 for this body, so there
                // is nothing here to authorize.
                return Ok(Vec::new());
            };
            match composition
                .archetype_details
                .and_then(|a| a.template_id)
                .map(|id| id.value)
            {
                Some(t) => out.push(t),
                None => {
                    return Err(forbidden(
                        principal,
                        "contribution COMPOSITION version has no template id",
                    ));
                }
            }
        }
    }
    Ok(out)
}

/// Run the engine and map the verdict: permit → `Ok`, deny → 403, error → 500.
async fn decide(
    abac: &AbacGate,
    principal: &Principal,
    req: &AuthzRequest<'_>,
) -> Result<(), Response> {
    match abac.engine.decide(req).await {
        Ok(Decision::Permit) => Ok(()),
        Ok(Decision::Deny) => Err(forbidden(principal, "policy denied")),
        Err(e) => Err(engine_error(
            principal,
            "reach an authorization decision",
            &e,
        )),
    }
}

/// Decompose a `UID_BASED_ID` the PEP has to attribute — a bare
/// `HIER_OBJECT_ID` or a full `OBJECT_VERSION_ID` — into the
/// `(vo_id, version_tree_id)` pair the attribute resolvers take.
///
/// The identifier grammar is NOT restated here: the string goes through this
/// adapter's one strict decoder ([`parse_uid_based_id`], which delegates the
/// lexical form to `ObjectVersionId::from_str` in `openehr-base` — BASE
/// `base_types` `master05-identification_package.adoc` §Syntaxes), so the PEP
/// can never read an id differently from the route it is guarding.
///
/// A value that decoder rejects is **denied**, not silently degraded. This is
/// the module's fail-closed rule (every attribute-resolution failure denies):
/// the resource attributes the policy binds on cannot be computed for an
/// unaddressable id, and permitting on an absent attribute would let a
/// malformed path parameter walk past a template- or patient-scoped policy. It
/// matches the existing hard deny for a CONTRIBUTION version whose template
/// cannot be determined. Authorization decisions are our own design — no
/// openEHR spec governs them (see the module header).
fn resolve_target(principal: &Principal, uid: &str) -> Result<(String, Option<String>), Response> {
    match parse_uid_based_id(uid) {
        Ok(decoded) => Ok((
            decoded.vo_id.to_string(),
            decoded
                .version
                .map(|ovid| ovid.version_tree_id().value().to_owned()),
        )),
        Err(e) => Err(forbidden(
            principal,
            &format!("unaddressable resource id: {e}"),
        )),
    }
}

/// A 403 carrying the principal (audited by the ATNA layer).
fn forbidden(principal: &Principal, detail: &str) -> Response {
    let mut resp =
        RestError(ApiError::Forbidden(format!("access denied: {detail}"))).into_response();
    resp.extensions_mut().insert(principal.clone());
    resp
}

/// A 403 for a request that reached a gate with NO authenticated principal.
///
/// Nothing is attached to the response extensions because there is no principal
/// to attribute: the ATNA layer records the denial from the request itself. This
/// replaces a fabricated `UNKNOWN` principal — attributing a denial to an
/// identity that never authenticated writes a false subject into the audit
/// trail, which is worse than recording an unattributed denial.
fn forbidden_unauthenticated(detail: &str) -> Response {
    RestError(ApiError::Forbidden(format!("access denied: {detail}"))).into_response()
}

/// A 500 (fail-closed) carrying the principal. `step` names what could not be
/// done and `error` the failure that broke it, whole cause chain included —
/// server-internal, so both go to the trace record and the body carries the
/// curated opaque message.
fn engine_error(
    principal: &Principal,
    step: &'static str,
    error: &(dyn std::error::Error + 'static),
) -> Response {
    let mut resp =
        RestError(crate::overview::error::internal_fault_caused(step, error)).into_response();
    resp.extensions_mut().insert(principal.clone());
    resp
}

/// The single value of a resolved attribute, or `None` for an absent/multi-valued
/// one (SMART maps one operation onto one resource id; a contribution's template
/// *set* has no single id, and the CONTRIBUTION family is not SMART-governed).
fn attr_single(attr: &Attr) -> Option<&str> {
    match attr {
        Attr::One(s) => Some(s.as_str()),
        Attr::Set(_) => None,
    }
}

/// The SMART configuration for this server, or `None` when SMART is disabled.
///
/// SMART is off by default and produces zero wire drift when disabled
/// (`ferroehr::config::smart::SmartConfig`), so the gate below is inert unless an
/// operator opts in via `FERROEHR_REST_SMART__ENABLED` (the `smart` field on
/// `crate::config::AppConfig`).
fn smart_config(state: &AppState) -> Option<&SmartConfig> {
    let cfg = &state.config().smart;
    cfg.enabled.then_some(cfg)
}

/// Run the SMART gate for one operation, AND-composed after the RBAC/Cedar
/// decision. A scope deny is a `403`; a `patient/` compartment grant additionally
/// binds the launch context to the target EHR through the ABAC [`subject_gate`].
/// Inert when SMART is disabled. Each deny carries the principal (audited by the
/// ATNA layer).
async fn smart_gate(
    state: &AppState,
    abac: &AbacGate,
    principal: &Principal,
    op: &str,
    resource_id: Option<&str>,
    ehr_id: Option<&str>,
) -> Result<(), Response> {
    let Some(cfg) = smart_config(state) else {
        return Ok(());
    };
    match smart_decide(cfg, principal, op, resource_id) {
        Err(reason) => Err(forbidden(principal, &reason)),
        // Permitted, no launch-context binding (a `user`/`system` scope, or no
        // SMART resource family governs the op).
        Ok(None) => Ok(()),
        // Permitted through a `patient/` compartment: bind the launch context to
        // the EHR being accessed (master07 §Context Selection) by reusing the
        // ABAC subject gate — it compares the target EHR's subject external ref to
        // the resolved context value. A target-less op (query, create-without-id)
        // passes the gate vacuously (no EHR to bind); per-row patient scoping then
        // stays with the ABAC query subject-scope pre-filter.
        //
        // NOTE: the context value prefers the `ehrId` claim then the SMART
        // `patient` claim (master07 token-response table), matched against the
        // EHR *subject* external ref.
        Ok(Some(ctx)) => subject_gate(abac, principal, Some(ctx.as_str()), ehr_id).await,
    }
}

/// The pure SMART scope decision (master08 §Resource Scopes) + launch-context
/// resolution (master07 §Context Selection). Returns:
/// - `Err(reason)` → deny (the reason is the audit/diagnostic detail);
/// - `Ok(None)` → permit, no launch-context binding required;
/// - `Ok(Some(ctx))` → permit, but a `patient/` compartment scope means the
///   caller must bind launch-context `ctx` to the EHR being accessed.
fn smart_decide(
    cfg: &SmartConfig,
    principal: &Principal,
    op: &str,
    resource_id: Option<&str>,
) -> Result<Option<String>, String> {
    // SMART resource scopes ride Bearer tokens only; `Principal.scopes` is the
    // token `scope`/`scp` claim (empty for Basic, so a Basic caller holds no
    // SMART scope — advisory mode defers, fail-closed mode denies).
    let scopes = SmartScope::parse_all(&principal.scopes.join(" "));
    let outcome = enforce::evaluate(
        &scopes,
        enforce::family_of_op(op),
        enforce::permission_of_op(op),
        resource_id,
        GateConfig {
            require_smart_scopes: cfg.require_smart_scopes,
        },
    );
    if let ScopeDecision::Deny(reason) = outcome.decision {
        return Err(reason);
    }
    if !outcome.bind_patient_compartment {
        return Ok(None);
    }
    // master07 §Context Selection: a `patient/` compartment scope requires a
    // resolved launch context; its absence is a deny.
    enforce::launch_context_ehr_id(&principal.claims, &cfg.ehr_id_claim, &cfg.patient_claim)
        .map(Some)
        .ok_or_else(|| {
            "SMART patient scope requires a launch-context ehrId/patient claim \
             (master07 §Context Selection)"
                .to_owned()
        })
}

/// The SMART resource-scope gate for the ABAC-`Skip` families — template and
/// AQL (master08 §Resource Scopes: `template-…`, `aql-…`). The COMPOSITION
/// family rides the ABAC-integrated [`smart_gate`] (it needs the ABAC-resolved
/// template id + the launch-context subject binding); these two families carry
/// their resource id in the route itself, so the gate runs directly off the
/// path parameters, before and independently of any ABAC wiring:
///
/// - template ops → the `{template_id}` path parameter (uploads and lists have
///   none — an unresolved id matches only a broad `*`/`**` scope, the
///   documented fail-closed reading of [`enforce::evaluate`]);
/// - AQL definition/stored-execution ops → `{qualified_query_name}` (ad-hoc
///   execution has none — same broad-scope rule).
///
/// A `patient/` compartment permit needs no per-request EHR binding here:
/// templates are unscoped and queries are cross-EHR — per-row patient scoping
/// stays with the ABAC query subject-scope pre-filter (adjudicated),
/// exactly as the ABAC-integrated gate treats target-less operations.
fn smart_skip_family_gate(
    state: &AppState,
    op: &str,
    parts: &RequestParts,
) -> Result<(), Response> {
    let Some(cfg) = smart_config(state) else {
        return Ok(());
    };
    let family = enforce::family_of_op(op);
    if !matches!(
        family,
        Some(
            openehr_its::rest::smart_scopes::ResourceFamily::Template
                | openehr_its::rest::smart_scopes::ResourceFamily::Aql
        )
    ) {
        return Ok(());
    }
    // No principal means no token, and therefore no SMART scopes — which is a
    // state the configuration already has an answer for. Advisory mode
    // (`smart.require_smart_scopes = false`) defers to the RBAC/ABAC tiers, so
    // it defers here too; fail-closed mode refuses. Refusing unconditionally
    // would brick `auth.enabled = false`, where the absence of a principal is
    // the operator's explicit choice rather than a missing credential.
    let Some(principal) = current_principal() else {
        if cfg.require_smart_scopes {
            return Err(forbidden_unauthenticated(
                "smart.require_smart_scopes demands a token carrying SMART resource scopes",
            ));
        }
        return Ok(());
    };
    let resource_id = parts
        .path
        .get("template_id")
        .or_else(|| parts.path.get("qualified_query_name"))
        .map(String::as_str);
    match smart_decide(cfg, &principal, op, resource_id) {
        Err(reason) => Err(forbidden(&principal, &reason)),
        // Permitted — including through a `patient/` compartment, whose EHR
        // binding is vacuous for these target-less families (see the doc).
        Ok(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::disallowed_types,
        reason = "a test builds the Principal's claim map directly; the claim set \
                  is RFC 7519-open, so the fixtures name serde_json's types"
    )]

    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::extensions::access::authz::engine::{AuthzError, PolicyEngine};
    use async_trait::async_trait;
    use http::StatusCode;

    use super::*;
    use crate::extensions::access::authn::AuthMethod;
    use crate::extensions::access::authz::{AuthzResolvers, ResolveError};

    /// A counting engine: records how often `decide` is called (the patient gate
    /// must deny *without* reaching it), always permits.
    #[derive(Debug, Default)]
    struct CountingEngine(Arc<AtomicUsize>);

    #[async_trait]
    impl PolicyEngine for CountingEngine {
        async fn decide(&self, _req: &AuthzRequest<'_>) -> Result<Decision, AuthzError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Decision::Permit)
        }
    }

    fn gate_with_subject(subject: Option<&'static str>, calls: Arc<AtomicUsize>) -> AbacGate {
        let subject = subject.map(str::to_owned);
        AbacGate {
            engine: Arc::new(CountingEngine(calls)),
            resolvers: AuthzResolvers {
                subject: Arc::new(move |_ehr_id: String| {
                    let subject = subject.clone();
                    Box::pin(async move { Ok::<_, ResolveError>(subject) })
                }),
                template_of_version: Arc::new(|_vo: String, _v: Option<String>| {
                    Box::pin(async move { Ok::<_, ResolveError>(None) })
                }),
            },
            organization_claim: None,
            patient_claim: Some("patient_id".to_owned()),
            directory_checked: false,
        }
    }

    fn principal_with_patient(patient: &str) -> Principal {
        let mut claims = serde_json::Map::new();
        claims.insert("patient_id".to_owned(), serde_json::json!(patient));
        Principal {
            subject: "alice".to_owned(),
            scopes: Vec::new(),
            roles: vec!["USER".to_owned()],
            claims,
            method: AuthMethod::Bearer,
        }
    }

    #[tokio::test]
    async fn subject_mismatch_denies_without_engine_call() {
        let calls = Arc::new(AtomicUsize::new(0));
        let gate = gate_with_subject(Some("P2"), Arc::clone(&calls));
        let principal = principal_with_patient("P1");
        let err = subject_gate(&gate, &principal, Some("P1"), Some("ehr-1"))
            .await
            .expect_err("mismatch must deny");
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "no engine call on gate deny"
        );
    }

    #[tokio::test]
    async fn subject_match_passes_gate() {
        let calls = Arc::new(AtomicUsize::new(0));
        let gate = gate_with_subject(Some("P1"), calls);
        let principal = principal_with_patient("P1");
        assert!(
            subject_gate(&gate, &principal, Some("P1"), Some("ehr-1"))
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn null_subject_passes_gate() {
        // A caller with no patient scope is unaffected by the subject gate, so
        // an anonymous EHR stays fully operable for it (RM ehr
        // `master04-ehr_package.adoc` §EHR Status).
        let calls = Arc::new(AtomicUsize::new(0));
        let gate = gate_with_subject(None, calls);
        let principal = principal_with_patient("P1");
        assert!(
            subject_gate(&gate, &principal, Some("P1"), Some("ehr-1"))
                .await
                .is_ok()
        );
    }

    #[test]
    fn missing_patient_claim_is_forbidden() {
        let calls = Arc::new(AtomicUsize::new(0));
        let gate = gate_with_subject(Some("P1"), calls);
        // A principal with no patient claim configured-but-absent → 403.
        let principal = Principal {
            subject: "alice".to_owned(),
            scopes: Vec::new(),
            roles: vec!["USER".to_owned()],
            claims: serde_json::Map::new(),
            method: AuthMethod::Bearer,
        };
        let err = resolve_patient_claim(&gate, &principal).expect_err("missing claim");
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn mode_matrix() {
        assert_eq!(mode_of("ehr_create"), Mode::Pre);
        assert_eq!(mode_of("ehr_get_by_subject"), Mode::Pre);
        assert_eq!(mode_of("composition_create"), Mode::Pre);
        assert_eq!(mode_of("composition_delete"), Mode::Pre);
        assert_eq!(mode_of("composition_get"), Mode::Post);
        assert_eq!(mode_of("versioned_composition_get"), Mode::Post);
        assert_eq!(mode_of("contribution_create"), Mode::Pre);
        assert_eq!(mode_of("contribution_get"), Mode::Post);
        // The contribution-list extension is pre-checked (EHR-scoped read).
        assert_eq!(mode_of("contribution_list"), Mode::Pre);
        assert_eq!(
            kind_of("contribution_list"),
            Some(ResourceKind::Contribution)
        );
        assert_eq!(mode_of("ehr_status_update"), Mode::Pre);
        assert_eq!(mode_of("versioned_ehr_status_get"), Mode::Pre);
        assert_eq!(mode_of("directory_create"), Mode::Pre);
        // Query is handled in the query path, not the generic PEP.
        assert_eq!(mode_of("query_execute_adhoc_query"), Mode::Skip);
        assert_eq!(mode_of("ehr_tags_get"), Mode::Skip);
        assert_eq!(mode_of("definition_template_adl1.4_upload"), Mode::Skip);
    }

    /// A minimal authenticated principal for the pure-function tests below.
    ///
    /// Deliberately built HERE rather than in production code: the gates now
    /// refuse when no principal is present, so production has no reason to
    /// construct one, and a shared fabricated principal is exactly what #1963
    /// removed (attributing a denial to an identity that never authenticated
    /// writes a false subject into the audit trail).
    fn test_principal() -> Principal {
        Principal {
            subject: "test-subject".to_owned(),
            scopes: Vec::new(),
            roles: Vec::new(),
            claims: serde_json::Map::new(),
            method: AuthMethod::Bearer,
        }
    }

    /// The PEP reads a resource id through this adapter's one strict
    /// decoder (BASE `base_types` `master05-identification_package.adoc`
    /// §Syntaxes), so a well-formed id decomposes and a malformed one is
    /// DENIED rather than degraded into a whole-string `vo_id`.
    #[test]
    fn object_version_id_parsing() {
        let principal = test_principal();
        let ok = |uid: &str| resolve_target(&principal, uid).map_err(|_resp| "denied");

        let uid = "0197f1c2-3aa0-7000-8000-000000000001::ferroehr.local::2";
        assert_eq!(
            ok(uid),
            Ok((
                "0197f1c2-3aa0-7000-8000-000000000001".to_owned(),
                Some("2".to_owned())
            ))
        );
        // A branch VERSION_TREE_ID is carried whole (RM common master06
        // §The 'Virtual Version Tree').
        assert_eq!(
            ok("0197f1c2-3aa0-7000-8000-000000000001::sys::2.1.3"),
            Ok((
                "0197f1c2-3aa0-7000-8000-000000000001".to_owned(),
                Some("2.1.3".to_owned())
            ))
        );
        // A bare HIER_OBJECT_ID (a UUID) → the versioned object, no version.
        assert_eq!(
            ok("0197f1c2-3aa0-7000-8000-000000000001"),
            Ok(("0197f1c2-3aa0-7000-8000-000000000001".to_owned(), None))
        );
    }

    /// The invalid twins: none of these is a `UID_BASED_ID` this CDR can
    /// address, so each is a fail-closed deny (403) instead of the former
    /// whole-string `vo_id` degradation.
    #[test]
    fn malformed_resource_ids_are_denied() {
        let principal = test_principal();
        for bad in [
            // not a UUID at all
            "abc",
            // two parts, not three (BASE master05 §Syntaxes)
            "0197f1c2-3aa0-7000-8000-000000000001::sys",
            // an empty creating_system_id component
            "0197f1c2-3aa0-7000-8000-000000000001::::2",
            // a VERSION_TREE_ID that is not `N` or `N.B.V`
            "0197f1c2-3aa0-7000-8000-000000000001::sys::v2",
            // a three-part id whose object_id is not this CDR's UUID key
            "not-a-uuid::sys::1",
            "",
        ] {
            let denied = resolve_target(&principal, bad).expect_err("must deny");
            assert_eq!(
                denied.status(),
                StatusCode::FORBIDDEN,
                "id {bad:?} must be a fail-closed deny"
            );
        }
    }

    // ── SMART scope + launch-context gate (master08 §Resource Scopes; ─────────
    //    master07 §Context Selection) ───────────────────────────────────────────

    fn smart(require: bool) -> SmartConfig {
        SmartConfig {
            enabled: true,
            require_smart_scopes: require,
            ..SmartConfig::default()
        }
    }

    fn principal_with_scopes(scope: &str, claims: &serde_json::Value) -> Principal {
        Principal {
            subject: "alice".to_owned(),
            scopes: scope.split_whitespace().map(str::to_owned).collect(),
            roles: vec!["USER".to_owned()],
            claims: claims.as_object().cloned().unwrap_or_default(),
            method: AuthMethod::Bearer,
        }
    }

    #[test]
    fn smart_user_scope_permits_matching_composition_no_binding() {
        // A `user/` grant permits with no launch-context binding required.
        let p = principal_with_scopes("user/composition-Vitals.v1.r", &serde_json::json!({}));
        assert_eq!(
            smart_decide(&smart(true), &p, "composition_get", Some("Vitals.v1")),
            Ok(None)
        );
    }

    #[test]
    fn smart_permission_mismatch_denies() {
        // A read-only scope cannot authorise a create.
        let p = principal_with_scopes("user/composition-*.r", &serde_json::json!({}));
        assert!(smart_decide(&smart(false), &p, "composition_create", Some("Vitals.v1")).is_err());
    }

    #[test]
    fn smart_advisory_defers_for_plain_oidc_token() {
        // No SMART resource scope + advisory mode → the gate does not engage.
        let p = principal_with_scopes("openid profile", &serde_json::json!({}));
        assert_eq!(
            smart_decide(&smart(false), &p, "composition_create", Some("Vitals.v1")),
            Ok(None)
        );
    }

    #[test]
    fn smart_fail_closed_denies_without_family_scope() {
        let p = principal_with_scopes("openid profile", &serde_json::json!({}));
        assert!(smart_decide(&smart(true), &p, "composition_create", Some("Vitals.v1")).is_err());
    }

    #[test]
    fn smart_patient_scope_yields_launch_context_for_binding() {
        // `patient/` compartment → permit, returning the launch-context id the
        // PEP must bind to the target EHR (via the subject gate).
        let p = principal_with_scopes(
            "patient/composition-*.r",
            &serde_json::json!({ "ehrId": "ehr-1" }),
        );
        assert_eq!(
            smart_decide(&smart(false), &p, "composition_get", Some("Vitals.v1")),
            Ok(Some("ehr-1".to_owned()))
        );
    }

    #[test]
    fn smart_patient_scope_requires_a_launch_context_claim() {
        // `patient/` compartment but no ehrId/patient claim → deny.
        let p = principal_with_scopes("patient/composition-*.r", &serde_json::json!({}));
        assert!(smart_decide(&smart(false), &p, "composition_get", Some("Vitals.v1")).is_err());
    }

    #[test]
    fn smart_patient_scope_falls_back_to_patient_claim() {
        // No `ehrId` claim → the standard SMART `patient` context claim is used
        // (master07 token-response table).
        let p = principal_with_scopes(
            "patient/composition-*.r",
            &serde_json::json!({ "patient": "subject-9" }),
        );
        assert_eq!(
            smart_decide(&smart(false), &p, "composition_get", Some("Vitals.v1")),
            Ok(Some("subject-9".to_owned()))
        );
    }

    #[test]
    fn smart_user_scope_needs_no_patient_binding() {
        // A `user/` grant permits without forcing the launch-context binding.
        let p = principal_with_scopes("user/composition-*.r", &serde_json::json!({}));
        assert_eq!(
            smart_decide(&smart(false), &p, "composition_get", Some("Vitals.v1")),
            Ok(None)
        );
    }

    #[test]
    fn smart_non_family_op_is_never_gated() {
        // EHR ops carry no SMART resource family (master08 lists only
        // template/composition/aql), so the gate defers even fail-closed.
        let p = principal_with_scopes("openid", &serde_json::json!({}));
        assert_eq!(smart_decide(&smart(true), &p, "ehr_create", None), Ok(None));
    }
}
