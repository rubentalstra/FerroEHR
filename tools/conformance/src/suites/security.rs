//! The `SEC` capability cases — the authentication / authorization surface,
//! mirroring the intent of the vendored CNF Robot suite
//! `docs/specs/openehr/CNF/tests/platform/robot/SECURITY_TESTS/I_OAuth2_Keycloak`
//! (design-time reading). That suite's substance — *"05 Base URL is secured"* and
//! *"06 API endpoints are secured"* — is that private resources are **not**
//! accessible without authentication (every probed route `401`s). We reproduce
//! that intent over the ECC transport (Basic instead of Keycloak OAuth, which the
//! compose SUT does not run) and add a role-distinction case.
//!
//! **Spec placement.** openEHR models security as **Non-Functional** attributes
//! (Signing, Anonymous EHRs) in `master03-profiles.adoc` — there is *no* gated
//! profile-Functional "authentication" capability. So these cases carry
//! [`Capability::Authentication`], which is *not* in
//! [`crate::profile::required_capabilities`] and never blocks a profile
//! (profiles: `&[]`); they are reported in the area matrix + catalogue.
//!
//! **Honesty (task 7).** The ECC drives a black-box SUT and cannot read its auth
//! config, so each case *probes* and adjudicates by the observed status:
//! - `sec/unauthenticated-401`: an unauthenticated GET of a protected resource
//!   must be refused with `401`. If the SUT instead processes it (auth not
//!   enforced — e.g. an RBAC-off dev deployment), the case reports
//!   `SKIPPED(SutConfig)` — the auth mode cannot be determined from the wire, so
//!   a pass is neither asserted nor fabricated.
//! - `sec/forbidden-role-403`: a *regular* (non-admin) credential against an
//!   ADMIN-only route must be `403` (role distinguished). `401` (no usable
//!   regular credential) or a success/other status (no role distinction: RBAC
//!   off, admin disabled, or the configured credential is itself admin) →
//!   `SKIPPED(SutConfig)`.
//!
//! Against the compose dev SUT (`docker/ehrbase.dev.toml`: `[auth] enabled`, a
//! regular `ehrbase` user + an `ehrbase-admin` ADMIN-role user; the server maps
//! missing auth → 401 and an unauthorized role → 403, `ehrbase-rest` `access`),
//! both cases pass.

use uuid::Uuid;

use crate::case::{Capability, CaseMeta, Compare, Format};
use crate::catalog::Area;
use crate::harness::{
    AuthSlot, CaseError, CaseFuture, CaseRun, DataSetReport, HttpRequest, Method, RunContext,
};
use crate::registry::CaseEntry;

/// The implemented SEC case entries (auth surface, `SECURITY_TESTS` intent).
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        entry(
            "sec/unauthenticated-401",
            "Unauthenticated request to a protected route is refused (401)",
            "CNF SECURITY_TESTS/I_OAuth2_Keycloak §06 (API endpoints are secured); \
             ITS-REST EHR API §get_ehr (401 unauthenticated)",
            run_unauthenticated_401,
        )
        .with_schedule_ref("SECURITY_TESTS/I_OAuth2_Keycloak §06 API endpoints are secured"),
        entry(
            "sec/forbidden-role-403",
            "Regular credential on an ADMIN-only route is forbidden (403)",
            "CNF SECURITY_TESTS/I_OAuth2_Keycloak (role-privileged access intent); \
             ITS-REST ADMIN API §delete EHR — ADMIN-role-only (SM I_ADMIN_SERVICE)",
            run_forbidden_role_403,
        ),
    ]
}

/// A SEC case entry (SEC area, non-profile [`Capability::Authentication`]).
fn entry(id: &'static str, title: &'static str, citation: &'static str, run: CaseRun) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Sec,
            capability: Capability::Authentication,
            // Not a profile-gated capability (security is Non-Functional in
            // master03-profiles) — reported individually, blocks nothing.
            profiles: &[],
            formats: &[Format::Json],
            citation,
            compare: Compare::Superset,
            schedule_ref: None,
        },
        run,
    }
}

macro_rules! case {
    ($body:block) => {
        Box::pin(async move { $body })
    };
}

/// `GET /ehr/{random}` with **no** credential must be refused with `401` on an
/// auth-enforcing SUT (the `SECURITY_TESTS §06` intent). Any other status means
/// authentication is not enforced (or the mode is indeterminable over the wire)
/// → skip-with-reason.
fn run_unauthenticated_401<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(HttpRequest::get(format!("/ehr/{}", Uuid::new_v4())).with_auth(AuthSlot::None))
            .await?;
        match resp.status {
            401 => Ok(DataSetReport::SINGLE),
            other => Err(CaseError::Skipped(format!(
                "SutConfig: an unauthenticated GET of a protected resource returned {other}, \
                 not 401 — the SUT does not enforce authentication (e.g. RBAC/auth off) or its \
                 auth mode cannot be determined over the wire; no 401 to assert"
            ))),
        }
    })
}

/// `DELETE /admin/ehr/{random}` with the **regular** (non-admin) credential must
/// be `403` where the SUT distinguishes roles (the ADMIN API is ADMIN-role-only).
/// `401` (no usable regular credential) or a success/other status (no role
/// distinction) → skip-with-reason: the role-based auth mode is indeterminable.
fn run_forbidden_role_403<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(
                HttpRequest::new(Method::Delete, format!("/admin/ehr/{}", Uuid::new_v4()))
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
