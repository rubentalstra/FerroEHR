# Server, database & telemetry

The listener and REST surface (`[server]` and its sub-tables), the PostgreSQL
connection (`[db]`), and the two observability sections (`[log]`,
`[telemetry]`). Precedence, the environment-name grammar, and file discovery
are on the [Configuration reference](configuration.md) index.

<!-- toc -->

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
| `base_path` | string | `/ferroehr/rest/openehr/v1` | ITS-REST base path all API routes hang off. The status, health and documentation routes hang off its parent (`/ferroehr/rest` by default), and the served OpenAPI document describes whatever paths this setting produces, never the defaults. |
| `max_in_flight` | int | `256` | Concurrent-request admission cap (not a rate). Requests beyond it are shed immediately with `503` + `Retry-After`, never queued, so offered load beyond capacity cannot exhaust memory. `0` installs no shedding layer at all. |
| `swagger_ui` | bool | `true` | Serve Swagger UI + the OpenAPI JSON under the REST root. Consider `false` in production. |
| `cors_permissive` | bool | `false` | Permissive (development) CORS. Left on, any origin may read API responses, so the server warns loudly at boot. Production configures explicit origins at the edge. |
| `system_id` | string | `ferroehr.local` | **This deployment's own openEHR system identifier**; see [below](#system_id-the-data-authoring-identity). Set a stable, deployment-unique name in production (`FERROEHR__SERVER__SYSTEM_ID`). |

The shed sits on the clinical API subtree only, as its outermost layer: a shed
request never reaches authentication, auditing, or the request body, and the
always-on status, health, discovery and management routes are never shed.

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
is admitted there as an additional, non-conflicting code, and is what
RFC 9110 §15.5.14 defines for this refusal.

The defaults are sized against the measured payloads in the vendored clinical
corpus rather than chosen as round numbers: the clinical tier clears the
largest operational template in that corpus several times over, and the bulk
tier is four times the clinical one, for payloads with no published bound (a
whole-EHR extract, a TDD batch). **Raise `body_bytes` if your compositions
embed large `DV_MULTIMEDIA` data**: a base64 radiology image can exceed either
tier on its own, and that is a deliberate operator decision rather than a
default.

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

**This is not `max_in_flight`, and you should be able to tell them apart from
the status alone.** `max_in_flight` protects *capacity*: too many requests in
flight at once, from anyone, and the excess is shed `503` + `Retry-After`.
Rate limiting protects *fairness*: one caller asking too often over time,
refused `429` + `Retry-After`. A `503` means the server is full; a `429` means
you are asking too fast.

Two tiers, because they defend different things. The **address** tier sits
outside authentication, so a flood of unauthenticated requests is refused
before it can make the server verify a signature per request; a limiter must
not itself be the expensive path. The **principal** tier sits inside
authentication, keyed on the authenticated subject, which is the only fair key
for a clinical API: a hospital behind one NAT is a single address, so an
address-keyed clinical limit would throttle an entire site because one client
was busy.

Both defaults sit above this implementation's own measured whole-server
ceiling, so neither tier can refuse a caller until it is asking for more than
the server could have served; below that line, capacity is `max_in_flight`'s
job. **A deployment sized for more than the reference measurement environment
should raise both in proportion.**

Refusals carry the limiter's own `Retry-After` and `x-ratelimit-*` headers
alongside the openEHR error body. `429` is the status RFC 6585 §4 defines for
this refusal, admitted by ITS-REST as an additional, non-conflicting code.

The always-on health family is covered by the address tier only, deliberately:
an orchestrator probe must never be refused because a principal-keyed bucket
was exhausted, and probe rates are nowhere near the address ceiling.

> [!TIP]
> **Benchmarking this server?** Turn the limiter off first, or you will measure
> it instead of the server. The project's own measurement lanes compose an
> overlay that does exactly that, and both instruments refuse to write a record
> if the server answered any `429`.

### `[server.connection]`: bounds before a request exists

```toml
[server.connection]
header_read_timeout_secs = 10
max_concurrent_streams = 256
http2_keep_alive_interval_secs = 30
http2_keep_alive_timeout_secs = 10
```

Every other limit on this page engages once a request has been parsed and
dispatched: body size, the request timeout, the rate limiter, the in-flight
shed. A client that opens a socket and then trickles request headers reaches
none of them, while costing itself almost nothing. This table is where that is
bounded, and HTTP/1 and HTTP/2 need different bounds because the exposure
differs: HTTP/1 streams a request head, so it can be trickled; HTTP/2
multiplexes streams, so the exposure is concurrency.

