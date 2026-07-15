# Configuration redesign — one `ehrbase.toml`

**WORKLIST W-13** (owner directive 2026-07-15: *"we have too many ENV variables
that need to be set. we need to redesign and rewrite and do a complete
rethinking of this setup using a toml file … we need the best possible
configuration setup."*)

**Status:** design (this document is the implementation contract — an
implementer builds from it without re-deciding anything). No openEHR spec
governs configuration mechanics — this is entirely our own design; the only
spec-adjacent constraints are noted inline (e.g. the System-Options identity
fields, the signing modes).

---

## 1. Inventory — what exists today (the evidence base)

### 1.1 The loaders

The server has **no root configuration object**. Fourteen independent loading
mechanisms each read the process environment on their own, twelve of them via
figment, each with its own prefix, its own optional TOML file pointer, and its
own env-key grammar:

| # | Config struct | Crate / file | Env prefix | Own TOML file var | Nesting grammar | Keys |
|---|---|---|---|---|---|---|
| 1 | `DbSettings` | `app/ehrbase/src/db/settings.rs:60-65` | `EHRBASE_DB_` | — (none) | flat | 4 (+ `DATABASE_URL` fallback) |
| 2 | `TelemetryConfig` | `app/ehrbase/src/telemetry/config.rs:109-131` | `EHRBASE_LOG_` + `EHRBASE_OTEL_` | — (none) | flat, custom `.map()` | 7 (+ `RUST_LOG` fallback) |
| 3 | `RestConfig` (incl. `AuthConfig`, `SmartConfig`, `SystemOptionsConfig`, `TenancyConfig`, `AdminConfig`, `TerminologyConfig`, `EventSubscriptionConfig`, `FhirConfig`) | `app/ehrbase-rest/src/config.rs:214-221` | `EHRBASE_REST_` | `EHRBASE_REST_CONFIG` | `__` | ~40 |
| 4 | `ManagementConfig` | `app/ehrbase-rest/src/extensions/management/config.rs:113-133` | `EHRBASE_MANAGEMENT_` | `EHRBASE_MANAGEMENT_CONFIG` | **single `_`** custom map (`ENDPOINTS_<EP>`) | 11 |
| 5 | `AuthzConfig` | `app/ehrbase-rest/src/extensions/access/authz/config.rs:268-275` | `EHRBASE_AUTHZ_` | `EHRBASE_AUTHZ_CONFIG` | `__` | ~14 |
| 6 | `AuditConfig` (ATNA) | `app/ehrbase/src/system_log/config.rs:122-129` | `EHRBASE_ATNA_` | `EHRBASE_ATNA_CONFIG` | `__` (all keys flat in practice) | 15 |
| 7 | `SigningConfig` | `app/ehrbase/src/versioning/signature/config.rs:97-104` | `EHRBASE_SIGNING_` | `EHRBASE_SIGNING_CONFIG` | `__` | 5 |
| 8 | `EventsConfig` | `app/ehrbase/src/extensions/events/config.rs:104-111` | `EHRBASE_EVENTS_` | `EHRBASE_EVENTS_CONFIG` | `__` | 9 |
| 9 | `FhirOutboundConfig` | `app/ehrbase/src/extensions/fhir/config.rs:99-106` | `EHRBASE_FHIR_OUTBOUND_` | `EHRBASE_FHIR_OUTBOUND_CONFIG` | `__` | 7 |
| 10 | `MultimediaConfig` | `app/ehrbase/src/extensions/multimedia/config.rs:89-96` | `EHRBASE_MULTIMEDIA_` | `EHRBASE_MULTIMEDIA_CONFIG` | `__` | 8 |
| 11 | `ExternalTerminologyConfig` | `app/ehrbase/src/service/terminology/config.rs:135-142` | `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_` | **`EHRBASE_VALIDATION_CONFIG`** (prefix mismatch) | `__` | ~8 |
| 12 | `SubjectProxyConfig` | `app/ehrbase/src/service/subject_proxy/config.rs:82-89` | `EHRBASE_SUBJECT_PROXY_` | `EHRBASE_SUBJECT_PROXY_CONFIG` | `__` | ~4 |
| 13 | raw `std::env::var` | `app/ehrbase/src/service/query/plan_cache.rs:52`, `.../query/execute.rs:245` | `EHRBASE_QUERY__*` | — | n/a (LazyLock, silent fallback) | 2 |
| 14 | clap `#[arg(env)]` | `app/ehrbase/src/main.rs:48` | `EHRBASE_HEALTHCHECK_URL` | — | n/a | 1 |

Totals: **≈130 distinct runtime configuration keys**, **9 separate optional
TOML file pointers**, **at least 4 different env→key grammars**, and **12
independent figment chains** each re-reading the global environment
(`app/ehrbase/src/main.rs:83-231` performs eleven separate `load()` calls in
sequence).

Not runtime config (out of scope for the redesign, listed for completeness):

- **Build-time** (`app/ehrbase-rest/build.rs:11-52`): `EHRBASE_GIT_SHA`,
  `EHRBASE_BUILD_EPOCH`, `EHRBASE_RUSTC` — compile-time build info, untouched.
- **Compose/infra-level** (`docker-compose.yml`): `EHRBASE_IMAGE`,
  `EHRBASE_POSTGRES_IMAGE`, `EHRBASE_PORT`, `EHRBASE_DB_PORT`,
  `EHRBASE_S3_PORT`, `KEYCLOAK_PORT`, `POSTGRES_PASSWORD`, `BENCH_PG_*`, and
  the postgres-init trio `EHRBASE_DB_USER`/`_PASSWORD`/`_NAME` (consumed by
  `docker/postgres`'s init script, not by the server binary) — deployment
  parameterization, untouched (though `EHRBASE_DB_USER/_PASSWORD/_NAME` are
  renamed in §6.1 to stop colliding with the server's namespace).
- **Tooling env** (`CONF_*` in `scripts/conformance.sh`, `BENCH_*` in the
  benchmark harness) — tool config, untouched.
- `EHRBASE_ACCESS_CONTROL_V1` — an RM `_type` discriminator string
  (`app/ehrbase-sm/src/extensions/ehr_access.rs:44`), not configuration.
- `ehrbase-sm` reads **no** environment configuration (verified by grep).

### 1.2 The defects this redesign fixes (numbered; each traceable)

- **C-1 — Four env grammars.** `EHRBASE_DB_MAX_CONNECTIONS` (flat single `_`,
  `db/settings.rs:62`) vs `EHRBASE_REST_AUTH__ENABLED` (`__` nesting,
  `config.rs:219`) vs `EHRBASE_MANAGEMENT_ENDPOINTS_PROMETHEUS` (custom
  single-`_` map, `management/config.rs:125-131`) vs
  `EHRBASE_QUERY__PLAN_CACHE_CAPACITY` (a `__` *spelling* on a raw
  `std::env::var` that is not figment at all, `plan_cache.rs:52`). The book
  page has to carry a warning box about it
  (`website/book/src/installation/configuration.md:27-37` "Getting the
  separator wrong is the most common configuration mistake").
- **C-2 — Prefix/file mismatch.** The external-terminology file pointer is
  `EHRBASE_VALIDATION_CONFIG` while its env prefix is
  `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_` (`terminology/config.rs:137,140`).
- **C-3 — A documented env form that cannot bind.** The subject-proxy docs and
  `main.rs:215` document `EHRBASE_SUBJECT_PROXY__SYSTEMS__PAS__BASE_URL`, but
  the loader strips the prefix `EHRBASE_SUBJECT_PROXY_` (single trailing `_`,
  `subject_proxy/config.rs:87`), leaving `_SYSTEMS__PAS__BASE_URL` → key path
  `_systems.pas.base_url`, which does not match the `systems` field. There is
  no Jail test of the env form (only the TOML/struct paths are tested) — the
  documented spelling is almost certainly dead. The redesign's mechanical
  mapping + env-binding tests make this class of bug impossible.
- **C-4 — Silent fallback on unparseable values.** `EHRBASE_QUERY__PLAN_CACHE_CAPACITY`
  and `EHRBASE_QUERY__TIMEOUT_MS` swallow parse failures
  (`plan_cache.rs:47-56` "Unset or unparseable → default";
  `execute.rs:244-250`). A typo'd value silently runs with defaults. Everywhere
  else a bad value is a boot error — inconsistent and dangerous.
- **C-5 — No unknown-key rejection anywhere.** All twelve figment chains
  extract permissively; a misspelled TOML key or env var (e.g.
  `EHRBASE_SIGNING_ENABELD`, `[signing] enabld = false`) is silently ignored.
  For a clinical server, silently-not-applied security config is the worst
  failure mode this redesign kills.
- **C-6 — Nine partial config files.** An operator wanting file-based config
  must maintain up to nine TOML files and nine `EHRBASE_*_CONFIG` pointer vars
  (`docker-compose.yml:79` mounts one of them). The Helm chart resorts to
  "point the matching `EHRBASE_*_CONFIG` at the file via extraEnv"
  (`deploy/helm/ehrbase-rs/values.yaml:339-355`).
- **C-7 — Inconsistent secret handling.** `SigningConfig.key_passphrase` is a
  `secrecy::SecretString` and deliberately does not derive `Serialize`
  (`signature/config.rs:12-14,70`); `AuthConfig` uses a local `Redacted`
  newtype (`authn/config.rs:10-18`); but `MultimediaConfig.secret_access_key`
  is a plain `Option<String>` **with** `Serialize`
  (`multimedia/config.rs:34,61`), and the AMQP broker URLs
  (`events/config.rs:53`, `fhir/config.rs:55`) embed credentials in plain
  serializable `String`s. The `/management/env` snapshot only includes
  rest/management/telemetry/db today (`main.rs:310-322`) — the others are
  saved from leaking only by *not being reported at all*.
- **C-8 — Defaults encoded twice per struct.** Every config struct carries
  both `#[serde(default = ...)]` functions and a hand-written `Default` impl
  that must agree (e.g. `events/config.rs:46-94`); nothing checks they do.
- **C-9 — No effective-config visibility or dry-run.** There is no way to
  validate a config without booting the server against a live database, and no
  way to print what the server *would* run with.
- **C-10 — `SigningConfig::load` skips the defaults provider.**
  It starts from `Figment::new()` (`signature/config.rs:98`) while every other
  loader starts from `Serialized::defaults(...)` — harmless today only because
  its serde defaults are complete; one more per-loader divergence.
- **C-11 — Two configuration crates pinned, one dead.** `config` (0.15.25,
  `Cargo.toml:176`, comment "or figment") is consumed by zero crates (grep: no
  `config.workspace = true` anywhere), while all twelve loaders sit on
  `figment` 0.10 — a crate that has not shipped a release in ~2 years. The
  owner resolved the "or" on 2026-07-15: **the redesign builds on `config`
  0.15.25 and removes `figment` from the workspace entirely** (§5.1).

---

## 2. Principles

- **P-1 — One file: `ehrbase.toml`.** The primary operator interface is a
  single TOML document whose sections cover the entire server. The nine
  per-subsystem files and their `EHRBASE_*_CONFIG` pointers are removed
  (hard boot error with a migration message, §6.2).
- **P-2 — Zero-config boot.** A server started with **no config file and no
  environment** boots with dev-appropriate defaults (§3.16 lists them and the
  production checklist). Concretely this requires one change from today:
  `db.url` gains the default `postgres://ehrbase:ehrbase@localhost:5432/ehrbase`
  (matching the compose dev stack, `docker-compose.yml:26-28,65`) instead of
  being required (`db/settings.rs:16` has no default today). Boot still fails
  if that PostgreSQL isn't reachable — but it fails at the *connection* step
  with a clear error, not at the config step.
- **P-3 — Layered precedence, lowest to highest:**
  1. built-in defaults (Rust `Default` impls),
  2. the config file (`--config` / `EHRBASE_CONFIG` / search order, §5.4),
  3. `EHRBASE_*` environment variables,
  4. CLI flags (`--set key=value`, repeatable).

  Two grandfathered conventional aliases sit *below* their `EHRBASE_` forms
  within layer 3: `DATABASE_URL` → `db.url` (12-factor convention, kept from
  `db/settings.rs:63`) and `RUST_LOG` → `log.filter` (kept from
  `telemetry/config.rs:124-129`). Nothing else gets a non-`EHRBASE_` name.
- **P-4 — One mechanical env mapping.** `EHRBASE_` + the TOML path upper-cased
  with `__` (double underscore) between path segments; single `_` only ever
  appears *inside* a key word:

  ```
  [db] max_connections = 20        ⇔  EHRBASE_DB__MAX_CONNECTIONS=20
  [auth.oidc] issuer = "…"         ⇔  EHRBASE_AUTH__OIDC__ISSUER=…
  [management.endpoints] env="off" ⇔  EHRBASE_MANAGEMENT__ENDPOINTS__ENV=off
  [terminology.external.providers.default] url = "…"
                                   ⇔  EHRBASE_TERMINOLOGY__EXTERNAL__PROVIDERS__DEFAULT__URL=…
  ```

  Why `__` and not single `_`: with single `_` the mapping is not a bijection —
  `EHRBASE_DB_MAX_CONNECTIONS` cannot be distinguished from a hypothetical
  `db_max.connections`, and map keys / multi-word fields (`max_in_flight`,
  `base_path`, `fail_on_error`) become unparseable. `__` is also already the
  *majority* convention in the code (loaders 3, 5–12 in §1.1), is what the
  `config` crate implements natively
  (`Environment::with_prefix("EHRBASE").separator("__")`), and matches the
  ecosystem norm (e.g. ASP.NET, Hasura, many Rust services). The two current
  single-`_` families (DB, LOG/OTEL, MANAGEMENT endpoints) migrate with
  aliases (§6.1).

  Env **values**: scalars are typed via the `config` crate's
  `try_parsing(true)` (bool/int/float; string fallback). **List-typed keys
  use comma-separated values** (`EHRBASE_AUTH__OIDC__AUDIENCES=ehrbase,other`),
  enabled per key with `list_separator(",")` + `with_list_parse_key(...)` —
  the loader registers every list-typed key path from the schema tree (a
  static list beside the wildcard list of §5.3), so scalar values containing
  commas are never mis-split. Arrays-of-tables (the Basic-auth user store)
  are file-only, documented as such.
- **P-5 — Strict validation at boot.**
  - **Unknown keys are rejected** — in the file *and* in the `EHRBASE_`
    env namespace — with a did-you-mean suggestion and, for file keys, the
    `file:line` of the offending key (§5.3).
  - **Type errors are boot errors** with the file:line (or the env var name)
    and the expected type. The C-4 silent fallbacks are eliminated: the
    `[query]` knobs become ordinary typed fields.
  - **Semantic validation is aggregated:** one `validate()` pass over the
    whole tree reports *all* errors at once (the existing
    `AuthzConfig::validate` at `authz/config.rs:282-298` and
    `SmartConfig::validate` at `smart/config.rs:190-200` fold into it), so an
    operator fixes a config in one iteration, not one error per boot.
- **P-6 — Secrets never render.** Every secret-typed field (a) is stored as
  `secrecy::SecretString` behind one shared `Secret` serde newtype, (b) is
  redacted in `Debug`, in the `/management/env` snapshot, and in
  `ehrbase config check` output, and (c) offers a `*_file` sibling key for
  file-based indirection (Kubernetes/Docker secrets mount files). Decision:
  **`*_file` siblings, no `${ENV}` interpolation** inside TOML values —
  interpolation makes `config check` output environment-dependent, invites
  secrets back into files by another name, and the `config` crate has no
  native support for it; the env layer (P-3/P-4) already covers "inject at
  deploy time",
  e.g. `EHRBASE_SIGNING__KEY_PASSPHRASE` from a K8s `secretKeyRef` exactly as
  the Helm chart does today (`templates/deployment.yaml`).
- **P-7 — Boot-only.** All configuration is read once at process start. No
  hot reload (§7); the two existing runtime-mutable surfaces are unaffected
  (the `/management/loggers` `EnvFilter` control and the Cedar
  `abac.cedar.reload_secs` policy re-read — both operate on their own state,
  not on the config file).
- **P-8 — Sections group by subsystem, not by crate.** The operator does not
  care that tenancy middleware lives in `ehrbase-rest` while the events
  publisher lives in `ehrbase`. E.g. the REST toggle for the
  event-subscription admin API moves under `[events]`, and the external
  terminology validation moves under `[terminology]` (fixing C-2's naming at
  the root).

---

## 3. The schema

This section is normative and is the source for the rewritten book page
(§6.4). Types: `string`, `bool`, `int`, `float`, `path`, `secret` (P-6),
`enum{…}` (lowercase / `snake_case` tokens, as today). Every key has a
default — the file may be empty or absent (P-2). The annotated template that
`ehrbase config default` emits (§5.5) is exactly this schema.

Top-level tables, in the order the default template prints them:
`[server]`, `[db]`, `[log]`, `[telemetry]`, `[auth]`, `[authz]`, `[admin]`,
`[tenancy]`, `[smart]`, `[management]`, `[signing]`, `[query]`, `[events]`,
`[fhir]`, `[terminology]`, `[multimedia]`, `[atna]`, `[subject_proxy]`.

### 3.1 `[server]` — the HTTP listener and REST surface

Today: `RestConfig` scalars (`app/ehrbase-rest/src/config.rs:12-87`) +
`SystemOptionsConfig` (`api/system/options.rs`).

| Key | Type | Default | Doc |
|---|---|---|---|
| `bind` | string | `"0.0.0.0:8080"` | Socket address the API listener binds. |
| `base_path` | string | `"/ehrbase/rest/openehr/v1"` | ITS-REST base path all API routes hang off. |
| `max_in_flight` | int | `256` | Concurrent-request admission cap; beyond it requests are shed with `503` + `Retry-After` (never queued). `0` disables shedding. Status/health/discovery endpoints are never limited. (RFC 9110 §15.6.4; W-11 sizing note in `config.rs:252-258`.) |
| `swagger_ui` | bool | `true` | Serve Swagger UI + the OpenAPI JSON at the REST root. Consider `false` in production. |
| `cors_permissive` | bool | `false` | Permissive CORS (dev only). Production configures explicit origins (Stage 2). |

`[server.identity]` — the `OPTIONS /` System-Options manifest identity
(ITS-REST System API; defaults from the shared provenance source, which must
stay measured — the manifest MUST NOT out-claim the last ECC verdict,
`api/system/options.rs`):

| Key | Type | Default | Doc |
|---|---|---|---|
| `solution` | string | `"EHRbase-RS"` | `Options.solution` — the product name. |
| `solution_version` | string | build version | `Options.solution_version`. |
| `vendor` | string | `"EHRbase-RS project"` | `Options.vendor`. |
| `restapi_specs_version` | string | tested-contract identity | `Options.restapi_specs_version` (defaults to the provenance-derived edition the build is tested against). |
| `conformance_profile` | string | last machine-computed ECC verdict | `Options.conformance_profile`. |

### 3.2 `[db]` — PostgreSQL

Today: `DbSettings` (`app/ehrbase/src/db/settings.rs`).

| Key | Type | Default | Doc |
|---|---|---|---|
| `url` | secret-bearing URL | `"postgres://ehrbase:ehrbase@localhost:5432/ehrbase"` | Connection DSN. The default matches the compose dev stack (P-2); **production MUST set it** (checklist §3.16). Credentials in the URL are redacted from every rendering (P-6, `DbUrl` newtype §5.6). `DATABASE_URL` is a recognized lower-priority alias (P-3). |
| `max_connections` | int | `20` | Pool ceiling (P20 default: 10 hard-capped realistic write concurrency ×2, `settings.rs:75-77`). |
| `min_connections` | int | `2` | Idle connections kept open (avoids cold reopen churn). |
| `acquire_timeout_secs` | int | `30` | Seconds to wait for a free connection before failing. |

### 3.3 `[log]` — logging

Today: `LogConfig` (`app/ehrbase/src/telemetry/config.rs:25-44`).

| Key | Type | Default | Doc |
|---|---|---|---|
| `format` | enum{`auto`,`json`,`pretty`} | `auto` | Stdout rendering; `auto` picks `json` when stdout is not a TTY. `json` also suppresses the boot banner (`main.rs:86-91`). |
| `filter` | string | `"info,ehrbase=info"` | Boot `EnvFilter` directives; also the `/management/loggers` reset target. `RUST_LOG` is a recognized lower-priority alias (P-3). |

### 3.4 `[telemetry]` — OpenTelemetry export

Today: `OtelConfig` (`telemetry/config.rs:47-89`).

| Key | Type | Default | Doc |
|---|---|---|---|
| `otlp_endpoint` | string | unset | OTLP/gRPC collector endpoint. **Unset ⇒ the OTel layer is not installed at all** (zero overhead). |
| `service_name` | string | `"ehrbase"` | `service.name` resource attribute. |
| `environment` | string | `"dev"` | `deployment.environment` resource attribute. |
| `traces_sample_ratio` | float | `1.0` | Head-sampling ratio (`0.1` is the documented prod starting point). |
| `metrics_push` | bool | `false` | Also push metrics over OTLP alongside the Prometheus pull surface. |

### 3.5 `[auth]` — authentication

Today: `AuthConfig` (`app/ehrbase-rest/src/extensions/access/authn/config.rs`).
The deprecated `admin_scope` back-compat alias (`authn/config.rs:43-49`)
**dies with this redesign** (it was already subsumed by the RBAC role gate;
§6.1 maps it).

| Key | Type | Default | Doc |
|---|---|---|---|
| `enabled` | bool | `true` | Master switch. `false` = all requests pass unauthenticated (dev only). With `true` and no mechanism configured, every API request 401s; boot logs a prominent hint (P-2 note in §3.16). |
| `verified_cache_ttl_seconds` | int | `60` | Verified Basic-credential cache TTL (`0` disables); bounds Argon2 cost per busy client and revocation lag alike. |

`[[auth.basic.users]]` — the Basic-auth user store (array of tables,
**file-only**, P-4):

| Key | Type | Default | Doc |
|---|---|---|---|
| `username` | string | required | Principal name. |
| `password_hash` | secret | required | Argon2 PHC hash (`$argon2id$v=19$…`). Never a plaintext password. |
| `roles` | list of string | `["USER"]` | Roles granted (upper-cased on authentication), feeding the RBAC gate. |

`[auth.oidc]` — OAuth2/OIDC bearer validation (resource-server role). Absent
table ⇒ bearer disabled:

| Key | Type | Default | Doc |
|---|---|---|---|
| `issuer` | string | required (when table present) | Expected `iss`; also the OIDC discovery base when no static key material is supplied. |
| `audiences` | list of string | `[]` | Accepted `aud`; empty = not checked. |
| `algorithms` | list of string | `["RS256"]` | Accepted signature algorithms. |
| `hmac_secret` / `hmac_secret_file` | secret / path | unset | Symmetric HS256 secret (dev/test). Exactly one of the pair (P-6). |
| `jwks_json` / `jwks_json_file` | string / path | unset | Static JWKS document; preferred over discovery when present. The `_file` form is new (a JWKS is a file-shaped blob that never belonged in an env var). |

### 3.6 `[authz]` — RBAC + ABAC

Today: `AuthzConfig` (`app/ehrbase-rest/src/extensions/access/authz/config.rs`).
Structure unchanged (it is already well-designed with boot validation); only
its loading moves into the tree.

`[authz.rbac]`: `enabled` (bool, `true`), `admin_role` (string, `"ADMIN"`),
`user_role` (string, `"USER"`), `role_claims` (list, `["realm_access.roles","scope"]`),
`management_access` (enum{`admin_only`,`private`,`public`}, `admin_only`).

`[authz.abac]`: `enabled` (bool, `false`), `engine` (enum{`cedar`,`remote`},
`cedar`), `organization_claim` (string, `"organization_id"`), `patient_claim`
(string, `"patient_id"`).
`[authz.abac.cedar]`: `policy_dir` (path, unset — required when engine=cedar
and abac enabled), `reload_secs` (int, unset).
`[authz.abac.remote]`: `server` (string, unset — required when engine=remote,
must end `/`), `connect_timeout_ms` (int, `2000`), `request_timeout_ms` (int,
`5000`).
`[authz.abac.policy.<kind>]` (kind ∈ ehr, ehr_status, composition,
contribution, query, directory): `name` (string), `parameters` (list of
enum{`organization`,`patient`,`template`}).

All the existing boot-validation rules (`authz/config.rs:282-352`) carry over
verbatim into the aggregated `validate()` (P-5).

### 3.7 `[admin]` — the ADMIN API group

Today: `AdminConfig` (`config.rs:130-136`).

| Key | Type | Default | Doc |
|---|---|---|---|
| `enabled` | bool | `false` | Mount the ADMIN API (physical, irreversible delete). Off ⇒ routes are absent (404), never 403. |

### 3.8 `[tenancy]` — multi-tenancy

Today: `TenancyConfig` (`config.rs:97-121`). Keys unchanged: `enabled` (bool,
`false`), `claim` (string, `"tenant"`), `header` (string, unset — dev-only;
a client-supplied header must not select a tenant in production).

### 3.9 `[smart]` — SMART App Launch

Today: `SmartConfig` (`app/ehrbase-rest/src/smart/config.rs`). Moves from
`EHRBASE_REST_SMART__*` to top-level `[smart]` (it is a platform posture, not
an HTTP-listener detail). Keys unchanged:

`[smart]`: `enabled` (bool, `false`), `platform_base_url` (string, unset →
REST root), `ehr_id_claim` (string, `"ehrId"`), `patient_claim` (string,
`"patient"`), `require_smart_scopes` (bool, `false`), `launch_base64_json`
(bool, `false`).
`[smart.episode]`: `enabled` (bool, `false`).
`[smart.endpoints]`: `issuer`, `jwks_uri`, `authorization_endpoint`,
`token_endpoint`, `registration_endpoint`, `introspection_endpoint`,
`revocation_endpoint`, `management_endpoint` (all string, unset ⇒ omitted from
the discovery document); `token_endpoint_auth_methods_supported`,
`grant_types_supported`, `response_types_supported`,
`code_challenge_methods_supported`, `scopes_supported` (all list of string,
`[]`). The deprecated-grant boot rejection (`smart/config.rs:190-200`,
master06 §Deprecated Flows) folds into the aggregated `validate()`.

### 3.10 `[management]` — the management/observability surface

Today: `ManagementConfig` (`app/ehrbase-rest/src/extensions/management/config.rs`).
Keys unchanged; the env spelling becomes mechanical
(`EHRBASE_MANAGEMENT__ENDPOINTS__PROMETHEUS`, fixing the C-1 special case):

`[management]`: `enabled` (bool, `false`), `base_path` (string,
`"/management"`), `port` (int, unset ⇒ share the main listener), `access_default`
(enum{`off`,`admin_only`,`private`,`public`}, `admin_only`), `probes_enabled`
(bool, `false`).
`[management.endpoints]`: `health`, `info`, `metrics`, `prometheus`, `env`,
`loggers` — each enum{`off`,`admin_only`,`private`,`public`}, default `off`.

New validation rule: `management.port`, when set, must differ from the port in
`server.bind`.

### 3.11 `[signing]` — VERSION signing

Today: `SigningConfig` (`app/ehrbase/src/versioning/signature/config.rs`).
Modes are the spec-blessed ones (RM common master06 §Digital Signature).

| Key | Type | Default | Doc |
|---|---|---|---|
| `enabled` | bool | `true` | Server-side signing of committed versions (default on so the STANDARD "Signing" capability is demonstrably met). |
| `mode` | enum{`digest`,`pgp`} | `digest` | Integrity digest vs RFC 4880 detached signature. |
| `key_path` | path | unset | Armored secret key; **required for `pgp`** (validated at boot, fail-closed — `main.rs:176-180`). |
| `key_passphrase` / `key_passphrase_file` | secret / path | unset | Key passphrase (P-6 pair). |
| `verify_on_read` | enum{`off`,`warn`,`strict`} | `off` | Recompute-and-compare policy at read time. |

### 3.12 `[query]` — AQL execution knobs

Today: two raw env reads (C-4). They become typed fields, plumbed into
`QueryService` at construction (the `LazyLock` env reads in
`plan_cache.rs:52-56` and `execute.rs:244-250` are deleted).

| Key | Type | Default | Doc |
|---|---|---|---|
| `plan_cache_capacity` | int | `256` | Max distinct cached query plans; `0` disables the cache. |
| `timeout_ms` | int | `0` | Per-query DB execution budget; `0` disables (the global request timeout remains the only guard). Overrun reports `408` (ITS-REST Requests_and_responses §HTTP status codes). |

### 3.13 `[events]` — contribution-outbox eventing (+ its admin API)

Today: `EventsConfig` (`app/ehrbase/src/extensions/events/config.rs`) + the
REST toggle `EventSubscriptionConfig` (`config.rs:154-168`) which moves here
(P-8).

| Key | Type | Default | Doc |
|---|---|---|---|
| `enabled` | bool | `false` | Master switch; also (with `fhir.outbound.enabled`) gates the per-commit outbox INSERT. |
| `url` | secret-bearing URL | `"amqp://guest:guest@localhost:5672/%2f"` | AMQP broker URL; credentials redacted from every rendering (`BrokerUrl` newtype, §5.6). |
| `exchange` | string | `"ehrbase.events"` | Topic exchange (PHI-free envelope stream). |
| `tls` | bool | `false` | Upgrade `amqp://` to `amqps://`. |
| `batch_size` | int | `128` | Rows drained per poll. |
| `poll_interval_ms` | int | `1000` | Idle poll interval. |
| `retention_days` | int | `7` | Published-row retention window. |
| `prune_interval_secs` | int | `3600` | Retention-prune cadence. |
| `publish_max_retries` | int | `3` | Per-row publish retries before backing off. |
| `admin_api` | bool | `false` | Mount the `/admin/event_subscription` CRUD routes (was `EHRBASE_REST_EVENT_SUBSCRIPTION__ENABLED`). |

### 3.14 `[fhir]` — the FHIR connector (inbound façade + outbound emitter)

Today split across `FhirConfig` (`config.rs:178-184`, inbound routes) and
`FhirOutboundConfig` (`app/ehrbase/src/extensions/fhir/config.rs`). They stay
**independent switches** (the emitter carries PHI by design — the PHI note in
`fhir/config.rs:19-26` carries over into the template comments):

`[fhir]`: `api_enabled` (bool, `false`) — mount `/fhir/r4/*` +
`/admin/fhir_mapping` (was `EHRBASE_REST_FHIR__ENABLED`).
`[fhir.outbound]`: `enabled` (bool, `false`), `url` (secret-bearing URL, same
AMQP default), `exchange` (string, `"ehrbase.fhir"` — deliberately distinct
from the events exchange for PHI isolation), `tls` (bool, `false`),
`batch_size` (int, `128`), `poll_interval_ms` (int, `1000`),
`publish_max_retries` (int, `3`).

### 3.15 `[terminology]`, `[multimedia]`, `[atna]`, `[subject_proxy]`

`[terminology]` (P-8 regrouping; fixes C-2):
- `api_enabled` (bool, `false`) — mount the terminology extension API (was
  `EHRBASE_REST_TERMINOLOGY__ENABLED`).
- `[terminology.external]`: `enabled` (bool, `false`), `fail_on_error` (bool,
  `false`) — was `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_*`.
- `[terminology.external.providers.<name>]`: `type` (enum{`fhir`}, `fhir`),
  `url` (string, required), `operation` (enum{`validate_code`,`expand`},
  `validate_code`), `connect_timeout_ms` (int, `2000`), `request_timeout_ms`
  (int, `10000`), `oauth2_client` (string, unset — accepted-not-yet-honoured,
  PORT NOTE at `terminology/config.rs:107-115` carries over).

`[multimedia]` (today `MultimediaConfig`,
`app/ehrbase/src/extensions/multimedia/config.rs`): `enabled` (bool, `false`),
`threshold_bytes` (int, `262144`), `endpoint` (string, unset ⇒ AWS default
resolution), `bucket` (string, `"openehr-multimedia"`), `region` (string,
`"us-east-1"`), `access_key_id` (string, unset), `secret_access_key` /
`secret_access_key_file` (secret / path, unset — fixing C-7; both unset ⇒
anonymous client), `allow_http` (bool, `false` — dev only).

`[atna]` (today `AuditConfig`, `app/ehrbase/src/system_log/config.rs`; keys
unchanged): `enabled` (bool, `false`), `enterprise_site_id` (string, unset),
`repository_host` (string, `"localhost"`), `repository_port` (int, `514`),
`transport` (enum{`udp`,`tls`}, `udp`), `source_id` (string, `"ehrbase"`),
`value_if_missing` (string, `"UNKNOWN"`), `suppress_login_events` (bool,
`true`), `fail_mode` (enum{`open`,`closed`}, `open`), `resolve_subject`
(bool, `false`), `queue_capacity` (int, `1024`), `server_host` (string,
unset), `tls_ca_path` / `tls_identity_cert_path` / `tls_identity_key_path`
(path, unset).

`[subject_proxy.systems.<name>]` (today `SubjectProxyConfig`,
`app/ehrbase/src/service/subject_proxy/config.rs`; fail-closed — no systems ⇒
every FHIR frame is a typed rejection): `base_url` (string, required),
`connect_timeout_ms` (int, `2000`), `request_timeout_ms` (int, `10000`).
The env form becomes `EHRBASE_SUBJECT_PROXY__SYSTEMS__PAS__BASE_URL` — which
now actually binds (fixing C-3).

### 3.16 The zero-config boot posture and the production checklist

With no file and no env, the server boots as: listener `0.0.0.0:8080` at the
ITS-REST base path with Swagger UI; DB at the compose-dev DSN; auth **enabled
with no mechanism ⇒ every API request 401s** (fail-closed; boot logs one
prominent `warn` naming the two ways out: add `[[auth.basic.users]]`/
`[auth.oidc]`, or set `auth.enabled=false` for dev); RBAC on; signing on
(digest); log `auto`/`info`; **everything else off** (management, ATNA,
events, FHIR, SMART, tenancy, admin API, terminology extension, multimedia,
external terminology, OTel, subject proxy, ABAC, query timeout).

Production MUST differ on (the template marks each with `# PROD:`):
`db.url` (real DSN via env/secret), an auth mechanism, `log.format = "json"`,
`server.cors_permissive` stays `false`, `server.swagger_ui` per posture,
`management.*` per posture (own `port` recommended), TLS everywhere a
transport supports it (`atna.transport="tls"`, `events.tls`,
`fhir.outbound.tls`, HTTPS S3), and real secrets via env or `*_file` — never
inline in a world-readable file.

---

## 4. The complete old→new mapping

Legend — **alias**: honoured for one transition release with a boot-time
`warn` deprecation (then removed); **dies**: hard boot error naming the
replacement (the var is in the reserved `EHRBASE_` namespace, so the strict
env sweep of §5.3 catches it); **unchanged**: not runtime server config,
untouched.

| Old | New key (env form per P-4) | Fate |
|---|---|---|
| `EHRBASE_DB_URL` | `db.url` (`EHRBASE_DB__URL`) | alias |
| `DATABASE_URL` | `db.url` | kept permanently (P-3) |
| `EHRBASE_DB_MAX_CONNECTIONS` | `db.max_connections` | alias |
| `EHRBASE_DB_MIN_CONNECTIONS` | `db.min_connections` | alias |
| `EHRBASE_DB_ACQUIRE_TIMEOUT_SECS` | `db.acquire_timeout_secs` | alias |
| `EHRBASE_LOG_FORMAT` / `EHRBASE_LOG_FILTER` | `log.format` / `log.filter` (`EHRBASE_LOG__*`) | alias |
| `RUST_LOG` | `log.filter` | kept permanently (P-3) |
| `EHRBASE_OTEL_OTLP_ENDPOINT` | `telemetry.otlp_endpoint` | alias |
| `EHRBASE_OTEL_SERVICE_NAME` | `telemetry.service_name` | alias |
| `EHRBASE_OTEL_ENVIRONMENT` | `telemetry.environment` | alias |
| `EHRBASE_OTEL_TRACES_SAMPLE_RATIO` | `telemetry.traces_sample_ratio` | alias |
| `EHRBASE_OTEL_METRICS_PUSH` | `telemetry.metrics_push` | alias |
| `EHRBASE_REST_CONFIG` | `--config` / `EHRBASE_CONFIG` | **dies** (pointed message: "merge into ehrbase.toml; see the migration guide") |
| `EHRBASE_REST_BIND` | `server.bind` (`EHRBASE_SERVER__BIND`) | alias |
| `EHRBASE_REST_BASE_PATH` | `server.base_path` | alias |
| `EHRBASE_REST_MAX_IN_FLIGHT` | `server.max_in_flight` | alias |
| `EHRBASE_REST_SWAGGER_UI` | `server.swagger_ui` | alias |
| `EHRBASE_REST_CORS_PERMISSIVE` | `server.cors_permissive` | alias |
| `EHRBASE_REST_SYSTEM__*` (5 keys) | `server.identity.*` | alias |
| `EHRBASE_REST_AUTH__ENABLED` | `auth.enabled` (`EHRBASE_AUTH__ENABLED`) | alias |
| `EHRBASE_REST_AUTH__VERIFIED_CACHE_TTL_SECONDS` | `auth.verified_cache_ttl_seconds` | alias |
| `EHRBASE_REST_AUTH__BASIC__USERS` | `[[auth.basic.users]]` (file-only) | **dies** (was never realistically env-settable; the book already says "set via TOML") |
| `EHRBASE_REST_AUTH__OIDC__ISSUER/AUDIENCES/ALGORITHMS/HMAC_SECRET/JWKS_JSON` | `auth.oidc.*` (`EHRBASE_AUTH__OIDC__*`) | alias |
| `EHRBASE_REST_AUTH__ADMIN_SCOPE` | — (subsumed by `authz.rbac.admin_role`, `authn/config.rs:43-49`) | **dies** |
| `EHRBASE_REST_ADMIN__ENABLED` | `admin.enabled` (`EHRBASE_ADMIN__ENABLED`) | alias |
| `EHRBASE_REST_TENANCY__ENABLED/CLAIM/HEADER` | `tenancy.*` (`EHRBASE_TENANCY__*`) | alias |
| `EHRBASE_REST_TERMINOLOGY__ENABLED` | `terminology.api_enabled` | alias |
| `EHRBASE_REST_EVENT_SUBSCRIPTION__ENABLED` | `events.admin_api` | alias |
| `EHRBASE_REST_FHIR__ENABLED` | `fhir.api_enabled` | alias |
| `EHRBASE_REST_SMART__*` (all ~21 keys) | `smart.*` (`EHRBASE_SMART__*`, same tails) | alias |
| `EHRBASE_MANAGEMENT_CONFIG` | — | **dies** |
| `EHRBASE_MANAGEMENT_ENABLED/BASE_PATH/PORT/ACCESS_DEFAULT/PROBES_ENABLED` | `management.*` (`EHRBASE_MANAGEMENT__*`) | alias |
| `EHRBASE_MANAGEMENT_ENDPOINTS_<EP>` (6 keys) | `management.endpoints.<ep>` (`EHRBASE_MANAGEMENT__ENDPOINTS__<EP>`) | alias |
| `EHRBASE_AUTHZ_CONFIG` | — | **dies** |
| `EHRBASE_AUTHZ_RBAC__*` / `EHRBASE_AUTHZ_ABAC__*` (all) | `authz.rbac.*` / `authz.abac.*` (same tails, `EHRBASE_AUTHZ__…`) | alias |
| `EHRBASE_ATNA_CONFIG` | — | **dies** |
| `EHRBASE_ATNA_<KEY>` (15 keys) | `atna.<key>` (`EHRBASE_ATNA__<KEY>`) | alias |
| `EHRBASE_SIGNING_CONFIG` | — | **dies** |
| `EHRBASE_SIGNING_ENABLED/MODE/KEY_PATH/KEY_PASSPHRASE/VERIFY_ON_READ` | `signing.*` (`EHRBASE_SIGNING__*`) | alias |
| `EHRBASE_EVENTS_CONFIG` | — | **dies** |
| `EHRBASE_EVENTS_<KEY>` (9 keys) | `events.<key>` (`EHRBASE_EVENTS__<KEY>`) | alias |
| `EHRBASE_FHIR_OUTBOUND_CONFIG` | — | **dies** |
| `EHRBASE_FHIR_OUTBOUND_<KEY>` (7 keys) | `fhir.outbound.<key>` (`EHRBASE_FHIR__OUTBOUND__<KEY>`) | alias |
| `EHRBASE_MULTIMEDIA_CONFIG` | — | **dies** |
| `EHRBASE_MULTIMEDIA_<KEY>` (8 keys) | `multimedia.<key>` (`EHRBASE_MULTIMEDIA__<KEY>`) | alias |
| `EHRBASE_VALIDATION_CONFIG` | — | **dies** |
| `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_ENABLED/FAIL_ON_ERROR` | `terminology.external.*` | alias |
| `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_PROVIDERS__<N>__<K>` | `terminology.external.providers.<n>.<k>` | alias |
| `EHRBASE_SUBJECT_PROXY_CONFIG` | — | **dies** |
| `EHRBASE_SUBJECT_PROXY__SYSTEMS__<N>__<K>` (the documented-but-broken form, C-3) | `subject_proxy.systems.<n>.<k>` — same spelling, now actually binding | **dies as alias** (it never worked; the new mechanical mapping gives it meaning for the first time — call it out in the changelog) |
| `EHRBASE_QUERY__PLAN_CACHE_CAPACITY` | `query.plan_cache_capacity` (`EHRBASE_QUERY__PLAN_CACHE_CAPACITY` — same spelling, now strict-parsed) | alias-in-place (spelling already matches P-4; the *behaviour* change — parse errors become boot errors — is the migration note) |
| `EHRBASE_QUERY__TIMEOUT_MS` | `query.timeout_ms` — same spelling, strict-parsed | alias-in-place |
| `EHRBASE_HEALTHCHECK_URL` | unchanged (a `healthcheck`-subcommand arg, not server config — `main.rs:48`) | unchanged |
| `EHRBASE_GIT_SHA` / `EHRBASE_BUILD_EPOCH` / `EHRBASE_RUSTC` | build-time only (`build.rs`) | unchanged |
| compose-level `EHRBASE_IMAGE`, `EHRBASE_POSTGRES_IMAGE`, `EHRBASE_PORT`, `EHRBASE_DB_PORT`, `EHRBASE_S3_PORT` | deployment parameterization | unchanged |
| compose-level `EHRBASE_DB_USER/_PASSWORD/_NAME` (postgres **init**, `docker-compose.yml:26-28`) | renamed to `PG_INIT_USER/_PASSWORD/_DB` in the compose files | renamed (they configure the DB container, not the server; the old names now collide with the server's reserved namespace and would trip the strict env sweep) |

Alias mechanics are §5.7. Alias removal target: the second minor release after
the one that ships this redesign.

---

## 5. Mechanics

### 5.1 Library: the `config` crate (and remove `figment` entirely)

**Decision: `config` 0.15.25** (already pinned in `Cargo.toml:176`; owner
ruling 2026-07-15). Reasons:

1. **Maintenance.** `figment` 0.10 has not shipped a release in ~2 years;
   `config` (rust-cli org) is actively maintained. A clinical server's config
   loader is security-relevant surface — it sits on a maintained crate.
2. `Environment::with_prefix("EHRBASE").separator("__").try_parsing(true)`
   implements P-4's mapping in one source; `File::...required(false)` gives
   the search-order semantics of §5.4; `set_override` implements `--set`.
3. **Hermetic testability without process-global env:** the `Environment`
   source accepts an injected map
   (`Environment::default().source(Some(map.into()))`), so the whole test
   plan (§6.6) runs on pure inputs — strictly better than `figment::Jail`'s
   serialized env mutation, and it shapes the loader as a pure function
   (§5.2).
4. Since this redesign deletes all twelve figment chains anyway (§5.2), there
   is no incumbent-code advantage to figment: after the migration **zero**
   uses remain, so `figment` is removed from `[workspace.dependencies]`
   (`Cargo.toml:177`) and from every crate manifest, including the
   `features = ["test"]` dev-dependency in `app/ehrbase/Cargo.toml:92` (the
   existing `Jail` tests are ported per §6.6). The `Cargo.toml:176` comment
   "or figment" is thereby resolved.

What figment provided that `config` does not, and how the design covers it:
per-value provenance metadata is weaker in `config` — but the strict pass
(§5.3) already builds its own span index from the `toml` crate
(`Cargo.toml:178`, already a dep) for file:line reporting, and env-sourced
values are attributed by reconstructing the P-4 env name from the key path
(a pure function, by construction). Nothing else is lost.

### 5.2 The config tree and its home across the three app crates

One root struct:

```rust
// app/ehrbase/src/config.rs  (new module; the `ehrbase` crate is the binary
// + Platform impl and already depends on ehrbase-rest and ehrbase-sm —
// app/ehrbase/Cargo.toml:88-89 — so the dependency arrows hold.)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EhrbaseConfig {
    pub server: ehrbase_rest::config::ServerConfig,      // §3.1 (renamed RestConfig, minus the sections that moved out)
    pub db: crate::db::DbConfig,                          // §3.2 (renamed DbSettings)
    pub log: crate::telemetry::LogConfig,                 // §3.3
    pub telemetry: crate::telemetry::OtelConfig,          // §3.4
    pub auth: ehrbase_rest::access::authn::AuthConfig,    // §3.5
    pub authz: ehrbase_rest::access::authz::AuthzConfig,  // §3.6
    pub admin: ehrbase_rest::config::AdminConfig,         // §3.7
    pub tenancy: ehrbase_rest::config::TenancyConfig,     // §3.8
    pub smart: ehrbase_rest::smart::SmartConfig,          // §3.9
    pub management: ehrbase_rest::management::ManagementConfig, // §3.10
    pub signing: crate::versioning::signature::SigningConfig,  // §3.11
    pub query: crate::service::query::QueryConfig,        // §3.12 (new)
    pub events: crate::extensions::events::EventsConfig,  // §3.13 (gains `admin_api`)
    pub fhir: crate::extensions::fhir::FhirConfig,        // §3.14 (api_enabled + outbound)
    pub terminology: crate::service::terminology::TerminologyConfig, // §3.15
    pub multimedia: crate::extensions::multimedia::MultimediaConfig,
    pub atna: crate::system_log::AuditConfig,
    pub subject_proxy: crate::service::subject_proxy::SubjectProxyConfig,
}
```

Ownership rules:

- **Each section struct stays in (or moves to) the crate that consumes it** —
  `ehrbase-rest` keeps the HTTP/auth/authz/management/smart/tenancy types;
  `ehrbase` keeps db/telemetry/signing/query/events/fhir-outbound/
  multimedia/atna/terminology/subject-proxy. `ehrbase-sm` contributes nothing
  (it reads no config, §1.1).
- Every section struct: `#[serde(default, deny_unknown_fields)]`, a `Default`
  impl that is the single source of defaults (C-8 is resolved by the
  template-sync test, §5.5), and **no `load()` associated function** — all
  twelve per-struct `load()`s are deleted. Subsystem constructors take the
  typed section by value/ref (e.g. `QueryService` takes `QueryConfig`,
  killing the LazyLock env reads of C-4).
- The binary calls exactly one loader:

```rust
let config = ehrbase::config::load(&cli)?;   // source assembly, §5.4
config.validate()?;                          // aggregated, all errors at once, §5.8
```

`load` is a thin process-environment shim over the pure assembly function the
tests exercise directly (§5.1 point 3):

```rust
pub fn assemble(
    file: Option<&Path>,                  // resolved per §5.4 discovery
    env: &HashMap<String, String>,        // the EHRBASE_* namespace snapshot
    overrides: &[(String, String)],       // --set pairs
) -> Result<EhrbaseConfig, ConfigErrors>
```

- `serve()` (`main.rs:81`) then distributes sections; the ad-hoc
  `env_snapshot()` (`main.rs:310-322`) is replaced by serializing the whole
  redacted tree (§5.6), so `/management/env` finally reports *all*
  configuration (C-7/C-9).

### 5.3 Strict validation: unknown keys, did-you-mean, file:line

Three passes, in order, all before any subsystem starts:

1. **File pass (span-aware).** The config file is parsed **twice**: once by
   the `toml` crate into a span-carrying document to drive strictness, once by
   the `config` builder for the merge. The strict walker recursively compares the parsed
   document's key paths against the schema's known-key tree and reports every
   unknown key with its `file:line:column` (the `toml` crate's `Spanned`/error
   machinery provides spans) plus a did-you-mean when the best candidate is
   close. Known-key tree source: `deny_unknown_fields` alone rejects but
   cannot suggest, so the walker gets the key tree from serializing
   `EhrbaseConfig::default()` to a `toml::Value` (fields = keys; map-valued
   nodes — `providers.*`, `systems.*`, `policy.*`, `basic.users` — are
   declared wildcard in a small static list). Suggestion metric: a ~20-line
   hand-rolled Damerau-Levenshtein (no new dependency), threshold ≤ 2, ties
   broken lexicographically.
2. **Env pass.** Every environment variable starting with `EHRBASE_` (the
   reserved namespace) must map to a known key after the P-4 mapping, be a
   registered alias (§5.7), or be on the small allowlist of non-config names
   (`EHRBASE_CONFIG`, `EHRBASE_HEALTHCHECK_URL`, the three build-time vars).
   Anything else is a boot error with a did-you-mean — this is what makes
   C-3/C-5-class bugs (a set-but-never-read env var) impossible.
3. **Type pass.** `try_deserialize::<EhrbaseConfig>()` on the built config,
   with enriched error rendering: a type error names the key path, the
   expected type, and the provenance (file:line via the span index from pass
   1 when the value came from the file; the reconstructed `EHRBASE_*` env
   name when it came from the environment — §5.1).

Boot collects the failures from all three passes and prints them as one
block; exit code 1. `ehrbase config check` runs the identical three passes.

### 5.4 File discovery and the CLI

Precedence for *which* file (first hit wins; later layers still override its
*values* per P-3):

1. `--config <path>` (boot error if missing/unreadable),
2. `EHRBASE_CONFIG=<path>` (boot error if missing/unreadable),
3. `./ehrbase.toml` (cwd),
4. `/etc/ehrbase/ehrbase.toml`.

No file found ⇒ defaults + env + flags (P-2). Mechanically: search-order
positions 3–4 load as `File::...required(false)` (absent is fine, unparseable
is fatal), while an explicitly-pointed-at file (positions 1–2) loads as
`required(true)` — missing or unreadable is always fatal, never silently
skipped.

CLI surface (clap, extends the existing `Cli` in `main.rs:34-51`):

```
ehrbase [--config <path>] [--set <key>=<value>]...   # serve
ehrbase config check [--config <path>] [--set ...]   # 3-pass validate + print effective config (redacted TOML) + provenance column; exit 0/1
ehrbase config default                                # emit the annotated default template to stdout
ehrbase healthcheck [--url ...]                       # unchanged
```

`--set` is a repeatable dotted-path override (`--set db.max_connections=40`),
the highest layer (P-3), applied via the builder's `set_override` with the
same value conventions as env (typed scalars; comma-separated lists for
list-typed keys, P-4). Decision: `--set` instead of one bespoke flag per
key — the schema is ~130 keys; per-key flags would recreate the sprawl this
redesign removes.

### 5.5 Defaults encoding and the annotated template

Defaults live **once**, in the Rust `Default` impls (+ field
`#[serde(default = ...)]` fns as today). No defaults *source* is layered into
the builder at all: every section carries `#[serde(default)]`, so
`try_deserialize` fills absent keys from the `Default` impls directly —
partial files/env extract cleanly, and C-10's outlier class (a loader
forgetting its defaults provider) cannot recur because there is nothing to
forget.

The annotated default file (`ehrbase config default`) is a **hand-maintained
static asset** — `app/ehrbase/assets/ehrbase.default.toml`, embedded with
`include_str!` — because doc-comment-quality annotations (`# PROD:` markers,
PHI warnings) cannot be generated from serde. Two tests keep it honest
(§6.6): (a) parsing the template through the real loader must yield exactly
`EhrbaseConfig::default()`; (b) the template must contain every key path of
the schema tree (commented-out optional tables included via a
`#? key = value` convention the test understands).

### 5.6 Secrets

One shared newtype in a small `ehrbase-config-types` location — decision: put
it in `ehrbase-sm` (the shared bottom app crate both `ehrbase` and
`ehrbase-rest` already depend on) rather than a new crate:

- `Secret` — wraps `secrecy::SecretString`; `Deserialize` from a TOML string;
  `Serialize` always renders `"***"`; `Debug` prints `"***"`. Replaces both
  the local `Redacted` (`authn/config.rs:10-18`) and the ad-hoc
  `SecretString` field handling (`signature/config.rs:70`) — and
  `SigningConfig` can then derive `Serialize` and join the snapshot.
- `SecretUrl` — for `db.url`, `events.url`, `fhir.outbound.url`: parses the
  URL, keeps the full form for connection use (exposed only via an
  `expose()` method), and `Serialize`/`Display` render with userinfo replaced
  by `***` (`postgres://***@host:5432/ehrbase`). Fixes C-7 for the broker
  URLs and centralizes what the management redactor does ad-hoc today.
- Every `Secret` field has a `*_file` sibling (§3); validation rejects both
  set at once, and the loader resolves `_file` (read, trim trailing newline)
  into the `Secret` immediately after extraction so consumers see one field.

`/management/env`, `config check`, and any `Debug` of the tree therefore
redact by construction, not by a per-endpoint redactor list.

### 5.7 Alias layer (one transition release)

A static table `&[(old_env_name, new_key_path)]` (exactly the §4 "alias"
rows). Before building the config, `assemble` sweeps the table; for each set
old var it (a) emits
`warn!("EHRBASE_DB_MAX_CONNECTIONS is deprecated; use EHRBASE_DB__MAX_CONNECTIONS — removed in 3.x+2")`
and (b) collects `new_key_path → value` into a map added as an injected
`Environment` source (`.source(Some(map.into()))`) **before** the real
`EHRBASE_`-prefixed source in the builder — `config` sources layer in add
order, later wins, so if both old and new are set, the new wins silently. The "dies" rows are a second static table checked in §5.3's
env pass, each with its bespoke message (the nine `*_CONFIG` pointers all
say: merge that file's contents into `ehrbase.toml` under `[<section>]`).
Removing the alias release later = deleting the first table; the strict env
sweep then catches stragglers automatically.

### 5.8 Aggregated semantic validation

`EhrbaseConfig::validate() -> Result<(), ConfigErrors>` where `ConfigErrors`
is a non-empty `Vec` of typed errors rendered as one block. Contents: the
existing `AuthzConfigError` rules (moved verbatim), the SMART
deprecated-grant rule, plus the new cross-field rules — `signing.mode=pgp` ⇒
`key_path` set; `secret`/`*_file` mutual exclusion (§5.6);
`management.port` ≠ `server.bind` port (§3.10); `terminology.external.enabled`
⇒ at least one provider; `auth.enabled && !has_mechanism()` ⇒ the §3.16
boot warning (a warning, not an error — the fail-closed 401 posture is
legitimate for probe-only smoke environments). The actual key/passphrase
*usability* checks stay where they are (fail-closed at subsystem init,
`main.rs:176-180`) — `validate()` is shape+consistency, not I/O.

---

## 6. Migration and documentation plan (same PR as the implementation)

### 6.1 `docker-compose.yml` + `docker/ehrbase.dev.toml`

- `docker/ehrbase.dev.toml` becomes a **full** `ehrbase.toml` (same content
  reorganized under the new sections: `[server] cors_permissive`, `[admin]`,
  `[auth]` + the two dev users), still mounted at
  `/etc/ehrbase/ehrbase.toml` — which is now search-order position 4, so the
  `EHRBASE_REST_CONFIG` env line (`docker-compose.yml:79`) is deleted rather
  than replaced.
- The service env block migrates to the new spellings:
  `EHRBASE_DB__URL`, `EHRBASE_DB__MAX_CONNECTIONS`,
  `EHRBASE_SERVER__MAX_IN_FLIGHT`, `EHRBASE_SIGNING__ENABLED`,
  `EHRBASE_LOG__FORMAT`, `EHRBASE_LOG__FILTER`.
- The postgres-**init** trio is renamed `PG_INIT_USER/_PASSWORD/_DB`
  (`docker-compose.yml:26-28,50,65` + `docker/postgres` init script) so the
  compose namespace no longer squats on server-config names (§4, last row).
- The commented SeaweedFS/Keycloak recipes (`docker-compose.yml:98-137`)
  update to the new spellings (`EHRBASE_MULTIMEDIA__ENABLED`,
  `[auth.oidc]`-in-TOML pointer text).

### 6.2 Helm chart (`deploy/helm/ehrbase-rs/`)

- The chart renders **one ConfigMap key `ehrbase.toml`** from a new
  `values.yaml` `config:` block (typed values mapped 1:1 onto §3 sections),
  mounted at `/etc/ehrbase/ehrbase.toml`; the existing per-feature env
  rendering in `templates/deployment.yaml`/`configmap.yaml` collapses to:
  secrets as env (`EHRBASE_DB__URL`, `EHRBASE_AUTH__OIDC__HMAC_SECRET`,
  `EHRBASE_SIGNING__KEY_PASSPHRASE`, `EHRBASE_MULTIMEDIA__SECRET_ACCESS_KEY`,
  broker URLs — same secretKeyRef pattern, new names) + `extraEnv` as the
  escape hatch. The checksum/config pod-roll annotation
  (`deployment.yaml:18`) keeps working unchanged.
- `config.files` (`values.yaml:345-355`) stays for genuinely file-shaped
  material (Cedar policies, ATNA PEMs, the PGP key, JWKS) — pointed at by the
  in-TOML `*_path`/`*_file` keys instead of `EHRBASE_*_CONFIG` vars.
- Golden renders (`deploy/helm/golden/*.yaml`) and the two CI value sets
  regenerate via `deploy/helm/validate.sh --update`.

### 6.3 Scripts and tools

- `scripts/benchmark.sh`, `scripts/conformance.sh`, `scripts/profile.sh`,
  `docker/benchmark/docker-compose.yml`: update the four exported knobs found
  by the sweep (`EHRBASE_DB_MAX_CONNECTIONS`, `EHRBASE_REST_MAX_IN_FLIGHT`,
  `EHRBASE_SIGNING_ENABLED`, `EHRBASE_LOG_FILTER`) to the new spellings.
  `CONF_*`/`BENCH_*` tool vars are untouched.
- `docker/smoke-test.sh` and CI workflows: sweep for the same four + any
  `EHRBASE_REST_CONFIG` reference.
- An ECC run (`scripts/conformance.sh`) must show **zero drift** after the
  migration — the config surface is wire-invisible when values are unchanged,
  and the run proves it.

### 6.4 Book page rewrite (`website/book/src/installation/configuration.md`)

Rewritten around the file: (1) quickstart — `ehrbase config default > ehrbase.toml`,
edit, run; (2) the layering + P-4 mapping stated **once** (the C-1 warning
box about two conventions is deleted); (3) one section per §3 table (the
schema section of this document is the source); (4) the §4 old→new table as a
"migrating from 3.x env vars" appendix; (5) `config check`/`config default`
reference; (6) the production checklist (§3.16). Cross-page sweeps:
`installation/docker.md`, `installation/helm.md`, `smart-app-launch.md`,
`beyond-core/subject-proxy.md` and every page that spells an `EHRBASE_*` var
(grep the book tree).

### 6.5 Changelog

`CHANGELOG.md` `[Unreleased]`: **Changed** — configuration is now one
`ehrbase.toml` + mechanically-mapped `EHRBASE_*` env overrides; every old var
aliased for this release with boot warnings (table in the book). **Removed** —
the nine per-subsystem `EHRBASE_*_CONFIG` file pointers,
`EHRBASE_REST_AUTH__ADMIN_SCOPE`. **Fixed** — unknown/misspelled config now
rejected at boot (was silently ignored); the documented
`EHRBASE_SUBJECT_PROXY__SYSTEMS__*` env form now actually binds; unparseable
`[query]` values now error instead of silently using defaults.

### 6.6 Test plan

No test touches the process environment: everything drives the pure
`assemble(file, env_map, overrides)` function (§5.2) with injected env maps
and `tempfile`-based config files (the `config` crate's
`Environment::default().source(Some(map.into()))` is the seam, §5.1). The
existing `figment::Jail` tests across the twelve old loaders are **ported,
never deleted** — each Jail case maps 1:1 onto an `assemble` case asserting
the same key under its new name (the alias tests additionally assert the old
name still binds for the transition release):

1. **Layering:** defaults-only boot (P-2); file overrides defaults; env
   overrides file; `--set` overrides env; `DATABASE_URL`/`RUST_LOG` lose to
   their `EHRBASE_` forms.
2. **Mapping:** one test per section proving the P-4 env spelling binds
   (this is the test class whose absence let C-3 ship) — including a
   map-valued path (`EHRBASE_SUBJECT_PROXY__SYSTEMS__PAS__BASE_URL`) and a
   list value.
3. **Strictness:** unknown file key → error carrying `file:line` + the
   expected did-you-mean; unknown `EHRBASE_*` env var → error + suggestion;
   allowlist names pass; type error carries provenance; multiple errors
   reported together.
4. **Aliases:** old spelling binds + warns; new beats old when both set;
   each "dies" var produces its bespoke message.
5. **Secrets:** `Secret`/`SecretUrl` never appear in `Debug`, in
   `to_string_pretty` of the tree, in `config check` output, or in
   `/management/env` (an integration test on the management router);
   `*_file` resolution; both-set rejection.
6. **Template sync:** the two §5.5 assertions.
7. **Validation:** the §5.8 rules (ports collide, pgp-without-key,
   provider-less external terminology, existing authz/smart suites ported).
8. **CLI:** `config check` exit codes; `config default` output parses and
   round-trips; missing `--config` path is fatal while missing search-order
   files are not.
9. **End-to-end:** the compose stack boots with the migrated
   `ehrbase.dev.toml` and the ECC zero-drift run (§6.3).

### 6.7 Implementation sequencing (one PR, reviewable commits)

1. Shared types (`Secret`, `SecretUrl`) + the root `EhrbaseConfig` skeleton
   and single loader behind the existing structs (no callers change yet).
2. Move/rename sections into the tree; delete the twelve `load()`s; plumb
   `QueryConfig`; rewire `main.rs` to one load + one validate + distribute.
3. Strict passes + did-you-mean + aliases + `config` subcommands.
4. Template asset + sync tests + full test plan.
5. Deployment migration (§6.1–6.3) + book rewrite (§6.4) + changelog +
   remove the `figment` pin from `[workspace.dependencies]` and every crate
   manifest (C-11, §5.1) — after this commit `grep -r figment` over the
   workspace manifests must be empty.
6. `cargo nextest run --workspace`, clippy, ECC zero-drift.

---

## 7. Explicitly out of scope

- **Hot reload.** Rejected as not-cheap: config is distributed at boot into
  constructed subsystems (pools, listeners, background drainers, moka caches,
  the tower middleware stack); reloading safely means re-plumbing every
  consumer for interior mutability or restart-on-change supervision. The two
  knobs where runtime change genuinely matters already have dedicated runtime
  surfaces (`/management/loggers` for the log filter;
  `authz.abac.cedar.reload_secs` for Cedar policies). Everything else is a
  process restart — which Kubernetes/compose already orchestrate gracefully
  (the chart's checksum/config annotation rolls pods on config change,
  §6.2).
- **Remote config** (Consul/etcd/Spring-Cloud-style config servers): the env
  override layer is the deployment-injection seam; a config server would add
  a runtime dependency to a clinical system's boot path for no requirement on
  file.
- **Profiles** (`[dev]`/`[prod]` profile switching inside one file): one
  schema + the §3.16 checklist is simpler; environment differences are
  exactly what the env/CLI layers are for.
- Tool configuration (`tools/conformance` `CONF_*`, `tools/benchmark`
  `BENCH_*`) and compose/Helm infrastructure parameterization (image tags,
  host ports) — different audiences, deliberately separate namespaces.
