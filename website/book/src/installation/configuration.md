# Configuration reference

EHRbase-rs is configured entirely through `EHRBASE_*` environment variables,
optionally backed by TOML files for values that do not fit cleanly in env (a
Basic-auth user store, a full OIDC block, ABAC policies, and so on). This
chapter is the complete reference: how configuration loads, the two naming
conventions you must know, and a table per area listing every key with its
type, default, and meaning. Everything here is drawn from the server's own
configuration code.

<!-- toc -->

## How configuration loads

There is no single global configuration object. The server is composed of
independent modules, each of which loads its own settings from three layers, in
increasing precedence:

1. **Built-in defaults** (the values in the tables below),
2. an **optional TOML file** for that module (pointed at by an
   `EHRBASE_<AREA>_CONFIG` variable), then
3. **`EHRBASE_<AREA>_`-prefixed environment variables**, which win.

The development Compose stack, for example, points `EHRBASE_REST_CONFIG` at
`docker/ehrbase.dev.toml` to supply the Basic-auth user store (which env cannot
carry as a list), while everything else stays on defaults or env overrides.

> [!WARNING]
> **Two naming conventions.** Most modules use a **double underscore** (`__`) to
> separate nested fields — `EHRBASE_REST_AUTH__ENABLED` maps to `auth.enabled`.
> There are two exceptions:
>
> - **Telemetry** is flat: `EHRBASE_OTEL_*` and `EHRBASE_LOG_*` have no nesting.
> - **Management** uses a **single** underscore for its one nested group:
>   `EHRBASE_MANAGEMENT_ENDPOINTS_HEALTH`, not `__`.
>
> Getting the separator wrong is the most common configuration mistake.

Enum values are case-sensitive on the wire. Where a column lists
`enum{a,b,c}`, use exactly those lowercase (or `snake_case`) tokens. Secret
values (`EHRBASE_SIGNING_KEY_PASSPHRASE`, `EHRBASE_REST_AUTH__OIDC__HMAC_SECRET`,
Basic-auth password hashes) are redacted from the management `/env` endpoint and
from logs.

## Server (REST)

Prefix `EHRBASE_REST_`, separator `__`, optional file `EHRBASE_REST_CONFIG`.

| Key | Type | Default | Description |
|---|---|---|---|
| `EHRBASE_REST_CONFIG` | path | none | Path to the REST TOML config file (loaded before env). |
| `EHRBASE_REST_BIND` | socket address | `0.0.0.0:8080` | Address the API listener binds. |
| `EHRBASE_REST_BASE_PATH` | string | `/ehrbase/rest/openehr/v1` | Base path all API routes hang off. |
| `EHRBASE_REST_MAX_IN_FLIGHT` | integer | `256` | Maximum API requests handled **concurrently** (not per second) before the server sheds load; raise it for high-throughput deployments. Requests beyond the cap are rejected immediately with `503 Service Unavailable` + `Retry-After: 1` (shed, never queued), so offered load beyond backend capacity cannot exhaust server memory. The `/status`, health, and discovery endpoints are never limited. `0` disables shedding. |
| `EHRBASE_REST_SWAGGER_UI` | boolean | `true` | Serve Swagger UI + the OpenAPI JSON. Consider off in production. |
| `EHRBASE_REST_CORS_PERMISSIVE` | boolean | `false` | Enable a permissive (development) CORS policy. |
| `EHRBASE_REST_ADMIN__ENABLED` | boolean | `false` | Mount the ADMIN API group (routes 404 when off). |
| `EHRBASE_REST_TERMINOLOGY__ENABLED` | boolean | `false` | Mount the terminology extension API group. |
| `EHRBASE_REST_EVENT_SUBSCRIPTION__ENABLED` | boolean | `false` | Mount the event-subscription admin extension API. |
| `EHRBASE_REST_FHIR__ENABLED` | boolean | `false` | Mount the FHIR R4 inbound/façade routes. |
| `EHRBASE_REST_TENANCY__ENABLED` | boolean | `false` | Activate multi-tenancy (tenant middleware + row-level scoping). |
| `EHRBASE_REST_TENANCY__CLAIM` | string | `tenant` | JWT-claim path carrying the tenant key. |
| `EHRBASE_REST_TENANCY__HEADER` | string | none | Dev-only request-header tenant override. Leave unset in production — a client header must not select a tenant. |

### System identity (`OPTIONS` conformance manifest)

