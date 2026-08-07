# syntax=docker/dockerfile:1
#
# The single Dockerfile for the `ferroehr` server image (issue #282 D1/D5).
# ONE runtime stage feeds TWO final targets so a compose-built image and a
# CI-published image can never drift (docs.docker.com/build/building/multi-stage):
#
#   runtime-from-source  the LOCAL/compose path — cargo-chef + cache mounts
#                         compile the binary in-image.
#   runtime-prebuilt     the CI path — COPY-only packaging of a context-provided
#                         native binary (bin/${TARGETARCH}/ferroehr); no RUN, so a
#                         multi-arch buildx needs no QEMU and finishes in seconds.
#
# Caching (D1, the F2/F3 fix): cargo-chef splits the dependency compile into its
# own layer keyed ONLY on the lockfile/recipe, so an app-only edit never
# recompiles dependencies, and — unlike a bare `--mount=type=cache` — that layer
# is a real image layer CI can export/import via the GHA backend
# (docs.docker.com/build/cache/optimize; docs.docker.com/guides/rust).
#
# The RUST_VERSION toolchain is single-sourced from rust-toolchain.toml; CI
# cross-checks the ARG against its `channel` so they never drift.

ARG RUST_VERSION=1.96.1
ARG CARGO_CHEF_VERSION=0.1.77

# ── chef: the builder toolchain + cargo-chef, a cached base for planner/builder ─
# Digest-pinned rust:1.96.1-slim-trixie (Debian 13, glibc 2.41), resolved
# 2026-07-24. The builder and the cc-debian13 runtime below share the Debian 13
# generation so the binary links the exact glibc it runs against. Pinning by
# digest (not the mutable tag) guarantees a reproducible base
# (docs.docker.com/build/building/best-practices — "pinning to a digest
# guarantees the same image version").
FROM rust:${RUST_VERSION}-slim-trixie@sha256:31ee7fc65186be7e0e0ccb3f2ca305f14e4739e7642a1ae65753aa5d7b874523 AS chef
ARG CARGO_CHEF_VERSION
# cargo-chef pinned by version, installed behind a cargo-registry cache mount so
# the download is not repeated on rebuilds (docs.docker.com/build/cache/optimize
# — RUN --mount=type=cache). This layer rebuilds only when RUST_VERSION or
# CARGO_CHEF_VERSION changes.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    cargo install cargo-chef --version ${CARGO_CHEF_VERSION} --locked
WORKDIR /app

# ── planner: distil the dependency recipe from the workspace manifests ─────────
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ── builder: cook dependencies (cached layer), then build the app ──────────────
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Cook ONLY the dependency graph of the server binary. This writes the compiled
# dependencies into the image layer (NOT a target cache mount — a mount would
# not export via the GHA backend and would shadow the cooked target on the next
# stage), so the layer is cache-hit until Cargo.lock/recipe.json changes and CI
# can reuse it cold (docs.docker.com/guides/rust; docs.docker.com/build/cache/backends).
# The registry cache mount is safe: it holds downloaded crate sources, never
# shadows /app/target, and only speeds a cache-missed cook.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    cargo chef cook --release --locked -p ferroehr-server --recipe-path recipe.json
COPY . .
# Build provenance: the OCI-standard REVISION build-arg (the git SHA CI's
# metadata-action already sets for org.opencontainers.image.revision) is the
# ONLY provenance input — no bespoke arg. Exposed as an ENV so build.rs reads it
# and stamps /management/info with the same SHA the image LABEL carries. Absent,
# build.rs degrades to a version-only identity with SHA=unknown BY DESIGN (never
# a broken build; .git is not in the context). Declaring it here (after
# `chef cook`) keeps the cooked dependency layer above untouched by an SHA change
# (docs.docker.com/reference/dockerfile — ARG/ENV scoping).
ARG REVISION=""
ENV REVISION=${REVISION}
# The binary is pure-Rust (rustls, no OpenSSL); the slim base needs no extra
# system libraries. Deps are already compiled by `chef cook`, so this step only
# builds the workspace's own crates.
# --jobs 2: the generated spec crates carry every emitted generation
# (#1936/#1942), and two of those rustc invocations in parallel with the app
# crates exceed a default Docker VM's memory at opt-level 3 (observed: SIGKILL
# compiling openehr-its in a 7.7 GB VM, 2026-08-05). Capping build parallelism
# changes no shipped byte — it only serializes compilation.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    cargo build --release --locked --jobs 2 -p ferroehr-server \
    && cp target/release/ferroehr /usr/local/bin/ferroehr \
    && strip /usr/local/bin/ferroehr

