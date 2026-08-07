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
| health | `GET /health`<br>`GET /health/liveness`<br>`GET /health/readiness` | always on — no toggle; mounted outside the API base path and outside authentication (app/ferroehr-rest/src/extensions/health.rs) |
| server-status | `GET /ferroehr/rest/status` | always on — no toggle; mounted at the REST root (the base path minus /openehr/v1), outside authentication (app/ferroehr-rest/src/overview/status.rs) |
| openapi-meta | `GET /ferroehr/rest/api-docs/openapi.json`<br>`GET /ferroehr/rest/api-docs/ferroehr-ehr.openapi.json`<br>`GET /ferroehr/rest/api-docs/ferroehr-query.openapi.json`<br>`GET /ferroehr/rest/api-docs/ferroehr-definition.openapi.json`<br>`GET /ferroehr/rest/api-docs/ferroehr-demographic.openapi.json`<br>`GET /ferroehr/rest/api-docs/ferroehr-admin.openapi.json`<br>`GET /ferroehr/rest/api-docs/ferroehr-management.openapi.json`<br>`GET /ferroehr/rest/api-docs/ferroehr-terminology.openapi.json`<br>`GET /ferroehr/rest/api-docs/ferroehr-relationships.openapi.json`<br>`GET /ferroehr/rest/api-docs/ferroehr-events.openapi.json`<br>`GET /ferroehr/rest/api-docs/ferroehr-tenancy.openapi.json`<br>`GET /ferroehr/rest/api-docs/ferroehr-fhir.openapi.json`<br>`GET /ferroehr/rest/api-docs/ferroehr-smart.openapi.json`<br>`GET /ferroehr/rest/swagger-ui`<br>`GET /ferroehr/rest/swagger-ui/{*file}` | server.swagger_ui (default on) for the whole family; the document paths follow server.openapi_json_path()/swagger_ui_path(), which derive from server.base_path |
| management | `GET /management/info`<br>`GET /management/prometheus`<br>`GET /management/metrics`<br>`GET /management/metrics/{name}`<br>`GET /management/env`<br>`GET /management/loggers`<br>`POST /management/loggers`<br>`DELETE /management/loggers` | observability management.enabled (default off); with management.port set the family moves to its own listener instead of the API listener — when off, the routes are absent (a router 404), not gated in-handler |
| terminology | `GET /ferroehr/rest/openehr/v1/terminology`<br>`GET /ferroehr/rest/openehr/v1/terminology/{terminology_id}`<br>`GET /ferroehr/rest/openehr/v1/terminology/{terminology_id}/term/{code}`<br>`GET /ferroehr/rest/openehr/v1/terminology/{terminology_id}/subsumes`<br>`GET /ferroehr/rest/openehr/v1/terminology/{terminology_id}/value_set/{value_set_id}`<br>`GET /ferroehr/rest/openehr/v1/terminology/{terminology_id}/value_set/{value_set_id}/validate` | [terminology].api_enabled (default off); the gate sits in-handler behind authentication, so a disabled group answers 401 to an unauthenticated caller and 404 to an authenticated one |
| event-subscription | `GET /ferroehr/rest/openehr/v1/admin/event_subscription`<br>`POST /ferroehr/rest/openehr/v1/admin/event_subscription`<br>`GET /ferroehr/rest/openehr/v1/admin/event_subscription/{subscription_id}`<br>`PUT /ferroehr/rest/openehr/v1/admin/event_subscription/{subscription_id}`<br>`DELETE /ferroehr/rest/openehr/v1/admin/event_subscription/{subscription_id}` | [events].admin_api (default off); the gate sits in-handler behind authentication (401 unauthenticated, 404 authenticated when off) |
| tenancy | `GET /ferroehr/rest/openehr/v1/admin/tenant`<br>`POST /ferroehr/rest/openehr/v1/admin/tenant`<br>`GET /ferroehr/rest/openehr/v1/admin/tenant/{tenant_id}`<br>`PUT /ferroehr/rest/openehr/v1/admin/tenant/{tenant_id}`<br>`DELETE /ferroehr/rest/openehr/v1/admin/tenant/{tenant_id}` | [tenancy].enabled (default off); the routes stay mounted and answer 404 in-handler when off (401 first for an unauthenticated caller) |
| fhir-r4-connector | `POST /ferroehr/rest/openehr/v1/fhir/r4/{resource_type}`<br>`GET /ferroehr/rest/openehr/v1/fhir/r4/{resource_type}` | [fhir].api_enabled (default off); the gate sits in-handler behind authentication and refuses with a FHIR OperationOutcome |
| fhir-mapping-store | `GET /ferroehr/rest/openehr/v1/admin/fhir_mapping`<br>`POST /ferroehr/rest/openehr/v1/admin/fhir_mapping`<br>`GET /ferroehr/rest/openehr/v1/admin/fhir_mapping/{mapping_id}`<br>`PUT /ferroehr/rest/openehr/v1/admin/fhir_mapping/{mapping_id}`<br>`DELETE /ferroehr/rest/openehr/v1/admin/fhir_mapping/{mapping_id}` | [fhir].api_enabled (default off) — the same gate as the connector, same in-handler posture |
| iti-81-audit-record-repository | `GET /ferroehr/rest/openehr/v1/fhir/r4/AuditEvent` | the local Audit Record Repository (ferroehr::system_log, on by default); answers 404 when the repository is disabled |
| party-relationship | `POST /ferroehr/rest/openehr/v1/demographic/party_relationship`<br>`GET /ferroehr/rest/openehr/v1/demographic/party_relationship/{uid_based_id}`<br>`PUT /ferroehr/rest/openehr/v1/demographic/party_relationship/{uid_based_id}`<br>`DELETE /ferroehr/rest/openehr/v1/demographic/party_relationship/{uid_based_id}`<br>`GET /ferroehr/rest/openehr/v1/demographic/versioned_party_relationship/{versioned_object_uid}`<br>`GET /ferroehr/rest/openehr/v1/demographic/versioned_party_relationship/{versioned_object_uid}/revision_history`<br>`GET /ferroehr/rest/openehr/v1/demographic/versioned_party_relationship/{versioned_object_uid}/version`<br>`GET /ferroehr/rest/openehr/v1/demographic/versioned_party_relationship/{versioned_object_uid}/version/{version_uid}` | always on with the DEMOGRAPHIC group (no separate toggle) |
| stored-query-bare-list | `GET /ferroehr/rest/openehr/v1/definition/query` | always on with the DEFINITION group (no separate toggle) |
| admin-extension-routes | `DELETE /ferroehr/rest/openehr/v1/admin/template/{template_id}`<br>`DELETE /ferroehr/rest/openehr/v1/admin/query/{qualified_query_name}/{version}`<br>`GET /ferroehr/rest/openehr/v1/admin/config` | [admin].enabled (default off) — the same gate as the released admin deletes; 405 when off, RBAC Admin class by the /admin/ path |
| adl14-archetype | `POST /ferroehr/rest/openehr/v1/definition/archetype/adl1.4`<br>`GET /ferroehr/rest/openehr/v1/definition/archetype/adl1.4`<br>`GET /ferroehr/rest/openehr/v1/definition/archetype/adl1.4/{archetype_id}`<br>`DELETE /ferroehr/rest/openehr/v1/definition/archetype/adl1.4/{archetype_id}` | always on with the DEFINITION group (no separate toggle) — the same posture as the released template routes |
| adl2-archetype | `GET /ferroehr/rest/openehr/v1/definition/archetype/adl2`<br>`GET /ferroehr/rest/openehr/v1/definition/archetype/adl2/count`<br>`GET /ferroehr/rest/openehr/v1/definition/artefact/adl2`<br>`GET /ferroehr/rest/openehr/v1/definition/artefact/adl2/count`<br>`DELETE /ferroehr/rest/openehr/v1/definition/artefact/adl2/{artefact_id}` | always on with the DEFINITION group (no separate toggle) |
| admin-activity-report | `GET /ferroehr/rest/openehr/v1/admin/report/contribution`<br>`GET /ferroehr/rest/openehr/v1/admin/report/contribution/count`<br>`GET /ferroehr/rest/openehr/v1/admin/report/versioned_composition/count`<br>`GET /ferroehr/rest/openehr/v1/admin/report/composition_version/count` | [admin].enabled (default off) — the same gate as the released admin deletes; 405 with an empty Allow when off, RBAC Admin class by the /admin/ path (401 unauthenticated, 403 non-admin). BOTH RBAC branches are CASED (I_ADMIN_SERVICE.contribution_count-unauthenticated / -forbidden sweep all four routes). The gate-OFF 405 branch is NOT cased and the reason is topology, not vocabulary: a case cannot ask the SUT to be reconfigured mid-run, and the ixit models no disabled-admin deployment (the same adjudication the released admin-bulk-delete-disabled-405 element records, which this family's gate is literally the same code path as). Composing a second, admin-disabled deployment purely for this branch would double the stack for one status code that carries no clinical semantics, so the honest exclusion stands; the branch is HTTP-tested in-process (app/ferroehr-rest/tests/admin_extension_http.rs the_admin_gate_covers_the_extension_groups, which drives all four extension groups). |
| admin-archive | `POST /ferroehr/rest/openehr/v1/admin/archive/ehrs`<br>`POST /ferroehr/rest/openehr/v1/admin/archive/parties` | [admin].enabled (default off) — the same gate and RBAC class as the released admin deletes. Both RBAC branches are CASED per half (I_ADMIN_ARCHIVE.archive_ehrs-unauthenticated / -forbidden and the archive_parties siblings); the gate-OFF 405 branch is excluded for the same topology reason the admin-activity-report family records. |
| admin-dump-load | `POST /ferroehr/rest/openehr/v1/admin/dump`<br>`POST /ferroehr/rest/openehr/v1/admin/load` | [admin].enabled (default off) — the same gate and RBAC class as the released admin deletes. Both RBAC branches are CASED over BOTH halves (I_ADMIN_DUMP_LOAD.export_ehrs-unauthenticated / -forbidden each drive export_ehrs and load_ehrs); the gate-OFF 405 branch is excluded for the same topology reason the admin-activity-report family records. |
| message-extract | `GET /ferroehr/rest/openehr/v1/message/export/{ehr_id}`<br>`POST /ferroehr/rest/openehr/v1/message/export`<br>`POST /ferroehr/rest/openehr/v1/message/import`<br>`POST /ferroehr/rest/openehr/v1/message/import/{ehr_id}` | always on with the API surface (no separate toggle); the ordinary clinical authentication class, NOT the /admin/ gate — SM places these operations in the MESSAGE component, not ADMIN, and the content they move is the same EHR content the released clinical routes serve. No openEHR spec governs the privilege level of an unspecified route: our own design/extension. |
| message-tdd | `POST /ferroehr/rest/openehr/v1/message/tdd/{ehr_id}`<br>`POST /ferroehr/rest/openehr/v1/message/tdd/{ehr_id}/batch` | always on with the API surface (no separate toggle); the ordinary clinical authentication class — a TDD import commits a COMPOSITION through the same validated path the released composition_create uses. |

## Verdicts

| Profile | Verdict |
| --- | --- |
| CORE | FAIL |
| STANDARD | FAIL |
| OPTIONS | not claimed |
| Performance class POC | not earned |

## Attestation

No attestation supplied.