Nested under `EHRBASE_REST_SYSTEM__` (part of the REST config). These fields
are reported by the conformance manifest served on `OPTIONS` at the API base
path (and at `/`); the endpoint list in that manifest is not configurable —
it always reflects the actually mounted API groups.

| Key | Type | Default | Description |
|---|---|---|---|
| `EHRBASE_REST_SYSTEM__SOLUTION` | string | `EHRbase-RS` | Product name reported. |
| `EHRBASE_REST_SYSTEM__SOLUTION_VERSION` | string | the build's version | Product version reported. |
| `EHRBASE_REST_SYSTEM__VENDOR` | string | `EHRbase-RS project` | Providing organisation. |
| `EHRBASE_REST_SYSTEM__RESTAPI_SPECS_VERSION` | string | the tested spec identity | The openEHR REST API edition reported (defaults to the development-edition identity the build is tested against). |
| `EHRBASE_REST_SYSTEM__CONFORMANCE_PROFILE` | string | the last machine-computed verdict | Advertised conformance profile. |

### SMART App Launch

Nested under `EHRBASE_REST_SMART__` (part of the REST config). Off by
default; when off, the discovery document is not served and the scope gate is
inert. See [SMART App Launch](../smart-app-launch.md) for what each piece
does.

| Key | Type | Default | Description |
|---|---|---|---|
| `EHRBASE_REST_SMART__ENABLED` | boolean | `false` | Master SMART switch. |
| `EHRBASE_REST_SMART__PLATFORM_BASE_URL` | string | none (REST root) | Base the `/.well-known/smart-configuration` document hangs off. |
| `EHRBASE_REST_SMART__EHR_ID_CLAIM` | string | `ehrId` | Token claim carrying the launch context's EHR id. |
| `EHRBASE_REST_SMART__PATIENT_CLAIM` | string | `patient` | Fallback launch-context claim. |
| `EHRBASE_REST_SMART__REQUIRE_SMART_SCOPES` | boolean | `false` | Fail-closed: deny Bearer tokens with no matching SMART scope on scope-governed operations. |
| `EHRBASE_REST_SMART__EPISODE__ENABLED` | boolean | `false` | Advertise + accept episode launch context (experimental, advisory). |
| `EHRBASE_REST_SMART__LAUNCH_BASE64_JSON` | boolean | `false` | Advertise the base64-JSON launch-parameter capability (experimental). |
| `EHRBASE_REST_SMART__ENDPOINTS__ISSUER` | URL | none (falls back to the OIDC issuer) | Advertised token issuer. |
| `EHRBASE_REST_SMART__ENDPOINTS__JWKS_URI` | URL | none | Advertised JWKS URL. |
| `EHRBASE_REST_SMART__ENDPOINTS__AUTHORIZATION_ENDPOINT` | URL | none | Advertised OAuth2 authorization endpoint. |
| `EHRBASE_REST_SMART__ENDPOINTS__TOKEN_ENDPOINT` | URL | none | Advertised OAuth2 token endpoint. |
| `EHRBASE_REST_SMART__ENDPOINTS__REGISTRATION_ENDPOINT` | URL | none | Advertised client-registration endpoint. |
| `EHRBASE_REST_SMART__ENDPOINTS__INTROSPECTION_ENDPOINT` | URL | none | Advertised introspection endpoint. |
| `EHRBASE_REST_SMART__ENDPOINTS__REVOCATION_ENDPOINT` | URL | none | Advertised revocation endpoint. |
| `EHRBASE_REST_SMART__ENDPOINTS__MANAGEMENT_ENDPOINT` | URL | none | Advertised user-management endpoint. |
| `EHRBASE_REST_SMART__ENDPOINTS__TOKEN_ENDPOINT_AUTH_METHODS_SUPPORTED` | list | `[]` | Advertised client auth methods. |
| `EHRBASE_REST_SMART__ENDPOINTS__GRANT_TYPES_SUPPORTED` | list | `[]` | Advertised grant types (`implicit`/password are rejected at boot). |
| `EHRBASE_REST_SMART__ENDPOINTS__RESPONSE_TYPES_SUPPORTED` | list | `[]` | Advertised response types. |
| `EHRBASE_REST_SMART__ENDPOINTS__CODE_CHALLENGE_METHODS_SUPPORTED` | list | `[]` | Advertised PKCE methods. |
| `EHRBASE_REST_SMART__ENDPOINTS__SCOPES_SUPPORTED` | list | `[]` (built-in default list) | Advertised scopes; empty = the defaults the server enforces. |

