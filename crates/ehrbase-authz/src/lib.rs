//! Access control for the ehrbase-rs CDR — the two composable layers of
//! `docs/enterprise/access-control.md`:
//!
//! 1. **RBAC** (coarse, always on when auth is enabled): every generated
//!    ITS-REST operation is classified ([`classify`]) and gated by a role model
//!    ([`roles`]) driven by [`config::AuthzConfig`].
//! 2. **ABAC** (fine-grained, opt-in `abac.enabled`): a policy-decision-point
//!    seam ([`engine::PolicyEngine`]) consulted per clinical operation with
//!    resolved attributes (organization/patient/template), behind two
//!    interchangeable engines — an embedded Cedar engine (the default, added in
//!    step 5) and the v1-compatible [`remote::RemotePdp`].
//!
//! The crate is a leaf: it depends on `serde`/`serde_json`, `figment`,
//! `thiserror`, `cedar-policy`, `reqwest`, `arc-swap`, and `tokio` — **no
//! dependency on any `ehrbase-*` or `openehr-*` crate** (the generated
//! `openehr-its` `ROUTES` tables are consulted only by the dev-only
//! total-coverage guard, the same trick as `ehrbase-audit`).
//!
//! ## Module map (§4.1)
//! - [`config`] — the `figment` [`AuthzConfig`] (`EHRBASE_AUTHZ_` prefix) + boot
//!   validation.
//! - [`roles`] — the role model, JWT-claim role extraction, and the RBAC gate
//!   decision.
//! - [`classify`] — `operation_id → OperationClass` / `ResourceKind` /
//!   `AccessMode` (total-coverage-guarded).
//! - [`request`] — the ABAC [`AuthzRequest`] + multi-valued fan-out.
//! - [`engine`] — the [`PolicyEngine`] PDP seam + [`AuthzError`].
//! - [`cedar`] — the embedded Cedar engine (schema, policy loading, reload).
//! - [`remote`] — the v1-compatible remote PDP client.

pub mod cedar;
pub mod classify;
pub mod config;
pub mod engine;
pub mod remote;
pub mod request;
pub mod roles;

pub use classify::{OperationClass, access_of, class_of, kind_of};
pub use config::{
    AbacConfig, AbacEngineKind, AbacParam, AuthzConfig, AuthzConfigError, CedarConfig,
    ManagementAccess, PolicyRule, RbacConfig, RemoteConfig,
};
pub use engine::{AuthzError, PolicyEngine};
pub use request::{AccessMode, Attr, AuthzRequest, Combination, Decision, ResourceKind};
pub use roles::{RbacDecision, authorize, default_role_claims, extract_roles};
