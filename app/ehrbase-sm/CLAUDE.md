# `ehrbase-sm` — the SM Platform Service Model native API

The protocol-free service seam (ADR-010/011): **one trait per openEHR SM
Platform Service Model interface** (`I_EHR_SERVICE` → `EhrService`, etc. —
full component map in `docs/architecture.md`), plus shared service types
and the `SystemLog` event model.

- **The SM spec is the shape authority:** trait methods mirror the SM
  interface operations literally (a *literal catalog*, not a convenience
  API). Before adding/changing an operation, read the vendored SM text at
  `docs/specs/openehr/SM/docs/openehr_platform/` and cite it.
- **Protocol-free means protocol-free:** no axum/HTTP types, no status
  codes, no headers, no serialization concerns in this crate. Wire mapping
  belongs to `ehrbase-rest`; implementation belongs to `ehrbase`.
  Dependencies point downward only: `ehrbase-sm → crates/openehr-*`.
- SM operations with no ITS-REST binding stay native-API-only and are
  evidenced in ECC as skip-with-reason cases — never invent a wire for
  them ad hoc.
- Gates: `cargo clippy -p ehrbase-sm --all-targets` +
  `cargo nextest run -p ehrbase-sm` green before commit.
