# Installation

FerroEHR is a single static binary that connects to a PostgreSQL 18 database.
There is no application server to install and no runtime to provision — you
choose how to run the binary and where the database lives. This part covers the
three supported paths and the full configuration surface.

- **[Docker Compose](compose.md)** — the fastest way to run the server plus a
  preconfigured PostgreSQL 18, for development and evaluation: one downloadable
  file that pulls the published images and needs no configuration. Optional
  admin console, OIDC, and observability overlays.
- **[Kubernetes & Helm](kubernetes.md)** — the production path: a hardened,
  non-root, default-deny workload that connects to an externally managed
  PostgreSQL 18.
- **[From source](from-source.md)** — building the binary yourself with the
  pinned Rust toolchain.
- **[Configuration reference](configuration.md)** — every `FERROEHR_*`
  environment variable, grouped by area, with types, defaults, and meaning.

Whichever path you take, the server is configured entirely through `FERROEHR_*`
environment variables (with optional mounted TOML files for values that do not
fit cleanly in env, such as a Basic-auth user store or an OIDC block). The
database schema is created and updated by the binary's migrations, which run
automatically at boot.

> [!NOTE]
> A Clinical Data Repository stores PHI. In production the database must be an
> externally managed, backed-up, point-in-time-recoverable PostgreSQL 18 — never
> a throwaway sidecar. The Kubernetes chart deliberately ships **no** in-cluster
> database for this reason.