## Authentication

Nested under `EHRBASE_REST_AUTH__` (part of the REST config). The Basic-auth
user store is a list and is realistically supplied via the mounted TOML file;
the env forms are shown for completeness.

| Key | Type | Default | Description |
|---|---|---|---|
| `EHRBASE_REST_AUTH__ENABLED` | boolean | `true` | Master auth switch. `false` = all requests pass unauthenticated (development only). |
| `EHRBASE_REST_AUTH__ADMIN_SCOPE` | string | none | Deprecated back-compat scope→role alias; still consulted by the management admin gate. |
| `EHRBASE_REST_AUTH__BASIC__USERS` | list of `{username, password_hash, roles}` | none (Basic off) | Basic-auth user store. Passwords are Argon2 PHC hashes; per-user `roles` default to `["USER"]`. Set via TOML. |
| `EHRBASE_REST_AUTH__OIDC__ISSUER` | URL | none (bearer off) | Expected token issuer (`iss`); also the OIDC discovery base. |
| `EHRBASE_REST_AUTH__OIDC__AUDIENCES` | list | `[]` (not checked) | Accepted `aud` values. |
| `EHRBASE_REST_AUTH__OIDC__ALGORITHMS` | list | `["RS256"]` | Accepted JWT signature algorithms. |
| `EHRBASE_REST_AUTH__OIDC__HMAC_SECRET` | string (secret) | none | Symmetric HS256 secret (development/test). Prefer JWKS/discovery in production. |
| `EHRBASE_REST_AUTH__OIDC__JWKS_JSON` | string (JSON) | none | Static JWKS document; preferred over discovery when present. |

## Authorization (RBAC + ABAC)

Prefix `EHRBASE_AUTHZ_`, separator `__`, optional file `EHRBASE_AUTHZ_CONFIG`.

| Key | Type | Default | Description |
|---|---|---|---|
| `EHRBASE_AUTHZ_CONFIG` | path | none | Path to the authz TOML config file. |
| `EHRBASE_AUTHZ_RBAC__ENABLED` | boolean | `true` | Coarse role gate (active only when auth is enabled). |
| `EHRBASE_AUTHZ_RBAC__ADMIN_ROLE` | string | `ADMIN` | Role required for admin-class operations. |
| `EHRBASE_AUTHZ_RBAC__USER_ROLE` | string | `USER` | Baseline clinical role. |
| `EHRBASE_AUTHZ_RBAC__ROLE_CLAIMS` | list | `["realm_access.roles","scope"]` | JWT claim paths mined for roles. |
| `EHRBASE_AUTHZ_RBAC__MANAGEMENT_ACCESS` | enum{admin_only,private,public} | `admin_only` | Access level for the management surface. |
| `EHRBASE_AUTHZ_ABAC__ENABLED` | boolean | `false` | Master ABAC (attribute-based) switch. |
| `EHRBASE_AUTHZ_ABAC__ENGINE` | enum{cedar,remote} | `cedar` | Policy engine: embedded Cedar or a remote decision point. |
| `EHRBASE_AUTHZ_ABAC__ORGANIZATION_CLAIM` | string | `organization_id` | JWT claim carrying the caller's organization. |
| `EHRBASE_AUTHZ_ABAC__PATIENT_CLAIM` | string | `patient_id` | JWT claim carrying the patient id (blank disables the subject gate). |
| `EHRBASE_AUTHZ_ABAC__CEDAR__POLICY_DIR` | path | none | Directory of `*.cedar` policy files (required for the `cedar` engine). |
| `EHRBASE_AUTHZ_ABAC__CEDAR__RELOAD_SECS` | integer | none | Optional Cedar hot-reload interval (seconds). |
| `EHRBASE_AUTHZ_ABAC__REMOTE__SERVER` | URL | none | Remote decision-point base URL (required for the `remote` engine). |
| `EHRBASE_AUTHZ_ABAC__REMOTE__CONNECT_TIMEOUT_MS` | integer (ms) | `2000` | Remote-PDP connect timeout. |
| `EHRBASE_AUTHZ_ABAC__REMOTE__REQUEST_TIMEOUT_MS` | integer (ms) | `5000` | Remote-PDP request timeout. |

## Database

