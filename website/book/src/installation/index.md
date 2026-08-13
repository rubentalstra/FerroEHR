# Installation

FerroEHR is one self-contained binary that connects to a PostgreSQL 18
database. There is no application server to install and no language runtime to
provision — the binary links a pure-Rust TLS stack and needs no OpenSSL and no
JVM, so all you choose is how to run it and where the database lives. This part
covers the three paths and the full configuration surface.

- **[Docker Compose](compose.md)** — the fastest way to run the server plus a
  preconfigured PostgreSQL 18, for development and evaluation: one downloadable
  file that pulls the published images and needs no configuration. Optional
  admin console, OIDC, and observability overlays.
- **[Kubernetes & Helm](kubernetes.md)** — the production path: a hardened,
  non-root workload that connects to an externally managed PostgreSQL 18. Its
  companion, [Cluster hardening](kubernetes-hardening.md), covers what the
  chart cannot do for you.
- **[From source](from-source.md)** — building the binary yourself with the
  pinned Rust toolchain, and running the test suite.
- **[Configuration reference](configuration.md)** — how configuration loads and
  what every key means, split by area: [server, database and
  telemetry](config-server.md), [authentication and access](config-auth.md),
  [integrations](config-integrations.md), [audit and subject
  proxy](config-audit.md), and the [CLI and production
  checklist](config-cli.md).

## What every path has in common

**One configuration file, with environment overrides on top.** The server reads
a single `ferroehr.toml` covering every subsystem; `FERROEHR__*` environment
variables override individual keys, and repeatable `--set key=value` flags
override those. Anything the environment grammar cannot spell — the Basic-auth
user store, which is an array of tables — is file-only. `ferroehr config
default` writes an annotated template with every key at its default, and
`ferroehr config check` validates the result without touching the database.

**The schema is the binary's, and who applies it is a choice.** By default
(`db.migrate = "apply"`) the server applies its embedded migrations at boot, so
an empty database self-provisions. Setting `db.migrate = "verify"` makes the
server issue no DDL at all — it checks the schema and refuses to start if it is
not this build's — which lets the runtime role hold no DDL rights. Something
else then runs `ferroehr db migrate` under a migrator role first. Both postures
are laid out in [Operations → Applying migrations](../operations.md#applying-migrations).

## Choosing a specification generation

One top-level key, **`spec_profile`**, selects which openEHR specification
generation set the deployment runs — `development` (the default: Reference
Model 1.2.0 with BASE 1.3.0 and LANG 1.1.0) or `stable` (the latest released
generations: Reference Model 1.1.0 with BASE 1.2.0 and LANG 1.0.0). It is a
single coupled choice, and `stable → development` is always safe while the
reverse is not, so decide it before you commit clinical data. The full
semantics, defaults, and the direction contract are in
[`spec_profile`](configuration.md#spec_profile).

> [!NOTE]
> A Clinical Data Repository stores PHI. In production the database must be an
> externally managed, backed-up, point-in-time-recoverable PostgreSQL 18 — never
> a throwaway sidecar. The Kubernetes chart deliberately ships **no** in-cluster
> database for this reason.
