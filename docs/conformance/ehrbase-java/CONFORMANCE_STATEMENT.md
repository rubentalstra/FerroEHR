# Conformance Statement (SDoC)

Product: EHRbase 2.34.0 — EHRbase (vitagroup / upstream open-source project) (urn:ehrbase:ehrbase)
Schedule release: cnf-2.0-w2

## Declared spec versions

- rm: 1.1.0
- base: 1.2.0
- aql: 1.1.0
- its_rest: 1.0.3
- term: 3.0.0

## Claims

Profiles claimed: CORE, STANDARD

Capabilities claimed:

- Adl14ArchetypeProvisioning
- Adl14OptProvisioning
- TemplateExamples
- QueryProvisioning
- EhrOperations
- EhrStatus
- CompositionOps
- DirectoryOps
- ChangeSets
- Versioning
- ArchetypeValidation
- AqlBasic
- AqlAdvanced
- PhysicalDeletion
- EhrApi
- DefinitionApi
- QueryApi
- AdminApi
- EhrDemographicSeparation
- AuthenticatedAccess
- AuthorizationSeparation

Options declared: contribution-xml-unsupported, directory-empty-error, directory-xml-supported, ehr-status-xml-supported, ehr-xml-supported, adl14-partial-id-exact, xml-namespace-fixed

## Additional non-openEHR surface

Beside the openEHR resources of ITS-REST 1.0.3, this product serves the route families below. **None of them is part of any conformance claim in this statement**: no openEHR specification governs them, no conformance case exercises them, and no verdict below depends on them. They are declared here so a reader of this document learns the surface exists rather than discovering it on the wire. Paths are the default deployment spelling; a non-default API base path moves the base-path-relative ones.

