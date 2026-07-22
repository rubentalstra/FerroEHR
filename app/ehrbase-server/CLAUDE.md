# `ehrbase-server` — the binary

The one bin crate. `main.rs` wires config → telemetry → PG pool → migrations
(`db::run_migrations`) → ATNA audit sender → `EhrbaseService` →
`ehrbase_rest::serve_full`, and owns graceful shutdown (the audit queue drains
before exit). Also exposes a `status` subcommand that probes the running
server's status endpoint (exit 0 on 2xx).

- **No logic lives here.** Anything beyond wiring belongs in `ehrbase`
  (platform) or `ehrbase-rest` (protocol adapter). `anyhow` is allowed here
  (binary), `thiserror` everywhere else.
- The bin target is named `ehrbase` (`[[bin]] name = "ehrbase"`; container
  entrypoints/compose/Helm and `scripts/*` invoke that name — do not rename).
- Gates: `cargo clippy -p ehrbase-server --all-targets` +
  `cargo nextest run -p ehrbase-server`.
