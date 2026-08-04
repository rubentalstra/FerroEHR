# From source

You can build the `ferroehr` binary yourself — for a platform without a
published image, for local development, or to run the test suite. This chapter
covers the prerequisites and the build. Most operators should prefer the
published container images ([Docker Compose](compose.md),
[Kubernetes & Helm](kubernetes.md)); build from source when you need to.

## Prerequisites

- **The pinned Rust toolchain.** The repository pins Rust **1.96.1** (edition
  2024) via `rust-toolchain.toml`, so `rustup` installs and selects it
  automatically the first time you build in the checkout — you do not choose a
  version by hand.
- **Docker** — required only for the integration tests, which spin up a real
  PostgreSQL 18 in a container.
- **`xmllint`** — required only for the canonical-XML tests.

## Building

From the repository root:

```shell
cargo build --workspace
```

To build just the server binary in release mode (what the container image
ships):

```shell
cargo build --release --locked -p ferroehr
```

The resulting binary is `target/release/ferroehr`. It is statically linked
against a pure-Rust TLS stack — no OpenSSL, no JVM, no runtime dependencies —
so it drops into a minimal base image or runs directly on the host.

## Running the tests

```shell
cargo nextest run --workspace
```

The suite includes integration tests that start PostgreSQL 18 via
`testcontainers`, so Docker must be running.

## Running the binary

The binary is configured entirely through `FERROEHR_*` environment variables
(see the [configuration reference](configuration.md)). At minimum it needs a
database URL:

```shell
export FERROEHR__DB__URL='postgres://ferroehr:ferroehr@localhost:5432/ferroehr'
target/release/ferroehr
```

It runs its schema migrations at boot and then serves on the configured bind
address (default `0.0.0.0:8080`). The binary also has a `healthcheck`
subcommand (used by the container healthcheck and Kubernetes exec probes) that
hits the status endpoint and exits 0 or 1.

> [!NOTE]
> Building from source gives you the same binary the images use — the container
> Dockerfile pins its Rust version from the same `rust-toolchain.toml`, and CI
> cross-checks the two so they cannot drift.

## Build features

The server builds with three additive cargo features, all **on by default**:
`fhir` (the FHIR connector, outbound emitter, FHIR terminology providers, and
the FHIR `AuditEvent` audit sinks), `events` (contribution-outbox eventing
and the AMQP transport), and `multimedia` (`DV_MULTIMEDIA` externalization
to S3-compatible object storage).

A slim build compiles them out entirely:

```bash
cargo build --release --no-default-features
```

A slim binary refuses loudly at boot if the configuration enables an
integration it was built without — `multimedia.enabled`, `events.enabled`,
`fhir.outbound.enabled`, `audit.store.enabled`, `audit.fhir_feed.enabled`,
or a configured external FHIR terminology provider. The syslog ATNA feed and
the in-process terminology bundle remain available in slim builds.
