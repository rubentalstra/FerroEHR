//! Access control for the ehrbase-rs CDR — the two composable layers of
//! `docs/enterprise/access-control.md`:
//!
//! 1. **RBAC** (coarse, always on when auth is enabled): every generated
//!    ITS-REST operation is classified ([`classify`]) and gated by a role model
//!    ([`roles`]) driven by [`config::AuthzConfig`].
//! 2. **ABAC** (fine-grained, opt-in): a policy-decision-point seam consulted
//!    per clinical operation — *added in the ABAC PR* (steps 4–8 of §11); this
//!    crate ships only the RBAC layer for now (steps 1–3).
//!
//! The crate is a pure leaf: it depends only on `serde`/`serde_json`, `figment`,
//! and `thiserror` — **no dependency on any `ehrbase-*` or `openehr-*` crate**
//! (the generated `openehr-its` `ROUTES` tables are consulted only by the
//! dev-only total-coverage guard, the same trick as `ehrbase-audit`).
//!
//! ## Module map (§4.1)
//! - [`config`] — the `figment` [`AuthzConfig`] (`EHRBASE_AUTHZ_` prefix) + boot
//!   validation.
//! - [`roles`] — the role model, JWT-claim role extraction, and the RBAC gate
//!   decision.
//! - [`classify`] — `operation_id → OperationClass` (total-coverage-guarded).

pub mod classify;
pub mod config;
pub mod roles;

pub use classify::{OperationClass, class_of};
pub use config::{AuthzConfig, AuthzConfigError, ManagementAccess, RbacConfig};
pub use roles::{RbacDecision, authorize, default_role_claims, extract_roles};
