//! Extension surface — **nothing in this module is governed by ITS-REST**.
//!
//! Quarantined spec-silent designs, each flagged at its module: [`access`]
//! (authn Basic/OAuth2/OIDC + Cedar RBAC/ABAC authz — ITS-REST places auth
//! out of band, `overview/Requests_and_responses.md` §Authentication and
//! authorization), [`abac`] (the ABAC PEP), [`management`] (health/metrics/
//! info), [`audit`]/[`audit_table`] (the ATNA middleware + op
//! classification), [`openapi`] (Swagger serving), [`event_subscription`] +
//! [`fhir`] (eventing / FHIR connector wires), [`terminology`] (the
//! I_TERMINOLOGY_SERVICE extension wire), [`tenant_routes`] (multi-tenancy).

pub mod abac;
pub mod access;
pub mod audit;
pub mod audit_table;
pub mod event_subscription;
pub mod fhir;
pub mod management;
pub mod openapi;
pub mod tenant_routes;
pub mod terminology;