Prefix `EHRBASE_DB_`, no nesting, environment-only (no config file).

| Key | Type | Default | Description |
|---|---|---|---|
| `EHRBASE_DB_URL` | URL | none (**required**) | PostgreSQL connection URL, `postgres://user:pass@host:port/db`. `DATABASE_URL` is accepted as a fallback. |
| `EHRBASE_DB_MAX_CONNECTIONS` | integer | `20` | Upper bound of the connection pool. Size to your PostgreSQL `max_connections` budget; write-heavy deployments benefit from 50+. |
| `EHRBASE_DB_MIN_CONNECTIONS` | integer | `2` | Idle connections the pool keeps open (avoids cold connection churn under variable load). |
| `EHRBASE_DB_ACQUIRE_TIMEOUT_SECS` | integer (s) | `30` | Wait for a free connection before failing. |

> [!NOTE]
> `EHRBASE_DB_NAME`, `EHRBASE_DB_USER`, and `EHRBASE_DB_PASSWORD` are **not**
> read by the server — they configure the PostgreSQL init image. The server
> takes a single `EHRBASE_DB_URL`.

## Query execution

A single environment-only key (no file, no nesting group behind it).

| Key | Type | Default | Description |
|---|---|---|---|
| `EHRBASE_QUERY__TIMEOUT_MS` | integer (ms) | unset (no per-query cap) | Per-query execution budget. `0` or unset disables it; when positive, an AQL query that exceeds the budget returns `408 Request Timeout`. |
| `EHRBASE_QUERY__PLAN_CACHE_CAPACITY` | integer | `256` | Maximum number of distinct AQL query plans held in the in-memory plan cache. A repeated query text reuses its lowered plan instead of re-parsing on every execution (parameter values, `fetch`/`offset` paging, and EHR scope still bind per request); queries that resolve terminology are never cached. `0` disables the cache. Cache activity is reported by the `aql_plan_cache_events_total` metric. |

## Telemetry and logging

Prefixes `EHRBASE_OTEL_` and `EHRBASE_LOG_`. **Flat** (no `__` nesting),
environment-only.

| Key | Type | Default | Description |
|---|---|---|---|
| `EHRBASE_OTEL_OTLP_ENDPOINT` | URL | none | OTLP/gRPC collector endpoint. **Unset = the OTel layer is not installed** (zero overhead). |
| `EHRBASE_OTEL_SERVICE_NAME` | string | `ehrbase` | `service.name` resource attribute. |
| `EHRBASE_OTEL_ENVIRONMENT` | string | `dev` | `deployment.environment` resource attribute. |
| `EHRBASE_OTEL_TRACES_SAMPLE_RATIO` | float | `1.0` | Head-sampling ratio. |
| `EHRBASE_OTEL_METRICS_PUSH` | boolean | `false` | Also push metrics over OTLP (alongside Prometheus pull). |
| `EHRBASE_LOG_FORMAT` | enum{auto,json,pretty} | `auto` | Log rendering. `json` for cluster log collectors; `auto` picks JSON when stdout is not a TTY. |
| `EHRBASE_LOG_FILTER` | string | `info,ehrbase=info` | Log-filter directives (`RUST_LOG` is the fallback when unset). |

## Management surface

Prefix `EHRBASE_MANAGEMENT_`, **single-underscore** nesting for endpoints,
optional file `EHRBASE_MANAGEMENT_CONFIG`. Off in the bare binary; the Helm
chart turns it on for probes.

