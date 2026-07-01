//! EHRbase server binary — port target of the `application` Maven module
//! (Spring Boot entry point) plus service layer, AQL engine, rm-db-format,
//! configuration, plugin SPI, and CLI.

fn main() {
    // TODO(port): P17 — config loading, tracing, PG pool, axum router, graceful shutdown.
    eprintln!(
        "openehr-server: Stage 1 port in progress; the server is not wired yet (see docs/PROGRESS.md)"
    );
    std::process::exit(1);
}
