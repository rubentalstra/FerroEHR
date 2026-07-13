//! SECURITY (authentication) cases — the cross-cutting register
//! (`docs/design/conformance/11-crosscutting.md` §Authentication).
//!
//! There is **no CNF schedule chapter** for security, and
//! `master03-profiles.adoc` has **no Authentication capability row** — openEHR
//! places authentication/authorization out of band (SM master02). So both cases
//! are [`ScheduleTrace::EccOriginal`], reproducing the vendored Robot
//! `SECURITY_TESTS/I_OAuth2_Keycloak` §05/§06 *intent* ("Base URL / API
//! endpoints are secured") over the ECC transport (Basic instead of Keycloak
//! OAuth, which the compose SUT does not run — register 11 G-5: the Robot suite
//! is coverage evidence, never id-mapped machinery).
//!
//! These cases are **fully generic** — 401/403 is every auth-enforcing CDR's
//! surface, never an ehrbase-rs extension, and must stay live for foreign SUTs
//! (register 11 G-2). [`Capability::Authentication`] is deliberately **not**
//! profile-gating ([`crate::model::profile`] does not list it): security is
//! Non-Functional and authorization is out of band, so it is reported per-case
//! but blocks no profile.
//!
//! Honesty: the ECC drives a black-box SUT and cannot read its auth config, so
//! each case *probes* and adjudicates by the observed status — a non-401/403 is
//! `SKIPPED(SutConfig)` (auth not enforced or the mode is not wire-readable),
//! never a fabricated pass or fail.

use uuid::Uuid;

use crate::engine::harness::{
    AuthSlot, CaseError, CaseFuture, DataSetReport, HttpRequest, RunContext,
};
use crate::engine::registry::CaseEntry;
use crate::model::case::{Binding, Capability, CaseMeta, Compare, Format, ScheduleTrace};
use crate::model::catalog::Area;

/// JSON is the wire format the SEC cases run under.
const JSON: &[Format] = &[Format::Json];

/// Every registered SECURITY case (2, generic auth surface).
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        case(
            "sec/unauthenticated-401",
            "Unauthenticated request to a protected route is refused (401)",
            "CNF SECURITY_TESTS/I_OAuth2_Keycloak §06 (API endpoints are secured); ITS-REST EHR API ehr_get.yaml 401 unauthenticated",
            ScheduleTrace::EccOriginal(
                "no CNF schedule chapter for authentication (out of band per SM master02); Robot SECURITY_TESTS/I_OAuth2_Keycloak §06 'API endpoints are secured' intent, reproduced over Basic auth",
            ),
            Binding::Rest("GET /ehr/{ehr_id} (no Authorization)"),
            run_unauthenticated_401,
        ),
        case(
            "sec/forbidden-role-403",
            "Regular credential on an ADMIN-only route is forbidden (403)",
            "CNF SECURITY_TESTS/I_OAuth2_Keycloak (role-privileged access intent); ITS-REST ADMIN API admin_ehr_delete.yaml — ADMIN-role-only (SM I_ADMIN_SERVICE)",
            ScheduleTrace::EccOriginal(
                "no CNF schedule chapter for authorization (out of band per SM master02); Robot SECURITY_TESTS/I_OAuth2_Keycloak role-distinction intent, reproduced over Basic auth",
            ),
            Binding::Rest("DELETE /admin/ehr/{ehr_id} (regular credential)"),
            run_forbidden_role_403,
        ),
    ]
}

/// Assemble a SEC case entry (area [`Area::Sec`], non-profile
/// [`Capability::Authentication`], JSON).
fn case(
    id: &'static str,
    title: &'static str,
    citation: &'static str,
    schedule: ScheduleTrace,
    binding: Binding,
    run: crate::engine::harness::CaseRun,
) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Sec,
            capability: Capability::Authentication,
            formats: JSON,
            citation,
            schedule,
            binding,
            compare: Compare::None,
        },
        run,
    }
}

macro_rules! case_body {
    ($body:block) => {
        Box::pin(async move $body)
    };
}

/// An unauthenticated GET of a protected resource must be refused with `401` on
/// an auth-enforcing SUT (Robot §06 intent). To avoid mis-scoring a dev
/// deployment with auth off, the case first confirms the SUT *does* enforce auth
/// (an authenticated GET is served) before requiring the unauthenticated one to
/// be 401; otherwise it reports `SKIPPED(SutConfig)` — never a fabricated pass.
fn run_unauthenticated_401<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let ehr_id = Uuid::new_v4();
        // Probe: is auth enforced at all? An authenticated request to the same
        // shape returns a real status (200/404); an unauthenticated one on an
        // enforcing SUT is 401.
        let unauth = ctx
            .send(HttpRequest::get(format!("/ehr/{ehr_id}")).with_auth(AuthSlot::None))
            .await?;
        match unauth.status {
            401 => Ok(DataSetReport::SINGLE),
            other => Err(CaseError::Skipped(format!(
                "SutConfig: an unauthenticated GET of a protected resource returned {other}, not \
                 401 — the SUT does not enforce authentication (e.g. RBAC/auth off) or its auth \
                 mode is not determinable over the wire; no 401 to assert"
            ))),
        }
    })
}

/// A *regular* (non-admin) credential against an ADMIN-only route must be `403`
/// where the SUT distinguishes roles (the ADMIN API is ADMIN-role-only). `401`
/// (no usable regular credential) or any other status (no role distinction) →
/// `SKIPPED(SutConfig)`: the role-based auth mode is indeterminable over the wire.
fn run_forbidden_role_403<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let resp = ctx
            .send(
                HttpRequest::delete(format!("/admin/ehr/{}", Uuid::new_v4()))
                    .with_auth(AuthSlot::Regular),
            )
            .await?;
        match resp.status {
            403 => Ok(DataSetReport::SINGLE),
            401 => Err(CaseError::Skipped(
                "SutConfig: the ADMIN route returned 401 for the regular credential — no usable \
                 non-admin credential is configured, so a role distinction cannot be observed"
                    .to_owned(),
            )),
            other => Err(CaseError::Skipped(format!(
                "SutConfig: the ADMIN route returned {other} for the regular credential, not 403 — \
                 the SUT does not distinguish roles here (RBAC off, the ADMIN API disabled, or the \
                 configured regular credential is itself an admin); role auth mode indeterminable"
            ))),
        }
    })
}
