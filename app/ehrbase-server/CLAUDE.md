# `ehrbase-server` — the binary

The one bin crate, split per the Book ch12.3: `src/lib.rs` carries ALL the
wiring as the testable `run(Cli)` path — config → telemetry → PG pool →
migrations (`db::run_migrations`) → ATNA audit sender → `EhrbaseService` →
`ehrbase_rest::serve_full`, plus graceful shutdown (the audit queue drains
before exit) and the `healthcheck`/`config` subcommands; `src/main.rs` is a
thin `ehrbase_server::run(Cli::parse())` shell and must stay that way (a
bin-only crate is untestable by construction — Book ch11.3).

- **No logic lives here.** Anything beyond wiring belongs in `ehrbase`
  (platform) or `ehrbase-rest` (protocol adapter). `anyhow` is allowed here
  (the lib target is the binary's own logic half, not a consumable library);
  `thiserror` everywhere else.
- This crate's `tests/` may test only the wiring seam (`run`, CLI parsing,
  config subcommands); tests that exercise `ehrbase` APIs live in
  `app/ehrbase/tests/`.
- The bin target is named `ehrbase` (`[[bin]] name = "ehrbase"`; container
  entrypoints/compose/Helm and `scripts/*` invoke that name — do not rename).
- Gates: `cargo clippy -p ehrbase-server --all-targets` +
  `cargo nextest run -p ehrbase-server`.
