# `ehrbase-server` — the binary

The one bin crate (W-14 B+C consolidation): `main.rs` wires config →
telemetry → PG pool → migrations → ATNA sender → `EhrbaseService` →
`ehrbase_rest::serve_full`, and owns graceful shutdown (audit drain).

- **No logic lives here.** Anything beyond wiring belongs in `ehrbase`
  (platform) or `ehrbase-rest` (protocol adapter). `anyhow` is allowed here
  (binary), `thiserror` everywhere else.
- The bin target is named `ehrbase` (container entrypoints/compose/Helm and
  `scripts/*` invoke that name — do not rename).
- Gates: `cargo clippy -p ehrbase-server --all-targets` +
  `cargo nextest run -p ehrbase-server`.
