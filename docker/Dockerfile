# syntax=docker/dockerfile:1
#
# Multi-stage build for the `ehrbase` server binary (ADR-006/008).
#
#   build context : the repository root  (`docker build -f docker/Dockerfile .`)
#   builder       : rust:${RUST_VERSION}-slim-bookworm + cargo-chef dependency
#                   layer caching → `cargo build --release --locked -p ehrbase`
#   runtime       : gcr.io/distroless/cc-debian12:nonroot (shell-less, non-root)
#
# The toolchain is single-sourced from rust-toolchain.toml; RUST_VERSION must
# equal its `channel` (CI cross-checks the two so they never drift).

ARG RUST_VERSION=1.96.1

# ── chef: rust base with cargo-chef installed ───────────────────────────────
FROM rust:${RUST_VERSION}-slim-bookworm AS chef
WORKDIR /app
RUN cargo install cargo-chef --locked
# The binary is pure-Rust (rustls, no OpenSSL); cargo-chef cook/build need no
# extra system libraries beyond the slim base.

# ── planner: distil the dependency graph into a recipe ──────────────────────
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ── builder: cook deps (cached), then build the workspace binary ────────────
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# This layer is cached and only re-runs when the dependency set changes.
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --locked -p ehrbase \
    && strip target/release/ehrbase

# ── runtime: distroless cc (glibc + CA certs), non-root ─────────────────────
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
COPY --from=builder /app/target/release/ehrbase /usr/local/bin/ehrbase

EXPOSE 8080

# Shell-less image: the healthcheck is the binary's own subcommand, which
# probes /rest/status and exits 0 (2xx) / 1 otherwise.
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD ["/usr/local/bin/ehrbase", "healthcheck"]

ENTRYPOINT ["/usr/local/bin/ehrbase"]

# OCI image labels. CI (docker/metadata-action) overrides source/revision/
# version/created at build time; these are sensible static defaults.
ARG VERSION="0.1.0"
ARG REVISION=""
LABEL org.opencontainers.image.title="ehrbase-rs" \
      org.opencontainers.image.description="Pure-Rust, openEHR-conformant clinical data repository (ITS-REST 1.0.3 + AQL 1.1)" \
      org.opencontainers.image.source="https://github.com/rubentalstra/ehrbase-rs" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${REVISION}"
