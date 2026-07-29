# Configuration reference

EHRbase-rs is configured by **one file — `ehrbase.toml`** — whose sections
cover the entire server, with `EHRBASE_*` environment variables (and repeatable
`--set` flags) as per-key overrides on top. This chapter is the complete
reference: the quickstart, how configuration loads and how env names map onto
the file, a subsection per configuration area, the CLI tools that validate and
scaffold a config, the production checklist, and — for upgraders — the old→new
variable map.

<!-- toc -->

## Quickstart

Generate an annotated template, edit it, and run:

```bash
# Write a fully-commented ehrbase.toml with every key at its default.
ehrbase config default > ehrbase.toml

# Edit it — at minimum set db.url and an auth mechanism (see the checklist below).
$EDITOR ehrbase.toml

# Validate without touching the database, then run.
ehrbase config check --config ehrbase.toml
ehrbase --config ehrbase.toml
```

A server started with **no file and no environment** still boots (see
[Zero-config boot](#zero-config-boot-and-the-production-checklist)) — the file
is optional, and every key has a default.

## How configuration loads

Configuration is assembled once at boot from four layers, lowest precedence to
highest:

1. **Built-in defaults** — the values in the tables below.
2. **The config file** — `ehrbase.toml` (see [file discovery](#file-discovery)).
3. **`EHRBASE_*` environment variables** — override individual keys.
4. **`--set key=value` CLI flags** (repeatable) — win over everything.

Two permanent conventional aliases sit *below* their `EHRBASE_` forms within layer 3:
`DATABASE_URL` → `db.url` and `RUST_LOG` → `log.filter`. Nothing else has a
non-`EHRBASE_` name.

### The environment-variable mapping

Every key has one mechanical env spelling: **`EHRBASE` + the TOML path,
upper-cased, with a double underscore (`__`) between every segment — including
after the `EHRBASE` prefix.** A single underscore only ever appears *inside* a
key word.

| TOML | Environment variable |
|---|---|
| `[db] max_connections = 20` | `EHRBASE__DB__MAX_CONNECTIONS=20` |
| `[auth.oidc] issuer = "…"` | `EHRBASE__AUTH__OIDC__ISSUER=…` |
| `[management.endpoints] env = "off"` | `EHRBASE__MANAGEMENT__ENDPOINTS__ENV=off` |
| `[terminology.external.providers.default] url = "…"` | `EHRBASE__TERMINOLOGY__EXTERNAL__PROVIDERS__DEFAULT__URL=…` |

Scalars are typed automatically (bool / int / float, else string).
**List-typed keys take comma-separated values**
(`EHRBASE__AUTH__OIDC__AUDIENCES=ehrbase,other`). Arrays of tables — the
Basic-auth user store — are **file-only**.

> [!NOTE]
> Enum values are lowercase / `snake_case` tokens, exactly as the tables show.
> Secret-typed keys are redacted everywhere the config is rendered (the
> `/management/env` snapshot, `ehrbase config check`, logs), and each has a
> `*_file` sibling that reads the value from a file (for Kubernetes/Docker
> secret mounts). Setting a secret and its `*_file` sibling at once is a boot
> error.

### File discovery

The first of these that exists is loaded (later layers still override its
values):

1. `--config <path>` (fatal if missing/unreadable),
2. `EHRBASE_CONFIG=<path>` (fatal if missing/unreadable),
3. `./ehrbase.toml` (current directory),
4. `/etc/ehrbase/ehrbase.toml`.

An explicitly pointed-at file (1–2) is fatal if absent; the search-order files
(3–4) are simply skipped when absent (but fatal if present and unparseable).

### Strict validation

Configuration is validated at boot (and by `ehrbase config check`), and the
server refuses to start on any error:

- **Unknown keys are rejected** — in the file (with the offending `file:line`)
  and in the `EHRBASE_` environment namespace — with a did-you-mean suggestion.
  A misspelled security key can no longer be silently ignored.
- **Type errors are boot errors**, naming the key, the expected type, and where
  the bad value came from.
- **Semantic errors are aggregated** — one pass reports every problem at once,
  so a broken config is fixed in a single iteration.

## `[server]`

The HTTP listener and REST surface.

```toml
[server]
bind = "0.0.0.0:8080"
base_path = "/ehrbase/rest/openehr/v1"
max_in_flight = 256
swagger_ui = true
cors_permissive = false
system_id = "ehrbase-rs.local"
```

| Key | Type | Default | Description |
|---|---|---|---|
| `bind` | string | `0.0.0.0:8080` | Socket address the API listener binds. |
| `base_path` | string | `/ehrbase/rest/openehr/v1` | ITS-REST base path all API routes hang off. The status, health-adjacent and documentation routes hang off its parent (`/ehrbase/rest` by default), and the served OpenAPI document describes whatever paths this setting produces — never the defaults. |
| `max_in_flight` | int | `256` | Concurrent-request admission cap (not per second). Requests beyond it are shed immediately with `503` + `Retry-After` — never queued — so offered load beyond capacity cannot exhaust memory. Status/health/discovery routes are never limited. `0` disables shedding. |
| `swagger_ui` | bool | `true` | Serve Swagger UI + the OpenAPI JSON at the REST root. Consider `false` in production. |
| `cors_permissive` | bool | `false` | Permissive (development) CORS. Production configures explicit origins. |
| `system_id` | string | `ehrbase-rs.local` | **This deployment's own openEHR system identifier** — see below. Set a stable, deployment-unique name in production (`EHRBASE__SERVER__SYSTEM_ID`). |

### `system_id`: the data-authoring identity

`system_id` is the identifier this CDR stamps into the data it authors. It
appears on the wire in three places:

- **`EHR.system_id`**, recorded when an EHR is created. The openEHR RM
  (`EHR Information Model`, *EHR Identifier Allocation*) says the
  `EHR.system_id` "should be set to the value that would normally be used for
  locally created EHRs" — i.e. a value the deployment chooses, not a product
  constant.
- **`AUDIT_DETAILS.system_id`** on every commit for which the client did not
  supply one through the `openehr-audit-details` header. The openEHR REST API
  requires that "when `system_id` is not provided by the client, the server
  MUST set it to its own configured system identifier".
- **`OBJECT_VERSION_ID.creating_system_id`** — the middle segment of every
  version identifier the server mints
  (`<object_id>::<creating_system_id>::<version>`).

Practical notes:

- **Choose it before going live and keep it stable.** The value is stored with
  each EHR and each version; changing it later affects only *newly* authored
  data — existing EHR ids, audit rows, and version identifiers are never
  rewritten, and previously issued `OBJECT_VERSION_ID`s stay valid.
- **Make it unique per system**, so data exchanged between openEHR systems
  keeps unambiguous provenance. A DNS-style host name (`cdr.hospital.example`)
  is the common convention.
- **With multi-tenancy on** (`[tenancy]`), a tenant's own `system_id` takes
  precedence over this value for requests resolved to that tenant.
- **Validated at boot.** An empty value is refused (the RM requires a
  non-empty `AUDIT_DETAILS.system_id`), as is one containing `::` — that is
  the `OBJECT_VERSION_ID` field separator, so it would make version
  identifiers unparseable.
- **`system_id` is not `[server.identity]`.** `system_id` says *which system
  authored the data*; `[server.identity]` below is the *display* identity of
  the `OPTIONS` System-Options manifest (product, version, vendor, advertised
  profile). Rebranding changes the manifest and nothing in stored data;
  changing `system_id` changes what new data says about its origin and leaves
  the manifest alone. They are set independently.

### `[server.tls]` — native TLS + mutual-TLS client authentication

Native TLS termination on the main listener (off by default — deployments
commonly terminate TLS at an ingress). Protocol floor: TLS 1.2+, per IETF
BCP 195. `client_auth = "required"` is the IHE ATNA ITI-19
mutually-authenticated-node posture (see the
[Audit trail chapter](../audit.md#node-authentication-iti-19-mutual-tls)).
The separate-port management listener always stays plain HTTP.

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | Terminate TLS natively on the main listener. |
| `cert_file` | path | unset | Server certificate chain (PEM). Required when enabled. |
| `key_file` | path | unset | Server private key (PEM). Required when enabled. |
| `client_auth` | enum{off,optional,required} | `off` | Client-certificate policy: `required` rejects any client without a verified certificate at the handshake. |
| `client_ca_file` | path | unset | The explicit CA bundle client certificates must chain to (never the web PKI). Required unless `client_auth = "off"`. |

### `[server.identity]`

The System-Options manifest identity (`OPTIONS` on the API base path, e.g.
`OPTIONS /ehrbase/rest/openehr/v1` — the System API's one location). Defaults
are measured, not asserted — the manifest never out-claims the last
conformance verdict. This is the deployment's *display* identity only; the
identifier stamped into authored data is
[`server.system_id`](#system_id-the-data-authoring-identity).

| Key | Type | Default | Description |
|---|---|---|---|
| `solution` | string | `EHRbase-RS` | Product name. |
| `solution_version` | string | build version | Product version. |
| `vendor` | string | `EHRbase-RS project` | Providing organisation. |
| `restapi_specs_version` | string | tested-contract identity | openEHR REST API edition the build is tested against. |
| `conformance_profile` | string | last CNF verdict | Advertised conformance profile. |

## `[db]`

PostgreSQL connection.

```toml
[db]
url = "postgres://ehrbase:ehrbase@localhost:5432/ehrbase"
max_connections = 20
min_connections = 2
acquire_timeout_secs = 30
```

| Key | Type | Default | Description |
|---|---|---|---|
| `url` | secret URL | `postgres://ehrbase:ehrbase@localhost:5432/ehrbase` | Connection DSN. The default matches the compose dev stack; **production MUST set it**. Credentials are redacted from every rendering. `DATABASE_URL` is a recognized lower-priority alias. |
| `max_connections` | int | `20` | Pool ceiling. Write-heavy deployments benefit from 50+. |
| `min_connections` | int | `2` | Idle connections kept open (avoids cold-reopen churn). |
| `acquire_timeout_secs` | int | `30` | Seconds to wait for a free connection before failing. |

## `[log]`

```toml
[log]
format = "auto"
filter = "info,ehrbase=info"
```

| Key | Type | Default | Description |
|---|---|---|---|
| `format` | enum{auto,json,pretty} | `auto` | Stdout rendering; `auto` picks `json` when stdout is not a TTY (and suppresses the boot banner). |
| `filter` | string | `info,ehrbase=info` | Boot `EnvFilter` directives; also the `/management/loggers` reset target. `RUST_LOG` is a recognized lower-priority alias. |

## `[telemetry]`

OpenTelemetry export. Unset `otlp_endpoint` ⇒ the OTel layer is not installed
(zero overhead).

| Key | Type | Default | Description |
|---|---|---|---|
| `otlp_endpoint` | string | unset | OTLP/gRPC collector endpoint. |
| `service_name` | string | `ehrbase` | `service.name` resource attribute. |
| `environment` | string | `dev` | `deployment.environment` resource attribute. |
| `traces_sample_ratio` | float | `1.0` | Head-sampling ratio (`0.1` is a common prod start). |
| `metrics_push` | bool | `false` | Also push metrics over OTLP alongside the Prometheus pull surface. |

## `[auth]`

Authentication (Basic + OAuth2/OIDC bearer).

```toml
[auth]
enabled = true
verified_cache_ttl_seconds = 60

[[auth.basic.users]]
username = "clinician"
password_hash = "$argon2id$v=19$..."   # Argon2 PHC hash, never plaintext
roles = ["USER"]

[auth.oidc]
issuer = "https://keycloak.example.com/realms/ehrbase"
audiences = ["ehrbase"]
algorithms = ["RS256"]
```

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `true` | Master switch. `false` = all requests pass unauthenticated (dev only). With `true` and no mechanism configured, every API request 401s (fail-closed). |
| `verified_cache_ttl_seconds` | int | `60` | Verified Basic-credential cache TTL (`0` disables); bounds Argon2 cost per busy client and revocation lag alike. |

`[[auth.basic.users]]` — the Basic-auth user store (array of tables,
**file-only**):

| Key | Type | Default | Description |
|---|---|---|---|
| `username` | string | required | Principal name. |
| `password_hash` | secret | required | Argon2 PHC hash (`$argon2id$v=19$…`), never a plaintext password. |
| `roles` | list of string | `["USER"]` | Roles granted (upper-cased on authentication). |

`[auth.oidc]` — bearer validation (absent table ⇒ bearer disabled):

| Key | Type | Default | Description |
|---|---|---|---|
| `issuer` | string | required when present | Expected `iss`; also the OIDC discovery base. |
| `audiences` | list of string | `[]` | Accepted `aud` (empty = not checked). |
| `algorithms` | list of string | `["RS256"]` | Accepted signature algorithms. |
| `hmac_secret` / `hmac_secret_file` | secret / path | unset | Symmetric HS256 secret (dev/test). At most one of the pair. |
| `jwks_json` / `jwks_json_file` | string / path | unset | Static JWKS document; preferred over discovery when present. |

## `[authz]`

RBAC + ABAC.

`[authz.rbac]`:

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `true` | Coarse role gate (active when auth is enabled). |
| `admin_role` | string | `ADMIN` | Role required for admin-class operations. |
| `user_role` | string | `USER` | Baseline clinical role. |
| `readonly_role` | string | `READONLY` | Role marking a principal read-only: refused on every write operation (create/update/delete/upload), even alongside granting roles. Reads and AQL queries are still allowed. |
| `role_claims` | list of string | `["realm_access.roles","scope"]` | JWT claim paths mined for roles. |
| `management_access` | enum{admin_only,private,public} | `admin_only` | Access level for the management surface. |

`[authz.abac]`:

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | Master ABAC switch. |
| `engine` | enum{cedar,remote} | `cedar` | Embedded Cedar or a remote decision point. |
| `organization_claim` | string | `organization_id` | JWT claim carrying the caller's organization. |
| `patient_claim` | string | `patient_id` | JWT claim carrying the patient id. |

`[authz.abac.cedar]`: `policy_dir` (path — required when `engine=cedar` and ABAC
on), `reload_secs` (int, unset — optional hot-reload interval).
`[authz.abac.remote]`: `server` (string — required when `engine=remote`, must
end `/`), `connect_timeout_ms` (int, `2000`), `request_timeout_ms` (int,
`5000`).
`[authz.abac.policy.<kind>]` (kind ∈ `ehr`, `ehr_status`, `composition`,
`contribution`, `query`, `directory`): `name` (string), `parameters` (list of
enum{organization,patient,template}).

## `[admin]`

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | Mount the ADMIN API (physical, irreversible delete). Off ⇒ every admin route answers `405 Method Not Allowed` with an empty `Allow` header, never 403. |

## `[tenancy]`

Multi-tenancy.

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | Activate tenant middleware + row-level scoping. |
| `claim` | string | `tenant` | JWT-claim path carrying the tenant key. |
| `header` | string | unset | Dev-only request-header tenant override. Leave unset in production — a client header must not select a tenant. |

## `[smart]`

SMART App Launch. Off by default; when off the discovery document is not served
and the scope gate is inert. See [SMART App Launch](../smart-app-launch.md).

`[smart]`: `enabled` (bool, `false`), `platform_base_url` (string, unset ⇒ REST
root), `public_base_url` (string, **required when enabled** — the external
origin, e.g. `https://cdr.example.com`, from which the discovery document's
absolute `services.*.baseUrl` values are built), `ehr_id_claim` (string,
`ehrId`), `patient_claim` (string, `patient`), `require_smart_scopes` (bool,
`false` — when `true` the SMART resource-scope gate is fail-closed across the
composition, template, and AQL families, and the `openehr-permission-v1`
capability is advertised; when `false` the gate is advisory and the capability
is not claimed), `launch_base64_json` (bool, `false`).
`[smart.episode]`: `enabled` (bool, `false`).
`[smart.endpoints]`: `issuer`, `jwks_uri`, `authorization_endpoint`,
`token_endpoint`, `registration_endpoint`, `introspection_endpoint`,
`revocation_endpoint`, `management_endpoint` (all string, unset ⇒ omitted from
the discovery document); `token_endpoint_auth_methods_supported`,
`grant_types_supported`, `response_types_supported`,
`code_challenge_methods_supported`, `scopes_supported`, `capabilities` (all
list of string, `[]` — `capabilities` appends operator-advertised HL7 base
capabilities such as `launch-ehr`/`sso-openid-connect` to the derived openEHR
set). Deprecated grant types (`implicit`/password) are rejected at boot;
`enabled = true` additionally requires `public_base_url`,
`authorization_endpoint`, and `token_endpoint` at boot.

## `[management]`

The ops-introspection surface (build info, Prometheus, metric views, effective
config, runtime log control). Off in the bare binary and every endpoint off
individually.

The **health probes are not configured here**: `/health`, `/health/liveness`,
and `/health/readiness` are always served on the main API port without
authentication, whatever this section says (see
[Operations → Health probes](../operations.md#health-probes)).

```toml
[management]
enabled = false
base_path = "/management"
access_default = "admin_only"

[management.endpoints]
info = "off"
metrics = "off"
prometheus = "off"
env = "off"
loggers = "off"
```

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | Mount the management router. |
| `base_path` | string | `/management` | Base path for the management endpoints. |
| `port` | int | unset ⇒ share the main listener | Serve management on its own listener/port. Must differ from the `server.bind` port. |
| `access_default` | enum{off,admin_only,private,public} | `admin_only` | Global default access level (a per-endpoint level wins). |

`[management.endpoints]` — `info`, `metrics`, `prometheus`, `env`, `loggers`,
each enum{off,admin_only,private,public}, default `off`.

> [!WARNING]
> `probes_enabled` and `endpoints.health` were **removed**. Configuration is
> strict, so a file or environment variable still setting either one fails at
> boot with an unknown-key error — delete the key; the probes are always on.

## `[signing]`

VERSION signing. On by default in `digest` mode, with read-time verification of
the server's own signatures **strict** by default.

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `true` | Server-side signing of committed versions. |
| `mode` | enum{digest,pgp} | `digest` | SHA-256 integrity digest, or an OpenPGP (RFC 4880) detached signature. |
| `key_path` | path | unset | Armored secret key; **required for `pgp`**. |
| `key_passphrase` / `key_passphrase_file` | secret / path | unset | Key passphrase. |
| `verify_on_read` | enum{off,warn,strict} | `strict` when signing is enabled | Read-time recompute-and-compare policy for the server's own signatures. |

> [!NOTE]
> **`verify_on_read` defaults to `strict` when signing is enabled.** On every
> read the server recomputes the signature of a version it signed and, on a
> mismatch, returns a `500` integrity fault rather than silently serving a
> provably corrupt record. Set it explicitly to `warn` (log + emit
> `version_signature_invalid_total`, still serve) or `off` (never check) to opt
> out. **Client-supplied signatures** — an author's own signature, or one
> carried by an imported version — are always stored verbatim and **never
> re-verified** (the author may have signed a different agreed serialization),
> regardless of this setting.

> [!WARNING]
> `pgp` mode **fails closed at boot** if the key is missing or unusable — the
> server will not start. Verify the key and passphrase before switching modes.

## `[query]`

AQL execution knobs.

| Key | Type | Default | Description |
|---|---|---|---|
| `plan_cache_capacity` | int | `256` | Max distinct cached query plans; `0` disables the cache. Cache activity is reported by the `aql_plan_cache_events_total` metric. |
| `timeout_ms` | int | `0` | Per-query DB execution budget; `0` disables (the global request timeout remains). Overrun returns `408`. |

## `[events]`

Contribution-outbox eventing → AMQP, plus its admin API. Off by default;
envelopes are PHI-free by design.

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | Spawn the outbox publisher (with `fhir.outbound.enabled`, gates the per-commit outbox INSERT). |
| `url` | secret URL | `amqp://guest:guest@localhost:5672/%2f` | AMQP broker URL; credentials redacted from every rendering. |
| `exchange` | string | `ehrbase.events` | Topic exchange (PHI-free envelope stream). |
| `tls` | bool | `false` | Upgrade `amqp://` to `amqps://`. |
| `batch_size` | int | `128` | Rows drained per poll. |
| `poll_interval_ms` | int | `1000` | Idle poll interval. |
| `retention_days` | int | `7` | Published-row retention window. |
| `prune_interval_secs` | int | `3600` | Retention-prune cadence. |
| `publish_max_retries` | int | `3` | Per-row publish retries before backing off. |
| `admin_api` | bool | `false` | Mount the `/admin/event_subscription` CRUD routes. |

## `[fhir]`

The FHIR connector — an inbound façade and an independent outbound emitter.

`[fhir]`: `api_enabled` (bool, `false`) — mount `/fhir/r4/*` +
`/admin/fhir_mapping`.
`[fhir.outbound]`: `enabled` (bool, `false`), `url` (secret URL, same AMQP
default), `exchange` (string, `ehrbase.fhir` — deliberately distinct from the
events exchange for PHI isolation), `tls` (bool, `false`), `batch_size` (int,
`128`), `poll_interval_ms` (int, `1000`), `publish_max_retries` (int, `3`).

> [!WARNING]
> The outbound stream carries **PHI** — the mapped FHIR resource. It is a
> deliberately separate switch and exchange from the PHI-free change-event
> stream so broker access control can isolate it. Enable it only against a TLS,
> access-controlled broker.

## `[terminology]`

Terminology extension API and external FHIR-terminology servers.

`[terminology]`: `api_enabled` (bool, `false`) — mount the terminology
extension API.
`[terminology.external]`: `enabled` (bool, `false`), `fail_on_error` (bool,
`false` — on TS/connectivity error, reject vs accept).
`[terminology.external.providers.<name>]` (conventionally `default`): `type`
(enum{fhir}, `fhir`), `url` (string, required), `operation`
(enum{validate_code,expand}, `validate_code`), `connect_timeout_ms` (int,
`2000`), `request_timeout_ms` (int, `10000`), `oauth2_client` (string, unset —
must name an entry under `[terminology.external.oauth2_clients]`),
`client_cert_path` / `client_key_path` (paths, unset — the mutual-TLS client
identity, see below), `ca_bundle_path` (path, unset — the trust anchors for
this server, see below),
`cache_ttl_secs` (int, `300` — TTL of the per-provider response cache; a
repeated validate/expand/subsumes/lookup within the window is served locally
instead of one HTTPS round trip per validated code; `0` disables),
`cache_capacity` (int, `10000` — maximum cached responses per provider).

### Several terminology servers at once

**Every** entry under `[terminology.external.providers]` is materialised at
startup, so one instance can serve SNOMED CT from one server and LOINC or ICD
from others. `[terminology.external.routes]` maps a terminology to the provider
that answers for it — the key is a terminology id (`SNOMED-CT`) or a system URI
(`http://snomed.info/sct`), matched case-insensitively as a whole string, and
the value names a provider. A terminology with no route goes to the provider
named `default`, or to the sole configured provider when there is exactly one.
A route naming a provider that does not exist is a startup error.

```toml
[terminology.external]
enabled = true
fail_on_error = false

[terminology.external.providers.default]
type = "fhir"
url = "https://r4.ontoserver.csiro.au/fhir"

[terminology.external.providers.snomed]
type = "fhir"
url = "https://snowstorm.example.org/fhir"
oauth2_client = "ts-client"

[terminology.external.routes]
"SNOMED-CT" = "snomed"
"http://snomed.info/sct" = "snomed"
"http://loinc.org" = "default"
```

Routing applies everywhere terminology is consulted: the
`/terminology/*` extension API, AQL `TERMINOLOGY(…)` resolution, and the
composition-commit binding checks below.

### Authenticating to a terminology server

`[terminology.external.oauth2_clients.<name>]` configures an OAuth2
client-credentials client; a provider references it by name with
`oauth2_client`. The access token is cached and re-requested shortly before it
expires, so a validation burst costs one token request per token lifetime.

Keys: `token_url` (string, required), `client_id` (string, required),
`client_secret` (secret — or `client_secret_file` pointing at a file holding
it; exactly one of the two), `scopes` (list of strings, empty),
`refresh_leeway_secs` (int, `30` — how long before expiry the token is
renewed), `auth_method`
(enum{client_secret_basic,client_secret_post}, `client_secret_basic`).

```toml
[terminology.external.oauth2_clients.ts-client]
token_url = "https://idp.example.org/realms/ts/protocol/openid-connect/token"
client_id = "ehrbase-cdr"
client_secret_file = "/run/secrets/ts-client"
scopes = ["system/*.read"]
```

### Mutual TLS to a terminology server

A terminology server that authenticates its clients with certificates instead
of (or in addition to) a bearer token is configured **per provider**, because a
client certificate is issued by that server's PKI: a deployment enrolled with a
national SNOMED CT service, a commercial value-set server and an in-house HAPI
server holds three different certificates. Repeat the same paths in each
provider table if one identity really does serve them all.

Keys on `[terminology.external.providers.<name>]`:

| Key | Meaning |
|---|---|
| `client_cert_path` | PEM file with the client certificate (optionally a chain) presented to this server. |
| `client_key_path` | PEM file with that certificate's private key. |
| `ca_bundle_path` | PEM bundle of the trust anchors this server's certificate is verified against. |

```toml
[terminology.external.providers.snomed]
type = "fhir"
url = "https://snowstorm.example.org/fhir"
client_cert_path = "/run/secrets/ts-snomed-client.crt.pem"
client_key_path = "/run/secrets/ts-snomed-client.key.pem"
ca_bundle_path = "/run/secrets/ts-snomed-ca.pem"
```

`client_cert_path` and `client_key_path` are set together — one without the
other is a startup error, never a connection that silently presents no
certificate. Unreadable files, a certificate file with no certificate in it and
a key file with no key in it are startup errors too, so a broken identity never
waits until the first validated code to surface.

`ca_bundle_path` **replaces** the default trust anchors for that provider, so a
terminology server issued by a private PKI is pinned to that PKI instead of also
accepting the whole public web PKI. Leave it unset to use the platform's default
trust store.

> [!IMPORTANT]
> There is no option to disable certificate verification. Server-certificate and
> hostname verification are always on for every provider; `ca_bundle_path`
> changes *which* anchors are trusted, never *whether* the server is verified.

The client identity applies to the connection to the terminology server itself.
An OAuth2 token endpoint (`oauth2_client`) is a different host in a different
trust domain and keeps the default TLS stack.

Kubernetes deployments mount the PEM files with the chart's `config.files` map
(see the Helm chart values), which materialises them under `/etc/ehrbase/`.

### Archetype value-set bindings at commit

With `[terminology.external]` enabled, committing a COMPOSITION also resolves
the archetype **constraint bindings** its template declares: where a template
binds an `ac` code to an external terminology query, the coded value in the
composition must be a member of the value set that query returns. The query is
sent to the server the binding's terminology routes to.

- The code is in the value set → the commit proceeds.
- The code is **not** in the value set → `422` naming the path, the code, and
  the bound query. This is a real constraint violation, so `fail_on_error` does
  not change it.
- The value set could **not be resolved** (server down, error response, no
  provider routes to that terminology) → `fail_on_error` decides:
  `false` (default) accepts the commit and logs a warning; `true` rejects it
  with `422`.

With `[terminology.external] enabled = false` (the default) no binding is
resolved and no request is made, so commit behaviour is exactly as before.

> [!NOTE]
> The composition's `terminology_id` is sent verbatim as the FHIR `system`
> parameter. If your archetypes use `SNOMED-CT` where your terminology server
> expects `http://snomed.info/sct`, configure the server to accept the id your
> archetypes carry — the CDR does not rewrite it.

## `[multimedia]`

DV_MULTIMEDIA externalization → S3-compatible object store. Off by default
(blobs stay inline, byte-identical).

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | Externalize large multimedia data. |
| `threshold_bytes` | int | `262144` (256 KiB) | Decoded size strictly above which data is offloaded. |
| `endpoint` | string | unset ⇒ AWS default | S3-compatible endpoint. |
| `bucket` | string | `openehr-multimedia` | Target bucket. |
| `region` | string | `us-east-1` | AWS region (required even for non-AWS endpoints). |
| `access_key_id` | string | unset | S3 access key id (unset + no secret = anonymous). |
| `secret_access_key` / `secret_access_key_file` | secret / path | unset | S3 secret access key. |
| `allow_http` | bool | `false` | Allow plain-HTTP endpoints — dev only; prod S3 is HTTPS. |

## `[audit]`

The IHE ATNA audit trail (see the [Audit trail chapter](../audit.md)). **On
by default** with only the local store active; forwarding is opt-in per
sink. (Replaces the former `[atna]` section — old keys fail at boot with
did-you-mean guidance.)

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `true` | Master audit switch. |
| `enterprise_site_id` | string | unset | `AuditEnterpriseSiteID`. |
| `source_id` | string | `ehrbase` | Audit source id. |
| `value_if_missing` | string | `UNKNOWN` | Fill value for empty mandatory fields. |
| `suppress_login_events` | bool | `true` | Skip successful-login records (rejections are always recorded). |
| `fail_mode` | enum{open,closed} | `open` | On undeliverable audit: succeed and meter (`open`) or reject auditable operations with 503 (`closed` — includes an unhealthy local store). |
| `resolve_subject` | bool | `true` | Enrich the patient participant via a background subject lookup. |
| `queue_capacity` | int | `8192` | Bounded audit queue capacity (sized for write-path bursts; the drain persists in multi-row batches). |
| `server_host` | string | unset | This node's advertised address (`NetworkAccessPointID`). |

### `[audit.store]` — the local Audit Record Repository

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `true` | Persist every record in the `audit` schema (served via the ITI-81 `GET /fhir/r4/AuditEvent` search). |
| `retention_days` | int | `0` | Days to keep records; `0` = keep forever. Applied hourly. |

### `[audit.syslog]` — the classic DICOM/syslog feed (ITI-20)

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | Ship DICOM PS3.15 records to an external ARR over syslog. |
| `host` | string | `localhost` | ARR host. |
| `port` | int | `514` | ARR port (514 UDP / 6514 TLS typical). |
| `transport` | enum{udp,tls} | `udp` | Syslog transport. Use `tls` for PHI-adjacent audit. |
| `tls_ca_file` / `tls_identity_cert_file` / `tls_identity_key_file` | path | unset | PEM CA / client cert / client key for the TLS transport. |

### `[audit.fhir_feed]` — the RESTful-ATNA feed (ITI-20 ATX:FHIR Feed)

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | `POST` each FHIR `AuditEvent` to an external FHIR ARR. Outbox-driven (loss-free) when the local store is on. |
| `url` | url | `http://localhost:8080/fhir` | The ARR's FHIR base; records go to `{url}/AuditEvent`. URL credentials are redacted from every rendering. |
| `batch_size` | int | `64` | Outbox rows shipped per poll. |
| `poll_interval_ms` | int | `2000` | Outbox poll interval when idle. |
| `max_retries` | int | `3` | Per-record POST retries before the record is left pending (store on) or dropped + metered (store off). |

## `[subject_proxy]`

FHIR frames. Empty by default — no external FHIR system is reachable until one
is named here (fail-closed). Systems are keyed by the name subject-proxy frames
use as their `system_id`. See [Subject Proxy](../beyond-core/subject-proxy.md).

`[subject_proxy.systems.<name>]`: `base_url` (string, required per system),
`connect_timeout_ms` (int, `2000`), `request_timeout_ms` (int, `10000`).

```toml
[subject_proxy.systems.pas]
base_url = "https://pas.example.com/fhir"
```

The env form for a named system is
`EHRBASE__SUBJECT_PROXY__SYSTEMS__PAS__BASE_URL`.

## Process / CLI

| Variable | Type | Default | Description |
|---|---|---|---|
| `EHRBASE_HEALTHCHECK_URL` | URL | `http://127.0.0.1:8080/ehrbase/rest/status` | Target URL for the binary's `healthcheck` subcommand (container `HEALTHCHECK` and Kubernetes exec probes). Not part of `ehrbase.toml`. |

## The `config` subcommands

```
ehrbase config default             # print the annotated default ehrbase.toml
ehrbase config check [--config P]  # validate (file + env + --set), print the
                                   #   effective config (secrets redacted) with
                                   #   a provenance column; exit 0 on success, 1 on error
```

`ehrbase config check` runs the exact same three validation passes as boot but
touches no database — use it in CI and before a rollout.

## Zero-config boot and the production checklist

With no file and no environment, the server boots as: listener `0.0.0.0:8080`
at the ITS-REST base path with Swagger UI; DB at the compose-dev DSN; auth
**enabled with no mechanism ⇒ every API request 401s** (fail-closed; boot logs
a prominent warning naming the two ways out — add `[[auth.basic.users]]` /
`[auth.oidc]`, or set `auth.enabled = false` for dev); RBAC on; signing on
(digest); log `auto`/`info`; **everything else off**.

For production, set at least:

- **`db.url`** — the real DSN, via `EHRBASE__DB__URL` (from a secret) or a
  `*_file`-mounted value, never inline in a world-readable file.
- **an auth mechanism** — a Basic user store and/or `[auth.oidc]`.
- **`log.format = "json"`** for cluster log collectors.
- **`server.cors_permissive`** stays `false`; **`server.swagger_ui`** per posture.
- **`server.system_id`** — this deployment's own openEHR system identifier
  (default `ehrbase-rs.local`). Choose it before the first EHR is created: it
  is stored with every EHR, audit entry, and version identifier.
- **`management.*`** per posture (a dedicated `port` is recommended so
  `/management` is never reachable on the clinical listener).
- **TLS everywhere a transport supports it** — `audit.syslog.transport = "tls"`,
  `events.tls`, `fhir.outbound.tls`, HTTPS S3.
- **real secrets via env or `*_file`**, never inline.

## What belongs in a mounted file (vs env)

Env cannot carry an array of tables, so the **Basic-auth user store**
(`[[auth.basic.users]]`) is file-only. Genuinely file-shaped material — the
**PGP signing key**, **Cedar/ABAC policies**, **ATNA TLS PEMs**, a **JWKS
blob** — is referenced by an in-TOML `*_path` / `*_file` key pointing at a
mounted path (e.g. the Helm chart's `config.files`). Everything else is a
plain key you can set in the file or override with an `EHRBASE_*` env var.

See `docker/ehrbase.dev.toml` in the repository for a worked dev example
(server section, CORS, admin, and the Basic-auth users).

## Migrating from 3.x environment variables

The pre-redesign layout used ~14 independent loaders, several env-name
grammars, and nine `EHRBASE_*_CONFIG` file pointers. Every old server variable
now **fails at boot** with the exact uniform replacement suggested — there is
no legacy alias layer (greenfield: nothing was deployed to migrate). The table
below maps every old spelling to its replacement.

| Old variable | New key (env form) | Fate |
|---|---|---|
| `EHRBASE_DB_URL` | `db.url` (`EHRBASE__DB__URL`) | **boot error** — use the new spelling |
| `DATABASE_URL` | `db.url` | kept permanently |
| `EHRBASE_DB_MAX_CONNECTIONS` / `_MIN_CONNECTIONS` / `_ACQUIRE_TIMEOUT_SECS` | `db.*` (`EHRBASE__DB__*`) | **boot error** — use the new spelling |
| `EHRBASE_LOG_FORMAT` / `EHRBASE_LOG_FILTER` | `log.*` (`EHRBASE__LOG__*`) | **boot error** — use the new spelling |
| `RUST_LOG` | `log.filter` | kept permanently |
| `EHRBASE_OTEL_*` | `telemetry.*` | **boot error** — use the new spelling |
| `EHRBASE_REST_CONFIG` | `--config` / `EHRBASE_CONFIG` + `ehrbase.toml` | **removed** — merge the file into `ehrbase.toml` |
| `EHRBASE_REST_BIND` / `_BASE_PATH` / `_SWAGGER_UI` / `_CORS_PERMISSIVE` | `server.*` (`EHRBASE__SERVER__*`) | **boot error** — use the new spelling |
| `EHRBASE_REST_MAX_IN_FLIGHT` | `server.max_in_flight` (`EHRBASE__SERVER__MAX_IN_FLIGHT`) | **boot error** — use the new spelling |
| `EHRBASE_REST_SYSTEM__*` | `server.identity.*` | **boot error** — use the new spelling |
| `EHRBASE_REST_AUTH__ENABLED` / `_VERIFIED_CACHE_TTL_SECONDS` | `auth.*` (`EHRBASE__AUTH__*`) | **boot error** — use the new spelling |
| `EHRBASE_REST_AUTH__OIDC__*` | `auth.oidc.*` (`EHRBASE__AUTH__OIDC__*`) | **boot error** — use the new spelling |
| `EHRBASE_REST_AUTH__BASIC__USERS` | `[[auth.basic.users]]` (file-only) | **removed** — set in the file |
| `EHRBASE_REST_AUTH__ADMIN_SCOPE` | — (subsumed by `authz.rbac.admin_role`) | **removed** |
| `EHRBASE_REST_ADMIN__ENABLED` | `admin.enabled` | **boot error** — use the new spelling |
| `EHRBASE_REST_TENANCY__*` | `tenancy.*` | **boot error** — use the new spelling |
| `EHRBASE_REST_TERMINOLOGY__ENABLED` | `terminology.api_enabled` | **boot error** — use the new spelling |
| `EHRBASE_REST_EVENT_SUBSCRIPTION__ENABLED` | `events.admin_api` | **boot error** — use the new spelling |
| `EHRBASE_REST_FHIR__ENABLED` | `fhir.api_enabled` | **boot error** — use the new spelling |
| `EHRBASE_REST_SMART__*` | `smart.*` (`EHRBASE__SMART__*`) | **boot error** — use the new spelling |
| `EHRBASE_MANAGEMENT_*` | `management.*` (`EHRBASE__MANAGEMENT__*`) | **boot error** — use the new spelling |
| `EHRBASE_MANAGEMENT_ENDPOINTS_<EP>` | `management.endpoints.<ep>` (`EHRBASE__MANAGEMENT__ENDPOINTS__<EP>`) | **boot error** — use the new spelling |
| `EHRBASE_AUTHZ_RBAC__*` / `EHRBASE_AUTHZ_ABAC__*` | `authz.rbac.*` / `authz.abac.*` (`EHRBASE__AUTHZ__…`) | **boot error** — use the new spelling |
| `EHRBASE_ATNA_<KEY>` | `audit.<key>` (`EHRBASE__AUDIT__<KEY>`) | **boot error** — use the new spelling |
| `EHRBASE_SIGNING_<KEY>` | `signing.<key>` (`EHRBASE__SIGNING__<KEY>`) | **boot error** — use the new spelling |
| `EHRBASE_EVENTS_<KEY>` | `events.<key>` (`EHRBASE__EVENTS__<KEY>`) | **boot error** — use the new spelling |
| `EHRBASE_FHIR_OUTBOUND_<KEY>` | `fhir.outbound.<key>` (`EHRBASE__FHIR__OUTBOUND__<KEY>`) | **boot error** — use the new spelling |
| `EHRBASE_MULTIMEDIA_<KEY>` | `multimedia.<key>` (`EHRBASE__MULTIMEDIA__<KEY>`) | **boot error** — use the new spelling |
| `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_*` | `terminology.external.*` | **boot error** — use the new spelling |
| `EHRBASE__SUBJECT_PROXY__SYSTEMS__*` | `subject_proxy.systems.*` — same spelling, now actually binds | binds for the first time |
| `EHRBASE__QUERY__PLAN_CACHE_CAPACITY` / `_TIMEOUT_MS` | `query.*` — same spelling, now strict-parsed | behaviour change (bad values now error) |
| the nine `EHRBASE_*_CONFIG` file pointers | — | **removed** — merge each file's contents into `ehrbase.toml` under its `[section]` |

The `PostgreSQL init` container variables `EHRBASE_DB_USER` / `_PASSWORD` /
`_NAME` were renamed to `PG_INIT_USER` / `_PASSWORD` / `_DB` — they configure
the database container, not the server, and no longer collide with the
server's reserved `EHRBASE_` namespace.