| Key | Type | Default | Description |
|---|---|---|---|
| `EHRBASE_MANAGEMENT_CONFIG` | path | none | Path to the management TOML config file. |
| `EHRBASE_MANAGEMENT_ENABLED` | boolean | `false` | Master switch; off = no management routes mounted. |
| `EHRBASE_MANAGEMENT_BASE_PATH` | string | `/management` | Base path for the management endpoints. |
| `EHRBASE_MANAGEMENT_PORT` | integer (u16) | none | Serve management on its own listener/port instead of the main API listener. |
| `EHRBASE_MANAGEMENT_ACCESS_DEFAULT` | enum{off,admin_only,private,public} | `admin_only` | Global default access level (a per-endpoint level wins). |
| `EHRBASE_MANAGEMENT_PROBES_ENABLED` | boolean | `false` | Mount the public `/health/liveness` + `/health/readiness` probes. |
| `EHRBASE_MANAGEMENT_ENDPOINTS_HEALTH` | enum{off,admin_only,private,public} | `off` | Access level of `/management/health`. |
| `EHRBASE_MANAGEMENT_ENDPOINTS_INFO` | enum{off,admin_only,private,public} | `off` | Access level of `/management/info`. |
| `EHRBASE_MANAGEMENT_ENDPOINTS_METRICS` | enum{off,admin_only,private,public} | `off` | Access level of `/management/metrics`. |
| `EHRBASE_MANAGEMENT_ENDPOINTS_PROMETHEUS` | enum{off,admin_only,private,public} | `off` | Access level of `/management/prometheus`. |
| `EHRBASE_MANAGEMENT_ENDPOINTS_ENV` | enum{off,admin_only,private,public} | `off` | Access level of `/management/env` (redacted config). |
| `EHRBASE_MANAGEMENT_ENDPOINTS_LOGGERS` | enum{off,admin_only,private,public} | `off` | Access level of `/management/loggers` (runtime log control). |

## Version signing

Prefix `EHRBASE_SIGNING_`, separator `__`, optional file
`EHRBASE_SIGNING_CONFIG`. On by default in `digest` mode.

| Key | Type | Default | Description |
|---|---|---|---|
| `EHRBASE_SIGNING_CONFIG` | path | none | Path to the signing TOML config file. |
| `EHRBASE_SIGNING_ENABLED` | boolean | `true` | Server-side signing of committed versions. |
| `EHRBASE_SIGNING_MODE` | enum{digest,pgp} | `digest` | SHA-256 integrity digest, or an OpenPGP detached signature. |
| `EHRBASE_SIGNING_KEY_PATH` | path | none | Armored RFC 4880 secret key (required for `pgp`). |
| `EHRBASE_SIGNING_KEY_PASSPHRASE` | string (secret) | none | Key passphrase (kept in memory, never serialized). |
| `EHRBASE_SIGNING_VERIFY_ON_READ` | enum{off,warn,strict} | `off` | Read-time recompute-and-compare policy. |

> [!WARNING]
> `pgp` mode **fails closed at boot** if the key is missing or unusable — the
> server will not start. Verify the key and passphrase before switching modes.

## System log (ATNA auditing)

Prefix `EHRBASE_ATNA_`, separator `__`, optional file `EHRBASE_ATNA_CONFIG`.
Off by default.

| Key | Type | Default | Description |
|---|---|---|---|
| `EHRBASE_ATNA_CONFIG` | path | none | Path to the ATNA TOML config file. |
| `EHRBASE_ATNA_ENABLED` | boolean | `false` | Master ATNA audit switch. |
| `EHRBASE_ATNA_ENTERPRISE_SITE_ID` | string | none | Enterprise/site id (`AuditEnterpriseSiteID`). |
| `EHRBASE_ATNA_REPOSITORY_HOST` | string | `localhost` | Audit Record Repository (ARR) host. |
| `EHRBASE_ATNA_REPOSITORY_PORT` | integer (u16) | `514` | ARR port (514 UDP / 6514 TLS typical). |
| `EHRBASE_ATNA_TRANSPORT` | enum{udp,tls} | `udp` | Syslog transport to the ARR. Use `tls` for PHI-adjacent audit. |
| `EHRBASE_ATNA_SOURCE_ID` | string | `ehrbase` | Audit source id. |
| `EHRBASE_ATNA_VALUE_IF_MISSING` | string | `UNKNOWN` | Fill value for empty mandatory fields. |
| `EHRBASE_ATNA_SUPPRESS_LOGIN_EVENTS` | boolean | `true` | Skip auth/login activity events. |
| `EHRBASE_ATNA_FAIL_MODE` | enum{open,closed} | `open` | On undeliverable audit: succeed and meter (`open`) or reject with 503 (`closed`). |
| `EHRBASE_ATNA_RESOLVE_SUBJECT` | boolean | `false` | Enrich the patient participant via a subject lookup. |
| `EHRBASE_ATNA_QUEUE_CAPACITY` | integer | `1024` | Bounded audit queue capacity. |
| `EHRBASE_ATNA_SERVER_HOST` | string | none | This node's advertised address (`NetworkAccessPointID`). |
| `EHRBASE_ATNA_TLS_CA_PATH` | path | none | PEM CA file to trust for TLS transport. |
| `EHRBASE_ATNA_TLS_IDENTITY_CERT_PATH` | path | none | Client-certificate PEM for mutual TLS. |
| `EHRBASE_ATNA_TLS_IDENTITY_KEY_PATH` | path | none | Client-key PEM for mutual TLS. |