# ── runtime-base: the SHARED final base (digest-pinned distroless, non-root) ───
# gcr.io/distroless/cc-debian13:nonroot (glibc + libgcc_s + CA certs; non-root
# 65532, Helm-aligned), digest-pinned, resolved 2026-07-24. `cc` (not the
# smaller base-nossl) is REQUIRED: the Rust binary dynamically links
# libgcc_s.so.1 for stack unwinding, which the base-nossl image does not ship
# (verified 2026-07-24: base-nossl fails at load with "libgcc_s.so.1: cannot
# open shared object file"); the ~11 MB base-nossl would save is unusable.
# Both final targets inherit this stage's EXPOSE/HEALTHCHECK/ENTRYPOINT/LABELs,
# so they are byte-for-byte identical except the one binary-provenance layer
# (D5 — zero drift).
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:d97bc0a941b8d4be647dc0ee75b264ddbb772f1ac5ba690a4309c00723b23775 AS runtime-base
# The `:nonroot` tag already runs as uid 65532, but a scanner reading the
# Dockerfile cannot see a base image.s user — and neither can a reader. Stating
# it makes the posture a property of this file rather than of a tag someone
# could change (OWASP Docker Security Cheat Sheet rule 2).
#
# NUMERIC, not `nonroot:nonroot`: the kubelet cannot resolve a username against
# an image it does not read, so `runAsNonRoot: true` with a named USER refuses
# the pod with "cannot verify user is non-root" (hadolint DL3066). 65532 is what
# the distroless `:nonroot` tag sets, so this states the base uid rather than
# choosing one.
USER 65532:65532
EXPOSE 8080
# Shell-less image: the healthcheck is the binary's own subcommand, which probes
# /rest/status and exits 0 (2xx) / 1 otherwise.
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD ["/usr/local/bin/ferroehr", "healthcheck"]
ENTRYPOINT ["/usr/local/bin/ferroehr"]
# OCI image labels. CI (docker/metadata-action) overrides source/revision/
# version/created at build time; these are sensible static defaults.
ARG VERSION="0.1.0"
ARG REVISION=""
LABEL org.opencontainers.image.title="ferroehr" \
      org.opencontainers.image.description="Pure-Rust, openEHR-conformant clinical data repository (ITS-REST 1.1.0 + AQL 1.1)" \
      org.opencontainers.image.source="https://github.com/rubentalstra/FerroEHR" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${REVISION}"

# ── runtime-from-source: compose/local (COPY the in-image build) ───────────────
FROM runtime-base AS runtime-from-source
COPY --from=builder /usr/local/bin/ferroehr /usr/local/bin/ferroehr

# ── runtime-prebuilt: CI (COPY the context-provided native binary; no compile) ─
# The `ferroehr` binaries are built natively per architecture on the CI runners
# (inside rust:*-slim-trixie, so they target Debian 13 glibc 2.41 — exactly what
# this distroless base ships) and land in the build context as
# bin/amd64/ferroehr and bin/arm64/ferroehr. Because no RUN executes, a multi-arch
# buildx of this target needs no QEMU.
FROM runtime-base AS runtime-prebuilt
# amd64 | arm64 — supplied by buildx per platform.
ARG TARGETARCH
COPY --chmod=0755 bin/${TARGETARCH}/ferroehr /usr/local/bin/ferroehr