| Key | Type | Default | Description |
|---|---|---|---|
| `header_read_timeout_secs` | int | `10` | How long a connection may take to deliver a complete HTTP/1 request head, in seconds. `0` disables the bound. Applies to both listeners. |
| `max_concurrent_streams` | int | `256` | The most HTTP/2 streams one connection may have open at once. Bounds the request-setup work a peer can trigger by opening and immediately cancelling streams (the "HTTP/2 Rapid Reset" amplification, CVE-2023-44487). `0` leaves the HTTP library's own default. |
| `http2_keep_alive_interval_secs` | int | `30` | Interval between HTTP/2 keep-alive PINGs, in seconds. `0` disables them, and a peer that vanishes without a FIN is then held until the operating system notices. |
| `http2_keep_alive_timeout_secs` | int | `10` | How long to wait for a keep-alive PING response before closing the connection, in seconds. |

### `system_id`: the data-authoring identity

`system_id` is the identifier this CDR stamps into the data it authors. It
appears on the wire in three places:

- **`EHR.system_id`**, recorded when an EHR is created. The openEHR RM
  (`EHR Information Model`, *EHR Identifier Allocation*) says the
  `EHR.system_id` "should be set to the value that would normally be used for
  locally created EHRs": a value the deployment chooses, not a product
  constant.
- **`AUDIT_DETAILS.system_id`** on every commit for which the client did not
  supply one through the `openehr-audit-details` header. The openEHR REST API
  requires that "when `system_id` is not provided by the client, the server
  MUST set it to its own configured system identifier".
- **`OBJECT_VERSION_ID.creating_system_id`:** the middle segment of every
  version identifier the server mints
  (`<object_id>::<creating_system_id>::<version>`).

Practical notes:

- **The value must be a legal openEHR UID:** a UUID, an ISO OID, or an
  internet id (a reverse-domain / DNS-style name) per the openEHR BASE
  identification grammar. The server judges it with the same validating
  constructor its reader uses and **refuses to boot** on an illegal value,
  because it becomes the `creating_system_id` segment of every version
  identifier the server mints and an illegal value would produce ids the
  server's own reader rejects. A DNS-style name like `cdr.hospital.example` is
  valid; an empty value, or one containing the `::` field separator, is not.
- **Choose it before going live and keep it stable.** The value is stored with
  each EHR and each version; changing it later affects only *newly* authored
  data: existing EHR ids, audit rows and version identifiers are never
  rewritten, and previously issued `OBJECT_VERSION_ID`s stay valid.
- **Make it unique per system**, so data exchanged between openEHR systems
  keeps unambiguous provenance.