## Change events (AMQP outbox)

Prefix `EHRBASE_EVENTS_`, separator `__`, optional file `EHRBASE_EVENTS_CONFIG`.
Off by default. Envelopes are PHI-free by design.

| Key | Type | Default | Description |
|---|---|---|---|
| `EHRBASE_EVENTS_CONFIG` | path | none | Path to the events TOML config file. |
| `EHRBASE_EVENTS_ENABLED` | boolean | `false` | Spawn the outbox publisher. |
| `EHRBASE_EVENTS_URL` | AMQP URL | `amqp://guest:guest@localhost:5672/%2f` | RabbitMQ broker URL. |
| `EHRBASE_EVENTS_EXCHANGE` | string | `ehrbase.events` | Topic exchange for PHI-free event envelopes. |
| `EHRBASE_EVENTS_TLS` | boolean | `false` | Upgrade an `amqp://` URL to `amqps://`. |
| `EHRBASE_EVENTS_BATCH_SIZE` | integer | `128` | Outbox rows drained per poll. |
| `EHRBASE_EVENTS_POLL_INTERVAL_MS` | integer (ms) | `1000` | Poll interval when the outbox is idle. |
| `EHRBASE_EVENTS_RETENTION_DAYS` | integer (days) | `7` | Published-row retention window. |
| `EHRBASE_EVENTS_PRUNE_INTERVAL_SECS` | integer (s) | `3600` | Retention-prune cadence. |
| `EHRBASE_EVENTS_PUBLISH_MAX_RETRIES` | integer | `3` | Per-row publish retries before backing off. |

## FHIR outbound emitter

Prefix `EHRBASE_FHIR_OUTBOUND_`, separator `__`, optional file
`EHRBASE_FHIR_OUTBOUND_CONFIG`. Off by default.

| Key | Type | Default | Description |
|---|---|---|---|
| `EHRBASE_FHIR_OUTBOUND_CONFIG` | path | none | Path to the FHIR-outbound TOML config file. |
| `EHRBASE_FHIR_OUTBOUND_ENABLED` | boolean | `false` | Enable the FHIR resource emitter. |
| `EHRBASE_FHIR_OUTBOUND_URL` | AMQP URL | `amqp://guest:guest@localhost:5672/%2f` | Broker URL. |
| `EHRBASE_FHIR_OUTBOUND_EXCHANGE` | string | `ehrbase.fhir` | Topic exchange (separate from events, for PHI isolation). |
| `EHRBASE_FHIR_OUTBOUND_TLS` | boolean | `false` | Upgrade an `amqp://` URL to `amqps://`. |
| `EHRBASE_FHIR_OUTBOUND_BATCH_SIZE` | integer | `128` | Outbox rows scanned per poll. |
| `EHRBASE_FHIR_OUTBOUND_POLL_INTERVAL_MS` | integer (ms) | `1000` | Poll interval when idle. |
| `EHRBASE_FHIR_OUTBOUND_PUBLISH_MAX_RETRIES` | integer | `3` | Per-message publish retries before backing off. |

> [!WARNING]
> This stream carries **PHI** — the mapped FHIR resource. It is a deliberately
> separate switch and exchange from the PHI-free change-event stream so broker
> access control can isolate it. Enable it only against a TLS, access-controlled
> broker.

## S3 multimedia externalization

Prefix `EHRBASE_MULTIMEDIA_`, separator `__`, optional file
`EHRBASE_MULTIMEDIA_CONFIG`. Off by default (blobs stay inline, byte-identical).

