# ADR-018: Crate consolidation — the trait seam is deleted (B+C)

- **Status:** accepted (executed 2026-07-16, W-14 structural wave).
- **Supersedes:** ADR-010/ADR-011's *packaging* (the separate `ehrbase-sm`
  crate and the `Platform` trait catalog). The SM-literal *structure* —
  one service module per SM chapter, SM call semantics as the design
  authority — is unchanged and now lives directly in `ehrbase::service`.

## Context

The three-crate app (`ehrbase` ← `ehrbase-rest` ← `ehrbase-sm`) carried a
33-trait / ~225-method catalog whose impls were forwarding-dominated
(5 hops per operation), a 1.5k-line mock backend, and cross-crate re-export
seams. The owner ruled (2026-07-16): merge the seam away entirely — "no
weird gluing between crates", zero re-exports, proper implementation only.

Spec check (read first-hand, no prior ADR relied on): SM
`master02-overview.adoc` — "This view does not attempt to define a real
product architecture … implementers … may be organised quite differently";
conformance is *testable call semantics*. The spec imposes **zero packaging
requirements**; the ECC suite, not code shape, proves conformance.

## Decision

1. **`ehrbase-sm` is deleted.** All 33 traits (+ `Platform`) removed; the
   ~238 trait-impl methods became inherent methods on `EhrbaseService`,
   organised per SM chapter (`service/<chapter>/`). Cross-interface name
   collisions the trait system had scoped are resolved by SM-chapter-
   qualified names (`extract_ehrs` vs admin `export_ehrs`,
   `*_adl14`/`*_adl2`); trait-era wrapper/inner duplicates merged
   (`*_response` = the envelope-returning service op).
2. **Types moved to their real homes**: call-status/list/version-envelope →
   `service/{status,list,version_update}`, chapter types →
   `service/<chapter>/types`, config data → `ehrbase::config::*` (the whole
   one-TOML tree is platform-owned; the REST crate keeps only behaviour),
   health/build-info/log-reload/provenance/metric names →
   `ehrbase::telemetry`, ATNA event model → `ehrbase::system_log::event`,
   tenant context → `ehrbase::extensions`.
3. **Arrows inverted where they were glue**: `ehrbase-rest → ehrbase`
   (concrete `AppState` over `Arc<EhrbaseService>`, no generics, no dyn);
   the binary moved to the new wiring-only **`ehrbase-server`** crate
   (a package cannot depend on its own dependent). The committer identity
   is a platform-owned task-local the adapter publishes into — the last
   service→REST reference is gone.
4. **Zero re-exports** (owner hard rule): every import names the defining
   module; no shim modules, no root `pub use` of foreign types.
5. **Tests run real**: the scripted Mock died with the traits; the HTTP
   suite runs the shipped `EhrbaseService` over PostgreSQL 18
   testcontainers (one container per test binary, one database per test).

## Consequences

- The REST crate rebuilds when the platform changes (accepted), and pulls
  the platform's dependency tree at build time.
- HTTP tests need Docker and are slower — and stronger: the migration
  immediately exposed a Mock-masked ITS-REST MUST gap (committal-header
  persistence, W-14 F-43).
- The service layer keeps trait-era shapes (`*_response` splits, envelope
  types) — removed by the W-14 service-folder rewrite that follows.
