# syntax=docker/dockerfile:1
#
# From-source multi-stage build for the `ehrbase` server binary (ADR-006/008).
# This is the LOCAL/compose path (`docker compose build`, `docker build -f
# docker/Dockerfile .`); CI packages prebuilt native binaries with
# docker/Dockerfile.runtime instead (see .github/workflows/containers.yml).
#
#   builder : rust:${RUST_VERSION}-slim-bookworm, single `cargo build` with
#             BuildKit cache mounts for the registry and target dir — repeat
#             local builds are incremental, and nothing (cargo-chef, a second
#             dependency compile) runs besides the one real build.
#   runtime : gcr.io/distroless/cc-debian12:nonroot (shell-less, non-root;
#             glibc 2.36 matches the bookworm builder).
#
# The toolchain is single-sourced from rust-toolchain.toml; RUST_VERSION must
# equal its `channel` (CI cross-checks the two so they never drift).

ARG RUST_VERSION=1.96.1

# ── builder: one cached compile ──────────────────────────────────────────────
FROM rust:${RUST_VERSION}-slim-bookworm AS builder
WORKDIR /app
COPY . .
# The binary is pure-Rust (rustls, no OpenSSL); no system libraries are needed
# beyond the slim base. Cache mounts persist the crates.io registry and the
# incremental target dir across builds; the binary is copied out of the mount
# before it unmounts.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo build --release --locked -p ehrbase-server \
    && cp target/release/ehrbase /usr/local/bin/ehrbase \
    && strip /usr/local/bin/ehrbase

# ── runtime: distroless cc (glibc + CA certs), non-root ──────────────────────
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
COPY --from=builder /usr/local/bin/ehrbase /usr/local/bin/ehrbase

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
