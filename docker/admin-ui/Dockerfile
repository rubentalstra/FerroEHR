# syntax=docker/dockerfile:1
#
# From-source multi-stage build for the `ehrbase-admin-ui` console (the third
# OCI image, next to the app + postgres images). The console is a standalone
# Leptos SSR binary + WASM/CSS assets built by cargo-leptos; at runtime it is
# a pure ITS-REST client of the CDR — no database.
#
#   builder : rust:${RUST_VERSION}-slim-bookworm + wasm32 target +
#             cargo-leptos + the Tailwind v4 standalone binary (no Node/npm).
#   runtime : gcr.io/distroless/cc-debian12:nonroot serving the binary and
#             the built site assets.
#
# The toolchain is single-sourced from rust-toolchain.toml; RUST_VERSION must
# equal its `channel`.

ARG RUST_VERSION=1.96.1
ARG CARGO_LEPTOS_VERSION=0.3.7
ARG TAILWIND_VERSION=v4.3.3

# ── builder ──────────────────────────────────────────────────────────────────
FROM rust:${RUST_VERSION}-slim-bookworm AS builder
ARG CARGO_LEPTOS_VERSION
ARG TAILWIND_VERSION
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
      pkg-config curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    cargo install cargo-leptos --version ${CARGO_LEPTOS_VERSION} --locked
COPY . .
ENV LEPTOS_TAILWIND_VERSION=${TAILWIND_VERSION}
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cd app/ehrbase-admin-ui \
    && cargo leptos build --release \
    && mkdir -p /out \
    && cp /app/target/release/ehrbase-admin-ui /out/ \
    && cp -r /app/target/site /out/site

# ── runtime ──────────────────────────────────────────────────────────────────
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
COPY --from=builder /out/ehrbase-admin-ui /usr/local/bin/ehrbase-admin-ui
COPY --from=builder /out/site /site

ENV LEPTOS_SITE_ROOT=/site \
    LEPTOS_SITE_ADDR=0.0.0.0:3000 \
    LEPTOS_OUTPUT_NAME=ehrbase-admin-ui

EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/ehrbase-admin-ui"]