- **With multi-tenancy on** ([`[tenancy]`](config-auth.md#tenancy)), a tenant's
  own `system_id` takes precedence over this value for requests resolved to
  that tenant.
- **`system_id` is not `[server.identity]`.** `system_id` says *which system
  authored the data*; [`[server.identity]`](#serveridentity) is the *display*
  identity of the `OPTIONS` System-Options manifest. Rebranding changes the
  manifest and nothing in stored data; changing `system_id` changes what new
  data says about its origin and leaves the manifest alone.

### `[server.tls]`: native TLS and mutual-TLS client authentication

Native TLS termination on the main listener, off by default; deployments
commonly terminate TLS at an ingress. `client_auth = "required"` is the IHE
ATNA ITI-19 mutually-authenticated-node posture (see the
[Audit trail chapter](../audit.md#node-authentication-iti-19-mutual-tls)). The
separate-port management listener always stays plain HTTP.

```toml
[server.tls]
enabled = false
client_auth = "off"
min_version = "1.3"
```

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | Terminate TLS natively on the main listener. |
| `cert_file` | path | unset | Server certificate chain (PEM). Required when enabled; a missing, unreadable or certificate-less file stops startup. |
| `key_file` | path | unset | Server private key (PEM). Required when enabled. |
| `client_auth` | enum{off,optional,required} | `off` | Client-certificate policy. `optional` verifies a certificate when one is presented and still accepts connections without; `required` rejects any client without a verified certificate at the handshake. |
| `client_ca_file` | path | unset | The explicit CA bundle client certificates must chain to, never the web PKI. Required unless `client_auth = "off"`. |
| `min_version` | enum{"1.3","1.2"} | `"1.3"` | The lowest TLS version this listener negotiates. |

> [!NOTE]
> `min_version` defaults to **1.3 only**, following the OWASP Transport Layer
> Security Cheat Sheet: web applications must default to TLS 1.3 and may
> support TLS 1.2 for compatibility. Setting `"1.2"` enables 1.2 *alongside*
> 1.3, never instead of it; pick it only for a client that genuinely cannot
> do 1.3, such as an older integration engine or a pinned Java runtime.
> TLS 1.1 and 1.0 are not selectable at all: RFC 8996 deprecates them, and
> neither this key nor the TLS library offers them.

### `[server.identity]`

The System-Options manifest identity (`OPTIONS` on the API base path, e.g.
`OPTIONS /ferroehr/rest/openehr/v1`, the System API's one location). This is
the deployment's *display* identity only; the identifier stamped into authored
data is [`system_id`](#system_id-the-data-authoring-identity).

| Key | Type | Default | Description |
|---|---|---|---|
| `solution` | string | `FerroEHR` | Product name. |
| `solution_version` | string | the build's version | Product version. |
| `vendor` | string | `FerroEHR project` | Providing organisation. |
| `restapi_specs_version` | string | the ITS-REST release this build implements (`1.1.0`) | The openEHR REST API edition advertised. |
| `conformance_profile` | string | the profile the build's recorded conformance verdict earned | Advertised conformance profile. |

The defaults are derived from the build rather than typed into the handler, so
the manifest never out-claims what was actually measured. Override them only to
rebrand.

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
| `url` | secret URL | `postgres://ferroehr:ferroehr@localhost:5432/ferroehr` | Connection DSN. The default suits a local from-source run against a localhost PostgreSQL, and the server logs a prominent warning at boot while it is in use; **production MUST set it**. Credentials are redacted from every rendering. `DATABASE_URL` is a recognized lower-priority alias. |
| `url_file` | path | unset | Read the DSN from a file instead of the key above, for a mounted secret. Preferred over the environment form in Kubernetes: an environment value is readable through `/proc/<pid>/environ` and inherited by every child process. At most one of the pair, where the built-in development default does not count as "set". |
| `migrate` | enum{apply,verify} | `apply` | Whether the server applies its embedded migrations at boot. `apply` is what makes an empty configuration boot against an empty database. `verify` issues **no DDL at all**: it checks that the database already carries exactly this build's migrations and refuses to start otherwise, so the DSN can authenticate as a role with no DDL rights. Pair it with `ferroehr db migrate` run out of band; see [Operations](../operations.md#applying-migrations). |
| `max_connections` | int | `20` | Pool ceiling. Write-heavy deployments benefit from raising it. |
| `min_connections` | int | `2` | Idle connections kept open, avoiding cold-reopen churn under variable load. |
| `acquire_timeout_secs` | int | `30` | Seconds to wait for a free connection before failing. |
| `statement_timeout_ms` | int | `60000` | `statement_timeout` applied to every pooled connection; `0` leaves the server default. |

`statement_timeout_ms` is the backstop the HTTP request timeout cannot be:
answering a client by dropping the handler future does **not** cancel the
statement PostgreSQL is running, so without it a handful of expensive queries
can hold every pooled connection while every one of their callers has already
given up. Keep it **above** [`query.timeout_ms`](config-integrations.md#query)
so the AQL engine's own typed refusal fires first and this only catches what
the engine does not govern.

## `[log]`

```toml
[log]
format = "auto"
filter = "info,ferroehr=info"
```

| Key | Type | Default | Description |
|---|---|---|---|
| `format` | enum{auto,json,pretty} | `auto` | Stdout rendering. `auto` picks `json` when stdout is not a TTY and the coloured human format when it is; an explicit `pretty` forces colour even through a pipe. |
| `filter` | string | `info,ferroehr=info` | Boot log-filter directives; also the value `/management/loggers` resets to. `RUST_LOG` is a recognized lower-priority alias. |

> [!TIP]
> The ASCII boot banner follows the rendering that is actually installed, not the
> configured word: it prints for `pretty`, and for `auto` only when stdout is a
> terminal. Under `json`, and under `auto` off a terminal (a container, a pipe
> into a log collector) stdout is parseable JSON from the first byte.

## `[telemetry]`

OpenTelemetry export. With `otlp_endpoint` unset the trace export layer is not
installed at all, at zero overhead.

```toml
[telemetry]
service_name = "ferroehr"
environment = "dev"
traces_sample_ratio = 1.0
metrics_push = false
```

| Key | Type | Default | Description |
|---|---|---|---|
| `otlp_endpoint` | string | unset | OTLP/gRPC collector endpoint. Unset ⇒ no trace export. |
| `service_name` | string | `ferroehr` | The `service.name` resource attribute. |
| `environment` | string | `dev` | The `deployment.environment` resource attribute. |
| `traces_sample_ratio` | float | `1.0` | Head-sampling ratio (`0.1` is a common production start). |
| `metrics_push` | bool | `false` | Also **push** metrics over OTLP, alongside the Prometheus pull surface. Both surfaces are fed by one meter provider, so every instrument reaches both; the push needs `otlp_endpoint` set as well. |
| `flame_file` | path | unset | Span-timing flamegraph capture: write folded stack samples of every span to this file, and render offline with `inferno-flamegraph`. Unset ⇒ the layer is not installed. For diagnostic sessions, not a standing posture: the file grows with span traffic. |

> [!NOTE]
> Instrument names carry no Prometheus suffix; the exporter derives one. A
> counter named `auth_failures` is scraped as `auth_failures_total`, and units
> add `_seconds`/`_bytes`. Over OTLP the unsuffixed name is what a collector
> receives.

The scrape endpoint itself is not opened here; it is
[`management.endpoints.prometheus`](config-auth.md#management), which is `off`
until you name a level.
