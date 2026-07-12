//! Access control for the ITS-REST surface — **authentication** (who the
//! caller is) and **authorization** (what they may do), unified under one
//! module (the crate-layout redesign, wave-2c; formerly the two floating `auth/` + `authz/`
//! sibling folders).
//!
//! Spec grounding (`docs/specs/openehr/ITS-REST/.../Requests_and_responses.md`
//! §"Authentication and authorization"; SM `master02-overview.adoc` §General
//! Assumptions; CNF `profiles/master03-profiles.adoc`):
//!
//! - **Authentication is a `SHOULD`, not a `MUST`.** If a framework is present,
//!   the service MUST use `WWW-Authenticate` and return `401`/`403` as
//!   applicable: `401 Unauthorized` = missing/invalid credentials,
//!   `403 Forbidden` = authenticated but refused, valid credentials → the
//!   request proceeds to resource logic. This is the whole Stage-1 conformance
//!   bar for the security surface.
//! - **Fine-grained RBAC/ABAC is NOT spec-mandated.** No CNF profile
//!   (CORE/STANDARD/OPTIONS) carries any authentication/authorization/roles
//!   requirement, and the SM treats authorization as an out-of-band
//!   precondition. RBAC + ABAC here are therefore the **Stage-2 enterprise**
//!   layer (`CLAUDE.md`: "RBAC is a Stage-2 concern"), kept working but clearly
//!   separated from the spec-grounded authn core.
//!
//! Layout:
//! - [`authn`] — Stage-1 authentication: Basic (`argon2`) + OAuth2/OIDC bearer
//!   (`jsonwebtoken`/`openidconnect`), producing a [`Principal`].
//! - [`authz`] — the Stage-2 enterprise authorization layer: the coarse RBAC
//!   gate (runs in the authn middleware) + the fine-grained ABAC engine
//!   (Cedar / remote PDP, enforced in the dispatcher, see
//!   [`crate::dispatch::abac`]).
//!
//! The authn↔authz seam is the [`Principal`]: authn resolves it, the RBAC gate
//! and the ABAC PEP consume it, and the single 401/403 decision lives in
//! [`authn::middleware`].

pub mod authn;
pub mod authz;
pub mod tenant;

// The authn surface (identity + the request-scoped principal). The middleware
// + `AuthLayer` are `pub(crate)` and installed by the router via `authn::`
// directly, so they are not re-exported here.
pub use authn::{
    AuthConfig, AuthError, AuthMethod, AuthenticatedUser, Authenticator, Principal,
    current_principal,
};

// The authz surface (the Stage-2 RBAC/ABAC handle + its config/engine seams).
pub use authz::{
    AuthzConfig, AuthzHandle, AuthzResolvers, PolicyEngine, ResolveError, build_engine,
};