| Family | Routes | Enabled by |
| --- | --- | --- |
| health | `GET /health`<br>`GET /health/liveness`<br>`GET /health/readiness` | always on — no toggle; mounted outside the API base path and outside authentication (app/ehrbase-rest/src/extensions/health.rs) |
| server-status | `GET /ehrbase/rest/status` | always on — no toggle; mounted at the REST root (the base path minus /openehr/v1), outside authentication (app/ehrbase-rest/src/overview/status.rs) |
| openapi-meta | `GET /ehrbase/rest/api-docs/openapi.json`<br>`GET /ehrbase/rest/api-docs/ehrbase-ehr.openapi.json`<br>`GET /ehrbase/rest/api-docs/ehrbase-query.openapi.json`<br>`GET /ehrbase/rest/api-docs/ehrbase-definition.openapi.json`<br>`GET /ehrbase/rest/api-docs/ehrbase-demographic.openapi.json`<br>`GET /ehrbase/rest/api-docs/ehrbase-admin.openapi.json`<br>`GET /ehrbase/rest/api-docs/ehrbase-management.openapi.json`<br>`GET /ehrbase/rest/api-docs/ehrbase-terminology.openapi.json`<br>`GET /ehrbase/rest/api-docs/ehrbase-relationships.openapi.json`<br>`GET /ehrbase/rest/api-docs/ehrbase-events.openapi.json`<br>`GET /ehrbase/rest/api-docs/ehrbase-tenancy.openapi.json`<br>`GET /ehrbase/rest/api-docs/ehrbase-fhir.openapi.json`<br>`GET /ehrbase/rest/api-docs/ehrbase-smart.openapi.json`<br>`GET /ehrbase/rest/swagger-ui`<br>`GET /ehrbase/rest/swagger-ui/{*file}` | server.swagger_ui (default on) for the whole family; the document paths follow server.openapi_json_path()/swagger_ui_path(), which derive from server.base_path |
| management | `GET /management/info`<br>`GET /management/prometheus`<br>`GET /management/metrics`<br>`GET /management/metrics/{name}`<br>`GET /management/env`<br>`GET /management/loggers`<br>`POST /management/loggers`<br>`DELETE /management/loggers` | observability management.enabled (default off); with management.port set the family moves to its own listener instead of the API listener — when off, the routes are absent (a router 404), not gated in-handler |
| terminology | `GET /ehrbase/rest/openehr/v1/terminology`<br>`GET /ehrbase/rest/openehr/v1/terminology/{terminology_id}`<br>`GET /ehrbase/rest/openehr/v1/terminology/{terminology_id}/term/{code}`<br>`GET /ehrbase/rest/openehr/v1/terminology/{terminology_id}/subsumes`<br>`GET /ehrbase/rest/openehr/v1/terminology/{terminology_id}/value_set/{value_set_id}`<br>`GET /ehrbase/rest/openehr/v1/terminology/{terminology_id}/value_set/{value_set_id}/validate` | [terminology].api_enabled (default off); the gate sits in-handler behind authentication, so a disabled group answers 401 to an unauthenticated caller and 404 to an authenticated one |
| event-subscription | `GET /ehrbase/rest/openehr/v1/admin/event_subscription`<br>`POST /ehrbase/rest/openehr/v1/admin/event_subscription`<br>`GET /ehrbase/rest/openehr/v1/admin/event_subscription/{subscription_id}`<br>`PUT /ehrbase/rest/openehr/v1/admin/event_subscription/{subscription_id}`<br>`DELETE /ehrbase/rest/openehr/v1/admin/event_subscription/{subscription_id}` | [events].admin_api (default off); the gate sits in-handler behind authentication (401 unauthenticated, 404 authenticated when off) |
| tenancy | `GET /ehrbase/rest/openehr/v1/admin/tenant`<br>`POST /ehrbase/rest/openehr/v1/admin/tenant`<br>`GET /ehrbase/rest/openehr/v1/admin/tenant/{tenant_id}`<br>`PUT /ehrbase/rest/openehr/v1/admin/tenant/{tenant_id}`<br>`DELETE /ehrbase/rest/openehr/v1/admin/tenant/{tenant_id}` | [tenancy].enabled (default off); the routes stay mounted and answer 404 in-handler when off (401 first for an unauthenticated caller) |
| fhir-r4-connector | `POST /ehrbase/rest/openehr/v1/fhir/r4/{resource_type}`<br>`GET /ehrbase/rest/openehr/v1/fhir/r4/{resource_type}` | [fhir].api_enabled (default off); the gate sits in-handler behind authentication and refuses with a FHIR OperationOutcome |
| fhir-mapping-store | `GET /ehrbase/rest/openehr/v1/admin/fhir_mapping`<br>`POST /ehrbase/rest/openehr/v1/admin/fhir_mapping`<br>`GET /ehrbase/rest/openehr/v1/admin/fhir_mapping/{mapping_id}`<br>`PUT /ehrbase/rest/openehr/v1/admin/fhir_mapping/{mapping_id}`<br>`DELETE /ehrbase/rest/openehr/v1/admin/fhir_mapping/{mapping_id}` | [fhir].api_enabled (default off) — the same gate as the connector, same in-handler posture |
| iti-81-audit-record-repository | `GET /ehrbase/rest/openehr/v1/fhir/r4/AuditEvent` | the local Audit Record Repository (ehrbase::system_log, on by default); answers 404 when the repository is disabled |
| party-relationship | `POST /ehrbase/rest/openehr/v1/demographic/party_relationship`<br>`GET /ehrbase/rest/openehr/v1/demographic/party_relationship/{uid_based_id}`<br>`PUT /ehrbase/rest/openehr/v1/demographic/party_relationship/{uid_based_id}`<br>`DELETE /ehrbase/rest/openehr/v1/demographic/party_relationship/{uid_based_id}`<br>`GET /ehrbase/rest/openehr/v1/demographic/versioned_party_relationship/{versioned_object_uid}`<br>`GET /ehrbase/rest/openehr/v1/demographic/versioned_party_relationship/{versioned_object_uid}/revision_history`<br>`GET /ehrbase/rest/openehr/v1/demographic/versioned_party_relationship/{versioned_object_uid}/version`<br>`GET /ehrbase/rest/openehr/v1/demographic/versioned_party_relationship/{versioned_object_uid}/version/{version_uid}` | always on with the DEMOGRAPHIC group (no separate toggle) |
| stored-query-bare-list | `GET /ehrbase/rest/openehr/v1/definition/query` | always on with the DEFINITION group (no separate toggle) |
| admin-extension-routes | `DELETE /ehrbase/rest/openehr/v1/admin/template/{template_id}`<br>`DELETE /ehrbase/rest/openehr/v1/admin/query/{qualified_query_name}/{version}`<br>`GET /ehrbase/rest/openehr/v1/admin/config` | [admin].enabled (default off) — the same gate as the released admin deletes; 405 when off, RBAC Admin class by the /admin/ path |

## Verdicts

| Profile | Verdict |
| --- | --- |
| CORE | FAIL |
| STANDARD | FAIL |
| OPTIONS | not claimed |
| Performance class POC | not earned |

## Attestation

No attestation supplied.
