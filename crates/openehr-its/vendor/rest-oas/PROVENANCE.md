# Vendored openEHR ITS-REST OpenAPI (OAS)

Source repo: https://github.com/openEHR/specifications-ITS-REST
Pinned commit: `24058992d5fa96e8dfbd855d9c133f328387fc09` (tag Release-1.1.0, released 19-Jul-2026)
Upstream path: `computable/OAS/` · Fetched: 2026-07-20.

All **21** OpenAPI 3.0 bundles are vendored verbatim: **7 API groups** ×
**3 variants**.

- **Groups:** `admin`, `definition`, `demographic`, `ehr`, `overview`,
  `query`, `system`.
- **Variants:**
  - `-codegen.openapi.yaml` — authored **for code generation** → the `emit-rest`
    input (settled: the vendored OAS is the codegen input, never served).
  - `-html.openapi.yaml` — for documentation rendering (Redocly).
  - `-validation.openapi.yaml` — for request/response validation.

These are hand-authored, self-contained bundles (openEHR authors the OAS by
hand and publishes these variants); they are the **source of truth** for the
REST contract. Our generator runs spec→code (OAS → Rust), never code→OAS —
`ehrbase-rest` may additionally emit its own OAS via `utoipa` purely as a CI
drift-check against these files.