| Key | Type | Default | Description |
|---|---|---|---|
| `EHRBASE_MULTIMEDIA_CONFIG` | path | none | Path to the multimedia TOML config file. |
| `EHRBASE_MULTIMEDIA_ENABLED` | boolean | `false` | Externalize large multimedia data to an object store. |
| `EHRBASE_MULTIMEDIA_THRESHOLD_BYTES` | integer (bytes) | `262144` (256 KiB) | Decoded size strictly above which data is offloaded. |
| `EHRBASE_MULTIMEDIA_ENDPOINT` | URL | none | S3-compatible endpoint. None = AWS default resolution. |
| `EHRBASE_MULTIMEDIA_BUCKET` | string | `openehr-multimedia` | Target bucket. |
| `EHRBASE_MULTIMEDIA_REGION` | string | `us-east-1` | AWS region (required even for non-AWS endpoints). |
| `EHRBASE_MULTIMEDIA_ACCESS_KEY_ID` | string | none | S3 access key id (none + no secret = anonymous). |
| `EHRBASE_MULTIMEDIA_SECRET_ACCESS_KEY` | string (secret) | none | S3 secret access key. |
| `EHRBASE_MULTIMEDIA_ALLOW_HTTP` | boolean | `false` | Allow plain-HTTP endpoints — development/test only. |

## External terminology validation

Prefix `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_`, separator `__`, optional file
`EHRBASE_VALIDATION_CONFIG`. Off by default (the in-process openEHR bundle is
used). Providers are a map keyed by a provider name (below shown as `<NAME>`,
conventionally `default`).

| Key | Type | Default | Description |
|---|---|---|---|
| `EHRBASE_VALIDATION_CONFIG` | path | none | Path to the terminology-validation TOML config file. |
| `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_ENABLED` | boolean | `false` | Activate external terminology validation. |
| `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_FAIL_ON_ERROR` | boolean | `false` | On TS/connectivity error, reject (fail-closed) vs accept (fail-open). |
| `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_PROVIDERS__<NAME>__TYPE` | enum{fhir} | `fhir` | Provider kind (FHIR R4). |
| `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_PROVIDERS__<NAME>__URL` | URL | none (required) | FHIR R4 base URL of the terminology server. |
| `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_PROVIDERS__<NAME>__OPERATION` | enum{validate_code,expand} | `validate_code` | Value-set membership operation. |
| `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_PROVIDERS__<NAME>__CONNECT_TIMEOUT_MS` | integer (ms) | `2000` | Per-provider connect timeout. |
| `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_PROVIDERS__<NAME>__REQUEST_TIMEOUT_MS` | integer (ms) | `10000` | Per-provider request timeout. |
| `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_PROVIDERS__<NAME>__OAUTH2_CLIENT` | string | none | Name of an OAuth2 client-credentials client for the provider. |

## Subject Proxy (FHIR frames)

Prefix `EHRBASE_SUBJECT_PROXY_`, separator `__`, optional file
`EHRBASE_SUBJECT_PROXY_CONFIG`. Empty by default — no external FHIR system is
reachable until one is named here (fail-closed). Systems are a map keyed by
the name subject-proxy frames use as their `system_id` (shown as `<NAME>`).
See [Subject Proxy](../beyond-core/subject-proxy.md).

| Key | Type | Default | Description |
|---|---|---|---|
| `EHRBASE_SUBJECT_PROXY_CONFIG` | path | none | Path to the subject-proxy TOML config file. |
| `EHRBASE_SUBJECT_PROXY__SYSTEMS__<NAME>__BASE_URL` | URL | none (required per system) | FHIR R4 base URL of the named system. |
| `EHRBASE_SUBJECT_PROXY__SYSTEMS__<NAME>__CONNECT_TIMEOUT_MS` | integer (ms) | `2000` | Per-system connect timeout. |
| `EHRBASE_SUBJECT_PROXY__SYSTEMS__<NAME>__REQUEST_TIMEOUT_MS` | integer (ms) | `10000` | Per-system request timeout. |

## Process / CLI

| Key | Type | Default | Description |
|---|---|---|---|
| `EHRBASE_HEALTHCHECK_URL` | URL | `http://127.0.0.1:8080/ehrbase/rest/status` | Target URL for the binary's `healthcheck` subcommand (container `HEALTHCHECK` and Kubernetes exec probes). |

## What belongs in a mounted file

Env variables cannot carry lists or nested structures cleanly. Put these in the
module's TOML file (via `EHRBASE_<AREA>_CONFIG`, or the Helm chart's
`config.files`) instead:

- the Basic-auth **user store** (`[[auth.basic.users]]`),
- a full **OIDC** block with multiple audiences/algorithms,
- **RBAC** role-claim lists and **ABAC** (Cedar) policies,
- the external **terminology** provider map,
- **ATNA** TLS certificate paths and the **PGP** signing key.

See `docker/ehrbase.dev.toml` in the repository for a worked example of the
REST config file (bind address, CORS, admin, and the Basic-auth users).
