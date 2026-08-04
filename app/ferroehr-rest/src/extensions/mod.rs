//! Extension surface — **nothing in this module is governed by ITS-REST**.
//!
//! Every module here is re-audited against the vendored openEHR specs and kept
//! only when a spec in the oracle does *not* in fact govern it. Each keeps
//! either an explicit "no openEHR spec governs this — our own design/extension"
//! flag or the precise citation of the spec that governs the *operation
//! semantics* it wraps (never a bare "extension" shrug).
//!
//! The quarantined spec-silent designs, each flagged at its own module:
//!
//! - [`access`] — authn (Basic/OAuth2/OIDC) + Cedar RBAC/ABAC authz + the PEP.
//!   ITS-REST places authorization out of band
//!   (`docs/specs/openehr/ITS-REST/specifications/docs/overview/Requests_and_responses.md`
//!   §Authentication and authorization); the `401`/`403`/`WWW-Authenticate`
//!   discipline there IS normative and is cited at the authn layer.
//! - [`health`] — the always-on public `/health`, `/health/liveness`,
//!   `/health/readiness` family (no openEHR spec governs health probes; the
//!   ITS-REST System API defines only `OPTIONS /`).
//! - [`management`] — metrics/info/env/loggers, a pure ops-introspection
//!   surface (no openEHR spec governs it). Hosts the single spec-version
//!   `provenance` source below (the module constants).
//! - [`openapi`] — serves the server's OWN `utoipa`-generated document (never a
//!   vendored OAS), not an API the spec itself defines.
//! - [`terminology`] — the `/terminology` wire: `I_TERMINOLOGY_SERVICE` is SM
//!   `master12`; the development-edition OAS set defines no terminology API, so
//!   the operation semantics are cited from SM and the wire shape is our own.
//! - [`event_subscription`] + [`fhir`] — eventing and the FHIR R4B connector
//!   (enterprise features E1/E3); nothing in SM/ITS-REST governs them.
//! - [`tenant_routes`] — multi-tenancy (enterprise feature E2); zero spec
//!   mentions.
//!
//! The ATNA audit middleware is NOT here — it realizes the SM System Log
//! component and lives at [`crate::system_log`].

pub mod access;
pub mod event_subscription;
pub mod fhir;
pub mod health;
pub mod management;
pub mod openapi;
pub mod tenant_routes;
pub mod terminology;

// The **single spec-version provenance source** every server *identity*
// surface reads, so the openEHR pins are stated once and cannot drift between
// endpoints.
//
// No openEHR spec governs an endpoint that reports the server's own spec/build
// provenance — this is our own operational surface. What such endpoints
// *report* must nonetheless be one fact: management `/info`
// ([`management::info`]), the System Options manifest (`OPTIONS /`,
// `crate::api::system::options`), and `/status` (`crate::overview::status`)
// must all quote the same versions. Before this consolidation the pins were
// copied into all three; they now read the constants below.
//
// The ITS-REST identity is the released `Release-1.1.0` (19-Jul-2026): the
// server implements the contract generated (`emit-rest`) from the vendored
// `-codegen` OAS pinned in `crates/openehr-its/vendor/rest-oas/PROVENANCE.md`
// — openEHR `specifications-ITS-REST` tag `Release-1.1.0`
// @ `24058992d5fa96e8dfbd855d9c133f328387fc09` (byte-identical to the earlier
// pre-release snapshot the contract was first generated against). This is the
// same tree the codegen consumes, so the reported version equals the
// vendored contract's identity. The ITS-REST spec *text*
// (`docs/specs/openehr/ITS-REST/`) is vendored at the same release tag and is
// the source of per-section citations.
//
// All three identity surfaces now read these constants — the former
// per-endpoint copies are retired: management `/info` ([`management::info`]),
// the System Options manifest (`OPTIONS /`,
// [`crate::api::system::options::SystemOptionsConfig`], whose
// `restapi_specs_version` / `conformance_profile` defaults quote
// [`provenance::ITS_REST`] / [`provenance::CONFORMANCE_PROFILE`]), and
// `/status` ([`crate::overview::status`]) all quote [`provenance::ITS_REST`]
// for the tested-contract identity. Spec pins are `docs/VERSIONS.md` (the
// single source of truth for the pin values themselves).
