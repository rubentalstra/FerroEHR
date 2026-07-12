//! openEHR **SMART App Launch and Service Discovery** (development edition)
//! — `docs/specs/openehr/ITS-REST/docs/smart_app_launch/master00-09*.adoc`.
//!
//! Register + full design: `docs/design/its-rest/smart.md`. This server is
//! the SMART *Platform's openEHR CDR* (the `org.openehr.rest` resource
//! server) — the three capabilities that belong here (master02 §Overview
//! capability split):
//!
//! - [`discovery`] — the `/.well-known/smart-configuration` document
//!   (master04 §Service Discovery), served pre-auth;
//! - [`scope`] — the master08 resource-scope grammar
//!   (`compartment/resource.permission`, `*`/`**`/`ns::*` patterns) parsed
//!   from the validated token's `scope` claim;
//! - [`enforce`] — scope enforcement riding the existing ABAC PEP
//!   ([`crate::extensions::abac`]), AND-composed after RBAC/Cedar, plus the
//!   `ehrId`/`patient` launch-context binding (master07/master09).
//!
//! Registration (master03), token issuance/grants/PKCE (master06), and
//! launch-sequence UI (master07) are Authorization-Server/Launcher duties —
//! out of scope for a CDR, recorded as PORT NOTEs in the register.
//! Config-gated ([`config`]): off by default, zero wire drift when disabled.

pub mod config;
pub mod discovery;
pub mod enforce;
pub mod scope;
