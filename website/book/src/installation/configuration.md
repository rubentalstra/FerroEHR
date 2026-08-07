# Configuration reference

FerroEHR is configured by **one file — `ferroehr.toml`** — whose sections
cover the entire server, with `FERROEHR_*` environment variables (and repeatable
`--set` flags) as per-key overrides on top. This chapter is the complete
reference: the quickstart, how configuration loads and how env names map onto
the file, a subsection per configuration area, the CLI tools that validate and
scaffold a config, the production checklist, and — for upgraders — the old→new
variable map.

<!-- toc -->

## Quickstart

Generate an annotated template, edit it, and run:

```bash
# Write a fully-commented ferroehr.toml with every key at its default.
ferroehr config default > ferroehr.toml

# Edit it — at minimum set db.url and an auth mechanism (see the checklist below).
$EDITOR ferroehr.toml

# Validate without touching the database, then run.
ferroehr config check --config ferroehr.toml
ferroehr --config ferroehr.toml
```

A server started with **no file and no environment** still boots (see
[Zero-config boot](#zero-config-boot-and-the-production-checklist)) — the file
is optional, and every key has a default.

## How configuration loads

Configuration is assembled once at boot from four layers, lowest precedence to
highest:

1. **Built-in defaults** — the values in the tables below.
2. **The config file** — `ferroehr.toml` (see [file discovery](#file-discovery)).
3. **`FERROEHR_*` environment variables** — override individual keys.
4. **`--set key=value` CLI flags** (repeatable) — win over everything.

Two permanent conventional aliases sit *below* their `FERROEHR_` forms within layer 3:
`DATABASE_URL` → `db.url` and `RUST_LOG` → `log.filter`. Nothing else has a
non-`FERROEHR_` name.

### The environment-variable mapping

Every key has one mechanical env spelling: **`FERROEHR` + the TOML path,
upper-cased, with a double underscore (`__`) between every segment — including
after the `FERROEHR` prefix.** A single underscore only ever appears *inside* a
key word.

| TOML | Environment variable |
|---|---|
| `[db] max_connections = 20` | `FERROEHR__DB__MAX_CONNECTIONS=20` |
| `[auth.oidc] issuer = "…"` | `FERROEHR__AUTH__OIDC__ISSUER=…` |
| `[management.endpoints] env = "off"` | `FERROEHR__MANAGEMENT__ENDPOINTS__ENV=off` |
| `[terminology.external.providers.default] url = "…"` | `FERROEHR__TERMINOLOGY__EXTERNAL__PROVIDERS__DEFAULT__URL=…` |

Scalars are typed automatically (bool / int / float, else string).
**List-typed keys take comma-separated values**
(`FERROEHR__AUTH__OIDC__AUDIENCES=ferroehr,other`). Arrays of tables — the
Basic-auth user store — are **file-only**.

> [!NOTE]
> Enum values are lowercase / `snake_case` tokens, exactly as the tables show.
> Secret-typed keys are redacted everywhere the config is rendered (the
> `/management/env` snapshot, `ferroehr config check`, logs), and each has a
> `*_file` sibling that reads the value from a file (for Kubernetes/Docker
> secret mounts). Setting a secret and its `*_file` sibling at once is a boot
> error.

### File discovery

The first of these that exists is loaded (later layers still override its
values):

1. `--config <path>` (fatal if missing/unreadable),
2. `FERROEHR_CONFIG=<path>` (fatal if missing/unreadable),
3. `./ferroehr.toml` (current directory),
4. `/etc/ferroehr/ferroehr.toml`.

An explicitly pointed-at file (1–2) is fatal if absent; the search-order files
(3–4) are simply skipped when absent (but fatal if present and unparseable).

### Strict validation

Configuration is validated at boot (and by `ferroehr config check`), and the
server refuses to start on any error:

- **Unknown keys are rejected** — in the file (with the offending `file:line`)
  and in the `FERROEHR_` environment namespace — with a did-you-mean suggestion.
  A misspelled security key is a boot error, never silently ignored.
- **Type errors are boot errors**, naming the key, the expected type, and where
  the bad value came from.
- **Semantic errors are aggregated** — one pass reports every problem at once,
  so a broken config is fixed in a single iteration.

## `spec_profile`

```toml
# The openEHR specification generation set the server runs.
spec_profile = "development"   # or "stable"
```

openEHR publishes released specification versions and keeps developing the
next ones. FerroEHR generates **both** — each generation is a complete peer,
with its own type model, canonical JSON and XML codecs, Reference Model
attribute model, invariant cores and validation behaviour — and this key
decides which set the running server serves.

| value | RM | BASE | LANG | Choose it when |
|---|---|---|---|---|
| `development` *(default)* | 1.2.0 | 1.3.0 | 1.1.0 | You want the generations this build is developed against. This is today's behaviour and the default for every existing deployment. |
| `stable` | 1.1.0 | 1.2.0 | 1.0.0 | Your governance requires running on RELEASED openEHR specifications only. |

Environment form: `FERROEHR__SPEC_PROFILE=stable`. The key is a top-level
scalar, not a section — there is no `[spec_profile]` table.

### Why it is one key and not three

The components' generations are modelled against each other, not
independently: RM 1.1.0's own machine-readable model declares that it
includes BASE 1.2.0. Letting you pick RM 1.1.0 with BASE 1.3.0 would offer a
combination openEHR never published, so the profile is a single coupled
choice and incoherent sets are unrepresentable rather than merely
discouraged.

### Seeing which profile is active

The profile and the exact generation versions it selects appear in three
places, so it is never a guess:

- the **boot banner**, on every start;
- **`GET /management/info`**, alongside the build provenance;
- the **openEHR system identity** the server reports for conformance.

### What changes on the wire

Under `stable`, a request that addresses specification surface the released
generations do not define is **refused with an error naming the active
profile** — never answered as though the surface existed. That boundary is
exact in both directions, which is the part most implementations get wrong:
released surface the development line later dropped **stays accepted** under
`stable`. Concretely, a demographic party carrying the RM 1.1.0
`PARTY.reverse_relationships` attribute is validated and accepted under
`stable` (the attribute is derived data the server recomputes, so the copy
you send is not stored), while `development` refuses it as undeclared —
because RM 1.2.0 removed it.

The generation delta is machine-pinned in the build, so a future openEHR
re-vendoring cannot silently widen or narrow what a profile accepts.

### Changing the profile on an existing deployment

Treat the profile as a deployment commitment. Both directions are defined,
but they are not symmetric:

| Direction | Supported? | Why |
|---|---|---|
| `stable` → `development` | **Always safe** | openEHR minor releases are additive by the Foundation's own release strategy, so every object stored under the released generations is valid under the development ones. |
| `development` → `stable` | **Only for data that never used a development-only construct** | Stored objects that did are refused **loudly at read**, with an error naming the profile conflict. They are never silently down-converted, and never hidden from a query. |

If you need to stay on released specifications, choose `stable` on day one
rather than migrating into it later. There is no in-place down-conversion
tool, by design: silently rewriting stored clinical content to fit an older
generation would be data loss disguised as a setting.

> [!NOTE]
> No openEHR specification governs runtime version selection — this key is
> FerroEHR's own design. What the specifications do govern is the
> compatibility direction it relies on: minor releases within a major line
> are additive supersets.

## `[server]`

The HTTP listener and REST surface.

```toml
[server]
bind = "0.0.0.0:8080"
base_path = "/ferroehr/rest/openehr/v1"
max_in_flight = 256
swagger_ui = true
cors_permissive = false
system_id = "ferroehr.local"
```

| Key | Type | Default | Description |
|---|---|---|---|
| `bind` | string | `0.0.0.0:8080` | Socket address the API listener binds. |
| `base_path` | string | `/ferroehr/rest/openehr/v1` | ITS-REST base path all API routes hang off. The status, health-adjacent and documentation routes hang off its parent (`/ferroehr/rest` by default), and the served OpenAPI document describes whatever paths this setting produces — never the defaults. |
| `max_in_flight` | int | `256` | Concurrent-request admission cap (not per second). Requests beyond it are shed immediately with `503` + `Retry-After` — never queued — so offered load beyond capacity cannot exhaust memory. Status/health/discovery routes are never limited. `0` disables shedding. |
| `swagger_ui` | bool | `true` | Serve Swagger UI + the OpenAPI JSON at the REST root. Consider `false` in production. |
| `cors_permissive` | bool | `false` | Permissive (development) CORS. Production configures explicit origins. |
| `system_id` | string | `ferroehr.local` | **This deployment's own openEHR system identifier** — see below. Set a stable, deployment-unique name in production (`FERROEHR__SERVER__SYSTEM_ID`). |

### `[server.limits]`: request-body sizes

```toml
[server.limits]
body_bytes = 16777216        # 16 MiB
bulk_body_bytes = 67108864   # 64 MiB
```

| Key | Type | Default | Description |
|---|---|---|---|
| `body_bytes` | int | `16777216` | The largest request body the ordinary clinical surface accepts, in bytes. |
| `bulk_body_bytes` | int | `67108864` | The largest body the bulk routes accept: operational-template upload, `/message/import`, `/message/tdd`. |

A request over its tier's limit is refused `413 Payload Too Large` with the
standard openEHR error body. The status is not in the ITS-REST status table; it
is admitted there as an additional, non-conflicting code, and is what RFC 9110
§15.5.14 defines for this refusal.

The defaults are sized against measured payloads rather than chosen as round
numbers: the clinical tier was set to clear the largest operational template in
the vendored CKM corpus several times over, with the measurement recorded beside
the constant it justifies (`BodyLimits` in `app/ferroehr/src/config/server.rs`). **Raise `body_bytes` if your
compositions embed large `DV_MULTIMEDIA` data** — a base64 radiology image can
exceed either tier on its own, and that is a deliberate operator decision rather
than a default.

### `[server.rate_limit]`: per-caller request rates

```toml
[server.rate_limit]
enabled = true
principal_per_second = 1024
principal_burst = 2048
address_per_second = 2048
address_burst = 4096
```

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `true` | Whether rate limiting is active. Off allocates no limiter state and costs no per-request check. |
| `principal_per_second` | int | `1024` | Sustained requests per second per authenticated subject, on the clinical API. |
| `principal_burst` | int | `2048` | How far one principal may burst before refusal. |
| `address_per_second` | int | `2048` | Sustained requests per second per client address, across the whole tree. |
| `address_burst` | int | `4096` | How far one address may burst before refusal. |

**This is not `max_in_flight`, and an operator should be able to tell them apart
from the status alone.** `max_in_flight` protects *capacity*: too many requests
in flight at once, from anyone, and the excess is shed `503` + `Retry-After`.
Rate limiting protects *fairness*: one caller asking too often over time, refused
`429` + `Retry-After`. A `503` means the server is full; a `429` means you are
asking too fast.

Two tiers, because they defend different things. The **address** tier sits
outside authentication, so a flood of unauthenticated requests is refused before
it can make the server verify a signature per request — a limiter must not itself
be the expensive path. The **principal** tier sits inside authentication, keyed
on the authenticated subject, which is the only fair key for a clinical API: a
hospital behind one NAT is a single address, so an address-keyed clinical limit
would throttle an entire site because one client was busy.

The defaults are derived from this implementation's own measured ceiling. The
committed step-load record puts maximum sustainable whole-server throughput at
512 requests/second on the reference SUT, so the principal tier is set at twice
that and the address tier at four times: neither can refuse a caller until it is
asking for more than the whole server could serve, and below that line capacity
is `max_in_flight`'s job. **A deployment that earns a higher volumetric class
should raise both in proportion** — a limit derived from a laptop-class
measurement is too low for a server-class deployment.

Refusals carry the limiter's own `Retry-After` and `x-ratelimit-*` headers
alongside the openEHR error body. `429` is the status RFC 6585 §4 defines for
this refusal, admitted by ITS-REST as an additional, non-conflicting code.

The always-on health family is covered by the address tier only, deliberately: an
orchestrator probe must never be refused because a principal-keyed bucket was
exhausted, and probe rates are nowhere near the address ceiling.

**Benchmarking this server?** Turn the limiter off first, or you will measure it
instead of the server. Our own measurement lanes compose an overlay that does
exactly that, and both instruments refuse to write a record if the server
answered any `429`.

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

- **The value must be a legal openEHR UID** (a UUID, an ISO OID, or an
  internet-id / reverse-domain name per the openEHR BASE identification
  grammar). The server validates this at startup and refuses to boot on an
  illegal value — it becomes the `creating_system_id` segment of every
  version identifier the server mints, and an illegal value would produce
  ids the server's own reader rejects. A DNS-style name like
  `cdr.hospital.example` is valid.
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
`OPTIONS /ferroehr/rest/openehr/v1` — the System API's one location). Defaults
are measured, not asserted — the manifest never out-claims the last
conformance verdict. This is the deployment's *display* identity only; the
identifier stamped into authored data is
[`server.system_id`](#system_id-the-data-authoring-identity).

| Key | Type | Default | Description |
|---|---|---|---|
| `solution` | string | `FerroEHR` | Product name. |
| `solution_version` | string | build version | Product version. |
| `vendor` | string | `FerroEHR project` | Providing organisation. |
| `restapi_specs_version` | string | tested-contract identity | openEHR REST API edition the build is tested against. |
| `conformance_profile` | string | last CNF verdict | Advertised conformance profile. |

## `[db]`

PostgreSQL connection.

```toml
[db]
url = "postgres://ferroehr:ferroehr@localhost:5432/ferroehr"
migrate = "apply"
max_connections = 20
min_connections = 2
acquire_timeout_secs = 30
statement_timeout_ms = 60000
```

| Key | Type | Default | Description |
|---|---|---|---|
| `url` | secret URL | `postgres://ferroehr:ferroehr@localhost:5432/ferroehr` | Connection DSN. The default suits a local from-source run against a localhost PostgreSQL (the compose stacks set `FERROEHR__DB__URL` explicitly); **production MUST set it**. Credentials are redacted from every rendering. `DATABASE_URL` is a recognized lower-priority alias. |
| `url_file` | path | unset | Read the DSN from a file instead of the key above, for a mounted secret. Preferred over the environment form in Kubernetes: an environment value is readable through `/proc/<pid>/environ` and inherited by every child process. At most one of the pair, where the built-in dev default does not count as "set". |
| `migrate` | enum{apply,verify} | `apply` | Whether the server applies its embedded migrations at boot. `apply` is what makes an empty configuration boot against an empty database. `verify` issues **no DDL at all**: it checks that the database already carries exactly this build's migrations and refuses to start otherwise, so the DSN can authenticate as a role with no DDL rights. See [Operations](../operations.md#applying-migrations). |
| `max_connections` | int | `20` | Pool ceiling. Write-heavy deployments benefit from 50+. |
| `min_connections` | int | `2` | Idle connections kept open (avoids cold-reopen churn). |
| `acquire_timeout_secs` | int | `30` | Seconds to wait for a free connection before failing. |
| `statement_timeout_ms` | int | `60000` | `statement_timeout` applied to every pooled connection; `0` leaves the server default. The backstop the HTTP request timeout cannot be — dropping a handler future does not cancel the statement PostgreSQL is running. Keep it above `query.timeout_ms` so the AQL engine's own typed refusal fires first. |

## `[log]`

```toml
[log]
format = "auto"
filter = "info,ferroehr=info"
```

| Key | Type | Default | Description |
|---|---|---|---|
| `format` | enum{auto,json,pretty} | `auto` | Stdout rendering; `auto` picks `json` when stdout is not a TTY (and suppresses the boot banner). |
| `filter` | string | `info,ferroehr=info` | Boot `EnvFilter` directives; also the `/management/loggers` reset target. `RUST_LOG` is a recognized lower-priority alias. |

## `[telemetry]`

OpenTelemetry export. Unset `otlp_endpoint` ⇒ the OTel layer is not installed
(zero overhead).

| Key | Type | Default | Description |
|---|---|---|---|
| `otlp_endpoint` | string | unset | OTLP/gRPC collector endpoint. |
| `service_name` | string | `ferroehr` | `service.name` resource attribute. |
| `environment` | string | `dev` | `deployment.environment` resource attribute. |
| `traces_sample_ratio` | float | `1.0` | Head-sampling ratio (`0.1` is a common prod start). |
| `metrics_push` | bool | `false` | Push the **OpenTelemetry SDK instruments** over OTLP alongside the Prometheus pull surface. **Partial by design today:** the families recorded through the `metrics`-crate recorder — `ferroehr_build_info`, the HTTP request histogram, active requests, the ATNA audit counters, process start time — reach `/management/prometheus` by scrape only and do **not** appear in an OTLP collector. The server warns at boot listing exactly which families are affected. Scrape `/management/prometheus` as well if you need them ([#2175](https://github.com/rubentalstra/FerroEHR/issues/2175)). |
| `flame_file` | path | unset ⇒ layer not installed | Span-timing flamegraph capture (`tracing-flame`): write folded stack samples of every span to this file; render offline with `inferno-flamegraph < file > flame.svg`. For diagnostic sessions, not a standing posture — the file grows with span traffic. |

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
issuer = "https://keycloak.example.com/realms/ferroehr"
audiences = ["ferroehr"]
algorithms = ["RS256"]
```

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `true` | Master switch. `false` = all requests pass unauthenticated (dev only). With `true` and **no mechanism configured the server refuses to start** — see below. |
| `verified_cache_ttl_seconds` | int | `60` | Verified Basic-credential cache TTL (`0` disables); bounds Argon2 cost per busy client and revocation lag alike. |

> [!IMPORTANT]
> **`auth.enabled = true` with no mechanism is a boot error.** Such a server
> could only refuse every request while advertising an authentication scheme it
> does not implement, which RFC 9110 §11.6.1 forbids (a `401` challenge must
> name a scheme applicable to the target resource). Configure
> `[[auth.basic.users]]`, configure `[auth.oidc]`, or set
> `auth.enabled = false` for a development server.

`[[auth.basic.users]]` — the Basic-auth user store (array of tables,
**file-only**):

| Key | Type | Default | Description |
|---|---|---|---|
| `username` | string | required | Principal name. A blank or missing one is a boot error. |
| `password_hash` | secret | required | Argon2**id** PHC hash (`$argon2id$v=19$…`), never a plaintext password. Boot-validated against the OWASP floor — see below. |
| `password_hash_file` | path | unset | Read the hash from a file instead, for a mounted secret. A hash is an offline cracking target, so prefer this wherever the configuration file itself is not treated as sensitive. The Argon2id floor is validated identically either way. Exactly one of the pair is required. |
| `roles` | list of string | `["USER"]` | Roles granted (upper-cased on authentication). |

> [!IMPORTANT]
> **Every `password_hash` must meet the OWASP Argon2id floor: `m>=19456`
> (19 MiB), `t>=2`, `p>=1`, algorithm `argon2id`.** Anything weaker — or a
> non-`argon2id` PHC string, or an unparsable one — is a boot error naming the
> user. This is checked at startup because the verifier takes its cost
> parameters *from the stored hash*, so a deliberately cheap hash would
> otherwise verify happily and silently weaken every password in the store.
> The floor is the
> [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
> §Argon2id minimum.

`[auth.oidc]` — bearer validation (absent table ⇒ bearer disabled):

| Key | Type | Default | Description |
|---|---|---|---|
| `issuer` | string | required when present | Expected `iss`; also the OIDC discovery base. Must be an `https` URL with no query and no fragment (RFC 8414 §2) — boot-validated. |
| `audiences` | list of string | **required, non-empty** | Accepted `aud`. An empty list is a boot error. |
| `algorithms` | list of string | `["RS256"]` | Accepted signature algorithms. Boot-bound to the key source: `HS*` requires `hmac_secret`, `RS*`/`ES*`/`PS*` require a JWKS (static or discovered). `none` is refused. |
| `require_at_jwt` | bool | `false` | Refuse a token that does not carry `typ: at+jwt`. A token that DOES carry it is held to RFC 9068 §2.2 either way (`iat`, `jti`, `client_id` become mandatory). |
| `clock_skew_leeway_seconds` | int | `60` | Leeway on the time-based claims (`exp`/`nbf`). Capped at `300`; above that is a boot error. |
| `allow_insecure_issuer` | bool | `false` | Accept a non-`https` `issuer`. **Development and test only.** |
| `hmac_secret` / `hmac_secret_file` | secret / path | unset | Symmetric HS256 secret (dev/test), minimum 32 bytes. At most one of the pair. |
| `jwks_json` / `jwks_json_file` | string / path | unset | Static JWKS document. At most one of the pair. |
| `connect_timeout_ms` | int | `3000` | TCP connect timeout for the discovery + JWKS fetches. |
| `request_timeout_ms` | int | `5000` | Whole-request timeout for the discovery + JWKS fetches (connect, TLS, body read). |
| `negative_cache_ttl_seconds` | int | `10` | How long a *failed* discovery/JWKS fetch is remembered (`0` disables). |

The `[auth.oidc]` boot rules, and what each one prevents:

- **`audiences` must name at least one audience.** RFC 7519 §4.1.3 obliges a
  recipient that does not identify itself with a value in a present `aud` claim
  to reject the JWT, and RFC 9068 §4 step 4 makes the check unconditional for an
  access token. A resource server that declares no audience cannot reject a
  token minted for a *different* resource server, and cannot tell an OpenID
  Connect ID token (whose `aud` is a client id) from an access token
  (RFC 8725 §3.9, §3.12). Set it to whatever your identity provider puts in
  `aud` for this CDR.
- **`issuer` must be an `https` URL with no query or fragment.** That is the
  RFC 8414 §2 definition of an issuer identifier, and §6.2 requires TLS for
  issuer metadata — over plain HTTP an attacker on the network can serve their
  own signing keys. A development issuer (a Keycloak on the compose network,
  say) is opted in explicitly with `allow_insecure_issuer = true`; the
  no-query/no-fragment rules still apply, since those are structural.
- **`clock_skew_leeway_seconds` is capped at 300.** RFC 7519 §4.1.4 allows
  "some small leeway, usually no more than a few minutes, to account for clock
  skew", and RFC 9068 §4 step 6 repeats the bound. A large leeway silently
  extends the life of *every* token past its `exp`, so the key is capped rather
  than free.
- **`hmac_secret` must be at least 32 bytes.** RFC 8725 §3.5: "Human-memorizable
  passwords MUST NOT be directly used as the key to a keyed-MAC algorithm such
  as `HS256`". A symmetric key is also shared with the authorization server —
  meaning this server could mint the very tokens it accepts — so the boot log
  warns that it is a development posture. Prefer discovery or `jwks_json`.

The signing-key source is exactly one of: the symmetric secret, the static
JWKS, or (when neither is set) the issuer's OIDC discovery document.
Configuring both `hmac_secret` and `jwks_json` (in either direct or `_file`
form) is a boot-time configuration error — never resolved by silent
precedence. A validated token must also carry a non-blank `sub` claim: the
authenticated subject is stamped into the audit trail, so a token without one
is refused with `401` rather than recorded under a placeholder identity.

The last three keys apply only when keys come from the issuer's OIDC discovery
document — that is, when neither `hmac_secret` nor `jwks_json` is set. The
timeouts stop an unresponsive identity provider from parking bearer requests
until the operating system's TCP timeout; the negative cache means an identity
provider outage costs one discovery attempt per `negative_cache_ttl_seconds`
rather than one per incoming request, so callers get fast `401`s instead of slow
ones. Keep the negative TTL short: it is also how long recovery takes to be
noticed after the provider comes back.

## `[authz]`

RBAC + ABAC.

`[authz.rbac]`:

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `true` | Coarse role gate (active when auth is enabled). |
| `admin_role` | string | `ADMIN` | Role required for admin-class operations. |
| `user_role` | string | `USER` | Baseline clinical role. |
| `readonly_role` | string | `READONLY` | Role marking a principal read-only: refused on every write operation (create/update/delete/upload), even alongside granting roles. Reads and AQL queries are still allowed. |
| `role_claims` | list of string | `["roles","groups","entitlements","realm_access.roles"]` | JWT claim paths mined for roles, in order. Dotted paths walk nested claims. **`scope` is not a role source** — see [Security](../security.md#rbac-role-based-coarse). |
| `management_access` | enum{admin_only,private,public} | `admin_only` | Access level for the management surface. |

`[authz.abac]`:

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | Master ABAC switch. |
| `engine` | enum{cedar,remote} | `cedar` | Embedded Cedar or a remote decision point. |
| `organization_claim` | string | `organization_id` | JWT claim carrying the caller's organization. |
| `patient_claim` | string | `patient_id` | JWT claim carrying the patient id. |
| `check_directory` | bool | `false` | Submit DIRECTORY (`FOLDER`) operations to the decision point. Engine-independent, so it works under Cedar as well as a remote PDP. |

`[authz.abac.cedar]`: `policy_dir` (path — required when `engine=cedar` and ABAC
on), `reload_secs` (int, unset — optional hot-reload interval).
`[authz.abac.remote]`: `server` (string — required when `engine=remote`, must
end `/`), `connect_timeout_ms` (int, `2000`), `request_timeout_ms` (int,
`5000`).
`[authz.abac.policy.<kind>]` (kind ∈ `ehr`, `ehr_status`, `composition`,
`contribution`, `query`, `directory`): `name` (string), `parameters` (list of
enum{organization,patient,template}).

> [!IMPORTANT]
> **With `engine = remote`, every resource kind the enforcement point consults
> needs a policy entry: `ehr`, `ehr_status`, `composition`, `contribution`,
> `query`** (plus `directory` when `check_directory = true`). A missing one is a
> boot error. At runtime a kind with no policy can only be **denied** — there is
> no policy to ask, and permitting would be a silent hole — so the
> misconfiguration is caught at startup rather than turning into blanket `403`s
> on live traffic. The Cedar engine reads its policies from
> `abac.cedar.policy_dir` and needs no entries here.

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
set); `allow_insecure_endpoints` (bool, `false`).

Everything in `[smart.endpoints]` is **published** at
`/.well-known/smart-configuration` for third-party applications to act on, so
`smart.enabled = true` boot-validates it rather than relaying whatever is
configured:

| Rule | Why |
|---|---|
| Deprecated grant types (`implicit`, password) rejected | master06 §Deprecated Flows |
| `public_base_url`, `authorization_endpoint`, `token_endpoint` required | an enabled Platform without them publishes an unusable document |
| Every advertised endpoint an absolute `https` URL | the document tells apps where to send an authorization request and exchange a code, so a plaintext endpoint exposes the code and the access token ([RFC 6749 §3.1.2.1](https://www.rfc-editor.org/rfc/rfc6749#section-3.1.2.1), [RFC 8414 §6.2](https://www.rfc-editor.org/rfc/rfc8414#section-6.2)). `allow_insecure_endpoints = true` opts out for development |
| `issuer` has no query and no fragment | [RFC 8414 §2](https://www.rfc-editor.org/rfc/rfc8414#section-2) — the same rule `auth.oidc.issuer` follows, because it is the same identity |
| `response_types_supported` non-empty | RFC 8414 §2 marks the field **REQUIRED** |
| `token_endpoint_auth_methods_supported` non-empty | an empty list advertises a server that authenticates no client |
| `code_challenge_methods_supported` includes `S256` | SMART App Launch requires PKCE ([RFC 7636](https://www.rfc-editor.org/rfc/rfc7636)); publishing a list without it tells every app the server cannot do PKCE, and `plain` alone is not sufficient (§7.2) |
| `smart.endpoints.issuer` equals `auth.oidc.issuer` | one says where apps **obtain** tokens, the other which tokens this server **accepts**. A mismatch means every app gets a valid token and every request is refused |
| `smart.enabled` requires `[auth.oidc]` | the CDR cannot validate the tokens it directs applications to obtain |

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
flamegraph = "off"

[management.profiling]
max_seconds = 30
max_frequency = 999
```

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | Mount the management router. |
| `base_path` | string | `/management` | Base path for the management endpoints. |
| `port` | int | unset ⇒ share the main listener | Serve management on its own listener/port. Must differ from the `server.bind` port. |
| `access_default` | enum{off,admin_only,private,public} | `admin_only` | Global default access level (a per-endpoint level wins). |

`[management.endpoints]` — `info`, `metrics`, `prometheus`, `env`, `loggers`,
`flamegraph`, each enum{off,admin_only,private,public}, default `off`.

`[management.profiling]` — limits for the on-demand CPU flamegraph behind
`endpoints.flamegraph` (see
[Operations → Profiling](../operations.md#profiling-the-on-demand-cpu-flamegraph)):

| Key | Type | Default | Description |
|---|---|---|---|
| `max_seconds` | int | `30` | Longest sample window one request may take. A request asking for more is refused with `400`, never clamped. |
| `max_frequency` | int | `999` | Highest sampling frequency (Hz) a request may ask for. Same refusal semantics. |

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
| `url_file` | path | unset | Read the broker URL from a file instead, for a mounted secret. At most one of the pair, where the built-in dev default does not count as "set". |
| `exchange` | string | `ferroehr.events` | Topic exchange (PHI-free envelope stream). |
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
default) or `url_file` (path, unset — read the broker URL from a mounted file
instead), `exchange` (string, `ferroehr.fhir` — deliberately distinct from the
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

Cached entries are the *decoded* responses, not raw JSON: a server answer that
is not a valid FHIR R4B `Parameters`/`ValueSet` resource — for example a
`$expand` result missing the required `ValueSet.status` or
`expansion.timestamp` — is rejected as an upstream fault (HTTP `500`, subject
to `fail_on_error`) rather than partially read.

> [!NOTE]
> External terminology servers need the `fhir` build feature (on in the
> published binary and container images). A binary built with
> `--no-default-features` refuses at startup if `terminology.external` is
> enabled with any provider configured; the in-process openEHR terminology
> bundle remains available.

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
client_id = "ferroehr-cdr"
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
(see the Helm chart values), which materialises them under `/etc/ferroehr/`.

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
sink. (`[atna]` keys are rejected at boot with did-you-mean guidance
pointing here.)

> [!NOTE]
> The local store and the ATX:FHIR Feed both carry a FHIR R4B `AuditEvent`
> document, so both need the `fhir` build feature (on in the published binary
> and container images). A binary built with `--no-default-features` refuses
> at startup if `audit.store.enabled` or `audit.fhir_feed.enabled` is set; the
> DICOM/syslog feed (`[audit.syslog]`) needs no FHIR and stays available.

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `true` | Master audit switch. |
| `enterprise_site_id` | string | unset | `AuditEnterpriseSiteID`. |
| `source_id` | string | `ferroehr` | Audit source id. |
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
`FERROEHR__SUBJECT_PROXY__SYSTEMS__PAS__BASE_URL`.

## Process / CLI

| Variable | Type | Default | Description |
|---|---|---|---|
| `FERROEHR_HEALTHCHECK_URL` | URL | `http://127.0.0.1:8080/ferroehr/rest/status` | Target URL for the binary's `healthcheck` subcommand (container `HEALTHCHECK` and Kubernetes exec probes). Not part of `ferroehr.toml`. |

## The `config` subcommands

```
ferroehr config default             # print the annotated default ferroehr.toml
ferroehr config check [--config P]  # validate (file + env + --set), print the
                                   #   effective config (secrets redacted) with
                                   #   a provenance column; exit 0 on success, 1 on error
```

`ferroehr config check` runs the exact same three validation passes as boot but
touches no database — use it in CI and before a rollout.

## Zero-config boot and the production checklist

With no file and no environment the server boots as: listener `0.0.0.0:8080`
at the ITS-REST base path with Swagger UI; DB at the compose-dev DSN; RBAC on;
signing on (digest); log `auto`/`info`; **everything else off**.

`auth.enabled` defaults to `true`, and **authentication enabled with no
mechanism configured is a boot error**, not a running server that refuses
everything: RFC 9110 §11.6.1 requires a `401` challenge to name a scheme
applicable to the resource, and a server with no mechanism has none — it could
only refuse every request while advertising a scheme it does not implement. The
error names the three ways out: add `[[auth.basic.users]]`, add an `[auth.oidc]`
issuer, or set `auth.enabled = false` for development. So a bare `docker run` of
the image with no configuration stops at startup with that message; the
downloadable Compose quickstart ships a user, which is why it boots.

For production, set at least:

- **`db.url`** — the real DSN, via `FERROEHR__DB__URL` (from a secret) or a
  `*_file`-mounted value, never inline in a world-readable file.
- **an auth mechanism** — a Basic user store and/or `[auth.oidc]`.
- **`log.format = "json"`** for cluster log collectors.
- **`server.cors_permissive`** stays `false`; **`server.swagger_ui`** per posture.
- **`server.system_id`** — this deployment's own openEHR system identifier
  (default `ferroehr.local`). Choose it before the first EHR is created: it
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
plain key you can set in the file or override with an `FERROEHR_*` env var.

For a worked dev example (server section, CORS, admin, management, and the
Basic-auth user store), read the configuration carried inline in the quickstart
`docker-compose.yml` — see [Docker Compose](compose.md); the repository's
`docker/ferroehr.dev.toml`, used by the from-source development stack, is a
fuller one with three users and RBAC enabled.

## Variables outside the server's namespace

The `PostgreSQL init` container variables are `PG_INIT_USER` / `_PASSWORD` /
`_DB` — they configure the database container, not the server, and sit
outside the server's reserved `FERROEHR_` namespace.

Inside that namespace, a handful of names are deliberately **not**
configuration keys and pass the strict sweep untouched: `FERROEHR_CONFIG`
(the config-file pointer), `FERROEHR_HEALTHCHECK_URL` (the container
healthcheck), the build-stamp variables, and the Compose parameterization
(image tags, host ports, CPU/memory limits). They keep a single `_` by
design, which is exactly what distinguishes them from configuration keys.
